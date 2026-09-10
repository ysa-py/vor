#!/usr/bin/env bash
# ══════════════════════════════════════════════════════════════
# MICAFP-UnifiedShield v6.0 — Quick Start Script
# One command to build, test, and run the daemon
# ══════════════════════════════════════════════════════════════

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo -e "${GREEN}[1/4]${NC} Building Rust daemon..."
cd daemon && cargo build --release 2>&1 | tail -3
cd "$SCRIPT_DIR"

echo -e "${GREEN}[2/4]${NC} Running unit tests..."
cd daemon && cargo test 2>&1 | tail -5
cd "$SCRIPT_DIR"

echo -e "${GREEN}[3/4]${NC} Creating default config..."
mkdir -p /tmp/unifiedshield
if [ ! -f /tmp/unifiedshield/config.toml ]; then
    cat > /tmp/unifiedshield/config.toml <<'EOF'
[daemon]
log_level = "info"
data_dir = "/tmp/unifiedshield"

[transport]
default_transport = "auto"
tls_fragmentation = true

[nain]
probe_interval_secs = 30

[battery]
strategy = "auto"

[security]
anti_forensics = true
ephemeral_identity = true
EOF
fi

echo -e "${GREEN}[4/4]${NC} Starting daemon..."
echo ""
echo "═══════════════════════════════════════════════════════════"
echo -e "  ${GREEN}MICAFP-UnifiedShield v6.0${NC}"
echo "  Anti-Censorship VPN for Iran"
echo "═══════════════════════════════════════════════════════════"
echo ""
./daemon/target/release/unifiedshield-daemon --config /tmp/unifiedshield/config.toml "$@"
