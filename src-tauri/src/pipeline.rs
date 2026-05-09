// ── PIPELINE ORCHESTRATOR ─────────────────────────────────────────────────────
// Ties Bronze → Silver → Gold together
// Publishes Bronze to Kafka (Flink handles Silver/Gold transforms)
// Also runs local Silver/Gold for immediate UI feedback (no Flink lag)
// Polls Trino for persisted Gold signals
// Writes Gold signal nodes to Neo4j

use std::time::Duration;
use tauri::{AppHandle, Emitter};
use crate::{
    bronze::BronzeProcessor,
    silver::SilverProcessor,
    gold::GoldProcessor,
    kafka::KafkaPublisher,
    trino::TrinoClient,
    neo4j::Neo4jClient,
};

pub async fn run(
    app:    AppHandle,
    symbol: String,
    kafka:  KafkaPublisher,
    trino:  TrinoClient,
    neo4j:  Neo4jClient,
) {
    let mut bronze_proc = BronzeProcessor::new();
    let mut silver_proc = SilverProcessor::new();
    let mut gold_proc   = GoldProcessor::new();

    let app2   = app.clone();
    let trino2 = trino.clone();
    let sym2   = symbol.clone();

    // Task A: fetch → local transform → emit to UI immediately
    //         also publish bronze to Kafka for Flink persistence
    let task_a = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            let bronze = match bronze_proc.fetch(&symbol).await {
                Ok(b) => b,
                Err(e) => { tracing::warn!("Fetch error: {e}"); continue; }
            };

            // Publish to Kafka → Flink picks up for Iceberg persistence
            if let Err(e) = kafka.publish_bronze(&bronze).await {
                tracing::warn!("Kafka publish error: {e}");
            }

            // Local transform for immediate UI (no waiting for Flink)
            let silver = match silver_proc.process(&bronze) {
                Some(s) => s,
                None => continue,
            };

            if let Some(gold) = gold_proc.process(&silver) {
                // Write signal node to Neo4j
                if let Err(e) = neo4j.write_signal(&gold).await {
                    tracing::warn!("Neo4j write error: {e}");
                }

                // Emit to React UI via Tauri IPC
                let _ = app.emit("stock-update", &gold);
                tracing::info!(
                    "{} ${:.2} {} (conf: {:.2})",
                    gold.symbol, gold.price, gold.signal, gold.confidence
                );
            }
        }
    });

    // Task B: poll Trino every 30s to confirm Flink-persisted Gold signals
    //         useful for verifying the full pipeline is working end-to-end
    let task_b = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            match trino2.latest_signal(&sym2).await {
                Ok(Some(gold)) => {
                    tracing::info!(
                        "Trino confirmed: {} {} @ ${:.2}",
                        gold.symbol, gold.signal, gold.price
                    );
                    let _ = app2.emit("trino-confirmed", &gold);
                }
                Ok(None) => tracing::debug!("No persisted signals yet for {}", sym2),
                Err(e)   => tracing::warn!("Trino poll error: {e}"),
            }
        }
    });

    tokio::join!(task_a, task_b);
}