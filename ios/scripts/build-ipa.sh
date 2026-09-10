#!/usr/bin/env bash
# Build the unsigned Vor.ipa for TrollStore sideloading.
# Requires macOS + Xcode. Produces build/Vor-iOS-unsigned.ipa.
set -euo pipefail

PROJECT="Vor.xcodeproj"
SCHEME="Vor"
BUILD_DIR="build"
IPA="$BUILD_DIR/Vor-iOS-unsigned.ipa"

command -v xcodebuild >/dev/null || { echo "xcodebuild not found (requires macOS + Xcode)"; exit 1; }

xcodebuild -project "$PROJECT" -scheme "$SCHEME" -configuration Release \
    -derivedDataPath "$BUILD_DIR" \
    CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO CODE_SIGN_IDENTITY="" \
    -destination 'generic/platform=iOS' build

APP="$BUILD_DIR/Build/Products/Release-iphoneos/Vor.app"
[ -d "$APP" ] || APP="$BUILD_DIR/Build/Products/Release-iphones/Vor.app"
[ -d "$APP" ] || { echo "built .app not found"; exit 1; }

rm -rf "$BUILD_DIR/Payload" "$IPA"
mkdir -p "$BUILD_DIR/Payload"
cp -R "$APP" "$BUILD_DIR/Payload/"
(cd "$BUILD_DIR" && zip -qry "$(basename "$IPA")" Payload)
echo "built $IPA"
echo "NOTE: this .ipa installs only via TrollStore on supported iOS versions"
echo "(14.0–16.6.1 era — see README.md); it is NOT a general iOS solution."
