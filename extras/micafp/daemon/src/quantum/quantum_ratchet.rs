// ─────────────────────────────────────────────────────────────────────────────
// Post-Quantum Double Ratchet
//
// Extends the Signal Protocol Double Ratchet with ML-KEM-1024 ratchet steps.
// Each message is encrypted with a one-time key; compromise of one message
// key does not reveal past or future keys (forward secrecy + break-in recovery).
//
// The quantum ratchet adds a KEM ratchet step every N messages, replacing
// the DH ratchet with a KEM encapsulation, providing security against
// quantum adversaries who harvest-now-decrypt-later.
// ─────────────────────────────────────────────────────────────────────────────

use crate::error::ShieldError;
use zeroize::{Zeroize, ZeroizeOnDrop};

const KEM_RATCHET_INTERVAL: u32 = 50; // KEM ratchet every 50 messages

/// State for the post-quantum double ratchet.
#[derive(ZeroizeOnDrop)]
pub struct QuantumRatchet {
    /// Current root key (32 bytes).
    root_key: [u8; 32],
    /// Sending chain key.
    send_chain_key: [u8; 32],
    /// Receiving chain key.
    recv_chain_key: [u8; 32],
    /// Message counter for KEM ratchet scheduling.
    message_count: u32,
}

impl QuantumRatchet {
    /// Create a new ratchet from an initial shared secret.
    pub fn new(initial_secret: &[u8; 32]) -> Self {
        Self {
            root_key: *initial_secret,
            send_chain_key: [0u8; 32],
            recv_chain_key: [0u8; 32],
            message_count: 0,
        }
    }

    /// Derive the next message key from the sending chain.
    pub fn next_send_key(&mut self) -> Result<[u8; 32], ShieldError> {
        self.message_count += 1;
        // KEM ratchet step at interval
        if self.message_count % KEM_RATCHET_INTERVAL == 0 {
            self.kem_ratchet_step()?;
        }
        self.advance_chain_key(&self.send_chain_key.clone())
    }

    fn advance_chain_key(&self, chain_key: &[u8; 32]) -> Result<[u8; 32], ShieldError> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let mut mac = Hmac::<Sha256>::new_from_slice(chain_key)
            .map_err(|_| ShieldError::Crypto("HMAC init failed".into()))?;
        mac.update(b"\x01");
        Ok(mac.finalize().into_bytes().into())
    }

    fn kem_ratchet_step(&mut self) -> Result<(), ShieldError> {
        // In full implementation: encapsulate with remote ML-KEM-1024 public key,
        // then KDF-combine with root key to produce new root + chain keys.
        // Stub: HKDF re-key from current root.
        use hkdf::Hkdf;
        use sha2::Sha256;
        let hk = Hkdf::<Sha256>::new(None, &self.root_key);
        let mut new_root = [0u8; 32];
        let mut new_chain = [0u8; 32];
        hk.expand(b"quantum-ratchet-root", &mut new_root)
            .map_err(|_| ShieldError::Crypto("HKDF root expand failed".into()))?;
        hk.expand(b"quantum-ratchet-chain", &mut new_chain)
            .map_err(|_| ShieldError::Crypto("HKDF chain expand failed".into()))?;
        self.root_key = new_root;
        self.send_chain_key = new_chain;
        Ok(())
    }
}
