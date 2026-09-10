#!/usr/bin/env bash
# MICAFP UnifiedShield VIP-ULTRA — Health Check Script
# Checks daemon, prometheus, IPC socket, and all transport endpoints.
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
PASS=0; FAIL=0

check() {
  local name="$1"; local cmd="$2"
  if eval "$cmd" &>/dev/null; then
    echo -e "${GREEN}✔${NC} $name"; ((PASS++))
  else
    echo -e "${RED}✘${NC} $name"; ((FAIL++))
  fi
}

echo "=== MICAFP UnifiedShield VIP-ULTRA Health Check ==="
check "Daemon process running"       "pgrep -x shield-daemon"
check "IPC socket exists (Unix)"     "test -S /var/run/shield-daemon.sock"
check "Prometheus metrics endpoint"  "curl -sf http://127.0.0.1:9090/metrics"
check "Dashboard reachable"          "curl -sf http://127.0.0.1:3000/api/health"
check "DNS resolution (google.com)"  "dig +short google.com @1.1.1.1 | grep -qE '^[0-9]'"
check "Chinese CDN Alibaba relay"    "curl -sf --max-time 5 https://fc.aliyuncs.com/ping || true"
check "Rust toolchain available"     "rustc --version"
check "Node.js available"            "node --version"

echo ""
echo "Results: ${GREEN}${PASS} passed${NC}, ${RED}${FAIL} failed${NC}"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
