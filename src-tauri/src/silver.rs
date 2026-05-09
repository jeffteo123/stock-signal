// ── SILVER LAYER ─────────────────────────────────────────────────────────────
// Responsibility: validate ticks, compute MA5 / MA20 / RSI

use serde::{Deserialize, Serialize};
use crate::bronze::BronzeTick;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilverTick {
    pub tick_id:   String,
    pub symbol:    String,
    pub price:     f64,
    pub short_ma:  Option<f64>,  // MA5  — None until 5 ticks
    pub long_ma:   Option<f64>,  // MA20 — None until 20 ticks
    pub rsi:       Option<f64>,  // RSI14 — None until 14 ticks
    pub timestamp: u64,
}

pub struct SilverProcessor {
    prices: std::collections::VecDeque<f64>,
}

impl SilverProcessor {
    pub fn new() -> Self {
        Self { prices: std::collections::VecDeque::with_capacity(22) }
    }

    pub fn process(&mut self, bronze: &BronzeTick) -> Option<SilverTick> {
        // Validation gate — filter clearly bad ticks
        if bronze.raw_price <= 0.0 || bronze.raw_price > 1_000_000.0 {
            tracing::warn!("Filtered bad tick: {} @ {}", bronze.symbol, bronze.raw_price);
            return None;
        }

        if self.prices.len() == 22 { self.prices.pop_front(); }
        self.prices.push_back(bronze.raw_price);

        let n = self.prices.len();
        let prices: Vec<f64> = self.prices.iter().copied().collect();

        let short_ma = (n >= 5).then(|| avg(&prices[n-5..]));
        let long_ma  = (n >= 20).then(|| avg(&prices[n-20..]));
        let rsi      = (n >= 15).then(|| compute_rsi(&prices));

        Some(SilverTick {
            tick_id:   bronze.tick_id.clone(),
            symbol:    bronze.symbol.clone(),
            price:     bronze.raw_price,
            short_ma,
            long_ma,
            rsi,
            timestamp: bronze.fetched_at,
        })
    }
}

fn avg(slice: &[f64]) -> f64 {
    slice.iter().sum::<f64>() / slice.len() as f64
}

fn compute_rsi(prices: &[f64]) -> f64 {
    let changes: Vec<f64> = prices.windows(2).map(|w| w[1] - w[0]).collect();
    let last14 = &changes[changes.len().saturating_sub(14)..];
    let gains:  f64 = last14.iter().filter(|&&c| c > 0.0).sum::<f64>() / 14.0;
    let losses: f64 = last14.iter().filter(|&&c| c < 0.0).map(|c| c.abs()).sum::<f64>() / 14.0;
    if losses == 0.0 { return 100.0; }
    100.0 - (100.0 / (1.0 + gains / losses))
}