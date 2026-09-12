# vor-drm — the Vor offline license DRM core (Rust)

Native, air-gapped license enforcement for every Vor platform. One crate,
two deliberately separated cores:

| Core | Feature | Ships in | Capability |
|---|---|---|---|
| **Client** | `client` / `jni-client` | the Vor VPN app | **verify** (never sign): VOR1 JSON + VOR2 hardware-locked encrypted envelopes |
| **Manager** | `manager` / `jni-manager` | Vor License Manager (issuer flavor only) | **issue**: key generation, single + batch issuance, Argon2id portable vault |

The public verifier APK is built WITHOUT the manager feature — signing code
is structurally absent from it (enforced additionally by the Android
`verifyVerifierPurity` gate).

## Security layers

1. **Ed25519** signatures over the exact envelope/payload bytes;
2. **AES-256-GCM** payload confidentiality, key = HKDF-SHA256(app secret,
   salt = issuer public key);
3. **Hardware binding** — SHA-256 over domain-separated, length-prefixed
   (Widevine MediaDRM id, ANDROID_ID, Build fingerprint), constant-time match;
4. **Anti-clock-rollback** — `max(device, uptime-projected wall, persisted
   ratchet, trusted sample, signed issued_at)`;
5. **Anti-analysis** — TracerPid, /proc maps framework scan, frida port
   probe, dump hardening, strike escalation (manager aborts, client fails
   the verify);
6. **Obfuscation** — `obfstr` compile-time string encryption + the `cff!`
   control-flow-flattening macro (compile-time-random keyed dispatcher).

Honest limits (documented, not hidden): client-side DRM raises the bar; it
cannot beat an attacker with root and a patched binary. This core removes
the cheap attacks and makes the remaining ones expensive, deliberate and
device-specific.

## Token formats

* **VOR1** — `VOR1.<b64url(json)>.<b64url(sig)>`: the cross-platform format
  every Vor port already verifies (shared conformance:
  `license/vectors.json`).
* **VOR2** — `VOR2.<b64url(envelope)>.<b64url(sig)>` where
  `envelope = header(4) || nonce(12) || AES-256-GCM(payload || tag)`. The
  canonical binary payload carries expiration, bandwidth limit, tier,
  platforms and the locked HWID (see `src/token.rs` for the byte layout).

## Build

```sh
cargo build --features "client manager"     # both cores (tests)
cargo build --features jni-client           # libvor_drm.so for the VPN app
cargo build --features jni-manager          # libvor_drm.so for the issuer
```

Zero warnings under `cargo clippy --all-targets -- -D warnings` across every
feature combination — keep it that way.

### Air-gapped builds

All dependencies are pure Rust and locked by `Cargo.lock`. To build on a
machine with no network:

```sh
cargo vendor vendor
mkdir -p .cargo
printf '[source.crates-io]\nreplace-with = "vendored-sources"\n\n[source.vendored-sources]\ndirectory = "vendor"\n' > .cargo/config.toml
cargo build --offline --features jni-client
```

(The vendor tree is ~115 MB for all platforms, so it is intentionally NOT
committed; regenerate it on demand.)

### Android .so

Cross-compile with cargo-ndk and drop the result into the app's `jniLibs`:

```sh
cargo ndk -t arm64-v8a -t armeabi-v7a -o ../android/app/src/main/jniLibs \
  build --release --features jni-client
```

The Kotlin bridges degrade gracefully when the library is absent — stock
builds keep the pure-Kotlin verifier with zero behavior change:
`android/app/.../data/license/VorDrmClient.kt` (VPN app) and
`android/license-manager/src/issuer/.../drm/VorDrmManager.kt` (issuer).

## Tests

```sh
cargo test --features "client manager"        # unit suites (both cores)
cargo test --features "client manager" --test conformance
```

The conformance suite reads the shared `license/vectors.json` (14 vectors,
including the `impossible_day` calendar case) and pins this implementation
to the same verdicts as the Python reference, the Kotlin verifier, iOS and
the desktop Go port.

## Release overrides

* `VOR_DRM_APP_SECRET=<64 hex>` — the VOR2 payload secret (development
  default otherwise; manager and client .so of one release train must be
  built with the same value);
* `VOR_DRM_PUBLIC_KEY=<base64url>` — baked default verifying key (the
  Android bridge passes the BuildConfig key instead, so gradle stays the
  single source of truth in CI).
