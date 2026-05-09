// ── BRONZE LAYER ─────────────────────────────────────────────────────────────
// Responsibility: fetch raw price from Yahoo Finance, no transformation

use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BronzeTick {
    pub tick_id:    String,
    pub symbol:     String,
    pub raw_price:  f64,
    pub fetched_at: u64,
    pub source:     String,
}

pub struct BronzeProcessor {
    http: Client,
}

impl BronzeProcessor {
    pub fn new() -> Self {
        Self { http: Client::new() }
    }

    pub async fn fetch(&self, symbol: &str) -> anyhow::Result<BronzeTick> {
        let price = fetch_yahoo(&self.http, symbol).await?;
        Ok(BronzeTick {
            tick_id:    uuid::Uuid::new_v4().to_string(),
            symbol:     symbol.to_string(),
            raw_price:  price,
            fetched_at: now_secs(),
            source:     "yahoo_v10".to_string(),
        })
    }
}

// Yahoo Finance unofficial REST API
async fn fetch_yahoo(client: &Client, symbol: &str) -> anyhow::Result<f64> {
    #[derive(Deserialize)] struct R  { #[serde(rename="quoteSummary")] qs: QS }
    #[derive(Deserialize)] struct QS { result: Vec<QR> }
    #[derive(Deserialize)] struct QR { price: QP }
    #[derive(Deserialize)] struct QP { #[serde(rename="regularMarketPrice")] rmp: RV }
    #[derive(Deserialize)] struct RV { raw: f64 }

    let url = format!(
        "https://query1.finance.yahoo.com/v10/finance/quoteSummary/{}?modules=price",
        symbol
    );
    let r: R = client.get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send().await?
        .json().await?;

    Ok(r.qs.result[0].price.rmp.raw)
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}