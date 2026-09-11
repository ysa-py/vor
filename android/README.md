# Vor

[![Release](https://img.shields.io/github/v/release/ysa-py/Vor?include_prereleases)](https://github.com/ysa-py/Vor/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Android](https://img.shields.io/badge/Android-8.0%2B-green.svg)](https://www.android.com/)
[![Kotlin](https://img.shields.io/badge/Kotlin-2.x-7F52FF.svg)](https://kotlinlang.org/)

**Vor** (formerly V2RayEZ) is an Android VPN / proxy client. It speaks VLESS, VMESS, Trojan, and Shadowsocks through a vendored Xray core, with optional on-demand engine packs for Tor, pluggable transports, Psiphon, ByeDPI, DNS-tunnel, and more. Offline license gate, adaptive routing, and an on-device AI-assisted route engine round it out.

| | |
|---|---|
| Platform | Android 8.0+ (`minSdk 26`), `targetSdk 35` |
| Stack | Kotlin, Jetpack Compose, Hilt, Room, DataStore |
| Core | Xray (`libv2ray.aar`) + real `VpnService` |
| Identity | display name **Vor**, package `com.v2rayez.app` (kept so every release installs as a plain UPDATE over your existing app — no new-app Play Protect flow) |
| UI languages | English · فارسی · Русский |

---

## English

### What you get

- Share-link, QR, file, and subscription import
- Per-app proxy, routing/DNS (incl. Iran bypass when geo assets are installed)
- In-app Browser and MITM / domain-fronting tools (advanced)
- Tor + pluggable transports, Psiphon, DNS-tunnel via [Core manager](#addon-packs) when packs are installed
- Foreground notification, Quick Settings tile, boot auto-connect
- Offline license verification with clock-rollback protection (monotonic ratchet + trusted HTTPS time)
- License Manager: public verifier APK + a private `issuer` build variant that signs licenses fully on-device (see `docs/ISSUER-ON-DEVICE.md`)

### Install

1. Open the latest [Release](https://github.com/ysa-py/Vor/releases).
2. Download `Vor-v*-android-arm64-v8a-release.apk` (64-bit phones) or the `armeabi-v7a` build and install it (unknown sources allowed). If you already have any 1.0.x build installed, this lands as a normal app update — same package, same signing certificate, higher version code.
3. Optional: in the app open **Core manager** and install addon zips from the same Release.

Current release: [`vor-v1.0.4`](https://github.com/ysa-py/Vor/releases/latest).

### Addon packs

Release assets look like `tor-arm64-v8a.zip`, `lyrebird-arm64-v8a.zip`, `psiphon-arm64-v8a.zip`, `dnstunnel-arm64-v8a.zip`, …
The app resolves them from `ysa-py/Vor` at the release tag being installed (overridable with Gradle properties `v2rayez.addons.githubRepo` / `v2rayez.addons.releaseTag`). Local dev uses `gradle.properties`; CI release builds pass the publish tag automatically and attach the zips to that same release. Checksums are in `SHA256SUMS.txt`.

### Release notes — v1.0.4

- **One identity:** the app keeps the package name `com.v2rayez.app`, so every release installs as an update over the existing app instead of a brand-new install (which is what re-triggered Play Protect's unknown-developer block).
- **Clock-rollback protection:** license verification runs against `max(device clock, persisted monotonic ratchet, trusted HTTPS time)`; winding the clock back can no longer resurrect an expired license. Verification stays fully offline.
- **New brand icon** on Android (adaptive + themed monochrome) and the License Manager.
- **Self-hosted addon pipeline:** engine packs resolve from this repo's own releases.

### License Manager release notes — v1.1.0

- **On-device offline issuer** (`issuer` product flavor, private maintainer build): sign Vor licenses on the phone with no GitHub Actions, no server, no network. Keystore-wrapped seed + BiometricPrompt gate on every open; byte-compatible with the reference issuance tool; batch CSV to tokens/QR ZIP; Argon2id passphrase backups; build-time purity gate keeps issuer code out of the public APK (details: `docs/ISSUER-ON-DEVICE.md`).

### Release notes — v1.0.3

- Mid-session license expiry enforcement: while connected, the signed expiry claim is re-verified at most once a minute — fully offline — and the tunnel tears down automatically when it stops verifying.

### Release notes — v1.0.2

- **Crash-proof license activation** and automatic expiry revocation at launch/foreground/connect. License Manager 1.0.2: multi-license store with QR import/export.

### Release notes — v1.0.1

- Play Protect hardening: removed `QUERY_ALL_PACKAGES` and all advertising-identifier permissions; CI permission allowlist gate on every shipped APK.

### Release notes — v1.0.0

- First stable release. Xray TUN routing, domain fronting, per-app proxy UI, signed release pipeline.

### Build from source

```bash
cp app/google-services.json.example app/google-services.json   # then fill Firebase config
./gradlew assembleDebug
./gradlew testDebugUnitTest
```

Release signing needs a local `keystore.properties` pointing at your keystore. Neither that file nor `app/google-services.json` is committed.

### Privacy

- Firebase Crashlytics, Performance Monitoring, and Analytics are the always-on DevOps telemetry channels.
- Hosts, URIs, bridges, IPs, certificates, fingerprints, and subscription bodies are scrubbed before upload.
- Real Firebase config comes from local `app/google-services.json` or the CI `GOOGLE_SERVICES_JSON` secret, not source.
- The license clock's trusted-time check contacts a small fixed set of independent HTTPS endpoints (Google/Cloudflare/Microsoft/GitHub/Apple) to read their `Date` headers — headers only, no payload, no identifiers, at most a couple of times a day.

### License

[MIT](LICENSE)

---

## فارسی

**Vor** (V2RayEZ سابق) کلاینت VPN / پروکسی اندروید است و پروتکل‌های VLESS، VMESS، Trojan و Shadowsocks را با هسته Xray پشتیبانی می‌کند. بسته‌های اختیاری Tor، Psiphon، تونل DNS و ابزارهای مرتبط از Releases نصب می‌شوند. تأیید لایسنس کاملاً آفلاین است و در برابر تغییر عقب ساعت هم مقاوم شده است.

### نصب

1. از صفحه [Releases](https://github.com/ysa-py/Vor/releases) فایل `Vor-v*-android-arm64-v8a-release.apk` را بگیرید و نصب کنید (گوشی‌های ۳۲بیتی: نسخهٔ `armeabi-v7a`).
2. اگر از قبل هر نسخهٔ 1.0.x را نصب کرده‌اید، این نسخه به‌صورت **بروزرسانی عادی** نصب می‌شود — همان شناسهٔ بسته و همان گواهی امضا؛ تنظیمات و سرورهای شما حفظ می‌شود.
3. برای بسته‌های افزونه، در اپ به **مدیر هسته** بروید یا زیپ‌های همان Release را نصب کنید.

نسخه فعلی: [`vor-v1.0.4`](https://github.com/ysa-py/Vor/releases/latest).

### ساخت

```bash
cp app/google-services.json.example app/google-services.json
./gradlew assembleDebug
```

فایل‌های `keystore.properties` و `google-services.json` در مخزن نیستند.

### حریم خصوصی

Firebase Crashlytics، Performance Monitoring و Analytics همیشه برای دید عملیاتی فعال هستند؛ آدرس‌ها و اطلاعات حساس پیش از ارسال پاک‌سازی می‌شوند. بررسیِ «زمانِ مطمئن» لایسنس تنها هدر تاریخِ چند سرویس مستقل HTTPS را می‌خواند — بدون هیچ دادهٔ ارسالی.

### مجوز

[MIT](LICENSE)
