// ── GOLD LAYER ────────────────────────────────────────────────────────────────
// Responsibility: generate Buy/Sell/Hold signals from Silver ticks
// Uses MA crossover + RSI confirmation

use serde::{Deserialize, Serialize};
use crate::silver::SilverTick;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldSignal {
    pub tick_id:    String,
    pub symbol:     String,
    pub price:      f64,
    pub signal:     String,     // "Buy" | "Sell" | "Hold"
    pub short_ma:   f64,
    pub long_ma:    f64,
    pub rsi:        Option<f64>,
    pub confidence: f64,        // 0.0–1.0, distance between MAs normalised
    pub timestamp:  u64,
}

pub struct GoldProcessor {
    prev_above: Option<bool>,
}

impl GoldProcessor {
    pub fn new() -> Self {
        Self { prev_above: None }
    }

    pub fn process(&mut self, silver: &SilverTick) -> Option<GoldSignal> {
        let (short_ma, long_ma) = match (silver.short_ma, silver.long_ma) {
            (Some(s), Some(l)) => (s, l),
            _ => return None, // not enough history yet
        };

        let above = short_ma > long_ma;

        // RSI confirmation:
        //   Buy  only if RSI < 70 (not overbought)
        //   Sell only if RSI > 30 (not oversold)
        let rsi_ok = match silver.rsi {
            Some(rsi) => match self.prev_above {
                Some(prev) if !prev && above => rsi < 70.0, // Buy confirm
                Some(prev) if prev && !above => rsi > 30.0, // Sell confirm
                _ => true,
            },
            None => true, // no RSI yet, allow signal through
        };

        let signal = match self.prev_above {
            Some(prev) if prev != above && rsi_ok => {
                if above { "Buy" } else { "Sell" }
            }
            _ => "Hold",
        };

        self.prev_above = Some(above);

        // Confidence = how far apart the MAs are (capped at 1.0)
        let confidence = ((short_ma - long_ma).abs() / long_ma * 100.0).min(1.0);

        Some(GoldSignal {
            tick_id:    silver.tick_id.clone(),
            symbol:     silver.symbol.clone(),
            price:      silver.price,
            signal:     signal.to_string(),
            short_ma,
            long_ma,
            rsi:        silver.rsi,
            confidence,
            timestamp:  silver.timestamp,
        })
    }
}