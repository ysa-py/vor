#!/bin/bash
set -euo pipefail
echo "=== Cross-compile for Android ==="
cargo install cargo-ndk
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
cd daemon
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -t x86 build --release --lib
echo "Copying .so files..."
cp target/aarch64-linux-android/release/libunifiedshield.so ../android/app/jniLibs/arm64-v8a/
cp target/armv7-linux-androideabi/release/libunifiedshield.so ../android/app/jniLibs/armeabi-v7a/
cp target/x86_64-linux-android/release/libunifiedshield.so ../android/app/jniLibs/x86_64/
cp target/i686-linux-android/release/libunifiedshield.so ../android/app/jniLibs/x86/
echo "=== Android cross-compile complete ==="
