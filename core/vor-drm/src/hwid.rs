//! Hardware-ID binding.
//!
//! The license is locked to a digest of device identity components gathered
//! by the Kotlin layer WITHOUT any permission-gated APIs:
//!
//! * **Widevine MediaDRM device ID** — a per-device factory identity, no
//!   permission required (the same signal commercial DRM stacks use);
//! * **ANDROID_ID** (SSAID) — resets only on factory reset;
//! * **Build fingerprint** — model/hardware/build stability marker.
//!
//! The digest is SHA-256 over a domain-separated, length-prefixed
//! concatenation:
//!
//! ```text
//! hwid = SHA-256( "VOR-HWID-v1" || len(drm_id)||drm_id ||
//!                         len(android_id)||android_id ||
//!                         len(fingerprint)||fingerprint )
//! ```
//!
//! Rules:
//!
//! * components are trimmed; `drm_id`/`android_id` lowercased (both are hex
//!   by definition, so this never changes meaning);
//! * an unavailable component is an EMPTY component — the digest remains
//!   deterministic and stable, and a license issued for the resulting HWID
//!   still verifies only on devices producing the exact same components;
//! * matching is CONSTANT-TIME ([`subtle::ConstantTimeEq]) — no early-exit
//!   byte compare to side-channel.
//!
//! Honest limits: a rooted attacker can hook the framework APIs that supply
//! the components, and factory reset legitimately changes the identity (the
//! admin re-issues; the token is not transferable, which is the point).

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Digest size (bytes).
pub const HWID_LEN: usize = 32;

/// Domain separation prefix.
const DOMAIN: &[u8] = b"VOR-HWID-v1";

/// Derive the 32-byte hardware fingerprint from raw identity components.
pub fn derive(drm_id: &str, android_id: &str, build_fingerprint: &str) -> [u8; HWID_LEN] {
    let drm = drm_id.trim().to_ascii_lowercase();
    let aid = android_id.trim().to_ascii_lowercase();
    let fp = build_fingerprint.trim();

    let mut hasher = Sha256::new();
    hasher.update(DOMAIN);
    absorb(&mut hasher, drm.as_bytes());
    absorb(&mut hasher, aid.as_bytes());
    absorb(&mut hasher, fp.as_bytes());
    let mut out = [0u8; HWID_LEN];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// Absorb one length-prefixed component (length prefix makes the
/// concatenation unambiguous regardless of component content).
fn absorb(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u32).to_le_bytes());
    hasher.update(bytes);
}

/// Lowercase hex of a digest (display + stable wire form).
pub fn to_hex(digest: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push(HEX[(*byte >> 4) as usize] as char);
        out.push(HEX[(*byte & 0x0F) as usize] as char);
    }
    out
}

/// Parse a 64-char lowercase/uppercase hex string into a digest.
pub fn from_hex(text: &str) -> Option<[u8; HWID_LEN]> {
    let trimmed = text.trim();
    if trimmed.len() != HWID_LEN * 2 {
        return None;
    }
    let mut out = [0u8; HWID_LEN];
    for (i, slot) in out.iter_mut().enumerate() {
        let high = (trimmed.as_bytes()[i * 2] as char).to_digit(16)?;
        let low = (trimmed.as_bytes()[i * 2 + 1] as char).to_digit(16)?;
        *slot = ((high << 4) | low) as u8;
    }
    Some(out)
}

/// Constant-time digest equality.
pub fn matches(a: &[u8; HWID_LEN], b: &[u8; HWID_LEN]) -> bool {
    a.ct_eq(b).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_same_components() {
        let a = derive("AABBCC", "0102030405060708", "google/redfin/redfin:13/TP1A");
        let b = derive("aabbcc", "0102030405060708", "google/redfin/redfin:13/TP1A");
        assert_eq!(a, b, "drm_id is lowercased before hashing");
    }

    #[test]
    fn changes_when_any_component_changes() {
        let base = derive("aabbcc", "0102030405060708", "fingerprint/1");
        assert_ne!(base, derive("bbccdd", "0102030405060708", "fingerprint/1"));
        assert_ne!(base, derive("aabbcc", "9999999999999999", "fingerprint/1"));
        assert_ne!(base, derive("aabbcc", "0102030405060708", "fingerprint/2"));
    }

    #[test]
    fn empty_components_are_stable_not_zero() {
        let a = derive("", "", "");
        let b = derive("", "", "");
        assert_eq!(a, b);
        assert_ne!(a, [0u8; HWID_LEN]);
    }

    #[test]
    fn length_prefix_prevents_concatenation_ambiguity() {
        // ("ab", "c", "") must differ from ("a", "bc", "") — naive
        // concatenation hashing would collide here; length prefixes don't.
        let a = derive("ab", "c", "f");
        let b = derive("a", "bc", "f");
        assert_ne!(a, b);
    }

    #[test]
    fn hex_roundtrip() {
        let digest = derive("x", "y", "z");
        let hex = to_hex(&digest);
        assert_eq!(hex.len(), 64);
        assert_eq!(from_hex(&hex), Some(digest));
        assert_eq!(from_hex(&hex.to_uppercase()), Some(digest));
        assert_eq!(from_hex("zz"), None);
        assert_eq!(from_hex(""), None);
        assert_eq!(from_hex(&hex[..63]), None);
    }

    #[test]
    fn matches_is_exact() {
        let a = derive("a", "b", "c");
        let b = derive("a", "b", "c");
        let c = derive("a", "b", "d");
        assert!(matches(&a, &b));
        assert!(!matches(&a, &c));
    }

    #[test]
    fn digests_differ_across_realistic_devices() {
        let s21 = derive(
            "0d3f4c1e8a9b7f2a6e5d1c0b4a9f8e7d6c5b4a3f2e1d0c9b8a7f6e5d",
            "c9f1e2d3a4b5",
            "samsung/o1s/o1s:14/UP1A/231005:user/release-keys",
        );
        let pixel8 = derive(
            "f4e5d6c7b8a97011223344556677889900aabbccddeeff",
            "9876543210fedcba",
            "google/shiba/shiba:15/AP4A/user/release-keys",
        );
        assert_ne!(s21, pixel8);
    }
}
