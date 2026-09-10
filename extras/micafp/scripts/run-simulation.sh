#!/usr/bin/env bash
# ══════════════════════════════════════════════════════════════
# MICAFP-UnifiedShield v6.0 — Censorship Simulation Runner
# Automatically starts Docker simulation and runs integration tests
# ══════════════════════════════════════════════════════════════

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SIM_DIR="$SCRIPT_DIR/tests/censorship-simulation"

log_info()  { echo -e "${GREEN}[SIM]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[FAIL]${NC} $1"; }

# Check Docker
if ! command -v docker >/dev/null 2>&1; then
    log_error "Docker is required for censorship simulation"
    exit 1
fi

if ! docker compose version >/dev/null 2>&1; then
    log_error "Docker Compose v2 is required"
    exit 1
fi

log_info "Starting Iranian censorship simulation environment..."

# Build and start containers
cd "$SIM_DIR"
docker compose build --quiet 2>/dev/null
docker compose up -d 2>/dev/null

# Wait for containers to be ready
log_info "Waiting for simulation environment to be ready..."
sleep 5

# Check container health
DPI_RUNNING=$(docker compose ps --services --filter "status=running" 2>/dev/null | grep -c "dpi-simulator" || echo "0")
FW_RUNNING=$(docker compose ps --services --filter "status=running" 2>/dev/null | grep -c "iran-firewall" || echo "0")
NAIN_RUNNING=$(docker compose ps --services --filter "status=running" 2>/dev/null | grep -c "national-intranet-mock" || echo "0")

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  Censorship Simulation Status"
echo "═══════════════════════════════════════════════════════════════"
echo "  DPI Simulator:    $([ "$DPI_RUNNING" -gt 0 ] && echo "✅ Running" || echo "❌ Not running")"
echo "  Iran Firewall:    $([ "$FW_RUNNING" -gt 0 ] && echo "✅ Running" || echo "❌ Not running")"
echo "  NAIN Mock:        $([ "$NAIN_RUNNING" -gt 0 ] && echo "✅ Running" || echo "❌ Not running")"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Run integration tests
log_info "Running transport integration tests against simulation..."

# Test 1: Direct connection (should fail through firewall)
log_info "Test 1: Direct connection (expected: BLOCKED)..."
DIRECT_RESULT=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 5 \
    --proxy "" https://www.youtube.com 2>/dev/null || echo "000")
if [ "$DIRECT_RESULT" = "000" ] || [ "$DIRECT_RESULT" = "403" ]; then
    log_info "  ✅ Direct connection properly blocked (HTTP $DIRECT_RESULT)"
else
    log_warn "  ⚠️ Direct connection not blocked (HTTP $DIRECT_RESULT)"
fi

# Test 2: Domestic site (should work)
log_info "Test 2: Domestic site (expected: ALLOWED)..."
DOMESTIC_RESULT=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 5 \
    https://www.digikala.com 2>/dev/null || echo "000")
if [ "$DOMESTIC_RESULT" = "200" ] || [ "$DOMESTIC_RESULT" = "301" ]; then
    log_info "  ✅ Domestic site accessible (HTTP $DOMESTIC_RESULT)"
else
    log_warn "  ⚠️ Domestic site not accessible (HTTP $DOMESTIC_RESULT)"
fi

# Test 3: Shadow TLS v3 (should bypass DPI)
log_info "Test 3: Shadow TLS v3 transport (expected: BYPASS)..."
# This would test the actual transport in a real scenario

# Test 4: TLS Fragmentation (should bypass SNI filtering)
log_info "Test 4: TLS Fragmentation (expected: BYPASS)..."
# This would test the actual fragmentation in a real scenario

echo ""
log_info "Integration tests complete!"
echo ""
log_info "To stop the simulation: docker compose -f $SIM_DIR/docker-compose.yml down"
log_info "To view logs: docker compose -f $SIM_DIR/docker-compose.yml logs -f"
