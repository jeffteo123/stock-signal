// ── TRINO REST CLIENT ─────────────────────────────────────────────────────────
// Queries Gold signals written by Flink into Iceberg
// Also handles DDL (CREATE TABLE IF NOT EXISTS) on startup

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::gold::GoldSignal;

#[derive(Clone)]
pub struct TrinoClient {
    http:     Client,
    base_url: String,
    user:     String,
    catalog:  String,
    schema:   String,
}

#[derive(Debug, Deserialize)]
struct TrinoResponse {
    id:       String,
    #[serde(rename = "nextUri")]
    next_uri: Option<String>,
    data:     Option<Vec<Vec<serde_json::Value>>>,
    stats:    TrinoStats,
    error:    Option<TrinoError>,
}

#[derive(Debug, Deserialize)]
struct TrinoStats { state: String }

#[derive(Debug, Deserialize)]
struct TrinoError {
    message:    String,
    #[serde(rename = "errorName")]
    error_name: String,
}

impl TrinoClient {
    pub fn new(base_url: &str, user: &str, catalog: &str, schema: &str) -> Self {
        Self {
            http:     Client::builder().timeout(Duration::from_secs(30)).build().unwrap(),
            base_url: base_url.to_string(),
            user:     user.to_string(),
            catalog:  catalog.to_string(),
            schema:   schema.to_string(),
        }
    }

    // Execute SQL, poll until finished, collect all rows
    pub async fn execute(&self, sql: &str) -> anyhow::Result<Vec<Vec<serde_json::Value>>> {
        let resp = self.http
            .post(format!("{}/v1/statement", self.base_url))
            .header("X-Trino-User",    &self.user)
            .header("X-Trino-Catalog", &self.catalog)
            .header("X-Trino-Schema",  &self.schema)
            .body(sql.to_string())
            .send().await?
            .json::<TrinoResponse>().await?;

        if let Some(e) = &resp.error {
            anyhow::bail!("Trino [{}]: {}", e.error_name, e.message);
        }

        let mut all_rows: Vec<Vec<serde_json::Value>> = Vec::new();
        if let Some(rows) = resp.data { all_rows.extend(rows); }

        let mut next_uri = resp.next_uri;
        loop {
            match next_uri {
                None => break,
                Some(uri) => {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    let poll = self.http.get(&uri)
                        .header("X-Trino-User", &self.user)
                        .send().await?
                        .json::<TrinoResponse>().await?;

                    if let Some(e) = &poll.error {
                        anyhow::bail!("Trino [{}]: {}", e.error_name, e.message);
                    }
                    if let Some(rows) = poll.data { all_rows.extend(rows); }

                    match poll.stats.state.as_str() {
                        "FINISHED" => break,
                        "FAILED"   => anyhow::bail!("Trino query failed: {}", poll.id),
                        _          => next_uri = poll.next_uri,
                    }
                }
            }
        }

        Ok(all_rows)
    }

    // Create Iceberg tables if they don't exist
    pub async fn setup_tables(&self) -> anyhow::Result<()> {
        self.execute(r#"
            CREATE TABLE IF NOT EXISTS iceberg.stocks.bronze_ticks (
                tick_id    VARCHAR,
                symbol     VARCHAR,
                raw_price  DOUBLE,
                fetched_at BIGINT,
                source     VARCHAR
            ) WITH (format = 'PARQUET', partitioning = ARRAY['symbol'])
        "#).await?;

        self.execute(r#"
            CREATE TABLE IF NOT EXISTS iceberg.stocks.silver_ticks (
                tick_id   VARCHAR,
                symbol    VARCHAR,
                price     DOUBLE,
                short_ma  DOUBLE,
                long_ma   DOUBLE,
                rsi       DOUBLE,
                timestamp BIGINT
            ) WITH (format = 'PARQUET', partitioning = ARRAY['symbol'])
        "#).await?;

        self.execute(r#"
            CREATE TABLE IF NOT EXISTS iceberg.stocks.gold_signals (
                tick_id    VARCHAR,
                symbol     VARCHAR,
                price      DOUBLE,
                signal     VARCHAR,
                short_ma   DOUBLE,
                long_ma    DOUBLE,
                rsi        DOUBLE,
                confidence DOUBLE,
                timestamp  BIGINT
            ) WITH (format = 'PARQUET', partitioning = ARRAY['symbol', 'signal'])
        "#).await?;

        tracing::info!("Iceberg tables ready");
        Ok(())
    }

    // Fetch the latest Gold signal for a symbol
    pub async fn latest_signal(&self, symbol: &str) -> anyhow::Result<Option<GoldSignal>> {
        let sql = format!(
            "SELECT tick_id, symbol, price, signal, short_ma, long_ma, rsi, confidence, timestamp
             FROM iceberg.stocks.gold_signals
             WHERE symbol = '{}'
             ORDER BY timestamp DESC LIMIT 1",
            escape(symbol)
        );

        let rows = self.execute(&sql).await?;
        Ok(rows.first().map(|r| GoldSignal {
            tick_id:    r[0].as_str().unwrap_or("").to_string(),
            symbol:     r[1].as_str().unwrap_or("").to_string(),
            price:      r[2].as_f64().unwrap_or(0.0),
            signal:     r[3].as_str().unwrap_or("Hold").to_string(),
            short_ma:   r[4].as_f64().unwrap_or(0.0),
            long_ma:    r[5].as_f64().unwrap_or(0.0),
            rsi:        r[6].as_f64(),
            confidence: r[7].as_f64().unwrap_or(0.0),
            timestamp:  r[8].as_u64().unwrap_or(0),
        }))
    }
}

fn escape(s: &str) -> String { s.replace('\'', "''") }