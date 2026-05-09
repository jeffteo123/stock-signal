// ═══════════════════════════════════════════════════════════════════
// lib.rs — Tauri app entrypoint
// Wires Bronze→Silver→Gold pipeline + Trino + Neo4j + Alpaca,
// exposes commands to React.
// ═══════════════════════════════════════════════════════════════════

mod alpaca;
mod bronze;
mod commands;
mod gold;
mod kafka;
mod neo4j;
mod pipeline;
mod silver;
mod trino;

use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

use alpaca::AlpacaClient;
use commands::{AlpacaState, LatestSignal};
use kafka::KafkaPublisher;
use neo4j::Neo4jClient;
use pipeline::ActiveSymbol;
use trino::TrinoClient;

const DEFAULT_SYMBOL: &str = "AAPL";

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let kafka_brokers = env_or("KAFKA_BROKERS", "localhost:9092");
    let trino_url     = env_or("TRINO_URL",     "http://localhost:8080");
    let trino_user    = env_or("TRINO_USER",    "stock-signal");
    let trino_catalog = env_or("TRINO_CATALOG", "iceberg");
    let trino_schema  = env_or("TRINO_SCHEMA",  "stocks");
    let neo4j_uri     = env_or("NEO4J_URI",     "bolt://localhost:7687");
    let neo4j_user    = env_or("NEO4J_USER",    "neo4j");
    let neo4j_pass    = env_or("NEO4J_PASS",    "password123");

    let initial_symbol = env_or("DEFAULT_SYMBOL", DEFAULT_SYMBOL).to_uppercase();

    tauri::Builder::default()
        .setup(move |app| {
            let handle = app.handle().clone();

            let active_symbol: ActiveSymbol =
                Arc::new(RwLock::new(initial_symbol.clone()));
            let latest: LatestSignal = Arc::new(RwLock::new(None));
            let alpaca_state: AlpacaState = Arc::new(AlpacaClient::from_env());

            match alpaca_state.as_ref() {
                Some(c) => tracing::info!(
                    "Alpaca configured ({})",
                    if c.is_paper() { "paper" } else { "live" }
                ),
                None => tracing::warn!(
                    "Alpaca not configured — set ALPACA_KEY/ALPACA_SECRET to enable orders"
                ),
            }

            app.manage(active_symbol.clone());
            app.manage(latest.clone());
            app.manage(alpaca_state);

            // Build infra clients & spawn pipeline. We move this into an async
            // task so the UI can come up even if a backend is briefly slow.
            tauri::async_runtime::spawn(async move {
                let kafka = match KafkaPublisher::new(&kafka_brokers) {
                    Ok(k) => k,
                    Err(e) => { tracing::error!("Kafka init failed: {e}"); return; }
                };

                let trino = TrinoClient::new(&trino_url, &trino_user, &trino_catalog, &trino_schema);
                if let Err(e) = trino.setup_tables().await {
                    tracing::error!("Trino setup failed: {e}");
                }

                let neo4j = match Neo4jClient::new(&neo4j_uri, &neo4j_user, &neo4j_pass).await {
                    Ok(n) => n,
                    Err(e) => { tracing::error!("Neo4j connect failed: {e}"); return; }
                };
                if let Err(e) = neo4j.setup_constraints().await {
                    tracing::error!("Neo4j setup failed: {e}");
                }

                handle.manage(neo4j.clone());

                pipeline::run(handle, active_symbol, latest, kafka, trino, neo4j).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_latest,
            commands::get_active_symbol,
            commands::set_symbol,
            commands::get_correlated,
            commands::record_action,
            commands::get_recommendations,
            commands::place_order,
            commands::list_orders,
            commands::get_position,
            commands::get_account,
            commands::alpaca_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
