#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# MICAFP UnifiedShield VIP-ULTRA 7.0 — Unified Build Script
# Builds all targets: daemon, flutter, extensions, workers, openwrt
# Zero-error guaranteed — exits on first error
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

VERSION="7.0.0-vip-ultra"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RELEASE_DIR="$SCRIPT_DIR/release/$VERSION"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

log()   { echo -e "${GREEN}[VIP-ULTRA]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; exit 1; }
step()  { echo -e "\n${BLUE}══════ $* ══════${NC}"; }

# ── Prerequisite check ───────────────────────────────────────────────────────
check_deps() {
  step "Checking dependencies"
  local missing=0
  for cmd in cargo rustup flutter dart bun jq; do
    if command -v "$cmd" &>/dev/null; then
      log "$cmd $(${cmd} --version 2>&1 | head -1)"
    else
      warn "$cmd not found — some targets will be skipped"
    fi
  done
}

# ── Rust daemon ──────────────────────────────────────────────────────────────
build_daemon_native() {
  step "Building daemon (native)"
  cd "$SCRIPT_DIR/daemon"
  cargo build --release 2>&1
  log "Native daemon built: target/release/micafp-unified-shield-daemon"
}

build_daemon_android() {
  step "Building daemon (Android — all ABIs)"
  if ! rustup target list --installed | grep -q aarch64-linux-android; then
    warn "Android targets not installed — run: rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android"
    return 0
  fi
  cd "$SCRIPT_DIR/daemon"
  for target in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android; do
    log "Building for $target"
    cargo build --release --target "$target" 2>&1
  done
}

build_daemon_ios() {
  step "Building daemon (iOS)"
  if [[ "$(uname)" != "Darwin" ]]; then
    warn "iOS build requires macOS — skipping"
    return 0
  fi
  cd "$SCRIPT_DIR/daemon"
  cargo build --release --target aarch64-apple-ios 2>&1
}

build_daemon_windows() {
  step "Building daemon (Windows x86_64)"
  if ! rustup target list --installed | grep -q x86_64-pc-windows-gnu; then
    warn "Windows cross-compile target not installed — skipping"
    return 0
  fi
  cd "$SCRIPT_DIR/daemon"
  cargo build --release --target x86_64-pc-windows-gnu 2>&1
}

build_daemon_openwrt() {
  step "Building daemon (OpenWrt mipsel)"
  if ! rustup target list --installed | grep -q mipsel-unknown-linux-musl; then
    warn "mipsel target not installed — skipping"
    return 0
  fi
  cd "$SCRIPT_DIR/daemon"
  cargo build --release --target mipsel-unknown-linux-musl 2>&1
}

# ── Flutter app ──────────────────────────────────────────────────────────────
build_flutter() {
  step "Building Flutter app"
  if ! command -v flutter &>/dev/null; then
    warn "flutter not found — skipping"
    return 0
  fi
  cd "$SCRIPT_DIR/flutter"
  flutter pub get
  flutter build apk --release 2>&1 && log "Android APK built"
  if [[ "$(uname)" == "Darwin" ]]; then
    flutter build ios --release --no-codesign 2>&1 && log "iOS IPA built"
  fi
}

# ── Browser extensions ───────────────────────────────────────────────────────
build_extensions() {
  step "Building browser extensions"
  if ! command -v bun &>/dev/null; then
    warn "bun not found — skipping extensions build"
    return 0
  fi
  for ext in chrome firefox; do
    if [[ -f "$SCRIPT_DIR/extensions/$ext/package.json" ]]; then
      cd "$SCRIPT_DIR/extensions/$ext"
      bun install && bun run build 2>&1
      log "$ext extension built"
    fi
  done
}

# ── Cloudflare Workers ───────────────────────────────────────────────────────
build_workers() {
  step "Building CDN Workers"
  if ! command -v bun &>/dev/null; then
    warn "bun not found — skipping workers build"
    return 0
  fi
  for worker in cloudflare alibaba-cdn tencent-cdn baidu-cdn arvan-cdn bytedance-cdn huawei-cdn universal deno-relay; do
    if [[ -d "$SCRIPT_DIR/workers/$worker" ]]; then
      cd "$SCRIPT_DIR/workers/$worker"
      [[ -f package.json ]] && bun install 2>&1 && log "Worker $worker: deps installed"
    fi
  done
}

# ── WASM Obfuscator ──────────────────────────────────────────────────────────
build_wasm() {
  step "Building WASM obfuscator"
  if ! command -v wasm-pack &>/dev/null; then
    warn "wasm-pack not found — skipping"
    return 0
  fi
  cd "$SCRIPT_DIR/wasm-obfuscator"
  wasm-pack build --release --target web 2>&1
  log "WASM obfuscator built"
}

# ── Tests ────────────────────────────────────────────────────────────────────
run_tests() {
  step "Running tests"
  cd "$SCRIPT_DIR/daemon"
  cargo test 2>&1
  log "Daemon tests passed"
}

# ── Package ──────────────────────────────────────────────────────────────────
package_release() {
  step "Packaging release"
  mkdir -p "$RELEASE_DIR"
  
  # Copy daemon binaries
  find "$SCRIPT_DIR/daemon/target" -name "micafp-unified-shield-daemon" -o -name "*.exe" 2>/dev/null | \
    while read -r f; do
      cp "$f" "$RELEASE_DIR/" 2>/dev/null || true
    done

  # Copy configs
  cp -r "$SCRIPT_DIR/configs" "$RELEASE_DIR/"

  # Compute checksums
  cd "$RELEASE_DIR"
  find . -type f -exec sha256sum {} \; > SHA256SUMS
  log "Release packaged at $RELEASE_DIR"
}

# ── Main ─────────────────────────────────────────────────────────────────────
main() {
  log "MICAFP UnifiedShield VIP-ULTRA $VERSION build starting"
  check_deps
  build_daemon_native
  build_daemon_android
  build_daemon_ios
  build_daemon_windows
  build_daemon_openwrt
  build_flutter
  build_extensions
  build_workers
  build_wasm
  run_tests
  package_release
  log "Build complete!"
}

main "$@"
