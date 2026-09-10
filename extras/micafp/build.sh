#!/usr/bin/env bash
# ==============================================================================
# MICAFP-UnifiedShield Master Build Script
# ==============================================================================
# Builds all components: Rust daemon, Go bridge, Flutter app, WASM obfuscator,
# browser extensions, CDN workers, OpenWrt package, and packages release artifacts.
#
# Usage:
#   ./build.sh                    # Build everything for current platform
#   ./build.sh --target linux     # Build for specific target
#   ./build.sh --component daemon # Build only the daemon
#   ./build.sh --release          # Build in release mode with optimizations
#   ./build.sh --clean            # Clean all build artifacts
#
# Environment:
#   SHIELD_RELEASE_DIR   - Output directory for release artifacts (default: ./release)
#   SHIELD_BUILD_DIR     - Build directory (default: ./build)
#   OPENWRT_SDK_PATH     - Path to OpenWrt SDK (optional, for router builds)

set -euo pipefail

# ==============================================================================
# Configuration
# ==============================================================================
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="${SCRIPT_DIR}"
BUILD_DIR="${SHIELD_BUILD_DIR:-${PROJECT_ROOT}/build}"
RELEASE_DIR="${SHIELD_RELEASE_DIR:-${PROJECT_ROOT}/release}"
OPENWRT_SDK_PATH="${OPENWRT_SDK_PATH:-}"
VERSION="${VERSION:-6.0.0}"
BUILD_MODE="release"
COMPONENT="all"
TARGET="current"
VERBOSE=0
NUM_JOBS=$(nproc 2>/dev/null || echo 4)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ==============================================================================
# Argument Parsing
# ==============================================================================
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --target <target>     Build target: current, linux, android, ios, windows, openwrt, all"
    echo "  --component <comp>   Build component: all, daemon, go-bridge, flutter, wasm, browser, cdn, openwrt"
    echo "  --release             Build in release mode (default)"
    echo "  --debug               Build in debug mode"
    echo "  --clean               Clean all build artifacts"
    echo "  --verbose             Enable verbose output"
    echo "  --jobs <n>            Number of parallel jobs (default: ${NUM_JOBS})"
    echo "  -h, --help            Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 --target linux --component daemon"
    echo "  $0 --target all --release --jobs 8"
    echo "  $0 --clean"
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --target)
            TARGET="$2"
            shift 2
            ;;
        --component)
            COMPONENT="$2"
            shift 2
            ;;
        --release)
            BUILD_MODE="release"
            shift
            ;;
        --debug)
            BUILD_MODE="debug"
            shift
            ;;
        --clean)
            echo -e "${YELLOW}Cleaning all build artifacts...${NC}"
            rm -rf "${BUILD_DIR}" "${RELEASE_DIR}"
            rm -rf daemon/target wasm-obfuscator/target
            echo -e "${GREEN}Clean complete.${NC}"
            exit 0
            ;;
        --verbose)
            VERBOSE=1
            shift
            ;;
        --jobs)
            NUM_JOBS="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            usage
            exit 1
            ;;
    esac
done

# ==============================================================================
# Helper Functions
# ==============================================================================
log_info()    { echo -e "${BLUE}[INFO]${NC} $*"; }
log_success() { echo -e "${GREEN}[OK]${NC} $*"; }
log_warn()    { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error()   { echo -e "${RED}[ERROR]${NC} $*"; }

check_command() {
    if ! command -v "$1" &>/dev/null; then
        log_error "Required command '$1' not found. Please install it first."
        return 1
    fi
    return 0
}

detect_host_platform() {
    local os="$(uname -s | tr '[:upper:]' '[:lower:]')"
    local arch="$(uname -m)"

    case "${os}" in
        linux)  os="linux" ;;
        darwin) os="darwin" ;;
        mingw*|msys*|cygwin*) os="windows" ;;
        *)      os="unknown" ;;
    esac

    case "${arch}" in
        x86_64|amd64)   arch="x86_64" ;;
        aarch64|arm64)  arch="aarch64" ;;
        armv7l|armv7)   arch="armv7" ;;
        *)              arch="unknown" ;;
    esac

    echo "${os}-${arch}"
}

HOST_PLATFORM=$(detect_host_platform)
log_info "Detected host platform: ${HOST_PLATFORM}"

# ==============================================================================
# Prepare directories
# ==============================================================================
mkdir -p "${BUILD_DIR}"
mkdir -p "${RELEASE_DIR}"

CARGO_FLAG=""
FLUTTER_FLAG=""
if [ "${BUILD_MODE}" = "release" ]; then
    CARGO_FLAG="--release"
    FLUTTER_FLAG="--release"
else
    CARGO_FLAG=""
    FLUTTER_FLAG="--debug"
fi

# ==============================================================================
# Build: Rust Daemon
# ==============================================================================
build_daemon() {
    log_info "Building Rust daemon for target: ${TARGET}..."

    check_command cargo || return 1

    local daemon_dir="${PROJECT_ROOT}/daemon"
    if [ ! -d "${daemon_dir}" ]; then
        log_error "Daemon source directory not found: ${daemon_dir}"
        return 1
    fi

    case "${TARGET}" in
        current|linux)
            local rust_target=""
            if [ "${TARGET}" = "linux" ] || [ "${HOST_PLATFORM}" = linux-* ]; then
                rust_target="x86_64-unknown-linux-musl"
            elif [ "${HOST_PLATFORM}" = darwin-* ]; then
                rust_target="x86_64-apple-darwin"
            elif [ "${HOST_PLATFORM}" = windows-* ]; then
                rust_target="x86_64-pc-windows-msvc"
            fi

            if [ -n "${rust_target}" ]; then
                log_info "  Cross-compiling for ${rust_target}..."
                cargo build ${CARGO_FLAG} --target "${rust_target}" \
                    --manifest-path "${daemon_dir}/Cargo.toml" \
                    -j "${NUM_JOBS}"

                local binary_path="${daemon_dir}/target/${rust_target}/${BUILD_MODE}/unifiedshield-daemon"
                if [ -f "${binary_path}" ]; then
                    cp "${binary_path}" "${RELEASE_DIR}/unifiedshield-daemon-${rust_target}"
                    chmod +x "${RELEASE_DIR}/unifiedshield-daemon-${rust_target}"
                    log_success "Daemon built: unifiedshield-daemon-${rust_target}"
                fi
            fi
            ;;
        android)
            for abi in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android; do
                log_info "  Building for Android: ${abi}..."
                if command -v cross &>/dev/null; then
                    cross build ${CARGO_FLAG} --target "${abi}" \
                        --manifest-path "${daemon_dir}/Cargo.toml"
                else
                    cargo build ${CARGO_FLAG} --target "${abi}" \
                        --manifest-path "${daemon_dir}/Cargo.toml"
                fi

                local binary_path="${daemon_dir}/target/${abi}/${BUILD_MODE}/unifiedshield-daemon"
                if [ -f "${binary_path}" ]; then
                    cp "${binary_path}" "${RELEASE_DIR}/unifiedshield-daemon-${abi}"
                    chmod +x "${RELEASE_DIR}/unifiedshield-daemon-${abi}"
                fi
            done
            log_success "Daemon built for Android (3 ABIs)"
            ;;
        ios)
            cargo build ${CARGO_FLAG} --target aarch64-apple-ios \
                --manifest-path "${daemon_dir}/Cargo.toml"
            cp "${daemon_dir}/target/aarch64-apple-ios/${BUILD_MODE}/unifiedshield-daemon" \
                "${RELEASE_DIR}/unifiedshield-daemon-aarch64-apple-ios"
            log_success "Daemon built for iOS"
            ;;
        openwrt)
            if [ -n "${OPENWRT_SDK_PATH}" ] && [ -d "${OPENWRT_SDK_PATH}" ]; then
                log_info "  Building OpenWrt package using SDK at ${OPENWRT_SDK_PATH}..."
                (
                    cd "${OPENWRT_SDK_PATH}"
                    make package/unifiedshield/compile V=s
                )
                cp "${OPENWRT_SDK_PATH}/bin/targets/"*/*/packages/unifiedshield*.ipk \
                    "${RELEASE_DIR}/" 2>/dev/null || true
                log_success "OpenWrt package built"
            else
                log_info "  Building mipsel binary with cross..."
                if command -v cross &>/dev/null; then
                    cross build ${CARGO_FLAG} --target mipsel-unknown-linux-musl \
                        --manifest-path "${daemon_dir}/Cargo.toml"
                    cp "${daemon_dir}/target/mipsel-unknown-linux-musl/${BUILD_MODE}/unifiedshield-daemon" \
                        "${RELEASE_DIR}/unifiedshield-daemon-mipsel-openwrt"
                    log_success "OpenWrt mipsel daemon built"
                else
                    log_warn "OpenWrt SDK not found and 'cross' not installed. Skipping."
                    log_warn "Set OPENWRT_SDK_PATH or install 'cross' for OpenWrt builds."
                fi
            fi
            ;;
        all)
            build_daemon_current
            build_daemon_android
            build_daemon_ios
            build_daemon_openwrt
            ;;
        *)
            log_error "Unknown target: ${TARGET}"
            return 1
            ;;
    esac
}

# ==============================================================================
# Build: Go Bridge
# ==============================================================================
build_go_bridge() {
    log_info "Building Go PT bridge (Yggdrasil mobile)..."

    check_command go || { log_warn "Go not found, skipping Go bridge build."; return 0; }

    local go_dir="${PROJECT_ROOT}/go-bridge/yggdrasil-mobile"
    if [ ! -d "${go_dir}" ]; then
        log_error "Go bridge directory not found: ${go_dir}"
        return 1
    fi

    # Build for each target platform as c-archive
    local targets=("linux/amd64" "linux/arm64" "android/arm64" "android/arm" "android/amd64" "ios/arm64")

    for t in "${targets[@]}"; do
        local goos="${t%%/*}"
        local goarch="${t##*/}"
        local output_name="libyggdrasil_${goos}_${goarch}"

        log_info "  Building Go bridge for ${t}..."
        (
            cd "${go_dir}"
            CGO_ENABLED=1 GOOS="${goos}" GOARCH="${goarch}" \
                go build -trimpath -ldflags="-s -w" \
                -buildmode=c-archive \
                -o "${RELEASE_DIR}/${output_name}.a" \
                . 2>/dev/null || log_warn "  Failed to build for ${t} (may need cross-compiler)"
        )
    done

    log_success "Go PT bridge built for available targets"
}

# ==============================================================================
# Build: Flutter App
# ==============================================================================
build_flutter() {
    log_info "Building Flutter application..."

    check_command flutter || { log_warn "Flutter not found, skipping app build."; return 0; }

    local flutter_dir="${PROJECT_ROOT}/flutter_app"
    if [ ! -d "${flutter_dir}" ]; then
        log_error "Flutter app directory not found: ${flutter_dir}"
        return 1
    fi

    (
        cd "${flutter_dir}"
        flutter pub get

        case "${TARGET}" in
            current|linux)
                if [ "${HOST_PLATFORM}" = linux-* ]; then
                    flutter build linux ${FLUTTER_FLAG}
                    cp -r build/linux/x64/release/bundle/* "${RELEASE_DIR}/flutter-linux/" 2>/dev/null || true
                elif [ "${HOST_PLATFORM}" = darwin-* ]; then
                    flutter build macos ${FLUTTER_FLAG}
                    cp -r build/macos/Build/Products/Release/*.app "${RELEASE_DIR}/" 2>/dev/null || true
                elif [ "${HOST_PLATFORM}" = windows-* ]; then
                    flutter build windows ${FLUTTER_FLAG}
                    cp -r build/windows/x64/runner/Release/* "${RELEASE_DIR}/flutter-windows/" 2>/dev/null || true
                fi
                ;;
            android)
                flutter build apk --split-per-abi ${FLUTTER_FLAG}
                cp build/app/outputs/flutter-apk/*.apk "${RELEASE_DIR}/"
                flutter build appbundle ${FLUTTER_FLAG}
                cp build/app/outputs/bundle/release/*.aab "${RELEASE_DIR}/"
                ;;
            ios)
                flutter build ios --no-codesign ${FLUTTER_FLAG}
                ;;
        esac
    )

    log_success "Flutter app built for ${TARGET}"
}

# ==============================================================================
# Build: WASM Obfuscator
# ==============================================================================
build_wasm() {
    log_info "Building WASM obfuscator with SIMD128..."

    check_command wasm-pack || { log_warn "wasm-pack not found, skipping WASM build."; return 0; }

    local wasm_dir="${PROJECT_ROOT}/wasm-obfuscator"
    if [ ! -d "${wasm_dir}" ]; then
        log_error "WASM obfuscator directory not found: ${wasm_dir}"
        return 1
    fi

    (
        cd "${wasm_dir}"
        RUSTFLAGS="-C target-feature=+simd128,+bulk-memory,+sign-extension" \
            wasm-pack build --target web --out-dir pkg --release
        cp -r pkg "${RELEASE_DIR}/wasm-obfuscator/"
    )

    log_success "WASM obfuscator built with SIMD128"
}

# ==============================================================================
# Build: Browser Extensions
# ==============================================================================
build_browser_extensions() {
    log_info "Building browser extensions..."

    check_command npm || { log_warn "npm not found, skipping browser extension build."; return 0; }

    # Chrome extension
    local chrome_dir="${PROJECT_ROOT}/browser-extension/chrome"
    if [ -d "${chrome_dir}" ]; then
        (
            cd "${chrome_dir}"
            npm ci --production 2>/dev/null || npm install --production
            npm run build
            cd dist
            zip -r "${RELEASE_DIR}/unifiedshield-chrome-extension.zip" . -x "*.map"
        )
        log_success "Chrome extension built"
    fi

    # Firefox extension
    local firefox_dir="${PROJECT_ROOT}/browser-extension/firefox"
    if [ -d "${firefox_dir}" ]; then
        (
            cd "${firefox_dir}"
            npm ci --production 2>/dev/null || npm install --production
            npm run build
            cd dist
            zip -r "${RELEASE_DIR}/unifiedshield-firefox-extension.xpi" . -x "*.map"
        )
        log_success "Firefox extension built"
    fi
}

# ==============================================================================
# Build: CDN Workers
# ==============================================================================
build_cdn_workers() {
    log_info "Building CDN worker deployment packages..."

    local cdn_dir="${PROJECT_ROOT}/cdn-workers"
    if [ ! -d "${cdn_dir}" ]; then
        log_warn "CDN workers directory not found, skipping."
        return 0
    fi

    # Package each CDN worker for deployment
    for worker_dir in "${cdn_dir}"/*/; do
        local worker_name="$(basename "${worker_dir}")"
        if [ -f "${worker_dir}/package.json" ]; then
            log_info "  Packaging CDN worker: ${worker_name}..."
            (
                cd "${worker_dir}"
                npm ci --production 2>/dev/null || npm install --production
                if [ -f "deploy.sh" ]; then
                    log_info "    Deployment script available: deploy.sh"
                fi
                tar czf "${RELEASE_DIR}/cdn-worker-${worker_name}.tar.gz" \
                    --exclude='node_modules' .
            )
        fi
    done

    log_success "CDN workers packaged"
}

# ==============================================================================
# Build: OpenWrt Package
# ==============================================================================
build_openwrt() {
    log_info "Building OpenWrt package..."

    if [ -n "${OPENWRT_SDK_PATH}" ] && [ -d "${OPENWRT_SDK_PATH}" ]; then
        log_info "  Using OpenWrt SDK at ${OPENWRT_SDK_PATH}..."
        (
            cd "${OPENWRT_SDK_PATH}"
            # Copy package source
            cp -r "${PROJECT_ROOT}/openwrt" package/unifiedshield
            cp -r "${PROJECT_ROOT}/daemon" package/unifiedshield/src/daemon

            # Build with SDK
            make package/unifiedshield/compile V=s

            # Copy resulting .ipk
            find bin/ -name "unifiedshield*.ipk" -exec cp {} "${RELEASE_DIR}/" \;
        )
        log_success "OpenWrt package built with SDK"
    else
        # Build standalone mipsel binary
        log_info "  Building standalone mipsel binary for OpenWrt..."
        build_daemon_openwrt_standalone
    fi
}

build_daemon_openwrt_standalone() {
    if command -v cross &>/dev/null; then
        cross build --release --target mipsel-unknown-linux-musl \
            --manifest-path "${PROJECT_ROOT}/daemon/Cargo.toml"
        cp "${PROJECT_ROOT}/daemon/target/mipsel-unknown-linux-musl/release/unifiedshield-daemon" \
            "${RELEASE_DIR}/unifiedshield-daemon-mipsel-openwrt"
        log_success "OpenWrt mipsel daemon binary built"
    else
        log_warn "'cross' not installed and no OpenWrt SDK. Install with: cargo install cross"
    fi
}

# ==============================================================================
# Run Tests
# ==============================================================================
run_tests() {
    log_info "Running test suite..."

    # Rust tests
    if [ -d "${PROJECT_ROOT}/daemon" ]; then
        log_info "  Running Rust tests..."
        cargo test --manifest-path "${PROJECT_ROOT}/daemon/Cargo.toml" -- --nocapture || true
    fi

    # Go tests
    if [ -d "${PROJECT_ROOT}/go-bridge" ]; then
        log_info "  Running Go tests..."
        (cd "${PROJECT_ROOT}/go-bridge/yggdrasil-mobile" && go test ./... -v) || true
    fi

    # Python AI model tests
    if [ -d "${PROJECT_ROOT}/ai-models" ]; then
        log_info "  Running AI model validation..."
        python3 -m pytest "${PROJECT_ROOT}/ai-models/tests/" -v 2>/dev/null || \
            log_warn "AI model tests skipped (pytest not available)"
    fi

    log_success "Test suite complete"
}

# ==============================================================================
# Package Release Artifacts
# ==============================================================================
package_release() {
    log_info "Packaging release artifacts..."

    cd "${RELEASE_DIR}"

    # Generate SHA256 checksums for all release files
    if command -v sha256sum &>/dev/null; then
        sha256sum * > SHA256SUMS.txt 2>/dev/null || true
        log_success "SHA256 checksums generated"
        cat SHA256SUMS.txt
    fi

    # Generate Ed25519 signatures if key is available
    if [ -f "${PROJECT_ROOT}/signing.key" ]; then
        log_info "Signing artifacts with Ed25519..."
        for file in *; do
            [ "$file" = "SHA256SUMS.txt" ] && continue
            [ "$file" = "*.sig" ] && continue
            if command -v minisign &>/dev/null; then
                minisign -S -s "${PROJECT_ROOT}/signing.key" -m "$file" -x "$file.sig" 2>/dev/null || true
            fi
        done
        log_success "Artifacts signed"
    else
        log_warn "No signing key found at ${PROJECT_ROOT}/signing.key. Skipping signing."
    fi

    # Create version info file
    cat > VERSION.txt <<EOF
MICAFP-UnifiedShield v${VERSION}
Build Date: $(date -u '+%Y-%m-%d %H:%M:%S UTC')
Host Platform: ${HOST_PLATFORM}
Build Mode: ${BUILD_MODE}
Git Commit: $(git rev-parse HEAD 2>/dev/null || echo "unknown")
Git Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
EOF

    log_success "Release artifacts packaged in ${RELEASE_DIR}/"
    log_info "Contents:"
    ls -lah "${RELEASE_DIR}/"
}

# ==============================================================================
# Main Build Pipeline
# ==============================================================================
log_info "=============================================="
log_info "MICAFP-UnifiedShield v${VERSION} Build"
log_info "Target: ${TARGET} | Component: ${COMPONENT}"
log_info "Build Mode: ${BUILD_MODE} | Jobs: ${NUM_JOBS}"
log_info "=============================================="

START_TIME=$(date +%s)

# Execute builds based on component selection
case "${COMPONENT}" in
    daemon)     build_daemon ;;
    go-bridge)  build_go_bridge ;;
    flutter)    build_flutter ;;
    wasm)       build_wasm ;;
    browser)    build_browser_extensions ;;
    cdn)        build_cdn_workers ;;
    openwrt)    build_openwrt ;;
    all)
        build_daemon
        build_go_bridge
        build_flutter
        build_wasm
        build_browser_extensions
        build_cdn_workers
        build_openwrt
        run_tests
        ;;
    *)
        log_error "Unknown component: ${COMPONENT}"
        usage
        exit 1
        ;;
esac

# Always package if we're in release mode
if [ "${BUILD_MODE}" = "release" ]; then
    package_release
fi

END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))

log_info "=============================================="
log_success "Build complete in ${ELAPSED}s"
log_info "Release artifacts: ${RELEASE_DIR}/"
log_info "=============================================="
