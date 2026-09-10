#!/bin/bash
set -euo pipefail
echo "=== Cross-compile for iOS ==="
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cd daemon
cargo build --release --target aarch64-apple-ios
cargo build --release --target aarch64-apple-ios-sim
echo "=== iOS cross-compile complete ==="
