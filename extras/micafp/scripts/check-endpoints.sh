#!/usr/bin/env bash
# ══════════════════════════════════════════════════════════════
# MICAFP-UnifiedShield v6.0 — CDN Endpoint Health Checker
# Automatically validates all 30 bundled CDN endpoints
# Run periodically to update endpoint health status
# ══════════════════════════════════════════════════════════════

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_FILE="$SCRIPT_DIR/configs/cdn-endpoints.json"
RESULTS_FILE="$SCRIPT_DIR/configs/endpoint-health.json"

log_info()  { echo -e "${GREEN}[CHECK]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[FAIL]${NC} $1"; }

# Check if jq is available
if ! command -v jq >/dev/null 2>&1; then
    log_error "jq is required. Install: apt install jq / brew install jq"
    exit 1
fi

log_info "Checking CDN endpoint health from $CONFIG_FILE..."

# Parse endpoints from JSON
ENDPOINTS=$(jq -r '.endpoints[] | .url' "$CONFIG_FILE")
HEALTHY=()
UNHEALTHY=()
TOTAL=0
HEALTHY_COUNT=0

for url in $ENDPOINTS; do
    TOTAL=$((TOTAL + 1))

    # Measure response time
    START=$(date +%s%N)
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
        --connect-timeout 5 \
        --max-time 10 \
        "$url/health" 2>/dev/null || echo "000")
    END=$(date +%s%N)
    LATENCY=$(( (END - START) / 1000000 ))

    if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "204" ]; then
        HEALTHY+=("$url")
        HEALTHY_COUNT=$((HEALTHY_COUNT + 1))
        log_info "✅ $url — ${LATENCY}ms (HTTP $HTTP_CODE)"
    else
        UNHEALTHY+=("$url")
        log_error "❌ $url — ${LATENCY}ms (HTTP $HTTP_CODE)"
    fi
done

# Generate health report
log_info "═════════════════════════════════════════════════════════"
log_info "Health check complete: $HEALTHY_COUNT/$TOTAL endpoints healthy"

# Write results to JSON
cat > "$RESULTS_FILE" <<EOF
{
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "total_endpoints": $TOTAL,
  "healthy_endpoints": $HEALTHY_COUNT,
  "unhealthy_endpoints": $((TOTAL - HEALTHY_COUNT)),
  "health_percentage": $(echo "scale=1; $HEALTHY_COUNT * 100 / $TOTAL" | bc),
  "results": [
$(for url in "${HEALTHY[@]}"; do echo "    {\"url\": \"$url\", \"status\": \"healthy\"},"; done)
$(for url in "${UNHEALTHY[@]}"; do echo "    {\"url\": \"$url\", \"status\": \"unhealthy\"},"; done)
  ]
}
EOF

log_info "Results saved to $RESULTS_FILE"

# Exit with error if less than 50% healthy
if [ "$HEALTHY_COUNT" -lt "$((TOTAL / 2))" ]; then
    log_error "Less than 50% of endpoints are healthy!"
    exit 1
fi
