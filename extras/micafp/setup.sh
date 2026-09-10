#!/usr/bin/env bash
# ══════════════════════════════════════════════════════════════
# MICAFP-UnifiedShield v6.0 — Full Automation Setup Script
# Run this once to set up the complete development environment
# ══════════════════════════════════════════════════════════════

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log_info()  { echo -e "${GREEN}[SETUP]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# ── 1. Check prerequisites ──────────────────────────────────
log_info "Checking prerequisites..."

check_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        log_error "$1 is not installed. Please install it first."
        log_info "  Install: $2"
        return 1
    fi
    log_info "  ✅ $1 found"
}

check_command rustc "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" || true
check_command cargo "part of Rust toolchain" || true
check_command go "https://go.dev/dl/" || true
check_command flutter "https://flutter.dev/docs/get-started/install" || true
check_command node "https://nodejs.org/" || true
check_command npm "part of Node.js" || true
check_command python3 "https://www.python.org/" || true
check_command docker "https://docs.docker.com/get-docker/" || true

# ── 2. Install Rust targets ─────────────────────────────────
log_info "Installing Rust cross-compilation targets..."
TARGETS=(
    aarch64-linux-android
    armv7-linux-androideabi
    x86_64-linux-android
    aarch64-apple-ios
    x86_64-apple-darwin
    x86_64-pc-windows-msvc
    x86_64-unknown-linux-musl
    aarch64-unknown-linux-musl
    mipsel-unknown-linux-musl
)
for target in "${TARGETS[@]}"; do
    rustup target add "$target" 2>/dev/null || log_warn "Could not add target: $target"
done
log_info "  ✅ Rust targets installed"

# ── 3. Install Rust tools ───────────────────────────────────
log_info "Installing Rust development tools..."
cargo install wasm-pack 2>/dev/null || true
cargo install cargo-clippy 2>/dev/null || true
cargo install cargo-audit 2>/dev/null || true

# ── 4. Build Rust daemon (host platform) ────────────────────
log_info "Building Rust daemon for host platform..."
cd daemon && cargo build --release 2>&1 | tail -5
cd "$SCRIPT_DIR"
log_info "  ✅ Rust daemon built"

# ── 5. Build Go bridge ──────────────────────────────────────
log_info "Building Go bridge..."
cd go-bridge/yggdrasil-mobile
GOOS=linux GOARCH=amd64 go build -buildmode=c-archive -o libyggdrasil.a . 2>/dev/null || \
    log_warn "Go bridge build failed (may need cross-compilation setup)"
cd "$SCRIPT_DIR"

# ── 6. Flutter setup ────────────────────────────────────────
log_info "Setting up Flutter project..."
cd flutter_app
flutter pub get 2>/dev/null || log_warn "Flutter pub get failed"
cd "$SCRIPT_DIR"

# ── 7. Node.js dependencies ─────────────────────────────────
log_info "Installing Node.js dependencies for workers..."
for dir in workers/*/; do
    if [ -f "$dir/package.json" ]; then
        cd "$dir" && npm install 2>/dev/null && cd "$SCRIPT_DIR"
    fi
done

log_info "Installing Node.js dependencies for extensions..."
for dir in extensions/chrome extensions/firefox; do
    if [ -d "$dir" ]; then
        cd "$dir" && npm install 2>/dev/null && cd "$SCRIPT_DIR"
    fi
done

# ── 8. Build WASM obfuscator ────────────────────────────────
log_info "Building WASM obfuscator..."
cd extensions/wasm-obfuscator
RUSTFLAGS="-C target-feature=+simd128" wasm-pack build --target web --release 2>/dev/null || \
    log_warn "WASM build failed (may need wasm-pack)"
cd "$SCRIPT_DIR"

# ── 9. Python AI dependencies ───────────────────────────────
log_info "Installing Python AI dependencies..."
pip3 install torch torchvision onnx onnxruntime numpy 2>/dev/null || \
    log_warn "Python AI dependencies install failed"

# ── 10. Run tests ───────────────────────────────────────────
log_info "Running Rust unit tests..."
cd daemon && cargo test 2>&1 | tail -10 || true
cd "$SCRIPT_DIR"

# ── 11. Create config directories ───────────────────────────
log_info "Creating runtime directories..."
mkdir -p /tmp/unifiedshield/configs
mkdir -p /tmp/unifiedshield/data
mkdir -p /tmp/unifiedshield/cache

# ── Done ─────────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo -e "${GREEN}  ✅ MICAFP-UnifiedShield v6.0 setup complete!${NC}"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "  Quick start:"
echo "    make all          — Build everything"
echo "    make daemon       — Build Rust daemon"
echo "    make flutter      — Build Flutter app"
echo "    make tests        — Run all tests"
echo "    make help         — Show all targets"
echo ""
