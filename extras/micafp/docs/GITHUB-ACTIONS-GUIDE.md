# GitHub Actions CI/CD Guide — UnifiedShield NextGen

## Overview

This guide covers the CI/CD pipeline for building and releasing UnifiedShield NextGen across all platforms. The pipeline uses GitHub Actions with special considerations for Iran's internet restrictions.

## Repository Structure

```
unifiedshield-nextgen/
├── .github/
│   └── workflows/
│       ├── ci.yml              # Main CI pipeline
│       ├── release.yml         # Release pipeline
│       ├── flutter-build.yml   # Flutter app builds
│       ├── dashboard-build.yml # Next.js dashboard builds
│       └── daemon-build.yml    # Rust daemon builds
├── flutter/                    # Flutter cross-platform app
├── dashboard/                  # Next.js dashboard
├── daemon/                     # Rust daemon
└── docs/                       # Documentation
```

## CI Pipeline (`ci.yml`)

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  FLUTTER_VERSION: '3.22.0'
  RUST_VERSION: '1.78.0'
  NODE_VERSION: '22.0.0'

jobs:
  # Flutter lint and test
  flutter-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: subosito/flutter-action@v2
        with:
          flutter-version: ${{ env.FLUTTER_VERSION }}
          channel: 'stable'
      - name: Install dependencies
        run: cd flutter && flutter pub get
      - name: Analyze
        run: cd flutter && flutter analyze
      - name: Test
        run: cd flutter && flutter test

  # Dashboard lint and test
  dashboard-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: ${{ env.NODE_VERSION }}
      - name: Install dependencies
        run: cd dashboard && npm ci
      - name: Lint
        run: cd dashboard && npm run lint
      - name: Type check
        run: cd dashboard && npm run type-check
      - name: Build
        run: cd dashboard && npm run build

  # Rust daemon check
  daemon-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: ${{ env.RUST_VERSION }}
      - name: Clippy
        run: cd daemon && cargo clippy -- -D warnings
      - name: Test
        run: cd daemon && cargo test
      - name: Build
        run: cd daemon && cargo build --release
```

## Release Pipeline (`release.yml`)

```yaml
name: Release

on:
  push:
    tags: ['v*']

jobs:
  # Build Flutter Android APK
  build-android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: subosito/flutter-action@v2
        with:
          flutter-version: '3.22.0'
      - name: Build APK
        run: |
          cd flutter
          flutter build apk --release --split-per-abi
          flutter build appbundle --release
      - uses: actions/upload-artifact@v4
        with:
          name: android-apks
          path: |
            flutter/build/app/outputs/flutter-apk/*.apk
            flutter/build/app/outputs/bundle/release/*.aab

  # Build Flutter iOS
  build-ios:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: subosito/flutter-action@v2
        with:
          flutter-version: '3.22.0'
      - name: Build iOS
        run: |
          cd flutter
          flutter build ios --release --no-codesign
      - uses: actions/upload-artifact@v4
        with:
          name: ios-build
          path: flutter/build/ios/ipa/*.ipa

  # Build Flutter Windows
  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: subosito/flutter-action@v2
        with:
          flutter-version: '3.22.0'
      - name: Build Windows
        run: |
          cd flutter
          flutter build windows --release
      - uses: actions/upload-artifact@v4
        with:
          name: windows-build
          path: flutter/build/windows/x64/runner/Release/**

  # Build Flutter macOS
  build-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: subosito/flutter-action@v2
        with:
          flutter-version: '3.22.0'
      - name: Build macOS
        run: |
          cd flutter
          flutter build macos --release
      - uses: actions/upload-artifact@v4
        with:
          name: macos-build
          path: flutter/build/macos/Build/Products/Release/*.app

  # Build Flutter Linux
  build-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: subosito/flutter-action@v2
        with:
          flutter-version: '3.22.0'
      - name: Install Linux dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-3-dev liblzma-dev libstdc++-12-dev
      - name: Build Linux
        run: |
          cd flutter
          flutter build linux --release
      - uses: actions/upload-artifact@v4
        with:
          name: linux-build
          path: flutter/build/linux/x64/release/bundle/**

  # Build Rust daemon for all targets
  build-daemon:
    strategy:
      matrix:
        target:
          - x86_64-unknown-linux-gnu
          - aarch64-unknown-linux-gnu
          - x86_64-pc-windows-msvc
          - x86_64-apple-darwin
          - aarch64-apple-darwin
          - armv7-linux-androideabi
          - aarch64-linux-android
    runs-on: ${{ contains(matrix.target, 'apple') && 'macos-latest' || contains(matrix.target, 'windows') && 'windows-latest' || 'ubuntu-latest' }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - name: Build daemon
        run: |
          cd daemon
          cargo build --release --target ${{ matrix.target }}
      - uses: actions/upload-artifact@v4
        with:
          name: daemon-${{ matrix.target }}
          path: daemon/target/${{ matrix.target }}/release/unifiedshield-daemon*

  # Build and deploy dashboard
  build-dashboard:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '22'
      - name: Install and build
        run: |
          cd dashboard
          npm ci
          npm run build
      - uses: actions/upload-artifact@v4
        with:
          name: dashboard-build
          path: dashboard/.next/**

  # Upload to Chinese CDNs (PRIMARY for Iran)
  upload-to-cdns:
    needs: [build-android, build-daemon]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
      - name: Upload to Alibaba Cloud OSS
        uses: manyuanrong/setup-ossutil@v3.0
        with:
          endpoint: oss-cn-shanghai.aliyuncs.com
          access-key-id: ${{ secrets.ALIBABA_OSS_KEY }}
          access-key-secret: ${{ secrets.ALIBABA_OSS_SECRET }}
      - name: Sync to Alibaba OSS
        run: |
          ossutil cp -r android-apks/ oss://unifiedshield/releases/${{ github.ref_name }}/ --force
          ossutil cp -r daemon-* oss://unifiedshield/daemon/${{ github.ref_name }}/ --force

      - name: Upload to Tencent COS
        uses: TencentCloud/cos-action@v1
        with:
          secret_id: ${{ secrets.TENCENT_COS_SECRET_ID }}
          secret_key: ${{ secrets.TENCENT_COS_SECRET_KEY }}
          bucket: unifiedshield-1258344699
          region: ap-hongkong
          source: android-apks/
          target: releases/${{ github.ref_name }}/
          sync: true

  # Create GitHub Release (SECONDARY - may be slow in Iran)
  create-release:
    needs: [build-android, build-ios, build-windows, build-macos, build-linux, build-daemon, build-dashboard]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
      - name: Create Release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            android-apks/*.apk
            android-apks/*.aab
            daemon-*/*
          body: |
            ## UnifiedShield NextGen ${{ github.ref_name }}

            ### Download Links
            - 🇨🇳 **Alibaba Cloud OSS** (PRIMARY for Iran): https://unifiedshield.oss-cn-shanghai.aliyuncs.com/releases/${{ github.ref_name }}/
            - 🇨🇳 **Tencent COS** (PRIMARY for Iran): https://unifiedshield-1258344699.cos.ap-hongkong.myqcloud.com/releases/${{ github.ref_name }}/
            - 🌍 **GitHub** (SECONDARY): See assets below

            ⚠️ **Note for Iranian users**: Cloudflare is BLOCKED in Iran. Use Alibaba or Tencent CDN mirrors for fastest downloads.

            ### 9 Anti-Censorship Cores
            🔮 XTLS-Reality | ⚡ Hysteria2 | 🚀 TUICv5 | 🕶️ Shadowsocks | 💫 VLESS | 🛡️ WireGuard | 🐴 Trojan | 🌐 NaïveProxy | 🤝 P2P-Relay

            ### National Intranet Mode
            🇮🇷 فارسی | English | حالت اینترانت ملی
```

## Required Secrets

| Secret | Description | Where to Get |
|--------|-------------|-------------|
| `ALIBABA_OSS_KEY` | Alibaba Cloud OSS Access Key | Alibaba Cloud Console |
| `ALIBABA_OSS_SECRET` | Alibaba Cloud OSS Secret Key | Alibaba Cloud Console |
| `TENCENT_COS_SECRET_ID` | Tencent Cloud Secret ID | Tencent Cloud Console |
| `TENCENT_COS_SECRET_KEY` | Tencent Cloud Secret Key | Tencent Cloud Console |
| `CODESIGN_CERT` | iOS/macOS code signing cert | Apple Developer Portal |
| `KEYSTORE_BASE64` | Android signing keystore | Generated locally |
| `KEYSTORE_PASSWORD` | Keystore password | Generated locally |

## CDN Upload Priority

Since **Cloudflare is BLOCKED in Iran**, release uploads follow this priority:

1. **Alibaba Cloud OSS** (Shanghai + Hong Kong) — FIRST
2. **Tencent COS** (Hong Kong) — SECOND
3. **GitHub Releases** — THIRD (fallback, may be slow)

## Branch Strategy

```
main ─────────────────────── Production releases
  │
  ├── develop ─────────────── Integration branch
  │     │
  │     ├── feature/xxx ──── Feature branches
  │     ├── fix/xxx ──────── Bug fixes
  │     └── core/xxx ─────── Core-specific changes
  │
  └── hotfix/xxx ──────────── Emergency fixes → merge to main + develop
```

## Docker Builds (Dashboard)

```dockerfile
FROM node:22-alpine AS builder
WORKDIR /app
COPY dashboard/package*.json ./
RUN npm ci
COPY dashboard/ ./
RUN npm run build

FROM node:22-alpine AS runner
WORKDIR /app
ENV NODE_ENV=production
COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static
COPY --from=builder /app/public ./public
EXPOSE 3000
CMD ["node", "server.js"]
```
