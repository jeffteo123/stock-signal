// ── PIPELINE ORCHESTRATOR ─────────────────────────────────────────────────────
// Ties Bronze → Silver → Gold together
// Publishes Bronze to Kafka (Flink handles Silver/Gold transforms)
// Also runs local Silver/Gold for immediate UI feedback (no Flink lag)
// Polls Trino for persisted Gold signals
// Writes Gold signal nodes to Neo4j
//
// The active symbol is held behind a tokio RwLock so the UI can switch
// tickers at runtime via the `set_symbol` Tauri command. When the symbol
// changes, the local Silver/Gold processors are reset so price history
// from the previous ticker doesn't pollute MA/RSI windows.

use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use crate::{
    bronze::BronzeProcessor,
    silver::SilverProcessor,
    gold::GoldProcessor,
    kafka::KafkaPublisher,
    trino::TrinoClient,
    neo4j::Neo4jClient,
    commands::LatestSignal,
};

pub type ActiveSymbol = Arc<RwLock<String>>;

pub async fn run(
    app:           AppHandle,
    active_symbol: ActiveSymbol,
    latest:        LatestSignal,
    kafka:         KafkaPublisher,
    trino:         TrinoClient,
    neo4j:         Neo4jClient,
) {
    let mut bronze_proc = BronzeProcessor::new();
    let mut silver_proc = SilverProcessor::new();
    let mut gold_proc   = GoldProcessor::new();
    let mut last_symbol: Option<String> = None;

    let app2     = app.clone();
    let trino2   = trino.clone();
    let active_b = active_symbol.clone();

    // Task A: fetch → local transform → emit to UI immediately
    //         also publish bronze to Kafka for Flink persistence
    let task_a = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            let symbol = active_symbol.read().await.clone();

            // If user switched tickers, reset windows so MAs/RSI restart cleanly
            if last_symbol.as_deref() != Some(symbol.as_str()) {
                silver_proc = SilverProcessor::new();
                gold_proc   = GoldProcessor::new();
                last_symbol = Some(symbol.clone());
                tracing::info!("Pipeline now tracking {}", symbol);
            }

            let bronze = match bronze_proc.fetch(&symbol).await {
                Ok(b) => b,
                Err(e) => { tracing::warn!("Fetch error for {symbol}: {e}"); continue; }
            };

            if let Err(e) = kafka.publish_bronze(&bronze).await {
                tracing::warn!("Kafka publish error: {e}");
            }

            let silver = match silver_proc.process(&bronze) {
                Some(s) => s,
                None => continue,
            };

            if let Some(gold) = gold_proc.process(&silver) {
                if let Err(e) = neo4j.write_signal(&gold).await {
                    tracing::warn!("Neo4j write error: {e}");
                }

                *latest.write().await = Some(gold.clone());

                let _ = app.emit("stock-update", &gold);
                tracing::info!(
                    "{} ${:.2} {} (conf: {:.2})",
                    gold.symbol, gold.price, gold.signal, gold.confidence
                );
            }
        }
    });

    // Task B: poll Trino every 30s to confirm Flink-persisted Gold signals
    let task_b = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let symbol = active_b.read().await.clone();
            match trino2.latest_signal(&symbol).await {
                Ok(Some(gold)) => {
                    tracing::info!(
                        "Trino confirmed: {} {} @ ${:.2}",
                        gold.symbol, gold.signal, gold.price
                    );
                    let _ = app2.emit("trino-confirmed", &gold);
                }
                Ok(None) => tracing::debug!("No persisted signals yet for {}", symbol),
                Err(e)   => tracing::warn!("Trino poll error: {e}"),
            }
        }
    });

    let _ = tokio::join!(task_a, task_b);
}
