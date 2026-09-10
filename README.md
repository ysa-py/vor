# Vor

**One unified cross-platform anti-censorship platform** — a careful
consolidation of 8 independently developed repositories into a single
product, built to keep working under Iran-grade DPI.

| Platform | Artifact | Where |
|---|---|---|
| Android | signed APK + License Manager companion app | [`android/`](android/) |
| Windows | single portable `.exe` (+ WinDivert files) | [`desktop/`](desktop/) |
| Linux | binary + `.deb` + AppDir archive | [`desktop/`](desktop/) |
| OpenWrt | **universal** `.ipk` (arch matrix) | [`openwrt/`](openwrt/) |
| iOS | unsigned `.ipa` for **TrollStore** (see [ios/README.md](ios/README.md) for the honest compatibility statement) | [`ios/`](ios/) |

> **Merge provenance:** every feature of all 8 source repos (V2RayEZ,
> MICAFP, AetherGUI/Aethon, EasySNI, UAC-SNI-Spoofer-Android,
> UAC-SNI-Spoofer-Windows, MSN-GUARD, MasterDnsVPN) is accounted for in
> [docs/MERGE-PLAN.md](docs/MERGE-PLAN.md) — ported, adapted, or flagged
> with the reason and the preserved source location. Nothing was silently
> dropped.

---

## Architecture

```
┌────────────────────────────────────────────────────────────────────┐
│                     core/vor-core (Rust, shared)                   │
│   THE single-source decision library for every platform:           │
│   UCB1 bandit · Q-learning champion cache · threat ladder (FAVA    │
│   v1/v2) · Iranian ISP profiles + UAC carrier presets (shared      │
│   JSON) · ClientHello fragment planner (SniSplit / RecordSplit /   │
│   fullN / tls_records / ...) · traffic shaper · NAIN detector ·    │
│   offline Ed25519 license verification                             │
│   Surface: C ABI (ffi.rs) · JNI (jni.rs) · stateless JSON in/out   │
└───────┬──────────────┬──────────────┬──────────────┬───────────────┘
        │              │              │              │
  ┌─────▼─────┐  ┌─────▼─────┐  ┌─────▼─────┐  ┌─────▼─────┐
  │  android/ │  │  desktop/ │  │ openwrt/  │  │   ios/    │
  │ V2RayEZ   │  │ EasySNI   │  │ C daemon  │  │ SwiftUI   │
  │ base app  │  │ Go base + │  │ + netifd  │  │ app +     │
  │ + gates + │  │ pattern   │  │ proto +   │  │ SOCKS     │
  │ engines   │  │ engine    │  │ LuCI      │  │ engine    │
  └───────────┘  └───────────┘  └───────────┘  └───────────┘
        │              │              │
        └──────────────┴──────┬───────┘
                       ┌──────▼────────┐
                       │ dns-tunnel/   │  MasterDnsVPN (Go) client+server
                       │ engine        │  + V2RayEZ's dnstt — both selectable
                       └───────────────┘
```

### The transport-engine abstraction (how to add a new engine)

Every platform exposes the same **engine selector**: `Auto (AI)` or manual.
An engine is anything that can turn "connect to this profile" into a
working tunnel, and it registers itself in one place per platform:

1. **Add the engine id** to the selector vocabulary
   (`core/vor-core/src/selector.rs` `KNOWN_ENGINES` and `engine.rs` catalog,
   mirrored by `desktop/internal/engine` and
   `android/.../data/ai/VorCoreKt.kt`).
2. **Implement the engine** on the platform (process core, in-process
   library, relay — the existing engines show every pattern:
   in-process Xray AAR, sing-box process core, SNI-tunnel relay, pattern
   WinDivert injector, DNS-tunnel subprocess, Tor, Psiphon, MITM fronting).
3. **Report outcomes** to the adaptive layer (`observe()` with
   success/rtt/dpi_kill/throughput). The bandit + Q-table learn per network
   fingerprint; the next `decide()` call prefers what worked.

The decision model is **one implementation in Rust** with vector-locked
ports in Kotlin and Go. All ports run the *same* shared conformance vectors
(`core/vor-core/tests/vectors/decision-vectors.json`,
`license/vectors.json`) in CI, so they provably agree — the honest way to
share one brain across five platforms without forking it into drifting
copies. When the native `libvor_core.so` is present (Android CI builds it),
the JNI path is the production fast path; otherwise the pure interpreter
runs (unit-testable everywhere).

### The Windows engine conflict — resolved, not hidden

`EasySNI` (Go) and `UAC-SNI-Spoofer-Windows` (Python) both implement
Windows SNI spoofing. Vor keeps **both**, runtime-switchable:

* **Default: the Go desync engine** (EasySNI) — fragmentation + fake
  ClientHello + wrong checksum/seq, one static binary, ~15–40 MB.
* **Alternate: the pattern engine** (`desktop/internal/pattern`) — the UAC
  fragment matrix + Patterniha wrong-sequence WinDivert injection, ported
  to Go. Deeper carrier tuning (MCI/Irancell presets), handshake
  choreography, hybrid mode.
* **Third path: Xray finalmask** via the config emission + Edge Bridge
  (core-independent external finalmask).

Full tradeoff analysis (performance, binary size, dependency footprint,
detection resistance): [docs/ENGINES.md](docs/ENGINES.md).

### Offline license system

Ed25519-signed tokens, verified **offline** on every platform with the
same shared conformance vectors; issuance via a manually-triggered GitHub
Actions workflow (private key only in encrypted secrets); a dedicated
License Manager Android app; the main app's **first screen is the license
gate** and expiry re-locks automatically. Spec:
[license/SPEC.md](license/SPEC.md).

Stated plainly: client-side checks are reverse-engineerable with enough
effort. This raises the bar; it is not "unbreakable".

### CI/CD

* [`.github/workflows/ci.yml`](.github/workflows/ci.yml) — per-platform
  test + lint on every push/PR (fails loudly).
* [`.github/workflows/release.yml`](.github/workflows/release.yml) — one
  tag `vor-v*` → all five platform artifacts on one GitHub release, with a
  hard "missing platform artifact = release fails" gate.
* [`.github/workflows/issue-license.yml`](.github/workflows/issue-license.yml)
  — the license-issuance workflow (workflow_dispatch).

## Repository layout

| Path | Contents |
|---|---|
| `android/` | Vor app (V2RayEZ base) + `:core-license` + `:license-manager` |
| `desktop/` | Vor Desktop (EasySNI Go base) + pattern engine + engine selector + unified dashboard |
| `core/vor-core/` | the shared Rust library (C ABI + JNI + conformance vectors) |
| `core/ai-models/` | MICAFP's Python model-training pipeline (tooling — no trained binaries exist upstream; Vor's AI is the on-device adaptive engine) |
| `dns-tunnel/` | MasterDnsVPN (Go) client + server |
| `openwrt/` | universal OpenWrt package (C daemon + netifd proto + LuCI) |
| `ios/` | TrollStore target (read its README before promising iOS compatibility) |
| `license/` | token spec + reference implementation + vectors + dev keys |
| `docs/` | MERGE-PLAN (100% feature accounting), ENGINES, architecture |
| `extras/` | full preserved sources of Aethon, MICAFP, MSN-GUARD, UAC Android/Windows (every flagged feature's reference implementation) |

## Licensing

Mixed per-directory (each imported component keeps its license):
V2RayEZ/EasySNI = MIT, MasterDnsVPN = per its LICENSE, vor-core (from
MICAFP) = GPL-3.0, MSN-GUARD/Aethon = AGPL-3.0, UAC pattern engine =
GPL-3.0 (patterniha lineage). See `LICENSE*` files in each directory and
`docs/MERGE-PLAN.md`. The maintainer is pseudonymous; no paid
infrastructure is required (GitHub Actions free tier, no license server,
no database).

## Building locally

```bash
# shared core (Rust)
cd core/vor-core && cargo test

# desktop (Go)
cd desktop && go test ./... && go run . -open=false

# android (needs Android SDK; see android/local.properties)
cd android && ./gradlew :app:testDebugUnitTest :app:assembleDebug

# openwrt daemon + conformance (any Linux with gcc)
cd openwrt/vor/src && gcc -Wall -Wextra -Werror -DVOR_PLANNER_ONLY \
  -o test_vor_planner vor_main.c test_vor_planner.c && ./test_vor_planner

# license reference (Python)
pip install cryptography && python license/python/test_vectors.py

# iOS (needs macOS + Xcode + xcodegen)
cd ios && xcodegen && scripts/build-ipa.sh
```
