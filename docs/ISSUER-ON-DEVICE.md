# On-device offline issuer — design & security notes

> Companion app: `android/license-manager`, `issuer` product flavor.
> Spec anchor: `license/SPEC.md` → "On-device issuer".
> Status: shipped in License Manager v1.1.0 (versionCode 105).

## Why it exists

License issuance used to require the GitHub Actions `issue-license.yml`
workflow (`workflow_dispatch` + `LICENSE_SIGNING_KEY` repository secret).
That has two operational problems: GitHub itself can become unreachable
(or the account/workflow gets locked), and issuance then stops entirely.
The issuer variant moves signing onto the maintainer's own phone: the same
tokens, produced by the same algorithm, with zero network dependency at
any point — no GitHub, no server, no laptop.

## The one-codebase, two-capabilities rule

`:license-manager` builds from a single source tree with two product
flavors on a `capability` dimension:

```
src/main    verifier UI + shared verification code (v1.0.4 behavior, untouched)
src/verifier  (reserved; empty today)
src/issuer    issuer UI + vault + engine glue + issuer-only strings/manifest
```

- **`verifier`** (the default flavor) is the public APK. Its behavior is
  byte-for-byte the v1.0.4 flow: same launcher, same screens, same
  permissions (none).
- **`issuer`** (applicationId `com.vor.licensemanager.issuer`,
  versionName `1.1.0-issuer`) is the maintainer's private build. It also
  CONTAINS the verifier screens — reachable from a "Verify a token" menu
  action — because a maintainer should see exactly what a license holder
  sees. The reverse (issuer code inside the verifier APK) is forbidden and
  mechanically enforced (below).

### The purity gate (public APK must never contain issuer code)

Three independent layers, all wired into CI:

1. **Compile-time scoping.** The signing engine lives in its own module
   `:core-license-issuer`, and the license-manager build wires it
   exclusively with `"issuerImplementation"(...)` — the verifier variant
   cannot even resolve issuer symbols, let alone link them.
2. **`verifyVerifierPurity` Gradle task.** Finalizes every
   `assembleVerifier{Debug,Release}`: unzips each verifier APK and scans
   dex files, resources.arsc, assets and XML entries for issuer markers
   (the two issuer package names, the `vor_issuer_wrap` / `vor_issuer_seed`
   / `vor_issuer_store` identifiers, `androidx/biometric`, and the
   issuer-only UI strings). Any hit fails the build with the exact
   location. Markers were chosen by diffing against a real v1.0.4 APK:
   BouncyCastle (whole-jar, already shipped for offline verification,
   including its unused `Ed25519PrivateKeyParameters` and
   `Argon2BytesGenerator` classes) is deliberately NOT a marker — an
   unused library class is not an issuer code path.
3. **`VerifierPurityTest`** (runs in both variants' unit-test tasks):
   reflection-asserts that issuer classes are absent from the verifier
   classpath and present on the issuer's.

### The permission allowlist gate

`scripts/audit-apk-permissions.sh` gained a `license-manager-issuer`
module: exactly `USE_BIOMETRIC` (from the androidx.biometric AAR) and
`USE_FINGERPRINT` (the AAR's pre-API-28 fallback declaration for our
minSdk 26). The verifier allowlist remains **empty**. Release workflows
route each APK to its module's allowlist, and the release job builds the
License Manager with `:license-manager:assembleVerifierRelease` only —
the issuer APK is never built into, or staged for, a public release, and
an explicit check fails the release if one ever appears in the outputs.

## Key custody

```
┌───────────────────────────────────────────────────────────────┐
│ Android Keystore                                              │
│  AES-256-GCM key "vor_issuer_wrap"                            │
│  setUserAuthenticationRequired(true)                           │
│  setUserAuthenticationValidityDurationSeconds(60)              │
└───────────────┬───────────────────────────────────────────────┘
                │ encrypts/decrypts (only within 60 s of unlock)
┌───────────────▼───────────────────────────────────────────────┐
│ filesDir/vor_issuer_seed.bin  = [12-byte IV][AES-GCM(ct+tag)] │
│ (the 32-byte Ed25519 seed — never in the clear anywhere)       │
└───────────────────────────────────────────────────────────────┘
```

- Android Keystore cannot hold Ed25519 keys, so the seed is software
  Ed25519 (BouncyCastle — the same library every Vor client already
  ships for offline verification; the issuer adds **zero** new crypto
  dependencies) and the *wrapper* is the hardware-backed Keystore key.
- Signing, backup, and key installation run through `IssuerOps.
  withAuthRetry`: if the 60-second keyguard window elapsed
  (`UserNotAuthenticatedException`), the BiometricPrompt gate re-arms and
  the operation retries once.
- `KeyPermanentlyInvalidatedException` (lock screen changed in a way
  that invalidates the key) surfaces as an explicit "the stored seed can
  no longer be unwrapped — restore from backup" state, never a crash.
- The gate accepts STRONG biometrics OR device credential (PIN/pattern/
  password): on API 30+ `BIOMETRIC_STRONG|DEVICE_CREDENTIAL`; below that
  `BIOMETRIC_WEAK|DEVICE_CREDENTIAL` (the androidx.biometric compat
  path). A device with no secure lock screen is blocked from issuing at
  all — the gate screen explains how to set one.

### Migration from the GitHub secret

`Keys → Import seed` accepts the exact base64url 43-character seed format
used by the `LICENSE_SIGNING_KEY` repository secret. After import, the
built-in **self-test** signs a canary token and verifies it against both
this device's derived public key and the public key embedded in the
build, reporting honestly:

- PASS — issued tokens verify in this app family;
- PASS with warning — signs fine but does NOT match the embedded key:
  tokens verify only in builds embedding the matching public key;
- FAIL — a real defect (this combination should be impossible).

## Backup format (v1)

```json
{"v":1,"format":"vor-issuer-key-backup","app":"vor-license-manager-issuer",
 "kdf":{"alg":"argon2id","t":3,"m_kib":65536,"p":1,"salt_b64":"…"},
 "wrap":"aes-256-gcm","iv_b64":"…","seed_enc_b64":"…",
 "pub_b64url":"…","created_at":"…"}
```

- Argon2id via BouncyCastle (`Argon2BytesGenerator`, v1.3 / 0x13),
  pinned against argon2-cffi golden vectors (incl. UTF-8 passphrase
  cases) so the digest is bit-for-bit the reference.
- AES-256-GCM (12-byte IV, 128-bit tag) over the 32-byte seed; the
  envelope carries the recorded public key and decrypt cross-checks that
  the recovered seed still derives it.
- Export/import only through the system file picker (SAF). No network
  permission exists in either variant — "never auto-uploaded" is not a
  promise, it is a build property.
- Restoring over an existing key warns first (overwrite semantics).
- The passphrase is never stored; losing it loses the backup. The dialog
  enforces ≥8 characters and confirmation-typing on creation.

## Issuance semantics

- Token format and canonical bytes are a byte-exact port of
  `license/python/vor_license.py` (compact separators, field order
  `v,id,product,issued_at,expires_at,entitlements`, entitlement extras
  sorted, `ensure_ascii=False` UTF-8, Python-compatible `\u00xx`
  escaping). Pinned by golden vectors produced WITH the repository's own
  reference tool — including Persian IDs/tiers, quote/backslash escaping,
  empty platform lists, and extras ordering — plus RFC 8032 test vectors
  1-2 independently confirmed against OpenSSL.
- License ID: free text (default: random UUID, with a regenerate button).
- Tier: free text with a LOCAL, editable suggestion list (`standard` by
  default — the reference tool's default; no hardcoded enum).
- Expiry: 7/30/90/365/5×365-day presets (end of local day, converted to
  UTC) or an arbitrary date + optional time-of-day picker. The signed
  RFC 3339 value is previewed before signing. Issuing a license already
  expired at signing time is refused (validation error, not a warning).
- Platforms: the five supported platforms, all on by default.
- Notes: local-only, stored in the history record, never in the token.
- Batch: `id[,tier][,expires]` lines (comments `#`, blank lines skipped),
  per-line validation errors reported with line numbers, one token per
  line output as `tokens.txt` and/or a ZIP of per-license QR PNGs (plus
  the tokens.txt inside) — all via SAF.
- History ("Issued by this device"): id, tier, issued-at, expiry, token,
  notes, with per-entry QR/copy and a clear-all. The non-revocation
  disclaimer is rendered directly under the header.

## Threat model — what this does and does not give you

Honest, in the same spirit as `license/SPEC.md`:

- A lost/stolen unlocked phone can issue licenses until the key is wiped
  or rotated — short validity windows are the actual damage control.
- A compromised (rooted/malware) OS can observe the seed during signing
  and read the local history. This design makes casual extraction hard
  (Keystore-wrapped at rest, auth-window-gated, biometric-per-open) but
  cannot defend a fully compromised host.
- No revocation exists anywhere in the Vor license system (online or
  off). The History tab is a ledger, not a kill switch.
- Backups are only as strong as their passphrase + Argon2id work factors
  (t=3, m=64 MiB, p=1 — roughly a second on a modern phone).
- Issuer-variant APKs must never be distributed publicly. Besides policy,
  the release pipeline mechanically refuses to stage one.

## Test coverage added

- `:core-license-issuer` — golden vectors (token parity vs the Python
  reference; RFC 8032; Argon2id vs argon2-cffi; seed→pub vs OpenSSL),
  backup round-trip / wrong passphrase / tamper / self-check failures,
  CSV parsing edge cases (Persian fields, offsets, control chars, line
  numbers).
- `:license-manager` — `VerifierPurityTest` (both variants),
  `IssuerStoreTest` + `IssuerTime` (flavor-scoped `src/testIssuer`).
- Build gates — `verifyVerifierPurity` on every verifier assemble; the
  permission audit for both variants' APKs; release staging guard
  against issuer APKs.
