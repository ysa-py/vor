#!/bin/bash
set -euo pipefail
echo "=== Package for OpenWrt ==="
for arch in mipsel mips aarch64 x86_64; do
    echo "Building for $arch..."
    cd daemon && cargo build --release --target ${arch}-unknown-linux-musl && cd ..
done
echo "=== OpenWrt packaging complete ==="
