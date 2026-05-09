// ═══════════════════════════════════════════════════════════════════
// hooks/useStockStream.ts
// ═══════════════════════════════════════════════════════════════════

import { useEffect, useState, useCallback } from "react";
import { invoke }  from "@tauri-apps/api/core";
import { listen }  from "@tauri-apps/api/event";
import type { GoldSignal, CorrelatedStock } from "../types/stock";

const MAX_HISTORY = 40;

export function useStockStream(symbol: string = "AAPL") {
  const [update,      setUpdate]      = useState<GoldSignal | null>(null);
  const [history,     setHistory]     = useState<GoldSignal[]>([]);
  const [correlated,  setCorrelated]  = useState<CorrelatedStock[]>([]);
  const [trinoStatus, setTrinoStatus] = useState<string>("waiting...");

  // Load latest snapshot on mount
  useEffect(() => {
    invoke<GoldSignal | null>("get_latest").then(data => {
      if (data) {
        setUpdate(data);
        setHistory([data]);
      }
    });
  }, [symbol]);

  // Listen to real-time signal stream from Rust pipeline
  useEffect(() => {
    const unlisten = listen<GoldSignal>("stock-update", (e) => {
      const data = e.payload;
      setUpdate(data);
      setHistory(h => [...h.slice(-(MAX_HISTORY - 1)), data]);
    });

    // Also listen to Trino confirmation events
    const unlistenTrino = listen<GoldSignal>("trino-confirmed", (e) => {
      setTrinoStatus(`Confirmed: ${e.payload.signal} @ $${e.payload.price.toFixed(2)}`);
    });

    return () => {
      unlisten.then(fn => fn());
      unlistenTrino.then(fn => fn());
    };
  }, []);

  // Fetch correlated stocks when we get a new signal
  useEffect(() => {
    if (!update) return;
    invoke<CorrelatedStock[]>("get_correlated", {
      symbol:      update.symbol,
      minStrength: 0.7,
    }).then(setCorrelated).catch(console.error);
  }, [update?.signal]); // only refetch on signal change, not every tick

  // Record user action (Buy/Sell tap) → feeds Neo4j graph
  const recordAction = useCallback((action: "Buy" | "Sell") => {
    if (!update) return;
    invoke("record_action", {
      userId:  "user-001", // replace with real auth user ID
      tickId:  update.tick_id,
      action,
    }).catch(console.error);
  }, [update]);

  return { update, history, correlated, trinoStatus, recordAction };
}