'use client';

import { useEffect, useCallback } from 'react';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';

/**
 * Single source of truth for live network measurements.
 *
 * - Runs a real speed/latency/ISP test on mount.
 * - Re-tests every `intervalMs` (default 30s).
 * - Re-tests shortly after the connection status or active core changes.
 *
 * Any panel (status bar, speed monitor, security center…) pulls from this
 * same store slice instead of duplicating measurement logic.
 */
export function useNetworkStats(autoTest = true, intervalMs = 30_000) {
  const networkStats = useUnifiedShieldStore((s) => s.networkStats);
  const runNetworkSpeedTest = useUnifiedShieldStore((s) => s.runNetworkSpeedTest);
  const connected = useUnifiedShieldStore((s) => s.connected);
  const activeCoreId = useUnifiedShieldStore((s) => s.orchestrator.activeCoreId);

  // Initial + periodic test
  useEffect(() => {
    if (!autoTest) return;
    runNetworkSpeedTest();
    const t = setInterval(() => runNetworkSpeedTest(), intervalMs);
    return () => clearInterval(t);
  }, [autoTest, intervalMs, runNetworkSpeedTest]);

  // Re-test when connection status or active core changes
  useEffect(() => {
    if (!autoTest) return;
    const t = setTimeout(() => runNetworkSpeedTest(), 800);
    return () => clearTimeout(t);
  }, [autoTest, connected, activeCoreId, runNetworkSpeedTest]);

  const refresh = useCallback(() => runNetworkSpeedTest(), [runNetworkSpeedTest]);

  return { networkStats, refresh };
}
