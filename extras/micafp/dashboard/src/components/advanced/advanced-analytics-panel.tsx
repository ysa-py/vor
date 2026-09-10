// MICAFP UnifiedShield VIP-ULTRA — Advanced Analytics Panel Component
"use client";

import { useEffect, useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import type { AdvancedAnalyticsResponse } from "@/app/api/advanced-analytics/route";

export function AdvancedAnalyticsPanel() {
  const [data, setData] = useState<AdvancedAnalyticsResponse | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchData = async () => {
      try {
        const res = await fetch("/api/advanced-analytics");
        setData(await res.json());
      } finally {
        setLoading(false);
      }
    };
    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, []);

  if (loading) return <div className="p-4 text-sm text-muted-foreground">Loading analytics...</div>;
  if (!data) return null;

  const dpiRisk = data.aiEngine.dpiDetectionProbability;
  const riskLevel = dpiRisk < 0.3 ? "low" : dpiRisk < 0.6 ? "medium" : "high";
  const riskColor = { low: "bg-green-500", medium: "bg-yellow-500", high: "bg-red-500" }[riskLevel];

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4 p-4">
      {/* AI Engine */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium">🧠 AI Engine</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">DPI Detection Risk</span>
            <div className="flex items-center gap-2">
              <div className={`w-2 h-2 rounded-full ${riskColor}`} />
              <span className="text-xs font-mono">{(dpiRisk * 100).toFixed(1)}%</span>
            </div>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">UCB1 Exploration Rate</span>
            <span className="text-xs font-mono">{(data.aiEngine.ucbBanditExplorationRate * 100).toFixed(1)}%</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">GAN Traffic Match</span>
            <span className="text-xs font-mono">{(data.aiEngine.ganTrafficMatchScore * 100).toFixed(1)}%</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">Core Switches (24h)</span>
            <Badge variant="outline" className="text-xs">{data.aiEngine.coreSwitchesLast24h}</Badge>
          </div>
          <div className="text-xs text-muted-foreground mt-1">
            Model: <span className="font-mono">{data.aiEngine.activeModelVersion}</span>
          </div>
        </CardContent>
      </Card>

      {/* Mesh Network */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium">🕸️ Mesh Network</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">Active Peers</span>
            <Badge variant={data.meshNetwork.activePeers > 0 ? "default" : "secondary"}>
              {data.meshNetwork.activePeers}
            </Badge>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">BLE Mesh Peers</span>
            <span className="text-xs font-mono">{data.meshNetwork.bleMeshPeers}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">WiFi Aware Peers</span>
            <span className="text-xs font-mono">{data.meshNetwork.wifiAwarePeers}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">Yggdrasil Peers</span>
            <span className="text-xs font-mono">{data.meshNetwork.yggdrasilPeers}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">Active Channel</span>
            <span className="text-xs">{data.meshNetwork.activeChannel ?? "—"}</span>
          </div>
        </CardContent>
      </Card>

      {/* Resilience */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium">🛡️ Resilience</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">Fallback Strategy</span>
            <Badge variant="outline" className="text-xs">{data.resilience.currentFallbackStrategy}</Badge>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">Chain Position</span>
            <span className="text-xs font-mono">{data.resilience.fallbackChainPosition + 1}/8</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">Watchdog Restarts (24h)</span>
            <Badge variant={data.resilience.watchdogRestarts24h > 0 ? "destructive" : "secondary"}>
              {data.resilience.watchdogRestarts24h}
            </Badge>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">PQ-KEX Total</span>
            <span className="text-xs font-mono">{data.postQuantumKexTotal}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-xs text-muted-foreground">Uptime</span>
            <span className="text-xs font-mono">{formatUptime(data.resilience.uptimeSeconds)}</span>
          </div>
        </CardContent>
      </Card>

      {/* Transport Latency Table */}
      <Card className="md:col-span-2 xl:col-span-3">
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium">⚡ Transport Latency (P50 / P95 / P99)</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="overflow-x-auto">
            <table className="w-full text-xs">
              <thead>
                <tr className="text-muted-foreground border-b">
                  <th className="text-left py-1 pr-4">Transport</th>
                  <th className="text-right pr-4">P50</th>
                  <th className="text-right pr-4">P95</th>
                  <th className="text-right pr-4">P99</th>
                  <th className="text-right pr-4">Failure Rate</th>
                  <th className="text-right">Circuit</th>
                </tr>
              </thead>
              <tbody>
                {data.transports.map((t) => (
                  <tr key={t.name} className="border-b border-muted/30">
                    <td className="py-1 pr-4 font-mono">{t.name}</td>
                    <td className="text-right pr-4 font-mono">{t.p50LatencyMs}ms</td>
                    <td className="text-right pr-4 font-mono">{t.p95LatencyMs}ms</td>
                    <td className="text-right pr-4 font-mono">{t.p99LatencyMs}ms</td>
                    <td className="text-right pr-4 font-mono">{(t.failureRate * 100).toFixed(1)}%</td>
                    <td className="text-right">
                      <Badge
                        variant={t.circuitBreakerState === "closed" ? "secondary" : "destructive"}
                        className="text-xs"
                      >
                        {t.circuitBreakerState}
                      </Badge>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function formatUptime(secs: number): string {
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`;
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return `${h}h ${m}m`;
}
