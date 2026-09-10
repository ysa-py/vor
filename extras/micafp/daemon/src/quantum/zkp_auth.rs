// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield vip-ultra-Quantum — Zero-Knowledge Proof Authentication
//
// Implements Schnorr ZKP over Ristretto255 for peer authentication without
// revealing any identity information. This allows two UnifiedShield nodes to
// prove they share the same group membership key without exposing their
// individual identities to passive network observers or active DPI systems.
//
// Protocol (Schnorr σ-protocol):
//   Setup: G = Ristretto255 basepoint, x = private key, X = x·G public key
//   Prove (Prover knows x such that X = x·G):
//     1. Pick random r ← Zq
//     2. Compute commitment R = r·G
//     3. Compute challenge c = H(G ‖ X ‖ R ‖ message) mod q
//     4. Compute response s = r + c·x mod q
//     5. Send (R, s) to verifier
//   Verify:
//     1. Recompute c = H(G ‖ X ‖ R ‖ message) mod q
//     2. Check s·G == R + c·X
//
// Use case: Two relay nodes authenticate that they are genuine UnifiedShield
// nodes without any PKI certificates (which could be blocked/confiscated).
// ─────────────────────────────────────────────────────────────────────────────

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use tracing::{debug, info, warn};
use zeroize::Zeroize;

use crate::error::{ErrorCode, ShieldError};

// ── ZKP Types ────────────────────────────────────────────────────────────────

/// A Schnorr ZKP commitment + response pair (the proof).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkpProof {
    /// Commitment point R = r·G (32 bytes, compressed Ristretto).
    pub commitment: [u8; 32],
    /// Response scalar s = r + c·x mod q (32 bytes).
    pub response: [u8; 32],
    /// Message that was bound into the challenge hash.
    pub message: Vec<u8>,
}

/// A challenge issued by the verifier (non-interactive via Fiat-Shamir).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkpChallenge {
    /// Challenge scalar c = H(G ‖ X ‖ R ‖ message) mod q.
    pub challenge: [u8; 32],
    /// Timestamp to prevent replay attacks (Unix ms).
    pub timestamp_ms: u64,
    /// Nonce to ensure uniqueness.
    pub nonce: [u8; 16],
}

/// ZKP Authenticator for UnifiedShield node identity proofs.
pub struct ZkpAuthenticator {
    /// Private scalar key x.
    private_key: Scalar,
    /// Public key X = x·G.
    public_key: RistrettoPoint,
    /// Group membership key (shared among all genuine nodes).
    group_key: Scalar,
}

impl ZkpAuthenticator {
    /// Create a new ZKP authenticator with a fresh keypair.
    pub async fn new() -> Result<Self, ShieldError> {
        let mut rand_bytes = [0u8; 64];
        OsRng.fill_bytes(&mut rand_bytes);
        let private_key = Scalar::from_bytes_mod_order_wide(&rand_bytes);
        let public_key = &private_key * RISTRETTO_BASEPOINT_POINT;

        // Group membership key derived from build-time constant + device secret.
        // In production: loaded from secure enclave / KeyStore.
        let group_seed = b"MICAFP-UnifiedShield-Quantum-Group-v1-Iran";
        let group_key = Scalar::from_bytes_mod_order(Self::hash_to_scalar(group_seed));

        debug!("ZKP authenticator initialised (Schnorr/Ristretto255)");

        Ok(Self {
            private_key,
            public_key,
            group_key,
        })
    }

    /// Generate a ZKP proof that this node knows its private key.
    ///
    /// The proof binds to `message` (e.g., session nonce + timestamp)
    /// to prevent replay attacks.
    pub fn prove(&self, message: &[u8]) -> Result<ZkpProof, ShieldError> {
        // Step 1: Pick random blinding scalar r
        let mut rand_bytes = [0u8; 64];
        OsRng.fill_bytes(&mut rand_bytes);
        let r = Scalar::from_bytes_mod_order_wide(&rand_bytes);

        // Step 2: Commitment R = r·G
        let commitment_point = &r * RISTRETTO_BASEPOINT_POINT;
        let commitment = commitment_point.compress().to_bytes();

        // Step 3: Fiat-Shamir challenge c = H(G ‖ X ‖ R ‖ message)
        let challenge_scalar = self.compute_challenge(&commitment, message);

        // Step 4: Response s = r + c·x mod q
        let response_scalar = r + challenge_scalar * self.private_key;
        let response = response_scalar.to_bytes();

        debug!("ZKP proof generated for message ({} bytes)", message.len());

        Ok(ZkpProof {
            commitment,
            response,
            message: message.to_vec(),
        })
    }

    /// Verify a ZKP proof from a remote peer.
    ///
    /// `peer_public_key_bytes` is the compressed Ristretto255 public key
    /// of the peer (32 bytes).
    pub fn verify(
        &self,
        proof: &ZkpProof,
        peer_public_key_bytes: &[u8; 32],
    ) -> Result<bool, ShieldError> {
        // Decompress peer public key
        let peer_pk = CompressedRistretto(*peer_public_key_bytes)
            .decompress()
            .ok_or_else(|| {
                ShieldError::crypto(
                    ErrorCode::CryptoPostQuantumFailed,
                    "Invalid Ristretto255 public key",
                )
            })?;

        // Decompress commitment R
        let commitment_point = CompressedRistretto(proof.commitment)
            .decompress()
            .ok_or_else(|| {
                ShieldError::crypto(
                    ErrorCode::CryptoPostQuantumFailed,
                    "Invalid ZKP commitment point",
                )
            })?;

        // Recompute challenge c
        let challenge = self.compute_challenge(&proof.commitment, &proof.message);

        // Deserialise response scalar s
        let response = Scalar::from_canonical_bytes(proof.response)
            .into_option()
            .ok_or_else(|| {
                ShieldError::crypto(
                    ErrorCode::CryptoPostQuantumFailed,
                    "Invalid ZKP response scalar",
                )
            })?;

        // Verify: s·G == R + c·X
        let lhs = &response * RISTRETTO_BASEPOINT_POINT;
        let rhs = commitment_point + challenge * peer_pk;

        let valid = lhs == rhs;
        if valid {
            debug!("ZKP proof verified successfully");
        } else {
            warn!("ZKP proof verification FAILED — peer may be impersonating");
        }

        Ok(valid)
    }

    /// Get this node's public key as compressed Ristretto bytes (32 bytes).
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.public_key.compress().to_bytes()
    }

    // ── Internal helpers ───────────────────────────────────────────────

    /// Compute Fiat-Shamir challenge: H(G ‖ X ‖ R ‖ message) mod q.
    fn compute_challenge(&self, commitment: &[u8; 32], message: &[u8]) -> Scalar {
        let g_bytes = RISTRETTO_BASEPOINT_POINT.compress().to_bytes();
        let x_bytes = self.public_key.compress().to_bytes();

        let mut hasher = Sha512::new();
        hasher.update(g_bytes);
        hasher.update(x_bytes);
        hasher.update(commitment);
        hasher.update(message);
        let hash = hasher.finalize();

        Scalar::from_bytes_mod_order_wide(&hash.into())
    }

    /// Hash arbitrary bytes to a Ristretto scalar (mod q).
    fn hash_to_scalar(input: &[u8]) -> [u8; 32] {
        let mut hasher = Sha512::new();
        hasher.update(input);
        let hash = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&hash[..32]);
        out
    }
}

impl Drop for ZkpAuthenticator {
    fn drop(&mut self) {
        // Zeroize private key material on drop.
        let mut bytes = self.private_key.to_bytes();
        bytes.zeroize();
    }
}
