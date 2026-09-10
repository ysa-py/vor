// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Mesh Cryptography
// Per-hop encryption with X25519 ECDH + ChaCha20-Poly1305.
// Each hop gets a fresh session key. Provides hop-level forward secrecy.
// ─────────────────────────────────────────────────────────────────────────────

use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret};
use zeroize::Zeroize;

use crate::error::{ErrorCode, ShieldError};

const NONCE_LEN: usize = 12;

/// Per-hop session key derived from ECDH.
pub struct MeshSessionKey {
    cipher: ChaCha20Poly1305,
}

impl MeshSessionKey {
    /// Derive a session key from an X25519 shared secret via HKDF.
    pub fn from_shared_secret(shared: &[u8], info: &[u8]) -> Result<Self, ShieldError> {
        use hkdf::Hkdf;
        use sha2::Sha256;

        let hk = Hkdf::<Sha256>::new(None, shared);
        let mut key = [0u8; 32];
        hk.expand(info, &mut key)
            .map_err(|_| ShieldError::new(ErrorCode::CryptoError, "HKDF expand failed"))?;

        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|_| ShieldError::new(ErrorCode::CryptoError, "key init failed"))?;
        key.zeroize();
        Ok(Self { cipher })
    }

    /// Encrypt a plaintext message. Returns nonce || ciphertext.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, ShieldError> {
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let mut output = Vec::with_capacity(NONCE_LEN + plaintext.len() + 16);
        output.extend_from_slice(&nonce);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|_| ShieldError::new(ErrorCode::CryptoError, "mesh encrypt failed"))?;
        output.extend_from_slice(&ciphertext);
        Ok(output)
    }

    /// Decrypt nonce || ciphertext. Returns plaintext.
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, ShieldError> {
        if data.len() < NONCE_LEN {
            return Err(ShieldError::new(ErrorCode::CryptoError, "data too short"));
        }
        let nonce = Nonce::from_slice(&data[..NONCE_LEN]);
        self.cipher
            .decrypt(nonce, &data[NONCE_LEN..])
            .map_err(|_| ShieldError::new(ErrorCode::CryptoError, "mesh decrypt failed"))
    }
}
