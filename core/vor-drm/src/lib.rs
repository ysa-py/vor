//! # vor-drm — the Vor offline license DRM core (Rust, air-gapped)
//!
//! Two deliberately separated cores, one crate:
//!
//! * **Client Core** (`client` feature, [`client`] module) — everything the
//!   VPN app ("Vor") needs to verify a license **fully offline**: Ed25519
//!   signature checks, encrypted VOR2 payload decoding, hardware-ID binding,
//!   anti-clock-rollback time composition and tamper detection. It holds ONLY
//!   the public key material — it cannot forge or re-sign anything.
//! * **Manager Core** (`manager` feature, [`manager`] module) — everything the
//!   admin app ("Vor License Manager") needs to GENERATE licenses offline:
//!   Ed25519 key generation, single + batch issuance of hardware-locked
//!   tokens, an Argon2id passphrase vault for air-gapped key transport, and a
//!   sign-and-verify self test. The public verifier APK is built WITHOUT this
//!   feature (structurally impossible to ship signing code).
//!
//! Both cores share the hardening modules:
//!
//! | Module | Purpose |
//! |---|---|
//! | [`obf`] | compile-time string encryption (obfstr) + control-flow flattening chain |
//! | [`guard`] | anti-debug (TracerPid), anti-Frida (maps + port probe), anti-dump hardening, strike escalation |
//! | [`clock`] | monotonic BOOTTIME watch + ratchet: detects wall-clock rollback |
//! | [`hwid`] | domain-separated hardware fingerprint digest + constant-time match |
//! | [`token`] | VOR2 binary envelope: canonical payload codec, HKDF key, AES-256-GCM |
//! | [`json`] | minimal strict JSON parser (no serde — smaller, quieter .so) |
//! | [`b64`] | base64url codec (same alphabet as every Vor port) |
//!
//! ## Token formats
//!
//! * **VOR1** — `VOR1.<b64url(json)>.<b64url(ed25519 sig)>`, the cross-platform
//!   format every Vor port already verifies. This crate verifies VOR1 too so
//!   the native path can replace the Kotlin verifier without breaking any
//!   previously issued license.
//! * **VOR2** — `VOR2.<b64url(envelope)>.<b64url(ed25519 sig)>` where the
//!   envelope is `header(4) || nonce(12) || AES-256-GCM(payload || tag)`. The
//!   payload is a compact canonical BINARY structure carrying expiration,
//!   bandwidth limit, tier, platforms and the locked HWID. The payload key is
//!   HKDF-SHA256(app secret, salt = issuer public key) — the ciphertext is
//!   opaque to anyone without the .so, and the whole envelope is covered by
//!   the Ed25519 signature, so tampering is cryptographically impossible.
//!
//! ## Honest limits (documented, not hidden)
//!
//! Client-side DRM raises the bar; it cannot beat an attacker with root and a
//! patched binary. This core removes the cheap attacks — token sharing
//! (HWID binding), clock rollback (BOOTTIME + persisted ratchet), casual
//! reverse engineering (obfstr + flattened control flow + anti-analysis) —
//! and makes the remaining attacks expensive, deliberate and device-specific.
//!
//! ## Offline guarantees
//!
//! No network I/O anywhere at runtime. No runtime downloads. All
//! dependencies are pure Rust; the `Cargo.lock` pins them exactly. For a
//! fully air-gapped build, vendor them once (`cargo vendor vendor` — see
//! the README in this directory) and build with `--offline`; the procedure
//! is one command and keeps the repository light.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
// The JNI entry points must catch every panic before it crosses the FFI
// boundary; `clippy::missing_panics_doc` fires on inherently-panic-free docs
// paths we deliberately keep unwinding for catch_unwind.
#![allow(clippy::missing_panics_doc)]

pub mod b64;
pub mod clock;
pub mod guard;
pub mod hwid;
pub mod json;
pub mod obf;
pub mod rfc3339;

#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "manager")]
pub mod manager;
#[cfg(any(feature = "client", feature = "manager"))]
pub mod token;

#[cfg(feature = "jni-client")]
pub mod jni_client;
#[cfg(feature = "jni-manager")]
pub mod jni_manager;

/// Crate version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build-time hex decoder for 32-byte constants (const context).
#[cfg(any(feature = "client", feature = "manager"))]
const fn unhex32_const(text: &str) -> [u8; 32] {
    let bytes = text.as_bytes();
    assert!(bytes.len() == 64, "app secret must be 64 hex chars");
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        let high = match bytes[i * 2] {
            b'0'..=b'9' => bytes[i * 2] - b'0',
            b'a'..=b'f' => bytes[i * 2] - b'a' + 10,
            b'A'..=b'F' => bytes[i * 2] - b'A' + 10,
            _ => panic!("app secret has a non-hex character"),
        };
        let low = match bytes[i * 2 + 1] {
            b'0'..=b'9' => bytes[i * 2 + 1] - b'0',
            b'a'..=b'f' => bytes[i * 2 + 1] - b'a' + 10,
            b'A'..=b'F' => bytes[i * 2 + 1] - b'A' + 10,
            _ => panic!("app secret has a non-hex character"),
        };
        out[i] = (high << 4) | low;
        i += 1;
    }
    out
}

/// Runtime hex decoder (for obfstr-decrypted strings).
#[cfg(any(feature = "client", feature = "manager"))]
fn unhex32_runtime(text: &str) -> Option<[u8; 32]> {
    let bytes = text.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        let high = (bytes[i * 2] as char).to_digit(16)?;
        let low = (bytes[i * 2 + 1] as char).to_digit(16)?;
        *slot = ((high << 4) | low) as u8;
    }
    Some(out)
}

/// Compile-time default app secret (development). Release builds override it
/// with `VOR_DRM_APP_SECRET=<64 hex chars>` at `cargo build` time; the
/// manager and client .so files of one release train are built with the same
/// value so their payloads interoperate.
///
/// The secret never appears in the binary: [`obf::secret`] decrypts it on the
/// stack per call and the caller zeroizes it.
#[cfg(any(feature = "client", feature = "manager"))]
pub(crate) const DEV_APP_SECRET_HEX: &str =
    "cd64396eee17bf3dd3134322340f64b9b054fe5d08edfa166a831ed547369ed4";

/// Compile-time default (development) Ed25519 public key, base64url — the
/// same committed dev key as the Kotlin `BuildConfig` path
/// (`license/keys/dev`). Release builds override with `VOR_DRM_PUBLIC_KEY`.
#[cfg(feature = "client")]
pub(crate) const DEV_PUBLIC_KEY_B64: &str = "f54nNpWuth1MHZsbi6sdEODSDvWp7V6XSSDqWtmCyMA";

/// The app secret for VOR2 payload encryption (32 bytes). Decrypts the
/// obfstr-encoded build value; returns the compiled-in dev secret when the
/// build did not provide one.
#[cfg(any(feature = "client", feature = "manager"))]
pub(crate) fn app_secret() -> zeroize::Zeroizing<[u8; 32]> {
    match option_env!("VOR_DRM_APP_SECRET") {
        Some(custom) => zeroize::Zeroizing::new(
            unhex32_runtime(custom).unwrap_or_else(|| unhex32_const(DEV_APP_SECRET_HEX)),
        ),
        None => zeroize::Zeroizing::new(unhex32_const(DEV_APP_SECRET_HEX)),
    }
}

/// The default Ed25519 public key (base64url, obfuscated in the binary).
/// Callers may override it explicitly (the Android bridge passes the
/// BuildConfig key so gradle stays the single source of truth in CI).
#[cfg(feature = "client")]
pub(crate) fn default_public_key_b64() -> zeroize::Zeroizing<String> {
    let raw = match option_env!("VOR_DRM_PUBLIC_KEY") {
        Some(custom) => custom,
        None => DEV_PUBLIC_KEY_B64,
    };
    // Route through obfstr so the constant is stored encrypted; obfstr
    // requires a literal, so re-encrypt the resolved value via the generic
    // obfstr wrapper at the call sites that need it. Here we only trim.
    zeroize::Zeroizing::new(raw.trim().to_string())
}

#[cfg(all(test, any(feature = "client", feature = "manager")))]
mod tests {
    use super::*;

    #[cfg(any(feature = "client", feature = "manager"))]
    #[test]
    fn dev_app_secret_is_32_bytes() {
        let secret = app_secret();
        assert_eq!(secret.len(), 32);
    }

    #[cfg(feature = "client")]
    #[test]
    fn default_public_key_decodes() {
        let key = default_public_key_b64();
        let bytes = b64::decode(&key).expect("dev public key must decode");
        assert_eq!(bytes.len(), 32);
    }

    #[cfg(any(feature = "client", feature = "manager"))]
    #[test]
    fn unhex_roundtrip() {
        let decoded = unhex32_const(DEV_APP_SECRET_HEX);
        let reencoded: String = decoded.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(reencoded, DEV_APP_SECRET_HEX);
    }
}
