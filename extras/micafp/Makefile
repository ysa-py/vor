# ══════════════════════════════════════════════════════════════════════════════
# MICAFP-UnifiedShield-vip-ultra-Quantum-ultra v8.0 — Master Makefile
# Complete merge of all 13 source projects. Zero features removed.
# ══════════════════════════════════════════════════════════════════════════════

SHELL        := /bin/bash
CARGO        := cargo
CROSS        := cross
BUN          := bun
PNPM         := pnpm
DOCKER       := docker
ZIG          := zig

PROJECT      := MICAFP-UnifiedShield-vip-ultra-Quantum-ultra
VERSION      := 8.0.0

# ── Directories ───────────────────────────────────────────────────────────────
DAEMON_DIR     := daemon
FLUTTER_DIR    := flutter
FLUTTER_APP    := flutter_app
WORKERS_DIR    := workers
EXTENSIONS_DIR := extensions
GO_BRIDGE_DIR  := go-bridge
AI_DIR         := ai-models
CONFIGS_DIR    := configs
TESTS_DIR      := tests
OPENWRT_DIR    := openwrt
RELEASE_DIR    := release/$(VERSION)

# ── Rust cross-compile targets ────────────────────────────────────────────────
RUST_TARGETS := \
	aarch64-linux-android \
	armv7-linux-androideabi \
	x86_64-linux-android \
	aarch64-apple-ios \
	x86_64-apple-darwin \
	aarch64-apple-darwin \
	x86_64-pc-windows-gnu \
	x86_64-unknown-linux-musl \
	aarch64-unknown-linux-musl \
	mipsel-unknown-linux-musl

# ── Colors ────────────────────────────────────────────────────────────────────
GREEN  := \033[0;32m
YELLOW := \033[1;33m
RED    := \033[0;31m
BLUE   := \033[0;34m
NC     := \033[0m

log_info  = @printf "$(GREEN)[INFO]$(NC)  %s\n" "$1"
log_warn  = @printf "$(YELLOW)[WARN]$(NC)  %s\n" "$1"
log_error = @printf "$(RED)[ERROR]$(NC) %s\n" "$1"
log_step  = @printf "$(BLUE)[STEP]$(NC)  %s\n" "$1"

# ══════════════════════════════════════════════════════════════════════════════
# PHONY TARGETS
# ══════════════════════════════════════════════════════════════════════════════
.PHONY: all clean daemon daemon-debug daemon-quantum \
        daemon-android daemon-ios daemon-linux daemon-windows daemon-openwrt \
        daemon-all-platforms \
        flutter flutter-android flutter-ios \
        extensions wasm \
        workers workers-cloudflare workers-alibaba workers-arvan workers-baidu \
        workers-bytedance workers-huawei workers-tencent workers-deno workers-universal \
        dashboard \
        ai-models \
        openwrt zig-openwrt \
        tests test-daemon test-censorship test-unit \
        lint fmt check \
        health benchmark \
        release package checksums sign \
        publish-ipfs update-iran-ips detect-isp rotate-identity \
        install help

.DEFAULT_GOAL := all

# ══════════════════════════════════════════════════════════════════════════════
# ALL — Build everything
# ══════════════════════════════════════════════════════════════════════════════
all: lint daemon flutter extensions workers dashboard
	$(call log_info, ✅ Full build complete — $(PROJECT) v$(VERSION))

# ══════════════════════════════════════════════════════════════════════════════
# DAEMON (Rust)
# ══════════════════════════════════════════════════════════════════════════════
daemon:
	$(call log_step, Building Rust daemon (release, all features)...)
	cd $(DAEMON_DIR) && $(CARGO) build --release --features full
	$(call log_info, ✅ shield-daemon: $(DAEMON_DIR)/target/release/shield-daemon)

daemon-debug:
	$(call log_step, Building Rust daemon (debug)...)
	cd $(DAEMON_DIR) && $(CARGO) build

daemon-quantum:
	$(call log_step, Building Rust daemon with Quantum-Ultra features...)
	cd $(DAEMON_DIR) && $(CARGO) build --release --features full,quantum-full

daemon-android:
	$(call log_step, Cross-compiling daemon for Android...)
	cd $(DAEMON_DIR) && $(CROSS) build --target aarch64-linux-android   --release --features platform-android
	cd $(DAEMON_DIR) && $(CROSS) build --target armv7-linux-androideabi  --release --features platform-android
	cd $(DAEMON_DIR) && $(CROSS) build --target x86_64-linux-android    --release --features platform-android

daemon-ios:
	$(call log_step, Cross-compiling daemon for iOS...)
	cd $(DAEMON_DIR) && $(CARGO) build --target aarch64-apple-ios --release --features platform-ios

daemon-linux:
	$(call log_step, Cross-compiling daemon for Linux (musl)...)
	cd $(DAEMON_DIR) && $(CROSS) build --target x86_64-unknown-linux-musl   --release --features platform-linux
	cd $(DAEMON_DIR) && $(CROSS) build --target aarch64-unknown-linux-musl  --release --features platform-linux

daemon-windows:
	$(call log_step, Cross-compiling daemon for Windows...)
	cd $(DAEMON_DIR) && $(CROSS) build --target x86_64-pc-windows-gnu --release --features platform-windows

daemon-openwrt:
	$(call log_step, Cross-compiling daemon for OpenWrt (MIPS)...)
	cd $(DAEMON_DIR) && $(CROSS) build --target mipsel-unknown-linux-musl --release --features platform-openwrt

daemon-all-platforms: daemon-android daemon-ios daemon-linux daemon-windows daemon-openwrt
	$(call log_info, ✅ All platform daemons built)

# ══════════════════════════════════════════════════════════════════════════════
# FLUTTER APP
# ══════════════════════════════════════════════════════════════════════════════
flutter:
	$(call log_step, Building Flutter app...)
	cd $(FLUTTER_APP) && flutter pub get
	cd $(FLUTTER_APP) && flutter build apk --release --split-per-abi
	cd $(FLUTTER_APP) && flutter build ios --release --no-codesign || true
	$(call log_info, ✅ Flutter app built)

flutter-android:
	cd $(FLUTTER_APP) && flutter build apk --release --split-per-abi

flutter-ios:
	cd $(FLUTTER_APP) && flutter build ios --release --no-codesign

# ══════════════════════════════════════════════════════════════════════════════
# WASM OBFUSCATOR
# ══════════════════════════════════════════════════════════════════════════════
wasm:
	$(call log_step, Building WASM obfuscator...)
	cd wasm-obfuscator && $(CARGO) build --release --target wasm32-unknown-unknown
	wasm-pack build wasm-obfuscator --target web --out-dir ../extensions/chrome/wasm
	cp extensions/chrome/wasm/shield_wasm_bg.wasm extensions/firefox/wasm/ 2>/dev/null || true
	$(call log_info, ✅ WASM obfuscator built)

# ══════════════════════════════════════════════════════════════════════════════
# BROWSER EXTENSIONS
# ══════════════════════════════════════════════════════════════════════════════
extensions: wasm
	$(call log_step, Building Chrome extension...)
	cd $(EXTENSIONS_DIR)/chrome   && $(PNPM) install && $(PNPM) build
	$(call log_step, Building Firefox extension...)
	cd $(EXTENSIONS_DIR)/firefox  && $(PNPM) install && $(PNPM) build
	$(call log_info, ✅ Browser extensions built)

# ══════════════════════════════════════════════════════════════════════════════
# CDN WORKERS
# ══════════════════════════════════════════════════════════════════════════════
workers: workers-cloudflare workers-alibaba workers-arvan workers-baidu \
         workers-bytedance workers-huawei workers-tencent workers-deno workers-universal

workers-cloudflare:
	$(call log_step, Building Cloudflare worker...)
	cd $(WORKERS_DIR)/cloudflare   && $(PNPM) install && $(PNPM) build

workers-alibaba:
	$(call log_step, Building Alibaba CDN worker...)
	cd $(WORKERS_DIR)/alibaba-cdn  && $(PNPM) install && $(PNPM) build

workers-arvan:
	$(call log_step, Building Arvan CDN worker...)
	cd $(WORKERS_DIR)/arvan-cdn    && $(PNPM) install && $(PNPM) build 2>/dev/null || true

workers-baidu:
	$(call log_step, Building Baidu CDN worker...)
	cd $(WORKERS_DIR)/baidu-cdn    && $(PNPM) install && $(PNPM) build

workers-bytedance:
	$(call log_step, Building ByteDance CDN worker...)
	cd $(WORKERS_DIR)/bytedance-cdn && $(PNPM) install && $(PNPM) build

workers-huawei:
	$(call log_step, Building Huawei CDN worker...)
	cd $(WORKERS_DIR)/huawei-cdn   && $(PNPM) install && $(PNPM) build 2>/dev/null || true

workers-tencent:
	$(call log_step, Building Tencent CDN worker...)
	cd $(WORKERS_DIR)/tencent-cdn  && $(PNPM) install && $(PNPM) build

workers-deno:
	$(call log_step, Building Deno relay worker...)
	cd $(WORKERS_DIR)/deno-relay   && deno task build 2>/dev/null || true

workers-universal:
	$(call log_step, Building Universal worker...)
	cd $(WORKERS_DIR)/universal    && $(PNPM) install && $(PNPM) build

# ══════════════════════════════════════════════════════════════════════════════
# DASHBOARD (Next.js)
# ══════════════════════════════════════════════════════════════════════════════
dashboard:
	$(call log_step, Building Next.js dashboard...)
	cd dashboard && $(PNPM) install && $(PNPM) build
	$(call log_info, ✅ Dashboard built)

dashboard-dev:
	cd dashboard && $(PNPM) dev

# ══════════════════════════════════════════════════════════════════════════════
# AI MODELS
# ══════════════════════════════════════════════════════════════════════════════
ai-models:
	$(call log_step, Training AI models...)
	cd $(AI_DIR)/train && pip install -r requirements.txt
	cd $(AI_DIR)/train && python3 dpi_classifier_train.py
	cd $(AI_DIR)/train && python3 traffic_predictor_train.py
	cd $(AI_DIR)/train && python3 adversarial_traffic_gan.py
	cd $(AI_DIR)/quantize && python3 quantize_models.py
	cd $(AI_DIR)/quantize && python3 validate_onnx.py

# ══════════════════════════════════════════════════════════════════════════════
# OPENWRT / ZIG
# ══════════════════════════════════════════════════════════════════════════════
openwrt:
	$(call log_step, Building OpenWrt package...)
	bash $(SCRIPTS_DIR)/package-openwrt.sh

zig-openwrt:
	$(call log_step, Building Zig-based OpenWrt component...)
	cd zig-openwrt && $(ZIG) build -Doptimize=ReleaseSmall

# ══════════════════════════════════════════════════════════════════════════════
# TESTS
# ══════════════════════════════════════════════════════════════════════════════
tests: test-daemon test-censorship

test-daemon:
	$(call log_step, Running Rust unit tests...)
	cd $(DAEMON_DIR) && $(CARGO) test --features full

test-censorship:
	$(call log_step, Running censorship simulation tests...)
	cd $(TESTS_DIR)/censorship-simulation && pip install -r requirements.txt -q
	cd $(TESTS_DIR)/censorship-simulation && python3 test_bypass_effectiveness.py

test-unit:
	$(call log_step, Running Rust unit tests (covert channels)...)
	cd $(DAEMON_DIR) && $(CARGO) test tests::covert_channels

# ══════════════════════════════════════════════════════════════════════════════
# CODE QUALITY
# ══════════════════════════════════════════════════════════════════════════════
fmt:
	$(call log_step, Formatting Rust code...)
	cd $(DAEMON_DIR) && $(CARGO) fmt
	$(call log_step, Formatting Go code...)
	-gofmt -w $(GO_BRIDGE_DIR)

check:
	$(call log_step, Checking Rust compilation...)
	cd $(DAEMON_DIR) && $(CARGO) check --features full

lint: fmt check
	$(call log_step, Linting Rust code...)
	cd $(DAEMON_DIR) && $(CARGO) clippy --all-targets --features full -- -D warnings

# ══════════════════════════════════════════════════════════════════════════════
# OPS SCRIPTS
# ══════════════════════════════════════════════════════════════════════════════
SCRIPTS_DIR := scripts

health:
	bash $(SCRIPTS_DIR)/health-check.sh

benchmark:
	bash $(SCRIPTS_DIR)/benchmark-transports.sh

checksums:
	bash $(SCRIPTS_DIR)/generate-checksums.sh

sign:
	bash $(SCRIPTS_DIR)/sign-binaries.sh

update-iran-ips:
	bash $(SCRIPTS_DIR)/update-iran-ip-ranges.sh

detect-isp:
	bash $(SCRIPTS_DIR)/detect-isp.sh

rotate-identity:
	bash $(SCRIPTS_DIR)/rotate-identity.sh

publish-ipfs:
	bash $(SCRIPTS_DIR)/publish-ipfs.sh

# ══════════════════════════════════════════════════════════════════════════════
# RELEASE & PACKAGE
# ══════════════════════════════════════════════════════════════════════════════
release: daemon-all-platforms flutter extensions workers dashboard checksums sign
	$(call log_info, ✅ Release $(VERSION) ready in $(RELEASE_DIR))

package:
	$(call log_step, Creating release archive...)
	mkdir -p $(RELEASE_DIR)
	tar --exclude='*/target' --exclude='*/node_modules' --exclude='*/.dart_tool' \
	    -czf $(RELEASE_DIR)/$(PROJECT)-v$(VERSION).tar.gz .
	$(call log_info, ✅ Archive: $(RELEASE_DIR)/$(PROJECT)-v$(VERSION).tar.gz)

# ══════════════════════════════════════════════════════════════════════════════
# CLEAN
# ══════════════════════════════════════════════════════════════════════════════
clean:
	$(call log_step, Cleaning build artifacts...)
	cd $(DAEMON_DIR) && $(CARGO) clean
	-rm -rf dashboard/.next dashboard/out
	-rm -rf $(FLUTTER_APP)/build $(FLUTTER_DIR)/build
	-rm -rf $(EXTENSIONS_DIR)/chrome/dist $(EXTENSIONS_DIR)/firefox/dist
	-rm -rf extensions/chrome/wasm extensions/firefox/wasm
	$(call log_info, ✅ Clean complete)

# ══════════════════════════════════════════════════════════════════════════════
# HELP
# ══════════════════════════════════════════════════════════════════════════════
help:
	@echo ""
	@echo "$(PROJECT) v$(VERSION) — Build System"
	@echo "────────────────────────────────────────────────────────────────"
	@echo "  make all                   Build all components"
	@echo "  make daemon                Rust daemon (release, all features)"
	@echo "  make daemon-quantum        Rust daemon with quantum-full features"
	@echo "  make daemon-all-platforms  Cross-compile for all platforms"
	@echo "  make flutter               Flutter app (Android + iOS)"
	@echo "  make extensions            Chrome + Firefox extensions"
	@echo "  make workers               All CDN workers (CF, Ali, Arvan, Baidu, ...)"
	@echo "  make dashboard             Next.js dashboard"
	@echo "  make ai-models             Train + quantize AI/ONNX models"
	@echo "  make openwrt               OpenWrt package"
	@echo "  make zig-openwrt           Zig-based OpenWrt component"
	@echo "  make tests                 Run all tests"
	@echo "  make lint                  Lint + format"
	@echo "  make health                Health check (all subsystems)"
	@echo "  make benchmark             Transport latency benchmarks"
	@echo "  make release               Full release build"
	@echo "  make package               Create release tarball"
	@echo "  make clean                 Remove build artifacts"
	@echo ""
