// ═══════════════════════════════════════════════════════════════════
// commands.rs — Tauri commands callable from React via invoke()
// ═══════════════════════════════════════════════════════════════════

use tauri::State;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{
    gold::GoldSignal,
    neo4j::{Neo4jClient, CorrelatedStock},
};

pub type LatestSignal = Arc<RwLock<Option<GoldSignal>>>;

// invoke("get_latest") — returns most recent signal snapshot
#[tauri::command]
pub async fn get_latest(
    latest: State<'_, LatestSignal>,
) -> Result<Option<GoldSignal>, String> {
    Ok(latest.read().await.clone())
}

// invoke("get_correlated", { symbol, minStrength })
#[tauri::command]
pub async fn get_correlated(
    symbol:      String,
    min_strength: f64,
    neo4j:       State<'_, Neo4jClient>,
) -> Result<Vec<CorrelatedStock>, String> {
    neo4j.correlated_stocks(&symbol, min_strength)
        .await
        .map_err(|e| e.to_string())
}

// invoke("record_action", { userId, tickId, action })
// Call when user taps Buy/Sell — feeds the recommendation graph
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

// invoke("get_recommendations", { userId })
#[tauri::command]
pub async fn get_recommendations(
    user_id: String,
    neo4j:   State<'_, Neo4jClient>,
) -> Result<Vec<String>, String> {
    neo4j.recommend_symbols(&user_id)
        .await
        .map_err(|e| e.to_string())
}

