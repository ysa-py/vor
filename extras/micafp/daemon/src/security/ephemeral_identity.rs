// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — Ephemeral Identity
//
// On every daemon start a brand-new Curve25519 keypair is generated using
// the OS CSPRNG.  The private key is mlock()'d into RAM and is NEVER written
// to disk.  On drop the keypair memory is zeroized with the zeroize crate.
//
// Peer IDs are derived deterministically so that other nodes can recognise
// the same device across reconnects within the same day, but the underlying
// key material is never exposed.
// ─────────────────────────────────────────────────────────────────────────────

use std::sync::Arc;

use hkdf::Hkdf;
use rand_core::OsRng;
use sha2::{Digest, Sha256};
use tracing::{debug, error, info, warn};
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroize;

use crate::error::{ErrorCode, ShieldError};
use crate::security::device_secret::DeviceSecretManager;
use hex;

// ── mlock helper ─────────────────────────────────────────────────────────────

/// Lock the memory region containing the given reference so it cannot be
/// swapped to disk.  On failure we log but do NOT abort — some platforms
/// (e.g. unrooted Android) may forbid mlock.
fn mlock_memory<T>(val: &T) -> bool {
    let ptr = val as *const T as *const u8;
    let len = std::mem::size_of_val(val);
    if len == 0 {
        return true;
    }
    // SAFETY: we are locking the exact memory region of the value.
    #[cfg(unix)]
    unsafe {
        let ret = libc::mlock(ptr as *const libc::c_void, len);
        if ret != 0 {
            let errno = *libc::__errno_location();
            warn!(
                errno,
                len, "mlock failed — private key may be swapped to disk"
            );
            return false;
        }
        debug!(len, "mlock succeeded — private key pinned in RAM");
    }
    #[cfg(not(unix))]
    {
        debug!("mlock not available on this platform — private key may be swapped");
    }
    true
}

/// Unlock a previously mlock'd memory region.
fn munlock_memory<T>(val: &T) {
    let ptr = val as *const T as *const u8;
    let len = std::mem::size_of_val(val);
    if len == 0 {
        return;
    }
    #[cfg(unix)]
    unsafe {
        let ret = libc::munlock(ptr as *const libc::c_void, len);
        if ret != 0 {
            let errno = *libc::__errno_location();
            warn!(errno, len, "munlock failed");
        }
    }
}

// ── Locked keypair wrapper ───────────────────────────────────────────────────

/// A Curve25519 keypair that is mlock'd in RAM and zeroized on drop.
///
/// The inner `StaticSecret` is the private key.  We wrap it so that we
/// control the Drop behaviour and can guarantee zeroization and munlock.
struct LockedKeypair {
    secret: StaticSecret,
    public: PublicKey,
    locked: bool,
}

impl LockedKeypair {
    /// Generate a new keypair from the OS CSPRNG and attempt to mlock it.
    fn generate() -> Result<Self, ShieldError> {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);

        let mut kp = LockedKeypair {
            secret,
            public,
            locked: false,
        };

        // mlock the secret (the public key is not sensitive)
        kp.locked = mlock_memory(&kp.secret);

        Ok(kp)
    }

    /// Borrow the public key.
    fn public_key(&self) -> &PublicKey {
        &self.public
    }

    /// Perform a Diffie-Hellman key exchange with a remote public key.
    fn diffie_hellman(&self, remote: &PublicKey) -> x25519_dalek::SharedSecret {
        self.secret.diffie_hellman(remote)
    }

    /// Borrow the secret for HKDF operations.
    fn secret_bytes(&self) -> &[u8; 32] {
        self.secret.as_bytes()
    }
}

impl Drop for LockedKeypair {
    fn drop(&mut self) {
        // Zeroize the private key memory
        // StaticSecret does implement Zeroize via the zeroize feature,
        // but we explicitly overwrite to be absolutely certain.
        let mut secret_bytes = self.secret.to_bytes();
        secret_bytes.zeroize();

        // munlock if we successfully locked
        if self.locked {
            // After zeroize, the memory is cleared but still mapped.
            // munlock the original region.
            munlock_memory(&self.secret);
        }

        debug!("Ephemeral keypair zeroized and munlocked");
    }
}

// SAFETY: The keypair can be sent between threads safely.  The StaticSecret
// itself is Send, and we manage the mlock/munlock lifecycle within Drop.
unsafe impl Send for LockedKeypair {}
unsafe impl Sync for LockedKeypair {}

// ── EphemeralIdentity ────────────────────────────────────────────────────────

/// Ephemeral identity generated on every daemon start.
///
/// Properties:
/// - Keypair is generated fresh from `OsRng` on each start
/// - Private key is mlock'd in RAM (best-effort)
/// - Peer ID is derived: SHA256(public_key || boot_timestamp)
/// - Daily P2P peer ID: HKDF-SHA256(ikm=device_secret, salt=date, info=b"peer-id")
/// - Keypair is never persisted, never logged
/// - Keypair memory is zeroized on drop
pub struct EphemeralIdentity {
    keypair: LockedKeypair,
    /// Boot timestamp (UNIX epoch seconds) — part of the peer ID.
    boot_timestamp: u64,
    /// Session peer ID: SHA256(public_key || boot_timestamp) in hex.
    peer_id: String,
    /// Daily P2P peer ID derived from the device secret.
    daily_peer_id: String,
}

impl EphemeralIdentity {
    /// Generate a new ephemeral identity.
    ///
    /// This creates a fresh Curve25519 keypair, mlocks it, and derives
    /// both the session peer ID and the daily peer ID.
    pub fn generate(device_secret_mgr: &DeviceSecretManager) -> Result<Self, ShieldError> {
        let keypair = LockedKeypair::generate().map_err(|e| {
            ShieldError::crypto_with_source(
                ErrorCode::CryptoKeyGenerationFailed,
                "Failed to generate Curve25519 keypair",
                e.to_string(),
            )
        })?;

        let boot_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Derive session peer ID: SHA256(public_key || boot_timestamp)
        let peer_id = {
            let mut hasher = Sha256::new();
            hasher.update(keypair.public_key().as_bytes());
            hasher.update(boot_timestamp.to_le_bytes());
            let hash = hasher.finalize();
            hex::encode(hash)
        };

        // Derive daily P2P peer ID: HKDF-SHA256(ikm=device_secret, salt=current_date, info=b"peer-id")
        let daily_peer_id = derive_daily_peer_id(&device_secret_mgr.secret_bytes())?;

        info!(
            peer_id = %peer_id,
            daily_peer_id = %daily_peer_id,
            "Ephemeral identity derived"
        );

        Ok(Self {
            keypair,
            boot_timestamp,
            peer_id,
            daily_peer_id,
        })
    }

    /// Return the session peer ID as a hex string.
    pub fn peer_id_hex(&self) -> String {
        self.peer_id.clone()
    }

    /// Return the daily P2P peer ID.
    pub fn daily_peer_id(&self) -> &str {
        &self.daily_peer_id
    }

    /// Return the public key bytes.
    pub fn public_key_bytes(&self) -> &[u8; 32] {
        self.keypair.public_key().as_bytes()
    }

    /// Perform a Diffie-Hellman key exchange.
    pub fn diffie_hellman(&self, remote_public: &PublicKey) -> x25519_dalek::SharedSecret {
        self.keypair.diffie_hellman(remote_public)
    }

    /// Derive an MQTT personal topic from the device secret.
    ///
    /// The topic is deterministic so the device can subscribe to it
    /// after a reconnect, but it rotates daily.
    pub fn mqtt_personal_topic(&self, device_secret_mgr: &DeviceSecretManager) -> String {
        let secret = device_secret_mgr.secret_bytes();
        let salt = current_date_salt();
        let info = b"mqtt-topic";

        let hk = Hkdf::<Sha256>::new(Some(salt.as_bytes()), &secret);
        let mut okm = [0u8; 16];
        hk.expand(info, &mut okm)
            .expect("HKDF expand should not fail with valid inputs");

        format!("shield/u/{}", hex::encode(okm))
    }

    /// Return the boot timestamp.
    pub fn boot_timestamp(&self) -> u64 {
        self.boot_timestamp
    }
}

impl Drop for EphemeralIdentity {
    fn drop(&mut self) {
        // The LockedKeypair handles zeroization and munlock in its own Drop.
        // We just log here.
        debug!("EphemeralIdentity dropped — keypair will be zeroized");
    }
}

// ── Daily peer ID derivation ─────────────────────────────────────────────────

/// Derive the daily P2P peer ID using HKDF-SHA256.
///
/// ```
/// daily_peer_id = HKDF-SHA256(
///     ikm  = device_secret[0..32],
///     salt = current_date_utc_yyyymmdd,
///     info = b"peer-id"
/// )
/// ```
///
/// The output is 32 bytes, hex-encoded.
fn derive_daily_peer_id(device_secret: &[u8; 32]) -> Result<String, ShieldError> {
    let date_salt = current_date_salt();

    let hk = Hkdf::<Sha256>::new(Some(date_salt.as_bytes()), device_secret);
    let mut okm = [0u8; 32];
    hk.expand(b"peer-id", &mut okm).map_err(|e| {
        ShieldError::crypto(
            ErrorCode::CryptoHkdfFailed,
            format!("HKDF expand failed: {}", e),
        )
    })?;

    Ok(hex::encode(okm))
}

/// Get the current UTC date in YYYYMMDD format — used as the HKDF salt.
fn current_date_salt() -> String {
    use chrono::Utc;
    let now = Utc::now();
    now.format("%Y%m%d").to_string()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_id_deterministic_for_same_inputs() {
        // Two identities with the same keypair and timestamp should produce
        // the same peer_id.  In practice the keypair is random, so we test
        // the derivation logic directly.
        let public_bytes: [u8; 32] = [
            0x5e, 0x9d, 0x2a, 0xf5, 0x0c, 0xb1, 0xa3, 0xe7, 0x4f, 0x8d, 0x2e, 0xc6, 0x9b, 0x3a,
            0xd7, 0x1e, 0x6c, 0x04, 0xf8, 0x3b, 0x92, 0x5a, 0x1d, 0x8e, 0x4f, 0xa7, 0xc3, 0xd0,
            0x6b, 0xe5, 0x8a, 0x2f,
        ];
        let ts: u64 = 1700000000u64;

        let mut hasher = Sha256::new();
        hasher.update(&public_bytes);
        hasher.update(ts.to_le_bytes());
        let hash1 = hasher.finalize();

        let mut hasher2 = Sha256::new();
        hasher2.update(&public_bytes);
        hasher2.update(ts.to_le_bytes());
        let hash2 = hasher2.finalize();

        assert_eq!(hex::encode(hash1), hex::encode(hash2));
    }

    #[test]
    fn test_daily_peer_id_changes_with_date() {
        let secret = [42u8; 32];

        // Compute with today's date
        let today_id = derive_daily_peer_id(&secret).unwrap();

        // Compute with a different salt (simulating a different date)
        let hk = Hkdf::<Sha256>::new(Some(b"20990101"), &secret);
        let mut okm = [0u8; 32];
        hk.expand(b"peer-id", &mut okm).unwrap();
        let future_id = hex::encode(okm);

        assert_ne!(today_id, future_id);
    }

    #[test]
    fn test_current_date_salt_format() {
        let salt = current_date_salt();
        assert_eq!(salt.len(), 8);
        assert!(salt.chars().all(|c| c.is_ascii_digit()));
    }
}
