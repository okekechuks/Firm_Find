import { useCallback, useEffect, useRef, useState } from "react";
import { cancelDiscovery, errorMessage, getDiscovery, onDiscoveryChanged, refreshDiscovery } from "./backend";
import type { DiscoverySnapshot } from "../types/contracts";

export function useDeviceInventory(desktop: boolean) {
  const generation = useRef(0);
  const [inventory, setInventory] = useState<DiscoverySnapshot | null>(null);
  const [readError, setReadError] = useState<string | null>(null);
  const [subscriptionError, setSubscriptionError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const accept = useCallback((next: DiscoverySnapshot) => {
    setInventory((current) => !current || next.revision >= current.revision ? next : current);
    setReadError(null);
  }, []);

  useEffect(() => {
    if (!desktop) return;
    let active = true;
    let unlisten: (() => void) | undefined;
    const read = async () => {
      try {
        const next = await getDiscovery();
        if (active) accept(next);
      } catch (failure) {
        if (active) setReadError(errorMessage(failure));
      }
    };
    // Subscribe before reading initial state; cleanup also covers registration resolving late.
    void onDiscoveryChanged(() => { void read(); }).then((stop) => {
      if (!active) { stop(); return; }
      unlisten = stop;
      void read();
    }).catch(() => {
      if (active) { setSubscriptionError("Live updates are unavailable. Use Refresh to check devices."); void read(); }
    });
    return () => { active = false; unlisten?.(); };
  }, [desktop, accept]);

  const refresh = useCallback(async () => {
    if (!desktop) return;
    const epoch = ++generation.current;
    setRefreshing(true);
    setReadError(null);
    try { const next = await refreshDiscovery(); if (epoch === generation.current) accept(next); }
    catch (failure) { if (epoch === generation.current) setReadError(errorMessage(failure)); }
    finally { if (epoch === generation.current) setRefreshing(false); }
  }, [desktop, accept]);
  const cancel = useCallback(async () => {
    generation.current++;
    try { await cancelDiscovery(); setRefreshing(false); setReadError("Discovery cancelled. Last successful inventory retained."); }
    catch (failure) { setReadError(errorMessage(failure)); }
  }, []);
  return { cancel, inventory, error: readError ?? subscriptionError, refreshing, refresh };
}
