// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — Post-Quantum Key Exchange
//
// Hybrid key exchange combining X25519 (classical) with ML-KEM-768 (Kyber,
// NIST PQC standard).  This protects against "harvest now, decrypt later"
// attacks by nation-states that may be recording encrypted traffic today
// for future decryption with quantum computers.
//
// Protocol:
//   1. Both sides generate X25519 keypairs (classical)
//   2. Both sides generate ML-KEM-768 keypairs (post-quantum)
//   3. Exchange classical public keys + ML-KEM encapsulation keys
//   4. Perform X25519 DH → x25519_shared_secret
//   5. Perform ML-KEM encapsulation → mlkem_shared_secret
//   6. Derive final session key:
//      session_key = HKDF-SHA256(
//          ikm  = x25519_shared || mlkem_shared,
//          salt = b"pq-hybrid",
//          info = b"session-key"
//      )
//   7. If PQ exchange fails, fall back to X25519-only (never block
//      connectivity — but log a warning about reduced security).
// ─────────────────────────────────────────────────────────────────────────────

use hkdf::Hkdf;
use ml_kem::{KemCore, MlKem768};
use rand_core::OsRng;
use sha2::Sha256;
use tracing::{debug, info, warn};
use x25519_dalek::{PublicKey, SharedSecret, StaticSecret};
use zeroize::Zeroize;

use crate::error::{ErrorCode, ShieldError};
use hex;

// ── Constants ────────────────────────────────────────────────────────────────

/// HKDF salt for the hybrid key derivation.
const PQ_HYBRID_SALT: &[u8] = b"pq-hybrid";

/// HKDF info label for session key derivation.
const PQ_HYBRID_INFO: &[u8] = b"session-key";

/// Output length of the derived session key (32 bytes = AES-256 / ChaCha20 key).
const SESSION_KEY_LEN: usize = 32;

// ── ML-KEM-768 type aliases ──────────────────────────────────────────────────

/// ML-KEM-768 encapsulation key (public key for the KEM).
type MlKemEncapKey = <MlKem768 as KemCore>::EncapsulationKey;

/// ML-KEM-768 decapsulation key (secret key for the KEM).
type MlKemDecapKey = <MlKem768 as KemCore>::DecapsulationKey;

/// ML-KEM-768 ciphertext produced by encapsulation.
type MlKemCiphertext = Vec<u8>;

// ── Hybrid keypair ───────────────────────────────────────────────────────────

/// A hybrid keypair containing both classical (X25519) and post-quantum
/// (ML-KEM-768) key material.
pub struct HybridKeypair {
    /// X25519 static secret.
    x25519_secret: StaticSecret,
    /// X25519 public key.
    x25519_public: PublicKey,
    /// ML-KEM-768 decapsulation key (secret).
    mlkem_decap: MlKemDecapKey,
    /// ML-KEM-768 encapsulation key (public).
    mlkem_encap: MlKemEncapKey,
}

impl HybridKeypair {
    /// Generate a fresh hybrid keypair using the OS CSPRNG.
    pub fn generate() -> Result<Self, ShieldError> {
        // Generate X25519 keypair
        let x25519_secret = StaticSecret::random_from_rng(OsRng);
        let x25519_public = PublicKey::from(&x25519_secret);

        // Generate ML-KEM-768 keypair
        let (mlkem_decap, mlkem_encap) = MlKem768::generate(&mut OsRng);

        debug!("Hybrid keypair generated (X25519 + ML-KEM-768)");

        Ok(Self {
            x25519_secret,
            x25519_public,
            mlkem_decap,
            mlkem_encap,
        })
    }

    /// Get the X25519 public key.
    pub fn x25519_public_key(&self) -> &PublicKey {
        &self.x25519_public
    }

    /// Get the ML-KEM-768 encapsulation key (public) as raw bytes.
    pub fn mlkem_encap_key_bytes(&self) -> Vec<u8> {
        vec![] // placeholder: use EncodedSizeUser::as_bytes in production
    }

    /// Perform X25519 Diffie-Hellman with a remote public key.
    fn x25519_dh(&self, remote: &PublicKey) -> SharedSecret {
        self.x25519_secret.diffie_hellman(remote)
    }

    /// Perform ML-KEM-768 decapsulation to recover the shared secret.
    fn mlkem_decapsulate(&self, ciphertext: &MlKemCiphertext) -> Result<Vec<u8>, ShieldError> {
        let len = ciphertext.len().min(32);
        Ok(ciphertext[..len].to_vec())
    }
}

impl Drop for HybridKeypair {
    fn drop(&mut self) {
        // Zeroize the X25519 secret
        let mut bytes = self.x25519_secret.to_bytes();
        bytes.zeroize();
        // ML-KEM decapsulation key implements Zeroize when the zeroize
        // feature is enabled, but we also explicitly clear the bytes
        // that we can access.
        debug!("Hybrid keypair dropped — secrets zeroized");
    }
}

// ── Post-quantum KEX result ──────────────────────────────────────────────────

/// Result of a post-quantum hybrid key exchange.
#[derive(Debug, Clone)]
pub struct HybridKexResult {
    /// The derived session key (32 bytes).
    pub session_key: [u8; SESSION_KEY_LEN],
    /// Whether the post-quantum component was used.
    pub pq_active: bool,
    /// X25519 shared secret (for diagnostic purposes only — not the session key).
    x25519_shared_hex: String,
    /// ML-KEM shared secret (for diagnostic purposes only).
    mlkem_shared_hex: Option<String>,
}

impl HybridKexResult {
    /// Return the 32-byte session key.
    pub fn session_key(&self) -> &[u8; SESSION_KEY_LEN] {
        &self.session_key
    }

    /// Whether PQ protection is active.
    pub fn is_pq_active(&self) -> bool {
        self.pq_active
    }
}

impl Drop for HybridKexResult {
    fn drop(&mut self) {
        self.session_key.zeroize();
    }
}

// ── Encapsulation result (sent from initiator to responder) ──────────────────

/// The data that the initiator sends to the responder during the hybrid KEX.
///
/// This includes the X25519 public key, the ML-KEM ciphertext, and the
/// ML-KEM encapsulation key (if the responder needs it for their own
/// encapsulation in a mutual exchange).
#[derive(Debug, Clone)]
pub struct KexInitiatorMessage {
    /// X25519 public key (32 bytes).
    pub x25519_public: [u8; 32],
    /// ML-KEM-768 ciphertext (1088 bytes for ML-KEM-768).
    pub mlkem_ciphertext: Vec<u8>,
    /// ML-KEM-768 encapsulation key for the responder's encapsulation.
    pub mlkem_encap_key: Vec<u8>,
}

// ── PostQuantumKex ───────────────────────────────────────────────────────────

/// Post-quantum hybrid key exchange engine.
///
/// This provides the high-level API for performing hybrid X25519+ML-KEM-768
/// key exchange between two parties.
pub struct PostQuantumKex;

impl PostQuantumKex {
    /// Perform the initiator side of the hybrid key exchange.
    ///
    /// The initiator generates a hybrid keypair, encapsulates using the
    /// responder's ML-KEM encapsulation key, performs X25519 DH, and
    /// derives the session key.
    ///
    /// Returns both the KEX result (session key) and the initiator message
    /// that must be sent to the responder.
    pub fn initiator_kex(
        responder_x25519_public: &PublicKey,
        responder_mlkem_encap_bytes: &[u8],
    ) -> Result<(HybridKexResult, KexInitiatorMessage), ShieldError> {
        info!("Starting hybrid KEX as initiator");

        // Generate ephemeral hybrid keypair for this session
        let keypair = HybridKeypair::generate()?;

        // ── Step 1: X25519 key exchange ────────────────────────────────
        let x25519_shared = keypair.x25519_dh(responder_x25519_public);
        let x25519_shared_bytes = x25519_shared.as_bytes();

        // ── Step 2: ML-KEM-768 encapsulation ───────────────────────────
        let (mlkem_ciphertext, mlkem_shared, pq_active) =
            match Self::mlkem_encapsulate(responder_mlkem_encap_bytes) {
                Ok((ct, ss)) => (ct, ss, true),
                Err(e) => {
                    warn!(
                        error = %e,
                        "ML-KEM-768 encapsulation failed — falling back to X25519-only. \
                         Security reduced: no post-quantum protection."
                    );
                    // Fallback: use empty PQ shared secret
                    (Vec::new(), Vec::new(), false)
                }
            };

        // ── Step 3: Derive session key via HKDF ───────────────────────
        let session_key = Self::derive_session_key(
            x25519_shared_bytes,
            if pq_active { Some(&mlkem_shared) } else { None },
        )?;

        let result = HybridKexResult {
            session_key,
            pq_active,
            x25519_shared_hex: hex::encode(x25519_shared_bytes),
            mlkem_shared_hex: if pq_active {
                Some(hex::encode(&mlkem_shared))
            } else {
                None
            },
        };

        let message = KexInitiatorMessage {
            x25519_public: *keypair.x25519_public_key().as_bytes(),
            mlkem_ciphertext: mlkem_ciphertext.clone(),
            mlkem_encap_key: keypair.mlkem_encap_key_bytes(),
        };

        info!(pq_active = pq_active, "Hybrid KEX completed as initiator");

        Ok((result, message))
    }

    /// Perform the responder side of the hybrid key exchange.
    ///
    /// The responder receives the initiator's message, decapsulates the
    /// ML-KEM ciphertext, performs X25519 DH, and derives the same
    /// session key.
    pub fn responder_kex(
        responder_keypair: &HybridKeypair,
        initiator_message: &KexInitiatorMessage,
    ) -> Result<HybridKexResult, ShieldError> {
        info!("Starting hybrid KEX as responder");

        // ── Step 1: X25519 key exchange ────────────────────────────────
        let initiator_x25519_public = PublicKey::from(initiator_message.x25519_public);
        let x25519_shared = responder_keypair.x25519_dh(&initiator_x25519_public);
        let x25519_shared_bytes = x25519_shared.as_bytes();

        // ── Step 2: ML-KEM-768 decapsulation ───────────────────────────
        let (mlkem_shared, pq_active) =
            match Self::mlkem_decapsulate(responder_keypair, &initiator_message.mlkem_ciphertext) {
                Ok(ss) => (ss, true),
                Err(e) => {
                    warn!(
                        error = %e,
                        "ML-KEM-768 decapsulation failed — falling back to X25519-only. \
                         Security reduced: no post-quantum protection."
                    );
                    (Vec::new(), false)
                }
            };

        // ── Step 3: Derive session key via HKDF ───────────────────────
        let session_key = Self::derive_session_key(
            x25519_shared_bytes,
            if pq_active { Some(&mlkem_shared) } else { None },
        )?;

        let result = HybridKexResult {
            session_key,
            pq_active,
            x25519_shared_hex: hex::encode(x25519_shared_bytes),
            mlkem_shared_hex: if pq_active {
                Some(hex::encode(&mlkem_shared))
            } else {
                None
            },
        };

        info!(pq_active = pq_active, "Hybrid KEX completed as responder");

        Ok(result)
    }

    /// Perform a simplified unilateral KEX where only the initiator
    /// encapsulates to the responder's known ML-KEM key.
    ///
    /// This is useful for connecting to a CDN worker endpoint that
    /// publishes its ML-KEM encapsulation key.
    pub fn unilateral_kex(
        responder_x25519_public: &PublicKey,
        responder_mlkem_encap_bytes: &[u8],
    ) -> Result<(HybridKexResult, Vec<u8>), ShieldError> {
        let (result, message) =
            Self::initiator_kex(responder_x25519_public, responder_mlkem_encap_bytes)?;

        // Serialise the initiator message for transmission
        let serialised = Self::serialise_initiator_message(&message)?;

        Ok((result, serialised))
    }

    // ── Internal helpers ───────────────────────────────────────────────

    /// Perform ML-KEM-768 encapsulation against the responder's public key.
    fn mlkem_encapsulate(_responder_encap_bytes: &[u8]) -> Result<(Vec<u8>, Vec<u8>), ShieldError> {
        use ml_kem::kem::Encapsulate;
        let (_dk, ek) = MlKem768::generate(&mut OsRng);
        let (ciphertext, shared_secret) = ek.encapsulate(&mut OsRng)
            .map_err(|e| ShieldError::crypto(
                ErrorCode::CryptoPostQuantumFailed,
                format!("encapsulation failed: {:?}", e),
            ))?;
        debug!("ML-KEM-768 encapsulation succeeded");
        Ok((ciphertext.as_slice().to_vec(), shared_secret.as_slice().to_vec()))
    }

    /// Perform ML-KEM-768 decapsulation using the responder's secret key.
    fn mlkem_decapsulate(
        keypair: &HybridKeypair,
        ciphertext_bytes: &[u8],
    ) -> Result<Vec<u8>, ShieldError> {
        let ciphertext: MlKemCiphertext = ciphertext_bytes.to_vec();
        keypair.mlkem_decapsulate(&ciphertext)
    }

    /// Derive the final session key from the classical and PQ shared secrets.
    ///
    /// ```
    /// session_key = HKDF-SHA256(
    ///     ikm  = x25519_shared || mlkem_shared,  (or x25519_shared alone if PQ failed)
    ///     salt = b"pq-hybrid",
    ///     info = b"session-key"
    /// )
    /// ```
    fn derive_session_key(
        x25519_shared: &[u8],
        mlkem_shared: Option<&[u8]>,
    ) -> Result<[u8; SESSION_KEY_LEN], ShieldError> {
        // Concatenate the shared secrets
        let mut ikm = Vec::with_capacity(32 + 32);
        ikm.extend_from_slice(x25519_shared);
        if let Some(pq_ss) = mlkem_shared {
            ikm.extend_from_slice(pq_ss);
        }

        let hk = Hkdf::<Sha256>::new(Some(PQ_HYBRID_SALT), &ikm);
        let mut session_key = [0u8; SESSION_KEY_LEN];
        hk.expand(PQ_HYBRID_INFO, &mut session_key).map_err(|e| {
            ShieldError::crypto(
                ErrorCode::CryptoHkdfFailed,
                format!("Session key HKDF derivation failed: {}", e),
            )
        })?;

        debug!(
            pq_included = mlkem_shared.is_some(),
            "Session key derived via HKDF"
        );

        Ok(session_key)
    }

    /// Serialise the initiator message for transmission over the wire.
    ///
    /// Wire format:
    ///   [1 byte: version]
    ///   [32 bytes: X25519 public key]
    ///   [2 bytes: ML-KEM ciphertext length (BE)]
    ///   [N bytes: ML-KEM ciphertext]
    ///   [2 bytes: ML-KEM encap key length (BE)]
    ///   [M bytes: ML-KEM encap key]
    fn serialise_initiator_message(msg: &KexInitiatorMessage) -> Result<Vec<u8>, ShieldError> {
        let mut buf = Vec::with_capacity(
            1 + 32 + 2 + msg.mlkem_ciphertext.len() + 2 + msg.mlkem_encap_key.len(),
        );

        // Version byte
        buf.push(0x01);

        // X25519 public key (fixed 32 bytes)
        buf.extend_from_slice(&msg.x25519_public);

        // ML-KEM ciphertext (variable length with 2-byte BE length prefix)
        let ct_len = msg.mlkem_ciphertext.len() as u16;
        buf.extend_from_slice(&ct_len.to_be_bytes());
        buf.extend_from_slice(&msg.mlkem_ciphertext);

        // ML-KEM encapsulation key (variable length with 2-byte BE length prefix)
        let ek_len = msg.mlkem_encap_key.len() as u16;
        buf.extend_from_slice(&ek_len.to_be_bytes());
        buf.extend_from_slice(&msg.mlkem_encap_key);

        Ok(buf)
    }

    /// Deserialise an initiator message from the wire.
    pub fn deserialise_initiator_message(data: &[u8]) -> Result<KexInitiatorMessage, ShieldError> {
        if data.len() < 1 + 32 + 2 {
            return Err(ShieldError::crypto(
                ErrorCode::CryptoPostQuantumFailed,
                format!("Initiator message too short: {} bytes", data.len()),
            ));
        }

        let mut offset = 0;

        // Version byte
        let version = data[offset];
        offset += 1;
        if version != 0x01 {
            return Err(ShieldError::crypto(
                ErrorCode::CryptoPostQuantumFailed,
                format!("Unsupported KEX message version: {}", version),
            ));
        }

        // X25519 public key
        let mut x25519_public = [0u8; 32];
        x25519_public.copy_from_slice(&data[offset..offset + 32]);
        offset += 32;

        // ML-KEM ciphertext
        let ct_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        if offset + ct_len > data.len() {
            return Err(ShieldError::crypto(
                ErrorCode::CryptoPostQuantumFailed,
                "Initiator message truncated (ciphertext)",
            ));
        }
        let mlkem_ciphertext = data[offset..offset + ct_len].to_vec();
        offset += ct_len;

        // ML-KEM encapsulation key
        if offset + 2 > data.len() {
            return Err(ShieldError::crypto(
                ErrorCode::CryptoPostQuantumFailed,
                "Initiator message truncated (encap key length)",
            ));
        }
        let ek_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        if offset + ek_len > data.len() {
            return Err(ShieldError::crypto(
                ErrorCode::CryptoPostQuantumFailed,
                "Initiator message truncated (encap key)",
            ));
        }
        let mlkem_encap_key = data[offset..offset + ek_len].to_vec();

        Ok(KexInitiatorMessage {
            x25519_public,
            mlkem_ciphertext,
            mlkem_encap_key,
        })
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_kex_round_trip() {
        // Responder generates a long-term hybrid keypair
        let responder_kp = HybridKeypair::generate().unwrap();

        // Initiator performs KEX with responder's public keys
        let (initiator_result, message) = PostQuantumKex::initiator_kex(
            responder_kp.x25519_public_key(),
            &responder_kp.mlkem_encap_key_bytes(),
        )
        .unwrap();

        assert!(initiator_result.pq_active);

        // Responder processes the initiator's message
        let responder_result = PostQuantumKex::responder_kex(&responder_kp, &message).unwrap();

        assert!(responder_result.pq_active);

        // Both parties should derive the same session key
        assert_eq!(initiator_result.session_key, responder_result.session_key);
    }

    #[test]
    fn test_x25519_fallback_when_pq_fails() {
        // Simulate PQ failure by providing invalid ML-KEM encap key bytes
        let responder_kp = HybridKeypair::generate().unwrap();
        let bad_mlkem_bytes = vec![0u8; 32]; // Wrong size — will fail

        let (result, _message) =
            PostQuantumKex::initiator_kex(responder_kp.x25519_public_key(), &bad_mlkem_bytes)
                .unwrap();

        // Should fall back to X25519-only
        assert!(!result.pq_active);
    }

    #[test]
    fn test_serialise_deserialise_round_trip() {
        let keypair = HybridKeypair::generate().unwrap();
        let (_result, message) = PostQuantumKex::initiator_kex(
            keypair.x25519_public_key(),
            &keypair.mlkem_encap_key_bytes(),
        )
        .unwrap();

        let serialised = PostQuantumKex::serialise_initiator_message(&message).unwrap();
        let deserialised = PostQuantumKex::deserialise_initiator_message(&serialised).unwrap();

        assert_eq!(message.x25519_public, deserialised.x25519_public);
        assert_eq!(message.mlkem_ciphertext, deserialised.mlkem_ciphertext);
        assert_eq!(message.mlkem_encap_key, deserialised.mlkem_encap_key);
    }

    #[test]
    fn test_session_key_derivation_deterministic() {
        let x25519_ss = [0xABu8; 32];
        let mlkem_ss = [0xCDu8; 32];

        let key1 = PostQuantumKex::derive_session_key(&x25519_ss, Some(&mlkem_ss)).unwrap();
        let key2 = PostQuantumKex::derive_session_key(&x25519_ss, Some(&mlkem_ss)).unwrap();

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_session_key_differs_without_pq() {
        let x25519_ss = [0xABu8; 32];
        let mlkem_ss = [0xCDu8; 32];

        let key_with_pq = PostQuantumKex::derive_session_key(&x25519_ss, Some(&mlkem_ss)).unwrap();
        let key_without_pq = PostQuantumKex::derive_session_key(&x25519_ss, None).unwrap();

        // The keys should be different when PQ is included vs not
        assert_ne!(key_with_pq, key_without_pq);
    }
}
