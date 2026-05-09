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

export type OrderSide = "buy" | "sell";

export interface Order {
  id:                 string;
  client_order_id:    string | null;
  symbol:             string;
  qty:                string | null;
  filled_qty:         string | null;
  side:               OrderSide;
  type:               string;
  time_in_force:      string;
  status:             string;
  filled_avg_price:   string | null;
  limit_price:        string | null;
  created_at:         string | null;
  submitted_at:       string | null;
}

export interface Position {
  symbol:           string;
  qty:              string;
  avg_entry_price:  string;
  market_value:     string | null;
  unrealized_pl:    string | null;
  current_price:    string | null;
  side:             string;
}

export interface Account {
  cash:             string;
  buying_power:     string;
  portfolio_value:  string;
  status:           string;
}

export interface AlpacaStatus {
  configured: boolean;
  paper:      boolean;
}
