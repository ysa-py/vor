//! VOR2 token format: encrypted, signed, hardware-locked license envelope.
//!
//! Wire form:
//!
//! ```text
//! token  = "VOR2" "." b64url(envelope) "." b64url(signature)
//! envelope = header(4) || nonce(12) || ciphertext || tag(16)
//! header   = magic "V2" || version 2 || flags (bit0 = encrypted payload)
//! signature = Ed25519 over the ENTIRE envelope bytes
//! ```
//!
//! The plaintext inside AES-256-GCM is a compact canonical BINARY payload
//! (see [`LicenseFields::encode`]) carrying the granular entitlements:
//! expiration, bandwidth limit, tier, platforms and the locked HWID. The
//! payload key is derived per-issuer:
//!
//! ```text
//! key = HKDF-SHA256(ikm = app secret, salt = issuer public key,
//!                   info = "VOR2-payload-v1")
//! ```
//!
//! so the ciphertext is opaque to anyone without the .so (the secret is
//! obfstr-encrypted in the binary), and the whole envelope is additionally
//! covered by the Ed25519 signature — confidentiality AND integrity, with
//! the signature verified BEFORE any decryption is attempted.
//!
//! Canonical binary payload layout (little-endian, versioned by a leading
//! format byte; all lengths bounds-checked):
//!
//! ```text
//! u8  fmt = 1
//! u32 id_len || id bytes (UTF-8, 1..=64)
//! i64 issued_at  (epoch seconds)
//! i64 expires_at (epoch seconds)
//! u64 bandwidth_limit_mib (0 = unlimited)
//! u8  tier_len || tier bytes (UTF-8, 1..=24)
//! u8  platform_count || per platform: u8 len || bytes (1..=16, count 0..=8)
//! u8  hwid_present || [u8;32] when present
//! ```
//!
//! Total payload <= 512 bytes; total envelope <= 544 bytes; total token
//! <= 820 chars — bounded everywhere, allocation-bounded parsing.

use crate::b64;
use crate::hwid::HWID_LEN;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hkdf::Hkdf;
use sha2::Sha256;
// `Digest` is only needed by the cfg(test) sha256 helper below.
#[cfg(test)]
use sha2::Digest;
use zeroize::Zeroizing;

/// Envelope magic bytes.
pub const MAGIC: [u8; 2] = *b"V2";
/// Envelope version byte.
pub const VERSION: u8 = 2;
/// Flag: payload encrypted (the only defined mode; unset = reserved).
pub const FLAG_ENCRYPTED: u8 = 0x01;

/// AES-GCM nonce length.
pub const NONCE_LEN: usize = 12;
/// AES-GCM tag length.
pub const TAG_LEN: usize = 16;
/// Envelope header length.
pub const HEADER_LEN: usize = 4;
/// Minimum envelope length (header + nonce + tag).
pub const MIN_ENVELOPE_LEN: usize = HEADER_LEN + NONCE_LEN + TAG_LEN;
/// Maximum plaintext payload (bytes).
pub const MAX_PAYLOAD_LEN: usize = 512;
/// Maximum token string length.
pub const MAX_TOKEN_LEN: usize = 820;

/// Maximum license id length.
pub const MAX_ID_LEN: usize = 64;
/// Maximum tier length.
pub const MAX_TIER_LEN: usize = 24;
/// Maximum platforms.
pub const MAX_PLATFORMS: usize = 8;
/// Maximum platform name length.
pub const MAX_PLATFORM_LEN: usize = 16;
/// Maximum bandwidth (MiB) — 1 EiB sanity cap, effectively "unlimited".
pub const MAX_BANDWIDTH_MIB: u64 = 1 << 50;

/// HKDF info string (domain separation for the payload key).
const HKDF_INFO: &[u8] = b"VOR2-payload-v1";

/// The granular license metadata carried by a VOR2 token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseFields {
    /// License id (1..=64 chars, no control characters).
    pub id: String,
    /// Issuance time (epoch seconds).
    pub issued_at: i64,
    /// Expiration time (epoch seconds).
    pub expires_at: i64,
    /// Bandwidth limit in MiB (0 = unlimited).
    pub bandwidth_limit_mib: u64,
    /// User tier (1..=24 chars, e.g. "vip" / "standard").
    pub tier: String,
    /// Allowed platforms (0..=8 entries, 1..=16 chars each).
    pub platforms: Vec<String>,
    /// Locked hardware digest (None = portable token).
    pub hwid: Option<[u8; HWID_LEN]>,
}

impl LicenseFields {
    /// Encode to the canonical binary payload.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64 + self.id.len() + self.tier.len());
        out.push(1u8); // fmt
        out.extend_from_slice(&(self.id.len() as u32).to_le_bytes());
        out.extend_from_slice(self.id.as_bytes());
        out.extend_from_slice(&self.issued_at.to_le_bytes());
        out.extend_from_slice(&self.expires_at.to_le_bytes());
        out.extend_from_slice(&self.bandwidth_limit_mib.to_le_bytes());
        out.push(self.tier.len() as u8);
        out.extend_from_slice(self.tier.as_bytes());
        out.push(self.platforms.len() as u8);
        for platform in &self.platforms {
            out.push(platform.len() as u8);
            out.extend_from_slice(platform.as_bytes());
        }
        match self.hwid {
            Some(digest) => {
                out.push(1u8);
                out.extend_from_slice(&digest);
            }
            None => out.push(0u8),
        }
        out
    }

    /// Decode from the canonical binary payload (strict: caps enforced,
    /// whole buffer must be consumed).
    pub fn decode(bytes: &[u8]) -> Option<LicenseFields> {
        let mut cursor = Cursor { bytes, pos: 0 };
        if cursor.u8()? != 1 {
            return None; // unknown payload format
        }
        let id_len = cursor.u32()? as usize;
        if !(1..=MAX_ID_LEN).contains(&id_len) {
            return None;
        }
        let id = cursor.utf8(id_len)?;
        if id.chars().any(|c| c.is_control()) {
            return None;
        }
        let issued_at = cursor.i64()?;
        let expires_at = cursor.i64()?;
        if issued_at < 0 || expires_at <= 0 {
            return None;
        }
        let bandwidth_limit_mib = cursor.u64()?;
        if bandwidth_limit_mib > MAX_BANDWIDTH_MIB {
            return None;
        }
        let tier_len = cursor.u8()? as usize;
        if !(1..=MAX_TIER_LEN).contains(&tier_len) {
            return None;
        }
        let tier = cursor.utf8(tier_len)?;
        if tier.chars().any(|c| c.is_control()) {
            return None;
        }
        let platform_count = cursor.u8()? as usize;
        if platform_count > MAX_PLATFORMS {
            return None;
        }
        let mut platforms = Vec::with_capacity(platform_count);
        for _ in 0..platform_count {
            let len = cursor.u8()? as usize;
            if !(1..=MAX_PLATFORM_LEN).contains(&len) {
                return None;
            }
            platforms.push(cursor.utf8(len)?);
        }
        let hwid = match cursor.u8()? {
            0 => None,
            1 => {
                let mut digest = [0u8; HWID_LEN];
                let raw = cursor.take(HWID_LEN)?;
                digest.copy_from_slice(raw);
                Some(digest)
            }
            _ => return None,
        };
        if cursor.remaining() != 0 {
            return None; // trailing bytes = not canonical
        }
        Some(LicenseFields {
            id,
            issued_at,
            expires_at,
            bandwidth_limit_mib,
            tier,
            platforms,
            hwid,
        })
    }
}

/// Bounded cursor over a byte slice (no panics, no allocation).
struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn u8(&mut self) -> Option<u8> {
        let value = *self.bytes.get(self.pos)?;
        self.pos += 1;
        Some(value)
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn i64(&mut self) -> Option<i64> {
        Some(i64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let slice = self.bytes.get(self.pos..self.pos.checked_add(len)?)?;
        self.pos += len;
        Some(slice)
    }
    fn utf8(&mut self, len: usize) -> Option<String> {
        let raw = self.take(len)?;
        std::str::from_utf8(raw).ok().map(str::to_string)
    }
    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }
}

/// Derive the payload encryption key for one issuer public key.
pub fn derive_payload_key(app_secret: &[u8; 32], issuer_public: &[u8; 32]) -> Zeroizing<[u8; 32]> {
    let mut okm = Zeroizing::new([0u8; 32]);
    let hk = Hkdf::<Sha256>::new(Some(issuer_public), app_secret);
    // info + expand are infallible for a 32-byte OKM with SHA-256.
    let _ = hk.expand(HKDF_INFO, okm.as_mut());
    okm
}

/// AAD for the AES-GCM seal: the 4-byte header (binds magic/version/flags
/// to the ciphertext; changing any header byte fails the tag).
fn aad_for_header(header: &[u8]) -> [u8; HEADER_LEN] {
    let mut aad = [0u8; HEADER_LEN];
    aad.copy_from_slice(&header[..HEADER_LEN]);
    aad
}

/// Build the envelope (header + nonce + ciphertext||tag) for a payload.
pub fn seal_envelope(
    payload: &[u8],
    app_secret: &[u8; 32],
    issuer_public: &[u8; 32],
) -> Option<Vec<u8>> {
    if payload.len() > MAX_PAYLOAD_LEN {
        return None;
    }
    let key = derive_payload_key(app_secret, issuer_public);
    let cipher = Aes256Gcm::new((&*key).into());
    let mut nonce_bytes = [0u8; NONCE_LEN];
    getrandom::getrandom(&mut nonce_bytes).ok()?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let header = [MAGIC[0], MAGIC[1], VERSION, FLAG_ENCRYPTED];
    let sealed = cipher
        .encrypt(
            nonce,
            Payload {
                msg: payload,
                aad: &aad_for_header(&header),
            },
        )
        .ok()?;
    let mut envelope = Vec::with_capacity(HEADER_LEN + NONCE_LEN + sealed.len());
    envelope.extend_from_slice(&header);
    envelope.extend_from_slice(&nonce_bytes);
    envelope.extend_from_slice(&sealed);
    Some(envelope)
}

/// Open the envelope: verify the header, decrypt (GCM tag), return plaintext.
pub fn open_envelope(
    envelope: &[u8],
    app_secret: &[u8; 32],
    issuer_public: &[u8; 32],
) -> Option<Vec<u8>> {
    if envelope.len() < MIN_ENVELOPE_LEN || envelope.len() > MIN_ENVELOPE_LEN + MAX_PAYLOAD_LEN {
        return None;
    }
    let header = envelope.get(..HEADER_LEN)?;
    if header[0] != MAGIC[0] || header[1] != MAGIC[1] || header[2] != VERSION {
        return None;
    }
    if header[3] != FLAG_ENCRYPTED {
        return None; // only the encrypted mode exists
    }
    let nonce_bytes = envelope.get(HEADER_LEN..HEADER_LEN + NONCE_LEN)?;
    let ciphertext = envelope.get(HEADER_LEN + NONCE_LEN..)?;
    if ciphertext.len() < TAG_LEN {
        return None;
    }
    let key = derive_payload_key(app_secret, issuer_public);
    let cipher = Aes256Gcm::new((&*key).into());
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(nonce_bytes),
            Payload {
                msg: ciphertext,
                aad: &aad_for_header(header),
            },
        )
        .ok()?;
    if plaintext.len() > MAX_PAYLOAD_LEN {
        return None;
    }
    Some(plaintext)
}

/// Sign the envelope with the issuer's Ed25519 signing key (64 bytes).
pub fn sign_envelope(envelope: &[u8], signing_key: &SigningKey) -> [u8; 64] {
    signing_key.sign(envelope).to_bytes()
}

/// Verify the envelope signature against a public key (constant-time
/// Ed25519 verification).
pub fn verify_envelope(
    envelope: &[u8],
    signature: &[u8; 64],
    verifying_key: &VerifyingKey,
) -> bool {
    let signature = Signature::from_bytes(signature);
    verifying_key.verify(envelope, &signature).is_ok()
}

/// Render the full token string.
pub fn render_token(envelope: &[u8], signature: &[u8; 64]) -> String {
    let mut token = String::with_capacity(MAX_TOKEN_LEN + 8);
    token.push_str(obfstr::obfstr!("VOR2"));
    token.push('.');
    token.push_str(&b64::encode(envelope));
    token.push('.');
    token.push_str(&b64::encode(signature));
    token
}

/// Split a candidate VOR2 token string into (envelope, signature).
/// Strict: prefix, segment counts, base64 validity, length bounds.
pub fn parse_token(token: &str) -> Option<(Vec<u8>, [u8; 64])> {
    let token = token.trim();
    if token.len() > MAX_TOKEN_LEN || !token.starts_with(obfstr::obfstr!("VOR2.")) {
        return None;
    }
    let rest = token.get(5..)?;
    let (envelope_b64, signature_b64) = rest.split_once('.')?;
    let envelope = b64::decode(envelope_b64)?;
    let signature_bytes = b64::decode(signature_b64)?;
    if envelope.len() < MIN_ENVELOPE_LEN || envelope.len() > MIN_ENVELOPE_LEN + MAX_PAYLOAD_LEN {
        return None;
    }
    let signature: [u8; 64] = signature_bytes.try_into().ok()?;
    Some((envelope, signature))
}

/// (test helper) deterministic SHA-256 of arbitrary bytes — used by tests
/// that need a stable "public key" stand-in for key derivation.
#[cfg(test)]
pub(crate) fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields() -> LicenseFields {
        LicenseFields {
            id: "11111111-2222-3333-4444-555555555555".to_string(),
            issued_at: 1_789_040_000,
            expires_at: 1_798_761_600,
            bandwidth_limit_mib: 512 * 1024,
            tier: "vip".to_string(),
            platforms: vec!["android".to_string()],
            hwid: Some(sha256(b"test-hwid")),
        }
    }

    #[test]
    fn payload_codec_roundtrip() {
        let original = fields();
        let bytes = original.encode();
        assert!(bytes.len() <= MAX_PAYLOAD_LEN);
        assert_eq!(LicenseFields::decode(&bytes), Some(original));
    }

    #[test]
    fn payload_without_hwid_roundtrip() {
        let mut no_hwid = fields();
        no_hwid.hwid = None;
        let bytes = no_hwid.encode();
        assert_eq!(LicenseFields::decode(&bytes), Some(no_hwid));
    }

    #[test]
    fn payload_decode_rejects_trailing_bytes() {
        let mut bytes = fields().encode();
        bytes.push(0);
        assert!(LicenseFields::decode(&bytes).is_none());
    }

    #[test]
    fn payload_decode_rejects_bad_format_byte() {
        let mut bytes = fields().encode();
        bytes[0] = 9;
        assert!(LicenseFields::decode(&bytes).is_none());
    }

    #[test]
    fn payload_decode_rejects_negative_epochs() {
        let mut evil = fields();
        evil.issued_at = -1;
        assert!(LicenseFields::decode(&evil.encode()).is_none());
        evil = fields();
        evil.expires_at = 0;
        assert!(LicenseFields::decode(&evil.encode()).is_none());
    }

    #[test]
    fn payload_decode_rejects_oversized_id() {
        let mut evil = fields();
        evil.id = "x".repeat(MAX_ID_LEN + 1);
        assert!(LicenseFields::decode(&evil.encode()).is_none());
        // Oversized tier / platform / count likewise.
        evil = fields();
        evil.tier = "t".repeat(MAX_TIER_LEN + 1);
        assert!(LicenseFields::decode(&evil.encode()).is_none());
        evil = fields();
        evil.platforms = vec!["p".repeat(MAX_PLATFORM_LEN + 1)];
        assert!(LicenseFields::decode(&evil.encode()).is_none());
        evil = fields();
        evil.platforms = (0..MAX_PLATFORMS + 1).map(|i| format!("p{i}")).collect();
        assert!(LicenseFields::decode(&evil.encode()).is_none());
    }

    #[test]
    fn payload_decode_rejects_absurd_bandwidth() {
        let mut evil = fields();
        evil.bandwidth_limit_mib = u64::MAX;
        assert!(LicenseFields::decode(&evil.encode()).is_none());
    }

    #[test]
    fn envelope_roundtrip() {
        let secret = sha256(b"app-secret");
        let issuer = sha256(b"issuer-pub");
        let payload = fields().encode();
        let envelope = seal_envelope(&payload, &secret, &issuer).expect("seal");
        assert_eq!(envelope[0], b'V');
        assert_eq!(envelope[1], b'2');
        assert_eq!(envelope[2], VERSION);
        assert_eq!(envelope[3], FLAG_ENCRYPTED);
        let opened = open_envelope(&envelope, &secret, &issuer).expect("open");
        assert_eq!(opened, payload);
    }

    #[test]
    fn envelope_rejects_wrong_secret() {
        let secret = sha256(b"app-secret");
        let issuer = sha256(b"issuer-pub");
        let envelope = seal_envelope(&fields().encode(), &secret, &issuer).expect("seal");
        let wrong = sha256(b"other-secret");
        assert!(open_envelope(&envelope, &wrong, &issuer).is_none());
    }

    #[test]
    fn envelope_rejects_wrong_issuer_key() {
        let secret = sha256(b"app-secret");
        let issuer = sha256(b"issuer-pub");
        let envelope = seal_envelope(&fields().encode(), &secret, &issuer).expect("seal");
        let wrong_issuer = sha256(b"other-issuer");
        assert!(open_envelope(&envelope, &secret, &wrong_issuer).is_none());
    }

    #[test]
    fn envelope_rejects_bitflips_everywhere() {
        let secret = sha256(b"app-secret");
        let issuer = sha256(b"issuer-pub");
        let envelope = seal_envelope(&fields().encode(), &secret, &issuer).expect("seal");
        for position in 0..envelope.len() {
            let mut tampered = envelope.clone();
            tampered[position] ^= 0x01;
            let opened = open_envelope(&tampered, &secret, &issuer);
            if position < HEADER_LEN {
                // Header flips are caught by the magic/version/flags check…
                assert!(opened.is_none(), "header flip at {position} accepted");
            } else {
                // …and every other flip by the GCM tag.
                assert!(opened.is_none(), "tag missed flip at {position}");
            }
        }
    }

    #[test]
    fn token_roundtrip() {
        let secret = sha256(b"app-secret");
        let issuer = sha256(b"issuer-pub");
        let envelope = seal_envelope(&fields().encode(), &secret, &issuer).expect("seal");
        let token = render_token(&envelope, &[7u8; 64]);
        let (parsed_envelope, parsed_signature) = parse_token(&token).expect("parse");
        assert_eq!(parsed_envelope, envelope);
        assert_eq!(parsed_signature, [7u8; 64]);
    }

    #[test]
    fn token_parse_rejects_garbage() {
        for bad in [
            "",
            "VOR2",
            "VOR2.only-two",
            "VOR1.a.b",
            "VOR2.!!.AAAA",
            "VOR2.Zm9v.AAAA", // envelope too short
            &format!("VOR2.{}.AAAA", "A".repeat(MAX_TOKEN_LEN)), // oversize
            "VOR2.Zm9v.AAA",  // signature not 64 bytes
            "VOR2.Zm9v.",
        ] {
            assert!(parse_token(bad).is_none(), "accepted {bad:?}");
        }
    }

    #[test]
    fn derive_key_is_deterministic_and_input_sensitive() {
        let secret = sha256(b"app-secret");
        let issuer = sha256(b"issuer-pub");
        assert_eq!(
            derive_payload_key(&secret, &issuer),
            derive_payload_key(&secret, &issuer)
        );
        assert_ne!(
            derive_payload_key(&secret, &issuer),
            derive_payload_key(&sha256(b"other"), &issuer)
        );
        assert_ne!(
            derive_payload_key(&secret, &issuer),
            derive_payload_key(&secret, &sha256(b"other"))
        );
    }

    #[test]
    fn sign_and_verify_envelope() {
        let seed = sha256(b"issuer-seed");
        let signing = SigningKey::from_bytes(&seed);
        let envelope = vec![1u8, 2, 3, 4, 5];
        let signature = sign_envelope(&envelope, &signing);
        let verifying = VerifyingKey::from(&signing);
        assert!(verify_envelope(&envelope, &signature, &verifying));
        let mut bad = signature;
        bad[0] ^= 0xFF;
        assert!(!verify_envelope(&envelope, &bad, &verifying));
        let mut bad_envelope = envelope.clone();
        bad_envelope[0] ^= 0xFF;
        assert!(!verify_envelope(&bad_envelope, &signature, &verifying));
    }
}
