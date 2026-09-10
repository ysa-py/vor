# Vor iOS — TrollStore sideload target

**Read this before promising anything about iOS compatibility.**

## What this is

An **unsigned `.ipa`** that installs via [TrollStore](https://github.com/opa334dev/TrollStore)
on iOS versions TrollStore supports. It contains:

- a SwiftUI app (`VorApp`) with the same offline Ed25519 license gate as the
  Android and desktop builds (`license/SPEC.md`, shared conformance vectors);
- a local SOCKS5 proxy engine (`SocksProxy.swift`) — the realistic transport
  for an unsigned sideload: it runs entirely in the app sandbox, requires no
  entitlements beyond network access, and any app that supports SOCKS
  (browsers with proxy extensions, etc.) can use it;
- the shared adaptive engine selector logic concepts (engine catalog +
  license state), kept as reference Swift sources.

## What TrollStore is — and is NOT (the honest part)

TrollStore exploits a **persistence vulnerability that Apple patches**. It is
NOT a general iOS solution:

- At the time of this build, TrollStore supports **iOS 14.0 through 16.6.1**
  (with some 17.0 betas, depending on device and TrollStore version — check
  the TrollStore repo's current compatibility matrix before installing).
- **It does not work on the latest iOS.** Apple patched the coretrust bug;
  new iOS versions require TrollStore updates that may never come for your
  version.
- On unsupported versions you need an alternative: **AltStore** or
  **Sideloadly** with a free or paid Apple ID (7-day / 1-year re-sign cycles),
  or a proper App Store / TestFlight distribution — a separate decision for
  the maintainer.

This repo makes **no claim** that Vor works on "any iPhone". The `.ipa` is
built per release; whether it installs depends entirely on your iOS version
and TrollStore's current support.

## Why no full-device VPN on TrollStore

A NetworkExtension PacketTunnelProvider (full-device VPN) requires the
`com.apple.developer.networking.network-extension` entitlement that a
normal provisioning profile gates. TrollStore *can* grant arbitrary
entitlements via ldid on supported versions, but:

- it is brittle across iOS versions,
- per-app NE providers need to be registered with the system in ways an
  unsigned sideload cannot always complete,
- an honest sideload target should not depend on exploit-dependent
  entitlement tricks for its core function.

So the TrollStore build ships the **local SOCKS engine** (no entitlements
beyond plain network access) and documents the NE-based full-device mode as
future work requiring proper signing (AltStore/Sideloadly with an
appropriate provisioning profile, or App Store distribution).

## Build

Requires macOS + Xcode (CI uses `macos-latest`):

```
cd ios
xcodebuild -project Vor.xcodeproj -scheme Vor -configuration Release \
    -derivedDataPath build CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO
# package the .app into an unsigned .ipa:
mkdir -p build/Payload
cp -r build/Build/Products/Release-iphones/Vor.app build/Payload/
cd build && zip -qry Vor.ipa Payload && cd ..
```

Or run `scripts/build-ipa.sh`. The release workflow produces
`Vor-iOS-unsigned.ipa` as a release artifact.

## License gate

Same as every platform: paste a `VOR1.…` token, Ed25519-verified offline
against the embedded public key (CryptoKit — see `License.swift`), re-checked
at launch, re-locks on expiry. The dev public key verifies test tokens; CI
release builds embed the production key.
