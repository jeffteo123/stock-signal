import { useState, useEffect, useRef, useCallback } from "react";

// ── Mock hook (replaces Tauri IPC in preview) ─────────────────────
const SIGNAL_COLORS = { Buy: "#22c55e", Sell: "#ef4444", Hold: "#f59e0b" };
const SIGNAL_BG     = { Buy: "#052e16", Sell: "#450a0a", Hold: "#422006" };
const MAX_HISTORY   = 40;

function useMockStockStream() {
  const [update,     setUpdate]     = useState(null);
  const [history,    setHistory]    = useState([]);
  const [correlated, setCorrelated] = useState([
    { symbol: "MSFT",  strength: 0.87, last_signal: "Hold" },
    { symbol: "GOOGL", strength: 0.81, last_signal: "Buy"  },
    { symbol: "NVDA",  strength: 0.76, last_signal: "Sell" },
  ]);
  const [trinoStatus, setTrinoStatus] = useState("waiting...");
  const prices   = useRef([]);
  const prevAbove = useRef(null);
  const tick = useRef(0);

  useEffect(() => {
    let price = 189.5;
    const id = setInterval(() => {
      price += (Math.random() - 0.48) * 1.2;
      price = Math.max(180, Math.min(200, price));
      prices.current.push(price);
      if (prices.current.length > 21) prices.current.shift();

      const n = prices.current.length;
      const shortMa = n >= 5  ? prices.current.slice(-5).reduce((a,b)=>a+b,0)/5  : price;
      const longMa  = n >= 20 ? prices.current.slice(-20).reduce((a,b)=>a+b,0)/20 : price;
      const rsi     = n >= 14 ? 50 + Math.random() * 20 : null;
      const above   = shortMa > longMa;

      let signal = "Hold";
      if (prevAbove.current !== null && prevAbove.current !== above)
        signal = above ? "Buy" : "Sell";
      prevAbove.current = above;

      const confidence = Math.min(Math.abs(shortMa - longMa) / longMa * 100, 1.0);
      const u = {
        tick_id: `tick-${++tick.current}`,
        symbol: "AAPL", price: +price.toFixed(2),
        signal, short_ma: +shortMa.toFixed(2),
        long_ma: +longMa.toFixed(2),
        rsi: rsi ? +rsi.toFixed(1) : null,
        confidence: +confidence.toFixed(3),
        timestamp: Date.now(),
      };
      setUpdate(u);
      setHistory(h => [...h.slice(-(MAX_HISTORY-1)), u]);

      // Simulate Trino confirmation every ~30 ticks
      if (tick.current % 20 === 0)
        setTrinoStatus(`Confirmed: ${u.signal} @ $${u.price}`);
    }, 1500);
    return () => clearInterval(id);
  }, []);

  const recordAction = useCallback((action) => {
    console.log("Action recorded:", action);
  }, []);

  return { update, history, correlated, trinoStatus, recordAction };
}

// ── Components ────────────────────────────────────────────────────

function SignalBadge({ signal, size = "md" }) {
  const icons = { Buy: "▲", Sell: "▼", Hold: "●" };
  const pad   = size === "sm" ? "2px 10px" : "4px 14px";
  const fs    = size === "sm" ? 12 : 14;
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

function PriceChart({ history }) {
  if (history.length < 2) return (
    <div style={{ height: 100, display:"flex", alignItems:"center", justifyContent:"center", color:"#475569", fontSize:12 }}>
      Collecting data...
    </div>
  );

  const W = 340, H = 100;
  const prices  = history.map(h => h.price);
  const shortMAs = history.map(h => h.short_ma);
  const longMAs  = history.map(h => h.long_ma);
  const min = Math.min(...prices, ...shortMAs, ...longMAs) - 0.3;
  const max = Math.max(...prices, ...shortMAs, ...longMAs) + 0.3;

  const x = i => (i / (MAX_HISTORY - 1)) * W;
  const y = p => H - ((p - min) / (max - min)) * H;

  const path = arr => arr.map((p,i) => `${i===0?"M":"L"}${x(i)},${y(p)}`).join(" ");
  const signals = history.map((h,i) => ({...h,i})).filter(h => h.signal !== "Hold");

  return (
    <svg width={W} height={H} style={{ display:"block", overflow:"visible" }}>
      {[0.25,0.5,0.75].map(t => (
        <line key={t} x1={0} y1={H*t} x2={W} y2={H*t} stroke="#ffffff08" strokeWidth={1}/>
      ))}
      <path d={path(longMAs)}  fill="none" stroke="#f59e0b" strokeWidth={1.5} opacity={0.5}/>
      <path d={path(shortMAs)} fill="none" stroke="#60a5fa" strokeWidth={1.5} opacity={0.7}/>
      <path d={path(prices)}   fill="none" stroke="#e2e8f0" strokeWidth={2}/>
      {signals.map((s,i) => (
        <circle key={i} cx={x(s.i)} cy={y(s.price)} r={5}
          fill={SIGNAL_COLORS[s.signal]} stroke="#0f172a" strokeWidth={1.5}/>
      ))}
    </svg>
  );
}

function CorrelatedCard({ stocks }) {
  if (!stocks.length) return null;
  return (
    <div style={{ background:"#1e293b", borderRadius:16, padding:16, marginBottom:16 }}>
      <div style={{ fontSize:11, color:"#64748b", marginBottom:12, letterSpacing:1, textTransform:"uppercase" }}>
        Correlated Stocks
      </div>
      {stocks.map(s => (
        <div key={s.symbol} style={{
          display:"flex", justifyContent:"space-between", alignItems:"center",
          padding:"8px 0", borderBottom:"1px solid #ffffff08",
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

// ── Main App ──────────────────────────────────────────────────────

export default function App() {
  const { update, history, correlated, trinoStatus, recordAction } = useMockStockStream();
  const [tab, setTab] = useState("chart");

  const priceChange = history.length > 1
    ? (update?.price ?? 0) - history[0].price : 0;

  const signalCounts = history.reduce((acc, h) => {
    acc[h.signal] = (acc[h.signal] || 0) + 1; return acc;
  }, {});

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
          <div style={{ fontSize:22, fontWeight:700 }}>AAPL</div>
        </div>
        <div style={{ textAlign:"right" }}>
          <div style={{ width:8, height:8, borderRadius:"50%", background:"#22c55e",
            boxShadow:"0 0 8px #22c55e", display:"inline-block",
            animation:"pulse 1.5s infinite" }}/>
          <div style={{ fontSize:10, color:"#475569", marginTop:4 }}>LIVE</div>
        </div>
      </div>

      {/* Price card */}
      {update && (
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

          {/* Indicators row */}
          <div style={{ display:"flex", gap:16, marginTop:14, fontSize:12 }}>
            <span style={{ color:"#60a5fa" }}>MA5: {update.short_ma.toFixed(2)}</span>
            <span style={{ color:"#f59e0b" }}>MA20: {update.long_ma.toFixed(2)}</span>
            {update.rsi && (
              <span style={{ color: update.rsi > 70 ? "#ef4444" : update.rsi < 30 ? "#22c55e" : "#94a3b8" }}>
                RSI: {update.rsi.toFixed(1)}
              </span>
            )}
          </div>

          {/* Confidence bar */}
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
      )}

      {/* Tabs */}
      <div style={{ display:"flex", gap:8, marginBottom:16 }}>
        {["chart","signals","graph"].map(t => (
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

      {/* Tab: Chart */}
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

      {/* Tab: Signals */}
      {tab === "signals" && (
        <>
          {/* Signal counts */}
          <div style={{ background:"#1e293b", borderRadius:16, padding:16, marginBottom:16 }}>
            <div style={{ fontSize:11, color:"#64748b", marginBottom:12, letterSpacing:1, textTransform:"uppercase" }}>
              Signal Counts
            </div>
            <div style={{ display:"flex", gap:12 }}>
              {["Buy","Sell","Hold"].map(s => (
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

          {/* Signal feed */}
          <div style={{ background:"#1e293b", borderRadius:16, padding:16, marginBottom:16 }}>
            <div style={{ fontSize:11, color:"#64748b", marginBottom:12, letterSpacing:1, textTransform:"uppercase" }}>
              Recent Signals
            </div>
            {history.filter(h => h.signal !== "Hold").slice(-6).reverse().map((h,i,arr) => (
              <div key={h.tick_id} style={{
                display:"flex", justifyContent:"space-between", alignItems:"center",
                padding:"8px 0", borderBottom: i < arr.length-1 ? "1px solid #ffffff08" : "none",
              }}>
                <SignalBadge signal={h.signal} size="sm"/>
                <span style={{ fontSize:13 }}>${h.price.toFixed(2)}</span>
                <span style={{ fontSize:11, color:"#475569" }}>
                  {new Date(h.timestamp).toLocaleTimeString()}
                </span>
              </div>
            ))}
          </div>

          {/* Action buttons */}
          {update && (
            <div style={{ display:"flex", gap:12, marginBottom:16 }}>
              <button onClick={() => recordAction("Buy")} style={{
                flex:1, padding:14, borderRadius:12, border:"none",
                background:"#052e16", color:"#22c55e",
                fontWeight:700, fontSize:15, cursor:"pointer",
                border:"1px solid #22c55e40",
              }}>▲ Mark Buy</button>
              <button onClick={() => recordAction("Sell")} style={{
                flex:1, padding:14, borderRadius:12, border:"none",
                background:"#450a0a", color:"#ef4444",
                fontWeight:700, fontSize:15, cursor:"pointer",
                border:"1px solid #ef444440",
              }}>▼ Mark Sell</button>
            </div>
          )}
        </>
      )}

      {/* Tab: Graph (correlated stocks) */}
      {tab === "graph" && (
        <CorrelatedCard stocks={correlated}/>
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