#!/bin/bash
# ══════════════════════════════════════════════════════════════════════════════
# MICAFP-UnifiedShield-vip-ultra-Quantum-ultra v8.0
# Master build script — ALL platforms, ALL components
# Zero features removed. Complete merge of all 13 source projects.
# ══════════════════════════════════════════════════════════════════════════════
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

VERSION="8.0.0"
PLATFORM="${1:-all}"

GREEN='\033[0;32m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; BLUE='\033[0;34m'; NC='\033[0m'
log_info()  { printf "${GREEN}[INFO]${NC}  %s\n" "$*"; }
log_step()  { printf "${BLUE}[STEP]${NC}  %s\n" "$*"; }
log_warn()  { printf "${YELLOW}[WARN]${NC}  %s\n" "$*"; }
log_error() { printf "${RED}[ERROR]${NC} %s\n" "$*"; }

ERRORS=0
ok()   { log_info "✅ $*"; }
fail() { log_error "❌ $*"; ERRORS=$((ERRORS+1)); }

# ── Rust daemon ───────────────────────────────────────────────────────────────
build_daemon() {
  log_step "Building Rust daemon (release, all features)..."
  cd daemon
  cargo build --release --features full && ok "Daemon (native)" || fail "Daemon (native)"

  case "$PLATFORM" in
    android|all)
      log_step "Cross-compiling for Android..."
      cross build --target aarch64-linux-android   --release --features platform-android && ok "Android arm64"   || fail "Android arm64"
      cross build --target armv7-linux-androideabi --release --features platform-android && ok "Android armv7"   || fail "Android armv7"
      cross build --target x86_64-linux-android    --release --features platform-android && ok "Android x86_64"  || fail "Android x86_64"
      ;;
  esac
  case "$PLATFORM" in
    ios|all)
      log_step "Cross-compiling for iOS..."
      cargo build --target aarch64-apple-ios --release --features platform-ios && ok "iOS arm64" || fail "iOS arm64" || true
      ;;
  esac
  case "$PLATFORM" in
    windows|all)
      log_step "Cross-compiling for Windows..."
      cross build --target x86_64-pc-windows-gnu --release --features platform-windows && ok "Windows x86_64" || fail "Windows x86_64"
      ;;
  esac
  case "$PLATFORM" in
    linux|all)
      log_step "Cross-compiling for Linux musl..."
      cross build --target x86_64-unknown-linux-musl  --release --features platform-linux && ok "Linux x86_64 musl"  || fail "Linux x86_64 musl"
      cross build --target aarch64-unknown-linux-musl --release --features platform-linux && ok "Linux aarch64 musl" || fail "Linux aarch64 musl"
      ;;
  esac
  case "$PLATFORM" in
    openwrt|all)
      log_step "Cross-compiling for OpenWrt (MIPS)..."
      cross build --target mipsel-unknown-linux-musl --release --features platform-openwrt && ok "OpenWrt mipsel" || fail "OpenWrt mipsel"
      ;;
  esac
  cd ..
}

# ── WASM Obfuscator ───────────────────────────────────────────────────────────
build_wasm() {
  log_step "Building WASM obfuscator..."
  if [ -d wasm-obfuscator ]; then
    wasm-pack build wasm-obfuscator --target web --out-dir ../extensions/chrome/wasm \
      && cp extensions/chrome/wasm/shield_wasm_bg.wasm extensions/firefox/wasm/ 2>/dev/null || true \
      && ok "WASM obfuscator" || fail "WASM obfuscator"
  else
    log_warn "wasm-obfuscator dir not found, skipping"
  fi
}

# ── Browser Extensions ────────────────────────────────────────────────────────
build_extensions() {
  log_step "Building Chrome extension..."
  (cd extensions/chrome   && pnpm install && pnpm build && ok "Chrome extension")   || fail "Chrome extension"
  log_step "Building Firefox extension..."
  (cd extensions/firefox  && pnpm install && pnpm build && ok "Firefox extension")  || fail "Firefox extension"
}

# ── CDN Workers ───────────────────────────────────────────────────────────────
build_workers() {
  for w in cloudflare alibaba-cdn arvan-cdn baidu-cdn bytedance-cdn huawei-cdn tencent-cdn universal; do
    if [ -d "workers/$w" ]; then
      log_step "Building worker: $w..."
      (cd "workers/$w" && pnpm install && pnpm build && ok "Worker: $w") || fail "Worker: $w"
    fi
  done
  if [ -d workers/deno-relay ]; then
    log_step "Building Deno relay..."
    (cd workers/deno-relay && deno task build && ok "Worker: deno-relay") || fail "Worker: deno-relay"
  fi
}

# ── Dashboard ─────────────────────────────────────────────────────────────────
build_dashboard() {
  log_step "Building Next.js dashboard..."
  (cd dashboard && pnpm install && pnpm build && ok "Dashboard") || fail "Dashboard"
}

# ── Flutter App ───────────────────────────────────────────────────────────────
build_flutter() {
  local FDIR="flutter_app"; [ -d flutter ] && FDIR="flutter"
  log_step "Building Flutter app (dir: $FDIR)..."
  (cd "$FDIR" && flutter pub get \
    && flutter build apk --release --split-per-abi && ok "Flutter Android" || fail "Flutter Android") \
    || true
  (cd "$FDIR" && flutter build ios --release --no-codesign && ok "Flutter iOS") || fail "Flutter iOS" || true
}

# ── AI Models ─────────────────────────────────────────────────────────────────
build_ai() {
  if [ -d ai-models/train ]; then
    log_step "Training AI models..."
    pip install -r ai-models/train/requirements.txt -q
    python3 ai-models/train/dpi_classifier_train.py && ok "DPI classifier" || fail "DPI classifier"
    python3 ai-models/train/traffic_predictor_train.py && ok "Traffic predictor" || fail "Traffic predictor"
    python3 ai-models/train/adversarial_traffic_gan.py && ok "GAN model" || fail "GAN model"
    python3 ai-models/quantize/quantize_models.py && ok "ONNX quantize" || fail "ONNX quantize"
  fi
}

# ── Main ─────────────────────────────────────────────────────────────────────
echo ""
log_info "══════════════════════════════════════════════════════════"
log_info " MICAFP-UnifiedShield-vip-ultra-Quantum-ultra v${VERSION}"
log_info " Master Build — Platform: ${PLATFORM}"
log_info "══════════════════════════════════════════════════════════"
echo ""

build_daemon
build_wasm
build_extensions
build_workers
build_dashboard
build_flutter
build_ai

echo ""
if [ "$ERRORS" -eq 0 ]; then
  log_info "══════════════════════════════════════════════════════════"
  log_info " ✅ ALL COMPONENTS BUILT SUCCESSFULLY — ZERO ERRORS"
  log_info "══════════════════════════════════════════════════════════"
else
  log_error "══════════════════════════════════════════════════════════"
  log_error " ⚠️  Build complete with ${ERRORS} non-fatal error(s)"
  log_error "══════════════════════════════════════════════════════════"
  exit 1
fi
