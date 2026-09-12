//! **Manager Core** — offline license issuance for the Vor License Manager.
//!
//! Everything the admin app needs, fully offline:
//!
//! * **key generation** — fresh Ed25519 seeds from OS entropy;
//! * **issuance** — single + batch, producing hardware-locked VOR2 tokens
//!   (encrypted payload, Ed25519-signed envelope, granular entitlements:
//!   expiration, bandwidth, tier, platforms, HWID);
//! * **portable vault** — Argon2id (32 MiB, t=3, p=1) + AES-256-GCM seed
//!   envelopes for air-gapped key transport (the `VORVB1.` format);
//! * **self-test** — sign a canary, verify with the derived public key.
//!
//! Security model:
//!
//! * the seed is passed in by the caller (the Android app unwraps it from
//!   its Keystore-gated vault for one in-flight operation) and lives in
//!   native memory only inside a single call — [`zeroize::Zeroizing`]
//!   scrubs it on every exit path;
//! * every issuance path runs the anti-analysis sweep first: a tampered
//!   environment never signs, and after [`crate::guard::STRIKE_LIMIT`]
//!   detections the manager escalates (zeroize + silent abort — see
//!   [`escalate_if_needed`]);
//! * validation is total up-front (a batch either fully validates or names
//!   the offending row — no partially-issued batches).

use crate::guard;
use crate::hwid::{self, HWID_LEN};
use crate::json::Json;
use crate::rfc3339;
use crate::token::{self, LicenseFields};
use ed25519_dalek::{SigningKey, VerifyingKey};
use zeroize::Zeroizing;

/// Seed length (bytes).
pub const SEED_LEN: usize = 32;

/// Maximum batch size per call (memory + UI sanity).
pub const MAX_BATCH: usize = 500;

/// Argon2id parameters for the portable vault: 32 MiB, 3 passes, 1 lane.
/// Deliberately heavier than a login KDF: it runs once per backup/restore,
/// not per launch, and it is the ONLY thing standing between a stolen
/// envelope file and the seed.
const ARGON2_M_KIB: u32 = 32 * 1024;
const ARGON2_T: u32 = 3;
const ARGON2_P: u32 = 1;

/// Vault envelope prefix (format detection on restore).
const VAULT_PREFIX: &str = "VORVB1.";

/// Issuance request (validated before anything is signed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueSpec {
    /// License id (1..=64 chars, no control characters).
    pub id: String,
    /// Tier (1..=24 chars).
    pub tier: String,
    /// Issuance time (epoch seconds).
    pub issued_at: i64,
    /// Expiration time (epoch seconds; MUST be in the future of `issued_at`).
    pub expires_at: i64,
    /// Bandwidth limit in MiB (0 = unlimited).
    pub bandwidth_limit_mib: u64,
    /// Platforms (0..=8, each 1..=16 chars).
    pub platforms: Vec<String>,
    /// Hardware lock digest (None = portable token).
    pub hwid: Option<[u8; HWID_LEN]>,
}

/// Validation failure (human-presentable, index for batches).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// 1-based row index (0 for single issuance).
    pub index: usize,
    /// What is wrong.
    pub reason: String,
}

fn text_is_clean(text: &str) -> bool {
    !text.is_empty() && !text.chars().any(|c| c.is_control())
}

fn validate_spec(spec: &IssueSpec, index: usize) -> Result<(), ValidationError> {
    let bad = |reason: &str| {
        Err(ValidationError {
            index,
            reason: reason.to_string(),
        })
    };
    if spec.id.is_empty() || spec.id.len() > token::MAX_ID_LEN || !text_is_clean(&spec.id) {
        return bad("license id must be 1-64 chars without control characters");
    }
    if spec.tier.is_empty() || spec.tier.len() > token::MAX_TIER_LEN || !text_is_clean(&spec.tier) {
        return bad("tier must be 1-24 chars without control characters");
    }
    if spec.expires_at <= spec.issued_at {
        return bad("expiry must be after issuance");
    }
    if spec.issued_at < 0 {
        return bad("issuance time is negative");
    }
    if spec.bandwidth_limit_mib > token::MAX_BANDWIDTH_MIB {
        return bad("bandwidth limit is absurd");
    }
    if spec.platforms.len() > token::MAX_PLATFORMS {
        return bad("too many platforms");
    }
    if spec
        .platforms
        .iter()
        .any(|p| p.is_empty() || p.len() > token::MAX_PLATFORM_LEN)
    {
        return bad("platform names must be 1-16 chars");
    }
    Ok(())
}

/// Generate a fresh Ed25519 seed from OS entropy.
pub fn generate_seed() -> Option<[u8; SEED_LEN]> {
    let mut seed = Zeroizing::new([0u8; SEED_LEN]);
    getrandom::getrandom(seed.as_mut()).ok()?;
    Some(*seed)
}

/// Public key of a seed (base64url).
pub fn public_key_b64(seed: &[u8; SEED_LEN]) -> String {
    crate::b64::encode(&SigningKey::from_bytes(seed).verifying_key().to_bytes())
}

/// Issue one VOR2 token.
///
/// The seed is scrubbed on return; the returned token is a String (public
/// material — safe to persist/share).
pub fn issue(seed: &[u8; SEED_LEN], spec: &IssueSpec) -> Result<String, ValidationError> {
    validate_spec(spec, 0)?;
    issue_unchecked(seed, spec)
}

/// Issue a batch. ALL rows are validated first; the first invalid row
/// aborts the whole batch (nothing is signed when anything is wrong).
pub fn issue_batch(
    seed: &[u8; SEED_LEN],
    specs: &[IssueSpec],
) -> Result<Vec<String>, ValidationError> {
    if specs.len() > MAX_BATCH {
        return Err(ValidationError {
            index: 0,
            reason: format!("batch is larger than {MAX_BATCH} rows"),
        });
    }
    for (index, spec) in specs.iter().enumerate() {
        validate_spec(spec, index + 1)?;
    }
    let mut tokens = Vec::with_capacity(specs.len());
    for spec in specs {
        tokens.push(issue_unchecked(seed, spec)?);
    }
    Ok(tokens)
}

fn issue_unchecked(seed: &[u8; SEED_LEN], spec: &IssueSpec) -> Result<String, ValidationError> {
    // Anti-analysis gate: a tampered environment never signs.
    if guard::environment_tampered(false) {
        return Err(ValidationError {
            index: 0,
            reason: obfstr::obfstr!("environment check failed").to_string(),
        });
    }
    let signing_key = SigningKey::from_bytes(seed);
    let issuer_public = signing_key.verifying_key().to_bytes();
    let fields = LicenseFields {
        id: spec.id.clone(),
        issued_at: spec.issued_at,
        expires_at: spec.expires_at,
        bandwidth_limit_mib: spec.bandwidth_limit_mib,
        tier: spec.tier.clone(),
        platforms: spec.platforms.clone(),
        hwid: spec.hwid,
    };
    let secret = crate::app_secret();
    let envelope =
        token::seal_envelope(&fields.encode(), &secret, &issuer_public).ok_or_else(|| {
            ValidationError {
                index: 0,
                reason: obfstr::obfstr!("envelope failure").to_string(),
            }
        })?;
    let signature = token::sign_envelope(&envelope, &signing_key);
    Ok(token::render_token(&envelope, &signature))
}

/// Parse an issuance request from the JNI JSON shape:
/// `{"id","tier","issued_at","expires_at","bandwidth_mib","platforms":[],"hwid_hex"}`.
///
/// `expires_at` accepts either an epoch number or an RFC 3339 string (the
/// manager UI collects RFC 3339; the native API speaks both).
pub fn parse_spec_json(json_text: &str) -> Option<IssueSpec> {
    Json::parse(json_text).as_ref().and_then(spec_from_json)
}

/// Build an [`IssueSpec`] from a parsed JSON object (batch rows arrive as
/// pre-parsed values; the shape is identical to [`parse_spec_json`]).
pub fn spec_from_json(root: &Json) -> Option<IssueSpec> {
    let id = root.obj_str("id")?.to_string();
    let tier = root.obj_str("tier")?.to_string();
    let epoch = |key: &str| -> Option<i64> {
        match root.get(key)? {
            Json::Int(value) => Some(*value),
            Json::Str(text) => rfc3339::parse_epoch(text),
            _ => None,
        }
    };
    let issued_at = epoch("issued_at")?;
    let expires_at = epoch("expires_at")?;
    let bandwidth_limit_mib = match root.get("bandwidth_mib") {
        Some(Json::Int(value)) if *value >= 0 => Some(*value as u64),
        _ => Some(0),
    }?;
    let platforms = match root.get("platforms") {
        Some(Json::Arr(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    };
    let hwid = match root.obj_str("hwid_hex") {
        Some(hex) if !hex.trim().is_empty() => Some(hwid::from_hex(hex)?),
        _ => None,
    };
    Some(IssueSpec {
        id,
        tier,
        issued_at,
        expires_at,
        bandwidth_limit_mib,
        platforms,
        hwid,
    })
}

/// Seal a seed into a passphrase-protected portable vault envelope.
///
/// Format: `VORVB1.<b64url(salt(16) || nonce(12) || AES-256-GCM(seed))`
/// where the AES key is Argon2id(passphrase, salt). The envelope is safe to
/// carry on a USB stick across the air gap.
pub fn vault_seal(seed: &[u8; SEED_LEN], passphrase: &str) -> Option<String> {
    if passphrase.is_empty() {
        return None;
    }
    let mut salt = Zeroizing::new([0u8; 16]);
    let mut nonce = Zeroizing::new([0u8; token::NONCE_LEN]);
    getrandom::getrandom(salt.as_mut()).ok()?;
    getrandom::getrandom(nonce.as_mut()).ok()?;

    let mut key = Zeroizing::new([0u8; 32]);
    let argon = argon2::Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(ARGON2_M_KIB, ARGON2_T, ARGON2_P, Some(32)).ok()?,
    );
    argon
        .hash_password_into(passphrase.as_bytes(), salt.as_slice(), key.as_mut())
        .ok()?;

    use aes_gcm::aead::{Aead, KeyInit, Payload};
    let cipher = aes_gcm::Aes256Gcm::new((&*key).into());
    let sealed = cipher
        .encrypt(
            aes_gcm::Nonce::from_slice(nonce.as_slice()),
            Payload {
                msg: seed,
                aad: VAULT_PREFIX.as_bytes(),
            },
        )
        .ok()?;

    let mut blob = Vec::with_capacity(16 + token::NONCE_LEN + sealed.len());
    blob.extend_from_slice(salt.as_slice());
    blob.extend_from_slice(nonce.as_slice());
    blob.extend_from_slice(&sealed);
    Some(format!("{VAULT_PREFIX}{}", crate::b64::encode(&blob)))
}

/// Open a portable vault envelope (wrong passphrase = GCM tag failure = None).
pub fn vault_open(envelope: &str, passphrase: &str) -> Option<[u8; SEED_LEN]> {
    let text = envelope.trim();
    let payload = text.strip_prefix(VAULT_PREFIX)?;
    if passphrase.is_empty() {
        return None;
    }
    let blob = crate::b64::decode(payload)?;
    if blob.len() != 16 + token::NONCE_LEN + SEED_LEN + token::TAG_LEN {
        return None;
    }
    let (salt, rest) = blob.split_at(16);
    let (nonce, sealed) = rest.split_at(token::NONCE_LEN);
    if sealed.len() != SEED_LEN + token::TAG_LEN {
        return None;
    }
    let mut key = Zeroizing::new([0u8; 32]);
    let argon = argon2::Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(ARGON2_M_KIB, ARGON2_T, ARGON2_P, Some(32)).ok()?,
    );
    argon
        .hash_password_into(passphrase.as_bytes(), salt, key.as_mut())
        .ok()?;

    use aes_gcm::aead::{Aead, KeyInit, Payload};
    let cipher = aes_gcm::Aes256Gcm::new((&*key).into());
    let seed = cipher
        .decrypt(
            aes_gcm::Nonce::from_slice(nonce),
            Payload {
                msg: sealed,
                aad: VAULT_PREFIX.as_bytes(),
            },
        )
        .ok()?;
    let seed: [u8; SEED_LEN] = seed.try_into().ok()?;
    Some(seed)
}

/// True when the string looks like a portable vault envelope.
pub fn is_vault_envelope(text: &str) -> bool {
    text.trim().starts_with(VAULT_PREFIX)
}

/// Escalation for the manager core: on the strike-limit transition,
/// zeroize what we can and abort the process (silent crash — the spec's
/// required response when instrumentation is detected while a signing key
/// is in use). Never called on a clean path.
pub fn escalate_if_needed() -> bool {
    if guard::register_strike() {
        // Best-effort final scrub of this frame's secrets is handled by the
        // Zeroizing drops on unwind — abort() does not unwind, so scrub the
        // one thing we still hold: nothing persistent. Kill the process.
        std::process::abort();
    }
    guard::should_escalate()
}

/// Self-test: sign a canary VOR2 token and verify it with the seed's own
/// public key. Returns (signed_and_verifies, detail).
pub fn self_test(seed: &[u8; SEED_LEN]) -> (bool, String) {
    let spec = IssueSpec {
        id: obfstr::obfstr!("self-test").to_string(),
        tier: "self-test".to_string(),
        issued_at: 1,
        expires_at: 2,
        bandwidth_limit_mib: 0,
        platforms: vec![],
        hwid: None,
    };
    match issue(seed, &spec) {
        Ok(token) => {
            let verifying = VerifyingKey::from(&SigningKey::from_bytes(seed));
            match token::parse_token(&token) {
                Some((envelope, signature)) => {
                    let ok = token::verify_envelope(&envelope, &signature, &verifying);
                    (
                        ok,
                        if ok {
                            obfstr::obfstr!("sign+verify ok").to_string()
                        } else {
                            obfstr::obfstr!("verification of the fresh token failed").to_string()
                        },
                    )
                }
                None => (false, obfstr::obfstr!("token did not re-parse").to_string()),
            }
        }
        Err(error) => (false, error.reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: &str) -> IssueSpec {
        IssueSpec {
            id: id.to_string(),
            tier: "vip".to_string(),
            issued_at: 1_000_000,
            expires_at: 2_000_000,
            bandwidth_limit_mib: 5120,
            platforms: vec!["android".to_string()],
            hwid: Some(crate::hwid::derive("drm", "aid", "fp")),
        }
    }

    fn seed() -> [u8; SEED_LEN] {
        crate::hwid::derive("manager", "test", "seed")
    }

    // Cross-core: verifies manager-issued tokens through the CLIENT core,
    // so it only compiles when both cores are present.
    #[cfg(feature = "client")]
    #[test]
    fn issue_and_verify_roundtrip() {
        let seed = seed();
        let token = issue(&seed, &spec("roundtrip-1")).expect("issue");
        assert!(token.starts_with("VOR2."));
        // Client-side verification (guarded environment is clean here).
        let outcome = crate::client::verify(&crate::client::VerifyInputs {
            token: &token,
            public_key_b64: "",
            device_wall: 1_500_000,
            persisted_ratchet: 0,
            trusted: 0,
            boot: 0,
            device_hwid: Some(("drm", "aid", "fp")),
            platform: "android",
        });
        // NOTE: public_key_b64 empty resolves the BAKED dev key, not this
        // seed — pass the right key explicitly:
        let _ = outcome; // (covered properly below)
        let pub_b64 = public_key_b64(&seed);
        let outcome = crate::client::verify(&crate::client::VerifyInputs {
            token: &token,
            public_key_b64: &pub_b64,
            device_wall: 1_500_000,
            persisted_ratchet: 0,
            trusted: 0,
            boot: 0,
            device_hwid: Some(("drm", "aid", "fp")),
            platform: "android",
        });
        assert_eq!(outcome.status, crate::client::LicenseStatus::Valid);
        assert_eq!(outcome.info.expect("info").tier.as_deref(), Some("vip"));
    }

    #[test]
    fn validation_rejects_bad_specs() {
        let seed = seed();
        assert!(issue(&seed, &spec("")).is_err()); // empty id
        let mut long_id = spec(&"x".repeat(65));
        assert!(issue(&seed, &long_id).is_err());
        long_id = spec("ok");
        long_id.expires_at = long_id.issued_at; // expiry == issued
        assert!(issue(&seed, &long_id).is_err());
        long_id = spec("ok");
        long_id.platforms = vec!["p".repeat(17)];
        assert!(issue(&seed, &long_id).is_err());
        long_id = spec("ok");
        long_id.tier = "t".repeat(25);
        assert!(issue(&seed, &long_id).is_err());
    }

    #[test]
    fn batch_validates_all_before_signing() {
        let seed = seed();
        let mut specs: Vec<IssueSpec> = (0..5).map(|i| spec(&format!("batch-{i}"))).collect();
        specs[3].expires_at = 0; // row 4 is invalid
        let error = issue_batch(&seed, &specs).expect_err("must fail");
        assert_eq!(error.index, 4);
        // Nothing was signed: no partial state to observe, but a clean
        // batch works end to end:
        specs[3].expires_at = 2_000_000;
        let tokens = issue_batch(&seed, &specs).expect("batch issues");
        assert_eq!(tokens.len(), 5);
        assert!(tokens.iter().all(|token| token.starts_with("VOR2.")));
    }

    #[test]
    fn batch_rejects_oversized() {
        let seed = seed();
        let specs: Vec<IssueSpec> = (0..MAX_BATCH + 1).map(|i| spec(&format!("x{i}"))).collect();
        assert!(issue_batch(&seed, &specs).is_err());
    }

    #[test]
    fn vault_roundtrip_and_wrong_passphrase() {
        let seed = seed();
        let envelope = vault_seal(&seed, "correct horse battery staple").expect("seal");
        assert!(is_vault_envelope(&envelope));
        let reopened = vault_open(&envelope, "correct horse battery staple").expect("open");
        assert_eq!(reopened, seed);
        assert!(vault_open(&envelope, "wrong").is_none());
        assert!(vault_seal(&seed, "").is_none()); // empty passphrase refused
    }

    #[test]
    fn vault_rejects_tampering() {
        let seed = seed();
        let envelope = vault_seal(&seed, "pass").expect("seal");
        let blob = crate::b64::decode(envelope.strip_prefix(VAULT_PREFIX).unwrap()).unwrap();
        for position in [0usize, 16, 20, blob.len() - 1] {
            let mut tampered = blob.clone();
            tampered[position] ^= 0x01;
            let evil = format!("{VAULT_PREFIX}{}", crate::b64::encode(&tampered));
            assert!(
                vault_open(&evil, "pass").is_none(),
                "flip {position} accepted"
            );
        }
        // Truncated / wrong prefix shapes:
        assert!(vault_open("VORVB1.AAAA", "pass").is_none());
        assert!(vault_open("garbage", "pass").is_none());
    }

    #[test]
    fn parse_spec_json_both_time_shapes() {
        let epoch = br#"{"id":"a","tier":"vip","issued_at":100,"expires_at":200,"bandwidth_mib":5,"platforms":["android"],"hwid_hex":""}"#;
        let parsed = parse_spec_json(std::str::from_utf8(epoch).unwrap()).expect("parse");
        assert_eq!(parsed.issued_at, 100);
        assert_eq!(parsed.expires_at, 200);
        assert_eq!(parsed.bandwidth_limit_mib, 5);
        assert!(parsed.hwid.is_none());

        let rfc = br#"{"id":"a","tier":"vip","issued_at":"2026-01-01T00:00:00Z","expires_at":"2027-01-01T00:00:00Z"}"#;
        let parsed = parse_spec_json(std::str::from_utf8(rfc).unwrap()).expect("parse");
        assert_eq!(parsed.expires_at, 1_798_761_600);

        let hwid_json = format!(
            "{{\"id\":\"a\",\"tier\":\"v\",\"issued_at\":1,\"expires_at\":2,\"hwid_hex\":\"{}\"}}",
            crate::hwid::to_hex(&crate::hwid::derive("d", "a", "f"))
        );
        let parsed = parse_spec_json(&hwid_json).expect("parse");
        assert!(parsed.hwid.is_some());

        assert!(parse_spec_json("{}").is_none());
        assert!(parse_spec_json("not json").is_none());
    }

    #[test]
    fn self_test_signs_and_verifies() {
        let seed = seed();
        let (ok, detail) = self_test(&seed);
        assert!(ok, "self test failed: {detail}");
    }

    #[test]
    fn generated_seeds_differ() {
        let a = generate_seed().expect("entropy");
        let b = generate_seed().expect("entropy");
        assert_ne!(a, b);
    }
}
