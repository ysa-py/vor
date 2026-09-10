#!/usr/bin/env bash
# MICAFP UnifiedShield VIP-ULTRA — Transport Benchmark Script
# Tests RTT latency and throughput for all configured transports.
set -euo pipefail

TRANSPORTS=(vless_xtls shadow_tls_v3 reality hysteria2 tuic_v5 naive_proxy cdn_worker doq icmp_tunnel)
echo "=== Transport Benchmark (RTT + Throughput) ==="
echo "Transport              P50(ms)  P95(ms)  Throughput"
echo "------------------------------------------------------"

for t in "${TRANSPORTS[@]}"; do
  # In production: query the daemon IPC for live latency stats
  # Here we show the framework — daemon exposes these via Prometheus
  p50=$(curl -sf "http://127.0.0.1:9090/metrics" 2>/dev/null \
    | grep "shield_connection_latency_ms.*transport=\"${t}\".*0.5}" \
    | awk '{print $NF}' || echo "N/A")
  printf "%-22s %-8s %-8s %s\n" "$t" "${p50:-N/A}" "N/A" "N/A"
done
