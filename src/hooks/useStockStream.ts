// ═══════════════════════════════════════════════════════════════════
// hooks/useStockStream.ts
// ═══════════════════════════════════════════════════════════════════

import { useEffect, useState, useCallback } from "react";
import { invoke }  from "@tauri-apps/api/core";
import { listen }  from "@tauri-apps/api/event";
import type {
  GoldSignal, CorrelatedStock, Order, Position, AlpacaStatus, OrderSide,
} from "../types/stock";

const MAX_HISTORY = 40;

export function useStockStream(symbol: string) {
  const [update,       setUpdate]       = useState<GoldSignal | null>(null);
  const [history,      setHistory]      = useState<GoldSignal[]>([]);
  const [correlated,   setCorrelated]   = useState<CorrelatedStock[]>([]);
  const [trinoStatus,  setTrinoStatus]  = useState<string>("waiting...");
  const [alpacaStatus, setAlpacaStatus] = useState<AlpacaStatus | null>(null);
  const [position,     setPosition]     = useState<Position | null>(null);
  const [orders,       setOrders]       = useState<Order[]>([]);

  // Tell the backend pipeline to switch symbols, then clear local state.
  useEffect(() => {
    let cancelled = false;
    setHistory([]);
    setUpdate(null);
    setPosition(null);

    invoke("set_symbol", { symbol })
      .then(() => invoke<GoldSignal | null>("get_latest"))
      .then(data => {
        if (cancelled) return;
        if (data) {
          setUpdate(data);
          setHistory([data]);
        }
      })
      .catch(console.error);

    return () => { cancelled = true; };
  }, [symbol]);

  // Real-time signal stream from the Rust pipeline. Filter by symbol so
  // late-arriving ticks for the previous symbol don't poison history.
  useEffect(() => {
    const unlisten = listen<GoldSignal>("stock-update", (e) => {
      const data = e.payload;
      if (data.symbol !== symbol) return;
      setUpdate(data);
      setHistory(h => [...h.slice(-(MAX_HISTORY - 1)), data]);
    });

    const unlistenTrino = listen<GoldSignal>("trino-confirmed", (e) => {
      if (e.payload.symbol !== symbol) return;
      setTrinoStatus(`Confirmed: ${e.payload.signal} @ $${e.payload.price.toFixed(2)}`);
    });

    return () => {
      unlisten.then(fn => fn());
      unlistenTrino.then(fn => fn());
    };
  }, [symbol]);

  // Probe Alpaca configuration once
  useEffect(() => {
    invoke<AlpacaStatus>("alpaca_status")
      .then(setAlpacaStatus)
      .catch(() => setAlpacaStatus({ configured: false, paper: true }));
  }, []);

  // Refresh position + correlated stocks when symbol or signal changes
  useEffect(() => {
    if (!update) return;
    invoke<CorrelatedStock[]>("get_correlated", {
      symbol:      update.symbol,
      minStrength: 0.7,
    }).then(setCorrelated).catch(console.error);
  }, [update?.signal, update?.symbol]);

  const refreshPosition = useCallback(async () => {
    if (!alpacaStatus?.configured) return;
    try {
      const p = await invoke<Position | null>("get_position", { symbol });
      setPosition(p);
    } catch {
      setPosition(null);
    }
  }, [symbol, alpacaStatus?.configured]);

  const refreshOrders = useCallback(async () => {
    if (!alpacaStatus?.configured) return;
    try {
      const o = await invoke<Order[]>("list_orders", { status: "all", limit: 10 });
      setOrders(o);
    } catch (e) {
      console.error(e);
    }
  }, [alpacaStatus?.configured]);

  useEffect(() => {
    refreshPosition();
    refreshOrders();
  }, [refreshPosition, refreshOrders]);

  const placeOrder = useCallback(async (side: OrderSide, qty: number) => {
    const order = await invoke<Order>("place_order", { symbol, qty, side });
    await Promise.all([refreshPosition(), refreshOrders()]);
    return order;
  }, [symbol, refreshPosition, refreshOrders]);

  const recordAction = useCallback((action: "Buy" | "Sell") => {
    if (!update) return;
    invoke("record_action", {
      userId:  "user-001",
      tickId:  update.tick_id,
      action,
    }).catch(console.error);
  }, [update]);

  return {
    update, history, correlated, trinoStatus,
    alpacaStatus, position, orders,
    placeOrder, refreshOrders, refreshPosition, recordAction,
  };
}
