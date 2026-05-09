// ═══════════════════════════════════════════════════════════════════
// types/stock.ts
// ═══════════════════════════════════════════════════════════════════

export interface GoldSignal {
  tick_id:    string;
  symbol:     string;
  price:      number;
  signal:     "Buy" | "Sell" | "Hold";
  short_ma:   number;
  long_ma:    number;
  rsi:        number | null;
  confidence: number;
  timestamp:  number;
}

export interface CorrelatedStock {
  symbol:      string;
  strength:    number;
  last_signal: "Buy" | "Sell" | "Hold";
}


