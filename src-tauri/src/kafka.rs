// ── KAFKA PRODUCER ────────────────────────────────────────────────────────────
// Publishes BronzeTick JSON to the bronze_ticks topic
// Flink consumes this topic for Silver/Gold transforms

use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::time::Duration;
use crate::bronze::BronzeTick;

pub struct KafkaPublisher {
    producer: FutureProducer,
}

impl KafkaPublisher {
    pub fn new(brokers: &str) -> anyhow::Result<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .set("compression.type", "lz4")
            .set("linger.ms", "100")          // slight batching for throughput
            .set("batch.size", "16384")
            .create()?;

        Ok(Self { producer })
    }

    pub async fn publish_bronze(&self, tick: &BronzeTick) -> anyhow::Result<()> {
        let payload = serde_json::to_string(tick)?;

        self.producer
            .send(
                FutureRecord::to("bronze_ticks")
                    .payload(&payload)
                    .key(&tick.symbol), // partition by symbol — same symbol always same partition
                Duration::from_secs(5),
            )
            .await
            .map_err(|(e, _)| anyhow::anyhow!("Kafka publish failed: {e}"))?;

        tracing::debug!("Published bronze tick: {} @ {}", tick.symbol, tick.raw_price);
        Ok(())
    }
}