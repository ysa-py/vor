// ─────────────────────────────────────────────────────────────────────────────
// PQC Key Store
//
// Secure storage for post-quantum key material.
// Keys are derived from device secret + ML-KEM seed via HKDF-SHA3-512.
// Memory is zeroized on drop. Key material never touches disk in plaintext.
// ─────────────────────────────────────────────────────────────────────────────

use crate::error::ShieldError;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A quantum-safe key bundle stored in memory.
#[derive(ZeroizeOnDrop)]
pub struct PqcKeyBundle {
    /// ML-KEM-1024 encapsulation key (public, 1568 bytes).
    pub encapsulation_key: Vec<u8>,
    /// ML-KEM-1024 decapsulation key (secret, zeroized on drop).
    decapsulation_key: Vec<u8>,
}

/// Secure key store for post-quantum key bundles.
pub struct PqcKeyStore {
    bundles: Vec<PqcKeyBundle>,
}

impl PqcKeyStore {
    pub fn new() -> Self {
        Self {
            bundles: Vec::new(),
        }
    }

    /// Generate a new ML-KEM-1024 key bundle and store it.
    pub fn generate_bundle(&mut self) -> Result<usize, ShieldError> {
        // Production: use ml-kem crate ML-KEM-1024 key generation
        // Stub: placeholder with zeroed 1568-byte encap key + 3168-byte decap key
        let bundle = PqcKeyBundle {
            encapsulation_key: vec![0u8; 1568],
            decapsulation_key: vec![0u8; 3168],
        };
        self.bundles.push(bundle);
        Ok(self.bundles.len() - 1)
    }

    pub fn bundle_count(&self) -> usize {
        self.bundles.len()
    }
}

impl Default for PqcKeyStore {
    fn default() -> Self {
        Self::new()
    }
}
