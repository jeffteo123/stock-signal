// ═══════════════════════════════════════════════════════════════════
// commands.rs — Tauri commands callable from React via invoke()
// ═══════════════════════════════════════════════════════════════════

use tauri::State;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{
    gold::GoldSignal,
    neo4j::{Neo4jClient, CorrelatedStock},
    alpaca::{AlpacaClient, OrderRequest, Order, Position, Account},
    pipeline::ActiveSymbol,
};

pub type LatestSignal = Arc<RwLock<Option<GoldSignal>>>;

// Optional Alpaca — None when ALPACA_KEY/ALPACA_SECRET not set
pub type AlpacaState = Arc<Option<AlpacaClient>>;

// ── Signal queries ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_latest(
    latest: State<'_, LatestSignal>,
) -> Result<Option<GoldSignal>, String> {
    Ok(latest.read().await.clone())
}

#[tauri::command]
pub async fn get_active_symbol(
    active: State<'_, ActiveSymbol>,
) -> Result<String, String> {
    Ok(active.read().await.clone())
}

/// Switch the symbol the pipeline is fetching. Clears the cached "latest"
/// so the UI doesn't display stale prices for the old ticker.
#[tauri::command]
pub async fn set_symbol(
    symbol: String,
    active: State<'_, ActiveSymbol>,
    latest: State<'_, LatestSignal>,
) -> Result<(), String> {
    let symbol = symbol.trim().to_uppercase();
    if symbol.is_empty() || symbol.len() > 10 {
        return Err("symbol must be 1-10 chars".into());
    }
    *active.write().await = symbol;
    *latest.write().await = None;
    Ok(())
}

// ── Neo4j graph ──────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_correlated(
    symbol:       String,
    min_strength: f64,
    neo4j:        State<'_, Neo4jClient>,
) -> Result<Vec<CorrelatedStock>, String> {
    neo4j.correlated_stocks(&symbol, min_strength)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn record_action(
    user_id: String,
    tick_id: String,
    action:  String,
    neo4j:   State<'_, Neo4jClient>,
) -> Result<(), String> {
    neo4j.record_user_action(&user_id, &tick_id, &action)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_recommendations(
    user_id: String,
    neo4j:   State<'_, Neo4jClient>,
) -> Result<Vec<String>, String> {
    neo4j.recommend_symbols(&user_id)
        .await
        .map_err(|e| e.to_string())
}

// ── Alpaca trading ───────────────────────────────────────────────────────────

fn get_alpaca<'a>(state: &'a State<'_, AlpacaState>) -> Result<&'a AlpacaClient, String> {
    state
        .inner()    // &Arc<Option<AlpacaClient>>
        .as_ref()   // &Option<AlpacaClient>  (via Arc::as_ref)
        .as_ref()   // Option<&AlpacaClient>  (via Option::as_ref)
        .ok_or_else(|| "Alpaca not configured. Set ALPACA_KEY and ALPACA_SECRET env vars.".into())
}

#[tauri::command]
pub async fn place_order(
    symbol: String,
    qty:    f64,
    side:   String,                 // "buy" | "sell"
    alpaca: State<'_, AlpacaState>,
) -> Result<Order, String> {
    let side = side.to_lowercase();
    if side != "buy" && side != "sell" {
        return Err("side must be 'buy' or 'sell'".into());
    }
    if !(qty > 0.0) {
        return Err("qty must be > 0".into());
    }

    let req = OrderRequest {
        symbol:        symbol.to_uppercase(),
        qty,
        side,
        r#type:        "market".into(),
        time_in_force: "day".into(),
        limit_price:   None,
    };

    get_alpaca(&alpaca)?
        .place_order(&req).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_orders(
    status: Option<String>,
    limit:  Option<u32>,
    alpaca: State<'_, AlpacaState>,
) -> Result<Vec<Order>, String> {
    let status = status.unwrap_or_else(|| "all".into());
    let limit  = limit.unwrap_or(20);
    get_alpaca(&alpaca)?
        .list_orders(&status, limit).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_position(
    symbol: String,
    alpaca: State<'_, AlpacaState>,
) -> Result<Option<Position>, String> {
    get_alpaca(&alpaca)?
        .get_position(&symbol.to_uppercase()).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_account(
    alpaca: State<'_, AlpacaState>,
) -> Result<Account, String> {
    get_alpaca(&alpaca)?
        .get_account().await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn alpaca_status(
    alpaca: State<'_, AlpacaState>,
) -> Result<serde_json::Value, String> {
    Ok(match alpaca.inner().as_ref() {
        Some(c) => serde_json::json!({ "configured": true, "paper": c.is_paper() }),
        None    => serde_json::json!({ "configured": false, "paper": true }),
    })
}
