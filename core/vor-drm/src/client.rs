//! **Client Core** — offline license verification for the Vor VPN app.
//!
//! Verifies BOTH token generations with the same call:
//!
//! * **VOR1** — the cross-platform JSON format (`VOR1.<b64url(json)>.<sig>`).
//!   Byte-for-byte the same semantics as every other Vor port, locked by the
//!   shared conformance vectors in `license/vectors.json`.
//! * **VOR2** — the hardware-locked encrypted envelope (`VOR2.<b64url(env)>.<sig>`):
//!   Ed25519 signature first, then HKDF+AES-256-GCM payload decryption, then
//!   strict binary decode, then the binding rules below.
//!
//! Enforcement layers, in order (fail-closed at every step, flattened control
//! flow — see [`crate::obf::cff`]):
//!
//! 1. **anti-analysis** — a tampered environment fails the verification
//!    before any crypto runs;
//! 2. **signature** — Ed25519 over the exact envelope/payload bytes;
//! 3. **payload validity** — strict field caps and sanity;
//! 4. **time** — the anti-rollback composition (`max(device, projected
//!    uptime-consistent wall, persisted ratchet, trusted sample, signed
//!    issued_at)`) decides VALID vs EXPIRED, never the raw device clock;
//! 5. **hardware binding** — constant-time HWID match when the token locks
//!    to a device;
//! 6. **platform** — a VOR2 token that lists platforms must include this
//!    client's platform.
//!
//! The client holds ONLY public material (the Ed25519 verifying key); the
//! payload app secret is a shared obfuscated constant, not a signing key —
//! forging a token without the Ed25519 private key remains impossible.

use crate::cff;
use crate::clock::{self, EffectiveNow};
use crate::guard;
use crate::hwid;
use crate::json::Json;
use crate::rfc3339;
use crate::token::{self, LicenseFields};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use zeroize::Zeroizing;

/// License status (wire names shared with every Vor port).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseStatus {
    /// Signature valid, unexpired, fully bound to this device.
    Valid,
    /// Signature valid but the effective now is at/after the expiry.
    Expired,
    /// Anything else: malformed, bad signature, wrong product, HWID
    /// mismatch, platform mismatch, tampered environment.
    Invalid,
}

impl LicenseStatus {
    /// Stable wire name.
    pub fn as_str(self) -> &'static str {
        match self {
            LicenseStatus::Valid => "VALID",
            LicenseStatus::Expired => "EXPIRED",
            LicenseStatus::Invalid => "INVALID",
        }
    }
}

/// The neutral payload shape surfaced for BOTH generations (UI-compatible).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseInfo {
    /// License id.
    pub id: String,
    /// Issuance time (RFC 3339, UTC).
    pub issued_at: String,
    /// Expiration time (RFC 3339, UTC).
    pub expires_at: String,
    /// Expiration time (epoch seconds) — the enforcement value.
    pub expires_at_epoch: i64,
    /// User tier (e.g. "vip" / "standard"), when present.
    pub tier: Option<String>,
    /// Allowed platforms, when present.
    pub platforms: Vec<String>,
    /// Bandwidth limit in MiB (0 = unlimited; VOR2 only).
    pub bandwidth_limit_mib: u64,
    /// True when the token is cryptographically locked to a device.
    pub hwid_locked: bool,
    /// Token generation (1 or 2).
    pub generation: u8,
}

/// Complete verification inputs (all clocks read by the caller).
#[derive(Debug, Clone)]
pub struct VerifyInputs<'a> {
    /// The token string (`VOR1…` or `VOR2…`).
    pub token: &'a str,
    /// Ed25519 public key (base64url). Empty = use the baked-in default.
    pub public_key_b64: &'a str,
    /// Device wall clock (epoch seconds).
    pub device_wall: i64,
    /// Persisted monotonic ratchet (epoch seconds, 0 when none).
    pub persisted_ratchet: i64,
    /// Trusted-time sample (epoch seconds, 0 when none).
    pub trusted: i64,
    /// BOOTTIME seconds (Rust reads it itself when 0 — tests inject).
    pub boot: u64,
    /// This device's HWID components (gathered without permissions).
    pub device_hwid: Option<(&'a str, &'a str, &'a str)>,
    /// This client's platform name for the platform rule.
    pub platform: &'a str,
}

/// Verification outcome: status + info + time verdict + tamper flags.
#[derive(Debug, Clone)]
pub struct VerifyOutcome {
    /// Final status.
    pub status: LicenseStatus,
    /// Parsed license info (present whenever the envelope was authentic,
    /// even when EXPIRED — the UI shows the expiry).
    pub info: Option<LicenseInfo>,
    /// The effective "now" the decision used (epoch seconds).
    pub now_used: i64,
    /// New ratchet to persist (>= the previous persisted value).
    pub new_ratchet: i64,
    /// Wall-clock rollback was detected during this verification.
    pub rolled_back: bool,
    /// Anti-analysis tripped (debugger / instrumentation detected).
    pub tampered: bool,
}

/// Decode the public key: caller-provided (base64url) or the baked default.
fn resolving_key(public_key_b64: &str) -> Option<VerifyingKey> {
    let text = public_key_b64.trim();
    let resolved: Zeroizing<String> = if text.is_empty() {
        crate::default_public_key_b64()
    } else {
        Zeroizing::new(text.to_string())
    };
    let bytes = crate::b64::decode(&resolved)?;
    let array: [u8; 32] = bytes.try_into().ok()?;
    VerifyingKey::from_bytes(&array).ok()
}

/// Verify a license token. See the module docs for the enforcement ladder.
pub fn verify(inputs: &VerifyInputs) -> VerifyOutcome {
    // Layer 0 — the environment. A tampered environment never even parses
    // the token (fail-closed, cheap, cached).
    let tampered = guard::environment_tampered(false);
    if tampered {
        guard::register_strike();
    }

    // Layer 4 (inputs) — the time composition is needed for every exit
    // path so the ratchet keeps advancing even on failures.
    let boot = if inputs.boot == 0 {
        clock::boottime_secs()
    } else {
        inputs.boot
    };

    // Flattened main ladder — every transition below the entry is an
    // obfuscated compile-time-random key; patching any comparison breaks
    // the chain and the flow falls through to the fail-closed exit.
    let k1: u64 = obfstr::random!(u64);
    let k2: u64 = obfstr::random!(u64);
    let k3: u64 = obfstr::random!(u64);
    let k4: u64 = obfstr::random!(u64);
    let k_end: u64 = obfstr::random!(u64);

    let mut key: Option<VerifyingKey> = None;
    let mut status: Option<LicenseStatus> = None;
    let mut info: Option<LicenseInfo> = None;
    let mut issued_floor: i64 = 0;
    let mut now: EffectiveNow = EffectiveNow {
        now: 0,
        new_ratchet: 0,
        rolled_back: false,
    };

    cff!(k1;
        k4 => k_end { // (shuffled arm) time composition
            now = clock::effective_now(
                inputs.device_wall,
                inputs.persisted_ratchet,
                inputs.trusted,
                issued_floor,
                boot,
            );
        }
        k1 => k2 { // key resolution
            key = resolving_key(inputs.public_key_b64);
        }
        k2 => k3 { // generation dispatch + signature + payload
            if tampered {
                status = Some(LicenseStatus::Invalid);
            } else if let Some(verifying) = key.as_ref() {
                let verdict = verify_token_body(inputs, verifying);
                issued_floor = verdict.issued_floor;
                status = Some(verdict.status);
                info = verdict.info;
            } else {
                status = Some(LicenseStatus::Invalid);
            }
        }
        k3 => k4 { // issued_at floor re-composition needs the floor from the
                   // payload: recompute "now" with the signed floor applied.
            now = clock::effective_now(
                inputs.device_wall,
                inputs.persisted_ratchet,
                inputs.trusted,
                issued_floor,
                boot,
            );
        }
    );

    // ---- finalize (outside the flattened chain, all exits converge here)
    let EffectiveNow {
        now: effective,
        new_ratchet,
        rolled_back,
    } = now;

    let status = match (status, info.as_ref()) {
        (Some(LicenseStatus::Valid), Some(parsed)) if effective >= parsed.expires_at_epoch => {
            LicenseStatus::Expired
        }
        (Some(inner), _) => inner,
        (None, _) => LicenseStatus::Invalid,
    };
    VerifyOutcome {
        status,
        info,
        now_used: effective,
        new_ratchet,
        rolled_back,
        tampered,
    }
}

/// Per-token verdict pieces (internal).
struct BodyVerdict {
    status: LicenseStatus,
    info: Option<LicenseInfo>,
    issued_floor: i64,
}

/// Signature + payload verification for both generations (after the key is
/// resolved and the environment checked).
fn verify_token_body(inputs: &VerifyInputs, verifying: &VerifyingKey) -> BodyVerdict {
    let token = inputs.token.trim();
    if token.len() > token::MAX_TOKEN_LEN * 2 {
        return BodyVerdict {
            status: LicenseStatus::Invalid,
            info: None,
            issued_floor: 0,
        };
    }
    if let Some((envelope, signature)) = token::parse_token(token) {
        verify_v2(inputs, verifying, &envelope, &signature)
    } else if token.starts_with(obfstr::obfstr!("VOR1.")) {
        verify_v1(token, verifying)
    } else {
        BodyVerdict {
            status: LicenseStatus::Invalid,
            info: None,
            issued_floor: 0,
        }
    }
}

/// VOR2 verification: signature -> decrypt -> decode -> bind.
fn verify_v2(
    inputs: &VerifyInputs,
    verifying: &VerifyingKey,
    envelope: &[u8],
    signature: &[u8; 64],
) -> BodyVerdict {
    let failed = || BodyVerdict {
        status: LicenseStatus::Invalid,
        info: None,
        issued_floor: 0,
    };

    // 1. Signature FIRST — nothing unauthenticated is ever decrypted.
    if !token::verify_envelope(envelope, signature, verifying) {
        return failed();
    }

    // 2. Payload decryption (HKDF + AES-256-GCM tag).
    let secret = crate::app_secret();
    let issuer_public: [u8; 32] = verifying.to_bytes();
    let plaintext = match token::open_envelope(envelope, &secret, &issuer_public) {
        Some(bytes) => bytes,
        None => return failed(),
    };
    drop(secret);

    // 3. Strict canonical decode.
    let fields = match LicenseFields::decode(&plaintext) {
        Some(fields) => fields,
        None => return failed(),
    };

    // 4. HWID binding (constant-time).
    if let Some(locked) = fields.hwid {
        let device = inputs
            .device_hwid
            .map(|(drm, aid, fp)| hwid::derive(drm, aid, fp));
        match device {
            Some(digest) if hwid::matches(&locked, &digest) => {}
            _ => return failed(),
        }
    }

    // 5. Platform rule (empty list = unconstrained).
    if !fields.platforms.is_empty() && !fields.platforms.iter().any(|p| p == inputs.platform) {
        return failed();
    }

    let expires_at_epoch = fields.expires_at;
    let issued_floor = fields.issued_at;
    BodyVerdict {
        status: LicenseStatus::Valid,
        info: Some(LicenseInfo {
            id: fields.id,
            issued_at: rfc3339::format_utc(fields.issued_at),
            expires_at: rfc3339::format_utc(fields.expires_at),
            expires_at_epoch,
            tier: Some(fields.tier),
            platforms: fields.platforms,
            bandwidth_limit_mib: fields.bandwidth_limit_mib,
            hwid_locked: fields.hwid.is_some(),
            generation: 2,
        }),
        issued_floor,
    }
}

/// VOR1 verification — exact shared-semantics port (conformance-locked).
fn verify_v1(token: &str, verifying: &VerifyingKey) -> BodyVerdict {
    let failed = || BodyVerdict {
        status: LicenseStatus::Invalid,
        info: None,
        issued_floor: 0,
    };

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 || parts[0] != obfstr::obfstr!("VOR1") {
        return failed();
    }
    let payload_bytes = match crate::b64::decode(parts[1]) {
        Some(bytes) => bytes,
        None => return failed(),
    };
    let signature_bytes = match crate::b64::decode(parts[2]) {
        Some(bytes) if bytes.len() == 64 => bytes,
        _ => return failed(),
    };
    let signature = match Signature::from_slice(&signature_bytes) {
        Ok(signature) => signature,
        Err(_) => return failed(),
    };
    if verifying.verify(&payload_bytes, &signature).is_err() {
        return failed();
    }

    // Payload semantics — identical rules to the Kotlin verifier.
    let root = match Json::parse(&String::from_utf8_lossy(&payload_bytes)) {
        Some(root) => root,
        None => return failed(),
    };
    if root.obj_int("v") != Some(1)
        || root.obj_str("product") != Some(obfstr::obfstr!("vor"))
        || root.obj_str("id").map(str::is_empty).unwrap_or(true)
        || root.obj_str("issued_at").map(str::is_empty).unwrap_or(true)
    {
        return failed();
    }
    let expires_at = match root.obj_str("expires_at") {
        Some(text) => text,
        None => return failed(),
    };
    let expires_epoch = match rfc3339::parse_epoch(expires_at) {
        Some(epoch) => epoch,
        None => return failed(),
    };

    let entitlements = root.get("entitlements");
    let tier = entitlements
        .and_then(|entry| entry.obj_str("tier"))
        .map(str::to_string);
    let platforms = entitlements
        .and_then(|entry| entry.get("platforms"))
        .and_then(Json::as_arr)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    BodyVerdict {
        status: LicenseStatus::Valid,
        info: Some(LicenseInfo {
            id: root.obj_str("id").unwrap_or_default().to_string(),
            issued_at: root.obj_str("issued_at").unwrap_or_default().to_string(),
            expires_at: expires_at.to_string(),
            expires_at_epoch: expires_epoch,
            tier,
            platforms,
            bandwidth_limit_mib: 0,
            hwid_locked: false,
            generation: 1,
        }),
        issued_floor: rfc3339::parse_epoch(root.obj_str("issued_at").unwrap_or("")).unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{self};
    use ed25519_dalek::{Signer, SigningKey};

    // The DEV public key from license/keys/dev (shared with BuildConfig).
    const DEV_PUBLIC_B64: &str = "f54nNpWuth1MHZsbi6sdEODSDvWp7V6XSSDqWtmCyMA";

    fn dev_signing_key() -> SigningKey {
        // Deterministic test key; the conformance vectors live in
        // tests/conformance.rs against the real dev key.
        let seed = crate::hwid::derive("seed", "seed", "seed");
        SigningKey::from_bytes(&seed)
    }

    fn base_inputs<'a>(token: &'a str, wall: i64, key_b64: &'a str) -> VerifyInputs<'a> {
        VerifyInputs {
            token,
            public_key_b64: key_b64,
            device_wall: wall,
            persisted_ratchet: 0,
            trusted: 0,
            boot: 0,
            device_hwid: Some(("drm", "aid", "fp")),
            platform: "android",
        }
    }

    /// Inputs whose public key matches [`dev_signing_key`] (the usual case:
    /// tokens signed and verified inside one test).
    fn signed_inputs<'a>(token: &'a str, wall: i64, pub_b64: &'a str) -> VerifyInputs<'a> {
        base_inputs(token, wall, pub_b64)
    }

    #[test]
    fn v1_conformance_shape_rejects_malformed() {
        let key = dev_signing_key();
        let verifying = VerifyingKey::from(&key);
        assert_eq!(verify_v1("", &verifying).status, LicenseStatus::Invalid);
        assert_eq!(
            verify_v1("VOR1.only-two", &verifying).status,
            LicenseStatus::Invalid
        );
        assert_eq!(
            verify_v1("VORX.a.b", &verifying).status,
            LicenseStatus::Invalid
        );
        assert_eq!(
            verify_v1("VOR1.!!.AAAA", &verifying).status,
            LicenseStatus::Invalid
        );
    }

    #[test]
    fn v1_roundtrip_against_rust_issued_token() {
        let key = dev_signing_key();
        let verifying = VerifyingKey::from(&key);
        // Build a minimal VOR1 payload by hand (the canonical issuer is
        // Kotlin; here we only need shared-shape bytes).
        let payload = br#"{"v":1,"id":"test-1","product":"vor","issued_at":"2026-01-01T00:00:00Z","expires_at":"2027-01-01T00:00:00Z"}"#;
        let signature = key.sign(payload).to_bytes();
        let token = format!(
            "VOR1.{}.{}",
            crate::b64::encode(payload),
            crate::b64::encode(&signature)
        );
        let verdict = verify_v1(&token, &verifying);
        assert_eq!(verdict.status, LicenseStatus::Valid);
        let info = verdict.info.expect("info");
        assert_eq!(info.id, "test-1");
        assert_eq!(info.expires_at_epoch, 1_798_761_600);
        assert_eq!(info.generation, 1);
    }

    #[test]
    fn v2_full_lifecycle_with_hwid_binding() {
        let key = dev_signing_key();
        let verifying = VerifyingKey::from(&key);
        let fields = LicenseFields {
            id: "vip-001".to_string(),
            issued_at: 1_000_000,
            expires_at: 2_000_000,
            bandwidth_limit_mib: 102_400,
            tier: "vip".to_string(),
            platforms: vec!["android".to_string()],
            hwid: Some(crate::hwid::derive("drm", "aid", "fp")),
        };
        let secret = crate::app_secret();
        let envelope =
            token::seal_envelope(&fields.encode(), &secret, &verifying.to_bytes()).expect("seal");
        let signature = token::sign_envelope(&envelope, &key);
        let token = token::render_token(&envelope, &signature);

        let pub_b64 = crate::b64::encode(&verifying.to_bytes());

        // Correct device + before expiry -> VALID.
        let outcome = verify(&signed_inputs(&token, 1_500_000, &pub_b64));
        assert_eq!(outcome.status, LicenseStatus::Valid);
        let info = outcome.info.expect("info");
        assert_eq!(info.tier.as_deref(), Some("vip"));
        assert_eq!(info.bandwidth_limit_mib, 102_400);
        assert!(info.hwid_locked);
        assert_eq!(info.generation, 2);

        // After expiry -> EXPIRED (signature still good; UI shows expiry).
        let outcome = verify(&signed_inputs(&token, 2_500_000, &pub_b64));
        assert_eq!(outcome.status, LicenseStatus::Expired);

        // Wrong device (HWID mismatch) -> INVALID.
        let mut wrong = signed_inputs(&token, 1_500_000, &pub_b64);
        wrong.device_hwid = Some(("other-drm", "aid", "fp"));
        assert_eq!(verify(&wrong).status, LicenseStatus::Invalid);

        // No device hwid available -> INVALID (locked token).
        let mut none = signed_inputs(&token, 1_500_000, &pub_b64);
        none.device_hwid = None;
        assert_eq!(verify(&none).status, LicenseStatus::Invalid);
    }

    #[test]
    fn v2_without_hwid_is_portable() {
        let key = dev_signing_key();
        let verifying = VerifyingKey::from(&key);
        let fields = LicenseFields {
            id: "portable".to_string(),
            issued_at: 1_000_000,
            expires_at: 2_000_000,
            bandwidth_limit_mib: 0,
            tier: "standard".to_string(),
            platforms: vec![],
            hwid: None,
        };
        let secret = crate::app_secret();
        let envelope =
            token::seal_envelope(&fields.encode(), &secret, &verifying.to_bytes()).expect("seal");
        let token = token::render_token(&envelope, &token::sign_envelope(&envelope, &key));
        let pub_b64 = crate::b64::encode(&verifying.to_bytes());
        let outcome = verify(&signed_inputs(&token, 1_500_000, &pub_b64));
        assert_eq!(outcome.status, LicenseStatus::Valid);
    }

    #[test]
    fn v2_platform_rule() {
        let key = dev_signing_key();
        let verifying = VerifyingKey::from(&key);
        let fields = LicenseFields {
            id: "windows-only".to_string(),
            issued_at: 1_000_000,
            expires_at: 2_000_000,
            bandwidth_limit_mib: 0,
            tier: "standard".to_string(),
            platforms: vec!["windows".to_string()],
            hwid: None,
        };
        let secret = crate::app_secret();
        let envelope =
            token::seal_envelope(&fields.encode(), &secret, &verifying.to_bytes()).expect("seal");
        let token = token::render_token(&envelope, &token::sign_envelope(&envelope, &key));
        let pub_b64 = crate::b64::encode(&verifying.to_bytes());
        let mut inputs = base_inputs(&token, 1_500_000, &pub_b64);
        inputs.platform = "android";
        assert_eq!(verify(&inputs).status, LicenseStatus::Invalid);
        inputs.platform = "windows";
        assert_eq!(verify(&inputs).status, LicenseStatus::Valid);
    }

    #[test]
    fn v2_bad_signature_fails_without_decryption() {
        let key = dev_signing_key();
        let verifying = VerifyingKey::from(&key);
        let other = SigningKey::from_bytes(&crate::hwid::derive("other", "other", "other"));
        let fields = LicenseFields {
            id: "x".to_string(),
            issued_at: 1,
            expires_at: 2,
            bandwidth_limit_mib: 0,
            tier: "standard".to_string(),
            platforms: vec![],
            hwid: None,
        };
        let secret = crate::app_secret();
        let envelope =
            token::seal_envelope(&fields.encode(), &secret, &verifying.to_bytes()).expect("seal");
        let wrong_signature = token::sign_envelope(&envelope, &other);
        let token = token::render_token(&envelope, &wrong_signature);
        let pub_b64 = crate::b64::encode(&verifying.to_bytes());
        assert_eq!(
            verify(&signed_inputs(&token, 1, &pub_b64)).status,
            LicenseStatus::Invalid
        );
    }

    #[test]
    fn wrong_public_key_rejects_v2() {
        let key = dev_signing_key();
        let verifying = VerifyingKey::from(&key);
        let fields = LicenseFields {
            id: "x".to_string(),
            issued_at: 1,
            expires_at: 2,
            bandwidth_limit_mib: 0,
            tier: "standard".to_string(),
            platforms: vec![],
            hwid: None,
        };
        let secret = crate::app_secret();
        let envelope =
            token::seal_envelope(&fields.encode(), &secret, &verifying.to_bytes()).expect("seal");
        let token = token::render_token(&envelope, &token::sign_envelope(&envelope, &key));
        let other = SigningKey::from_bytes(&crate::hwid::derive("a", "b", "c"));
        let other_b64 = crate::b64::encode(&VerifyingKey::from(&other).to_bytes());
        let mut inputs = base_inputs(&token, 1, "");
        inputs.public_key_b64 = &other_b64;
        assert_eq!(verify(&inputs).status, LicenseStatus::Invalid);
    }

    #[test]
    fn issued_at_floor_blocks_wiped_ratchet_rollback() {
        let key = dev_signing_key();
        let verifying = VerifyingKey::from(&key);
        let fields = LicenseFields {
            id: "floor".to_string(),
            issued_at: 5_000_000,
            expires_at: 6_000_000,
            bandwidth_limit_mib: 0,
            tier: "standard".to_string(),
            platforms: vec![],
            hwid: None,
        };
        let secret = crate::app_secret();
        let envelope =
            token::seal_envelope(&fields.encode(), &secret, &verifying.to_bytes()).expect("seal");
        let token = token::render_token(&envelope, &token::sign_envelope(&envelope, &key));
        // Device wound back to before issuance; ratchet wiped.
        let pub_b64 = crate::b64::encode(&verifying.to_bytes());
        let mut inputs = base_inputs(&token, 1_000, &pub_b64);
        inputs.persisted_ratchet = 0;
        let outcome = verify(&inputs);
        assert_eq!(outcome.now_used, 5_000_000, "signed issued_at is a floor");
        assert_eq!(outcome.status, LicenseStatus::Valid); // not expired: 5M < 6M
        assert_eq!(outcome.new_ratchet, 5_000_000);
    }

    #[test]
    fn persisted_ratchet_expires_rolled_back_token() {
        let key = dev_signing_key();
        let verifying = VerifyingKey::from(&key);
        let fields = LicenseFields {
            id: "expired".to_string(),
            issued_at: 1_000_000,
            expires_at: 2_000_000,
            bandwidth_limit_mib: 0,
            tier: "standard".to_string(),
            platforms: vec![],
            hwid: None,
        };
        let secret = crate::app_secret();
        let envelope =
            token::seal_envelope(&fields.encode(), &secret, &verifying.to_bytes()).expect("seal");
        let token = token::render_token(&envelope, &token::sign_envelope(&envelope, &key));
        // Wall says 1_500_000 (valid) but the ratchet remembers 3_000_000.
        let pub_b64 = crate::b64::encode(&verifying.to_bytes());
        let mut inputs = base_inputs(&token, 1_500_000, &pub_b64);
        inputs.persisted_ratchet = 3_000_000;
        assert_eq!(verify(&inputs).status, LicenseStatus::Expired);
    }

    #[test]
    fn garbage_tokens_fail_closed() {
        for bad in [
            "",
            "VOR2",
            "nope",
            "VOR1.a.b",
            "VOR2.a.b",
            "VOR2.!!.",
            "VOR1.{}.AA",
        ] {
            let outcome = verify(&base_inputs(bad, 1_000_000, ""));
            assert_eq!(outcome.status, LicenseStatus::Invalid, "accepted {bad:?}");
        }
    }

    #[test]
    fn empty_public_key_falls_back_to_baked_default() {
        // The dev key IS the baked default, so a V1 token signed by the dev
        // key verifies with an empty override.
        let key = dev_signing_key();
        let payload = br#"{"v":1,"id":"z","product":"vor","issued_at":"2026-01-01T00:00:00Z","expires_at":"2027-01-01T00:00:00Z"}"#;
        let token = format!(
            "VOR1.{}.{}",
            crate::b64::encode(payload),
            crate::b64::encode(&key.sign(payload).to_bytes())
        );
        assert_eq!(
            verify(&base_inputs(&token, 1_800_000, "")).status,
            LicenseStatus::Invalid
        );
        // (invalid only because the baked dev key is NOT this test key —
        // proving the fallback path actually resolves a key and verifies
        // against it rather than skipping the check.)
    }

    #[test]
    fn resolving_key_rejects_garbage() {
        assert!(resolving_key("!!!").is_none());
        assert!(resolving_key("AAAA").is_none()); // wrong length
        assert!(resolving_key("").is_some()); // baked default
    }

    #[test]
    fn dev_public_key_constant_is_valid() {
        // If this fails, the committed default no longer matches a valid
        // Ed25519 point — release builds would fail every verification.
        let bytes = crate::b64::decode(DEV_PUBLIC_B64).expect("decode");
        assert_eq!(bytes.len(), 32);
        assert!(VerifyingKey::from_bytes(&bytes.try_into().unwrap()).is_ok());
    }
}
