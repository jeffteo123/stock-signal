import { useState, useMemo } from "react";
import { useStockStream } from "./hooks/useStockStream";
import type { GoldSignal, CorrelatedStock, Order, Position, AlpacaStatus } from "./types/stock";

const SIGNAL_COLORS: Record<string, string> = { Buy: "#22c55e", Sell: "#ef4444", Hold: "#f59e0b" };
const SIGNAL_BG:     Record<string, string> = { Buy: "#052e16", Sell: "#450a0a", Hold: "#422006" };
const MAX_HISTORY = 40;
const PRESETS = ["AAPL", "MSFT", "GOOGL", "NVDA", "TSLA", "AMZN"];

// ── Components ────────────────────────────────────────────────────

function SignalBadge({ signal, size = "md" }: { signal: string; size?: "sm" | "md" }) {
  const icons: Record<string, string> = { Buy: "▲", Sell: "▼", Hold: "●" };
  const pad = size === "sm" ? "2px 10px" : "4px 14px";
  const fs  = size === "sm" ? 12 : 14;
  return (
    <span style={{
      background: SIGNAL_BG[signal],
      color: SIGNAL_COLORS[signal],
      border: `1px solid ${SIGNAL_COLORS[signal]}40`,
      borderRadius: 8, padding: pad,
      fontWeight: 700, fontSize: fs, letterSpacing: 1,
    }}>
      {icons[signal]} {signal}
    </span>
  );
}

function PriceChart({ history }: { history: GoldSignal[] }) {
  if (history.length < 2) return (
    <div style={{ height: 100, display:"flex", alignItems:"center", justifyContent:"center", color:"#475569", fontSize:12 }}>
      Collecting data...
    </div>
  );

  const W = 340, H = 100;
  const prices   = history.map(h => h.price);
  const shortMAs = history.map(h => h.short_ma);
  const longMAs  = history.map(h => h.long_ma);
  const min = Math.min(...prices, ...shortMAs, ...longMAs) - 0.3;
  const max = Math.max(...prices, ...shortMAs, ...longMAs) + 0.3;

  const x = (i: number) => (i / (MAX_HISTORY - 1)) * W;
  const y = (p: number) => H - ((p - min) / (max - min)) * H;

  const path = (arr: number[]) => arr.map((p, i) => `${i === 0 ? "M" : "L"}${x(i)},${y(p)}`).join(" ");
  const signals = history.map((h, i) => ({ ...h, i })).filter(h => h.signal !== "Hold");

  return (
    <svg width={W} height={H} style={{ display:"block", overflow:"visible" }}>
      {[0.25, 0.5, 0.75].map(t => (
        <line key={t} x1={0} y1={H * t} x2={W} y2={H * t} stroke="#ffffff08" strokeWidth={1}/>
      ))}
      <path d={path(longMAs)}  fill="none" stroke="#f59e0b" strokeWidth={1.5} opacity={0.5}/>
      <path d={path(shortMAs)} fill="none" stroke="#60a5fa" strokeWidth={1.5} opacity={0.7}/>
      <path d={path(prices)}   fill="none" stroke="#e2e8f0" strokeWidth={2}/>
      {signals.map((s, i) => (
        <circle key={i} cx={x(s.i)} cy={y(s.price)} r={5}
          fill={SIGNAL_COLORS[s.signal]} stroke="#0f172a" strokeWidth={1.5}/>
      ))}
    </svg>
  );
}

function CorrelatedCard({ stocks, onPick }: { stocks: CorrelatedStock[]; onPick: (s: string) => void }) {
  if (!stocks.length) return (
    <div style={{ background:"#1e293b", borderRadius:16, padding:16, marginBottom:16, color:"#475569", fontSize:12 }}>
      No correlated stocks yet — needs Neo4j data.
    </div>
  );
  return (
    <div style={{ background:"#1e293b", borderRadius:16, padding:16, marginBottom:16 }}>
      <div style={{ fontSize:11, color:"#64748b", marginBottom:12, letterSpacing:1, textTransform:"uppercase" }}>
        Correlated Stocks (tap to switch)
      </div>
      {stocks.map(s => (
        <div key={s.symbol}
          onClick={() => onPick(s.symbol)}
          style={{
            display:"flex", justifyContent:"space-between", alignItems:"center",
            padding:"8px 0", borderBottom:"1px solid #ffffff08", cursor:"pointer",
          }}>
          <span style={{ fontWeight:600, fontSize:14 }}>{s.symbol}</span>
          <div style={{ display:"flex", gap:8, alignItems:"center" }}>
            <span style={{ fontSize:11, color:"#64748b" }}>
              {(s.strength * 100).toFixed(0)}% corr
            </span>
            <SignalBadge signal={s.last_signal} size="sm"/>
          </div>
        </div>
      ))}
    </div>
  );
}

function SymbolPicker({ symbol, onChange }: { symbol: string; onChange: (s: string) => void }) {
  const [draft, setDraft] = useState(symbol);

  const submit = () => {
    const s = draft.trim().toUpperCase();
    if (s && s !== symbol) onChange(s);
  };

  return (
    <div style={{ background:"#1e293b", borderRadius:16, padding:14, marginBottom:16 }}>
      <div style={{ fontSize:11, color:"#64748b", marginBottom:8, letterSpacing:1, textTransform:"uppercase" }}>
        Ticker
      </div>
      <div style={{ display:"flex", gap:8, marginBottom:10 }}>
        <input
          value={draft}
          onChange={e => setDraft(e.target.value.toUpperCase())}
          onKeyDown={e => { if (e.key === "Enter") submit(); }}
          placeholder="e.g. AAPL"
          maxLength={10}
          style={{
            flex:1, background:"#0f172a", border:"1px solid #334155",
            color:"#e2e8f0", borderRadius:10, padding:"8px 12px",
            fontSize:14, fontWeight:600, letterSpacing:1, outline:"none",
          }}
        />
        <button onClick={submit} style={{
          padding:"0 16px", borderRadius:10, border:"none",
          background:"#334155", color:"#e2e8f0",
          fontWeight:600, fontSize:13, cursor:"pointer",
        }}>
          Switch
        </button>
      </div>
      <div style={{ display:"flex", flexWrap:"wrap", gap:6 }}>
        {PRESETS.map(p => (
          <button key={p} onClick={() => { setDraft(p); onChange(p); }} style={{
            padding:"4px 10px", borderRadius:8, border:"1px solid #334155",
            background: p === symbol ? "#334155" : "transparent",
            color: p === symbol ? "#e2e8f0" : "#94a3b8",
            fontSize:11, fontWeight:600, cursor:"pointer",
          }}>
            {p}
          </button>
        ))}
      </div>
    </div>
  );
}

function OrderPanel({
  symbol, price, alpacaStatus, position, orders, onPlace,
}: {
  symbol: string;
  price: number | undefined;
  alpacaStatus: AlpacaStatus | null;
  position: Position | null;
  orders: Order[];
  onPlace: (side: "buy" | "sell", qty: number) => Promise<unknown>;
}) {
  const [qty, setQty] = useState("1");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  if (!alpacaStatus?.configured) {
    return (
      <div style={{ background:"#1e293b", borderRadius:16, padding:16, marginBottom:16, fontSize:12, color:"#94a3b8" }}>
        Alpaca not configured. Set <code style={{color:"#f59e0b"}}>ALPACA_KEY</code> and{" "}
        <code style={{color:"#f59e0b"}}>ALPACA_SECRET</code> env vars to enable orders.
      </div>
    );
  }

  const submit = async (side: "buy" | "sell") => {
    const n = parseFloat(qty);
    if (!(n > 0)) { setErr("Quantity must be > 0"); return; }
    setBusy(true); setErr(null);
    try {
      await onPlace(side, n);
    } catch (e: any) {
      setErr(typeof e === "string" ? e : (e?.message ?? "Order failed"));
    } finally {
      setBusy(false);
    }
  };

  const estimatedCost = price ? price * (parseFloat(qty) || 0) : 0;

  return (
    <div style={{ background:"#1e293b", borderRadius:16, padding:16, marginBottom:16 }}>
      <div style={{ display:"flex", justifyContent:"space-between", alignItems:"center", marginBottom:10 }}>
        <div style={{ fontSize:11, color:"#64748b", letterSpacing:1, textTransform:"uppercase" }}>
          Place Order
        </div>
        <span style={{
          fontSize:10, padding:"2px 8px", borderRadius:6,
          background: alpacaStatus.paper ? "#1e3a8a" : "#7f1d1d",
          color: alpacaStatus.paper ? "#93c5fd" : "#fecaca",
          fontWeight:700, letterSpacing:1,
        }}>
          {alpacaStatus.paper ? "PAPER" : "LIVE"}
        </span>
      </div>

      {position && (
        <div style={{ fontSize:12, color:"#94a3b8", marginBottom:10, display:"flex", justifyContent:"space-between" }}>
          <span>Position: {position.qty} @ ${parseFloat(position.avg_entry_price).toFixed(2)}</span>
          {position.unrealized_pl && (
            <span style={{ color: parseFloat(position.unrealized_pl) >= 0 ? "#22c55e" : "#ef4444" }}>
              {parseFloat(position.unrealized_pl) >= 0 ? "+" : ""}${parseFloat(position.unrealized_pl).toFixed(2)}
            </span>
          )}
        </div>
      )}

      <div style={{ display:"flex", gap:8, alignItems:"center", marginBottom:10 }}>
        <label style={{ fontSize:12, color:"#94a3b8" }}>Qty</label>
        <input
          type="number" min="0" step="any" value={qty}
          onChange={e => setQty(e.target.value)}
          style={{
            flex:1, background:"#0f172a", border:"1px solid #334155",
            color:"#e2e8f0", borderRadius:8, padding:"6px 10px",
            fontSize:14, outline:"none",
          }}
        />
        {price !== undefined && (
          <span style={{ fontSize:11, color:"#64748b" }}>
            ≈ ${estimatedCost.toFixed(2)}
          </span>
        )}
      </div>

      <div style={{ display:"flex", gap:10 }}>
        <button onClick={() => submit("buy")} disabled={busy} style={{
          flex:1, padding:12, borderRadius:10,
          background:"#052e16", color:"#22c55e",
          fontWeight:700, fontSize:14, cursor: busy ? "wait" : "pointer",
          border:"1px solid #22c55e40", opacity: busy ? 0.6 : 1,
        }}>
          ▲ Buy {symbol}
        </button>
        <button onClick={() => submit("sell")} disabled={busy} style={{
          flex:1, padding:12, borderRadius:10,
          background:"#450a0a", color:"#ef4444",
          fontWeight:700, fontSize:14, cursor: busy ? "wait" : "pointer",
          border:"1px solid #ef444440", opacity: busy ? 0.6 : 1,
        }}>
          ▼ Sell {symbol}
        </button>
      </div>

      {err && (
        <div style={{ marginTop:10, fontSize:12, color:"#ef4444" }}>{err}</div>
      )}

      {orders.length > 0 && (
        <>
          <div style={{ fontSize:11, color:"#64748b", marginTop:14, marginBottom:8, letterSpacing:1, textTransform:"uppercase" }}>
            Recent Orders
          </div>
          {orders.slice(0, 5).map(o => (
            <div key={o.id} style={{
              display:"flex", justifyContent:"space-between", alignItems:"center",
              padding:"6px 0", borderBottom:"1px solid #ffffff08", fontSize:12,
            }}>
              <span style={{ color: o.side === "buy" ? "#22c55e" : "#ef4444", fontWeight:600 }}>
                {o.side.toUpperCase()} {o.qty} {o.symbol}
              </span>
              <span style={{ color:"#94a3b8" }}>{o.status}</span>
            </div>
          ))}
        </>
      )}
    </div>
  );
}

// ── Main App ──────────────────────────────────────────────────────

export default function App() {
  const [symbol, setSymbol] = useState("AAPL");
  const {
    update, history, correlated, trinoStatus,
    alpacaStatus, position, orders, placeOrder,
  } = useStockStream(symbol);
  const [tab, setTab] = useState("chart");

  const priceChange = history.length > 1
    ? (update?.price ?? 0) - history[0].price : 0;

  const signalCounts = useMemo(() => history.reduce((acc, h) => {
    acc[h.signal] = (acc[h.signal] || 0) + 1; return acc;
  }, {} as Record<string, number>), [history]);

  return (
    <div style={{
      background:"#0f172a", minHeight:"100vh",
      color:"#e2e8f0", fontFamily:"'Inter',system-ui,sans-serif",
      padding:"24px 16px", maxWidth:400, margin:"0 auto",
    }}>
      {/* Header */}
      <div style={{ display:"flex", justifyContent:"space-between", alignItems:"center", marginBottom:20 }}>
        <div>
          <div style={{ fontSize:11, color:"#64748b", letterSpacing:2, textTransform:"uppercase" }}>
            Stock Signal — Medallion
          </div>
          <div style={{ fontSize:22, fontWeight:700 }}>{symbol}</div>
        </div>
        <div style={{ textAlign:"right" }}>
          <div style={{ width:8, height:8, borderRadius:"50%", background:"#22c55e",
            boxShadow:"0 0 8px #22c55e", display:"inline-block",
            animation:"pulse 1.5s infinite" }}/>
          <div style={{ fontSize:10, color:"#475569", marginTop:4 }}>LIVE</div>
        </div>
      </div>

      <SymbolPicker symbol={symbol} onChange={setSymbol}/>

      {/* Price card */}
      {update ? (
        <div style={{ background:"#1e293b", borderRadius:16, padding:20, marginBottom:16 }}>
          <div style={{ display:"flex", justifyContent:"space-between", alignItems:"flex-start" }}>
            <div>
              <div style={{ fontSize:36, fontWeight:800, letterSpacing:-1 }}>
                ${update.price.toFixed(2)}
              </div>
              <div style={{ fontSize:13, color: priceChange >= 0 ? "#22c55e" : "#ef4444", marginTop:2 }}>
                {priceChange >= 0 ? "▲" : "▼"} {Math.abs(priceChange).toFixed(2)} session
              </div>
            </div>
            <SignalBadge signal={update.signal}/>
          </div>

          <div style={{ display:"flex", gap:16, marginTop:14, fontSize:12 }}>
            <span style={{ color:"#60a5fa" }}>MA5: {update.short_ma.toFixed(2)}</span>
            <span style={{ color:"#f59e0b" }}>MA20: {update.long_ma.toFixed(2)}</span>
            {update.rsi !== null && (
              <span style={{ color: update.rsi > 70 ? "#ef4444" : update.rsi < 30 ? "#22c55e" : "#94a3b8" }}>
                RSI: {update.rsi.toFixed(1)}
              </span>
            )}
          </div>

          <div style={{ marginTop:12 }}>
            <div style={{ fontSize:11, color:"#64748b", marginBottom:4 }}>
              Signal confidence: {(update.confidence * 100).toFixed(1)}%
            </div>
            <div style={{ background:"#0f172a", borderRadius:4, height:4 }}>
              <div style={{
                width:`${update.confidence * 100}%`,
                background: SIGNAL_COLORS[update.signal],
                height:4, borderRadius:4,
                transition:"width 0.5s ease",
              }}/>
            </div>
          </div>
        </div>
      ) : (
        <div style={{ background:"#1e293b", borderRadius:16, padding:20, marginBottom:16, color:"#475569", fontSize:13, textAlign:"center" }}>
          Waiting for first tick from {symbol}...
        </div>
      )}

      <OrderPanel
        symbol={symbol}
        price={update?.price}
        alpacaStatus={alpacaStatus}
        position={position}
        orders={orders.filter(o => o.symbol === symbol)}
        onPlace={placeOrder}
      />

      {/* Tabs */}
      <div style={{ display:"flex", gap:8, marginBottom:16 }}>
        {["chart", "signals", "graph"].map(t => (
          <button key={t} onClick={() => setTab(t)} style={{
            flex:1, padding:"8px 0", borderRadius:10, border:"none",
            background: tab === t ? "#334155" : "#1e293b",
            color: tab === t ? "#e2e8f0" : "#64748b",
            fontWeight: tab === t ? 600 : 400,
            fontSize:13, cursor:"pointer",
            textTransform:"capitalize",
          }}>
            {t}
          </button>
        ))}
      </div>

      {tab === "chart" && (
        <div style={{ background:"#1e293b", borderRadius:16, padding:16, marginBottom:16 }}>
          <div style={{ fontSize:11, color:"#64748b", marginBottom:10, letterSpacing:1, textTransform:"uppercase" }}>
            Price + Moving Averages
          </div>
          <PriceChart history={history}/>
          <div style={{ display:"flex", gap:16, marginTop:10, fontSize:11, color:"#64748b" }}>
            <span>─ <span style={{color:"#e2e8f0"}}>Price</span></span>
            <span>─ <span style={{color:"#60a5fa"}}>MA5</span></span>
            <span>─ <span style={{color:"#f59e0b"}}>MA20</span></span>
            <span style={{color:"#94a3b8"}}>● Signal</span>
          </div>
        </div>
      )}

      {tab === "signals" && (
        <>
          <div style={{ background:"#1e293b", borderRadius:16, padding:16, marginBottom:16 }}>
            <div style={{ fontSize:11, color:"#64748b", marginBottom:12, letterSpacing:1, textTransform:"uppercase" }}>
              Signal Counts
            </div>
            <div style={{ display:"flex", gap:12 }}>
              {["Buy", "Sell", "Hold"].map(s => (
                <div key={s} style={{
                  flex:1, background:SIGNAL_BG[s], borderRadius:10, padding:10,
                  textAlign:"center", border:`1px solid ${SIGNAL_COLORS[s]}30`,
                }}>
                  <div style={{ fontSize:22, fontWeight:700, color:SIGNAL_COLORS[s] }}>
                    {signalCounts[s] || 0}
                  </div>
                  <div style={{ fontSize:11, color:"#64748b", marginTop:2 }}>{s}</div>
                </div>
              ))}
            </div>
          </div>

          <div style={{ background:"#1e293b", borderRadius:16, padding:16, marginBottom:16 }}>
            <div style={{ fontSize:11, color:"#64748b", marginBottom:12, letterSpacing:1, textTransform:"uppercase" }}>
              Recent Signals
            </div>
            {history.filter(h => h.signal !== "Hold").slice(-6).reverse().map((h, i, arr) => (
              <div key={h.tick_id} style={{
                display:"flex", justifyContent:"space-between", alignItems:"center",
                padding:"8px 0", borderBottom: i < arr.length - 1 ? "1px solid #ffffff08" : "none",
              }}>
                <SignalBadge signal={h.signal} size="sm"/>
                <span style={{ fontSize:13 }}>${h.price.toFixed(2)}</span>
                <span style={{ fontSize:11, color:"#475569" }}>
                  {new Date(h.timestamp * 1000).toLocaleTimeString()}
                </span>
              </div>
            ))}
          </div>
        </>
      )}

      {tab === "graph" && (
        <CorrelatedCard stocks={correlated} onPick={setSymbol}/>
      )}

      {/* Trino status bar */}
      <div style={{
        background:"#1e293b", borderRadius:12, padding:"10px 14px",
        fontSize:11, color:"#475569", display:"flex", justifyContent:"space-between",
      }}>
        <span>Iceberg via Trino</span>
        <span style={{ color:"#64748b" }}>{trinoStatus}</span>
      </div>

      <style>{`@keyframes pulse{0%,100%{opacity:1}50%{opacity:0.4}}`}</style>
    </div>
  );
}
