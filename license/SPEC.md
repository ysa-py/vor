# Vor Offline License System — Specification (v1)

## Design goals

- **Fully offline verification.** No license server, no database, no network call ever
  required to validate a license. The VPN client embeds only a **public key**.
- **Tamper-evident.** Tokens are Ed25519-signed; any modification of the payload
  invalidates the signature.
- **Expiry-enforced.** The payload carries `expires_at`; clients re-check on every
  launch and periodically while running, and re-lock the UI automatically when the
  license expires.
- **Manually-triggered issuance.** Licenses are signed by a GitHub Actions workflow
  (`workflow_dispatch`) that reads the Ed25519 private key from GitHub encrypted
  secrets. No continuously-running server.
- **Honesty.** Client-side checks are reverse-engineerable with enough effort. This
  design raises the bar (signed, offline, tamper-evident) but is **not literally
  unbreakable**. Do not represent it as "100% forgery-proof".

## Token format

```
VOR1.<base64url(payload_json)>.<base64url(signature_64_bytes)>
```

- `VOR1` — literal prefix, version 1 of the envelope.
- `base64url` — RFC 4648 §5, **without padding** (`=` stripped).
- `signature` — Ed25519 signature over the **exact raw bytes** of the JSON payload
  (the base64url-decoded middle segment), produced with the issuer private key.

## Payload (JSON, UTF-8)

```json
{
  "v": 1,
  "id": "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0",
  "product": "vor",
  "issued_at": "2026-09-10T00:00:00Z",
  "expires_at": "2027-09-10T00:00:00Z",
  "entitlements": {
    "tier": "standard",
    "platforms": ["android", "windows", "linux", "openwrt", "ios"]
  }
}
```

Field rules:

| Field | Type | Required | Notes |
|---|---|---|---|
| `v` | int | yes | Payload format version. Must be `1`. |
| `id` | string | yes | Opaque unique license ID (UUID recommended). |
| `product` | string | yes | Must be `"vor"`. Tokens for other products are rejected. |
| `issued_at` | string | yes | RFC 3339 UTC timestamp. |
| `expires_at` | string | yes | RFC 3339 UTC timestamp. **Hard deadline.** |
| `entitlements` | object | no | Feature/platform flags. Unknown keys ignored. |

JSON is emitted by the reference tools in a **canonical form**: compact
(no insignificant whitespace), field order as listed above, UTF-8. Verifiers MUST
NOT re-serialize before checking the signature — they verify the raw payload
bytes as received, so any producer variation is still verifiable, only the
official tools emit canonical form.

## Verification algorithm (identical on every platform)

```
verify(token, pubkey, now) -> {status, payload}
  1. Split token on '.' -> must yield exactly 3 parts; parts[0] == "VOR1".
  2. b64url_decode(parts[1]) -> payload_bytes (must be valid UTF-8 JSON)
     b64url_decode(parts[2]) -> sig (must be exactly 64 bytes)
  3. Ed25519_verify(pubkey, payload_bytes, sig) -> else INVALID
  4. Parse JSON; require v==1 and product=="vor" -> else INVALID
  5. expires = parse RFC3339(expires_at) -> else INVALID
  6. If now >= expires: status = EXPIRED (payload still returned for display)
     else: status = VALID
```

Clock-skew note: verification is **strict**. A device clock set past `expires_at`
locks the app; a device clock far in the past extends validity only until the
next correct reading. Clients additionally warn the user when the device clock
looks implausible (before 2020 or after 2040) so honest users are not confused.

Statuses: `VALID`, `EXPIRED`, `INVALID` (malformed / bad signature / wrong product).

## Key management

- Algorithm: **Ed25519** (RFC 8032).
- **Production private key** lives ONLY in the GitHub repository secret
  `LICENSE_SIGNING_KEY` (base64 of the 32-byte seed). It is never committed,
  never shipped in any client binary.
- The **production public key** (32 bytes, base64) is committed at
  `license/keys/prod/VOR_LICENSE_PUBLIC_KEY.txt` and embedded in every client
  (Android, Windows/Linux desktop, OpenWrt, iOS, License Manager).
- **Dev keys** (`license/keys/dev/`) are committed for local testing and CI test
  vectors only. They are clearly named and are not used by the issuance workflow.
- Rotating keys: generate a new pair, add the new public key to the trusted set
  (clients accept a list), re-issue licenses, drop the old key in a later release.

## Issuance

GitHub Actions workflow `.github/workflows/issue-license.yml`:

- Trigger: `workflow_dispatch` with inputs `expires_at` (required, RFC 3339),
  `license_id` (optional, generated if absent), `tier` (optional, default
  `standard`), `use_dev_key` (optional boolean, default false — CI tests use it).
- Reads `LICENSE_SIGNING_KEY` secret (production) or the committed dev key.
- Signs via `license/python/vor_license.py issue`.
- Prints the token in the workflow summary and uploads it as a workflow artifact
  `vor-license-<id>.txt`. Nothing is persisted anywhere else.

## Client integration requirements

1. **License gate first screen.** The main app's first screen (before any main
   UI) is a license-entry gate: paste/import token → verify locally → unlock.
2. **Re-lock on expiry.** Re-verified at every launch and at least every 24h
   while running; on `EXPIRED` the app re-locks to the gate automatically.
3. **No network.** Verification never touches the network.
4. **License Manager companion app** (Android, `android/license-manager/`):
   import/paste a token, verify, display status (valid / expired / invalid),
   one primary action "Check / Activate License". It shares the same public key
   and the same verification code as the main app.

## Test vectors

`license/vectors.json` is generated from the reference implementation and is
consumed by the test suites of every port (Go, Kotlin, Rust) so all platforms
provably agree on accept/reject decisions.

## Limitations (stated plainly)

- Offline checks can be patched out of an open-source client by a determined
  attacker with the sources. Signature verification raises effort and makes
  tampering detectable, but cannot make it impossible.
- The dev keys committed here are for tests; anyone can forge tokens that verify
  against the **dev** key. Clients must embed the **production** public key in
  release builds (a Gradle/build-script constant and a Go/Rust build flag),
  with dev-key acceptance only in debug builds.
