// ── ALPACA REST CLIENT ────────────────────────────────────────────────────────
// Places real (or paper) orders via Alpaca's brokerage API.
// Credentials come from env vars:
//   ALPACA_KEY       — APCA-API-KEY-ID
//   ALPACA_SECRET    — APCA-API-SECRET-KEY
//   ALPACA_BASE_URL  — defaults to https://paper-api.alpaca.markets

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_BASE_URL: &str = "https://paper-api.alpaca.markets";

#[derive(Clone)]
pub struct AlpacaClient {
    http:     Client,
    base_url: String,
    key:      String,
    secret:   String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub symbol:        String,
    pub qty:           f64,
    pub side:          String,   // "buy" | "sell"
    pub r#type:        String,   // "market" | "limit"
    pub time_in_force: String,   // "day" | "gtc" | ...
    pub limit_price:   Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id:            String,
    pub client_order_id: Option<String>,
    pub symbol:        String,
    pub qty:           Option<String>,
    pub filled_qty:    Option<String>,
    pub side:          String,
    pub r#type:        String,
    pub time_in_force: String,
    pub status:        String,
    pub filled_avg_price: Option<String>,
    pub limit_price:   Option<String>,
    pub created_at:    Option<String>,
    pub submitted_at:  Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol:        String,
    pub qty:           String,
    pub avg_entry_price: String,
    pub market_value:  Option<String>,
    pub unrealized_pl: Option<String>,
    pub current_price: Option<String>,
    pub side:          String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub cash:           String,
    pub buying_power:   String,
    pub portfolio_value: String,
    pub status:         String,
}

impl AlpacaClient {
    /// Build from env. Returns None if credentials missing — orders just won't work.
    pub fn from_env() -> Option<Self> {
        let key = std::env::var("ALPACA_KEY").ok()?;
        let secret = std::env::var("ALPACA_SECRET").ok()?;
        let base_url = std::env::var("ALPACA_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());

        Some(Self {
            http: Client::builder().timeout(Duration::from_secs(15)).build().ok()?,
            base_url,
            key,
            secret,
        })
    }

    pub fn is_paper(&self) -> bool {
        self.base_url.contains("paper-api")
    }

    fn auth(&self, b: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        b.header("APCA-API-KEY-ID", &self.key)
            .header("APCA-API-SECRET-KEY", &self.secret)
    }

    pub async fn place_order(&self, req: &OrderRequest) -> anyhow::Result<Order> {
        let url = format!("{}/v2/orders", self.base_url);
        let resp = self.auth(self.http.post(&url))
            .json(req)
            .send().await?;

        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("Alpaca place_order [{status}]: {body}");
        }
        Ok(serde_json::from_str(&body)?)
    }

    pub async fn list_orders(&self, status: &str, limit: u32) -> anyhow::Result<Vec<Order>> {
        let url = format!("{}/v2/orders?status={status}&limit={limit}&direction=desc",
                          self.base_url);
        let resp = self.auth(self.http.get(&url)).send().await?;
        let s = resp.status();
        let body = resp.text().await?;
        if !s.is_success() {
            anyhow::bail!("Alpaca list_orders [{s}]: {body}");
        }
        Ok(serde_json::from_str(&body)?)
    }

    pub async fn get_position(&self, symbol: &str) -> anyhow::Result<Option<Position>> {
        let url = format!("{}/v2/positions/{}", self.base_url, symbol);
        let resp = self.auth(self.http.get(&url)).send().await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None); // no open position is normal
        }
        let s = resp.status();
        let body = resp.text().await?;
        if !s.is_success() {
            anyhow::bail!("Alpaca get_position [{s}]: {body}");
        }
        Ok(Some(serde_json::from_str(&body)?))
    }

    pub async fn get_account(&self) -> anyhow::Result<Account> {
        let url = format!("{}/v2/account", self.base_url);
        let resp = self.auth(self.http.get(&url)).send().await?;
        let s = resp.status();
        let body = resp.text().await?;
        if !s.is_success() {
            anyhow::bail!("Alpaca get_account [{s}]: {body}");
        }
        Ok(serde_json::from_str(&body)?)
    }
}
