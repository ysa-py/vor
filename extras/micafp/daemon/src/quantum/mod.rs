pub mod homomorphic_routing;
pub mod hybrid_handshake;
pub mod lattice_onion;
pub mod neural_steganography;
pub mod pqc_key_store;
pub mod qkd_simulation;
pub mod quantum_noise;
pub mod quantum_obfuscator;
pub mod quantum_ratchet;
pub mod quantum_seed_protocol;
pub mod zkp_auth;

pub use hybrid_handshake::HybridHandshake;
pub use lattice_onion::LatticeOnionRouter;
pub use neural_steganography::NeuralSteganographer;
pub use pqc_key_store::PqcKeyStore;
pub use quantum_noise::QuantumNoiseInjector;
pub use quantum_obfuscator::QuantumObfuscator;
pub use quantum_ratchet::QuantumRatchet;
pub use quantum_seed_protocol::QuantumSeedProtocolEngine;
pub type QuantumHybridHandshake = HybridHandshake;
pub type LatticeOnionEncoder = LatticeOnionRouter;
pub type NeuralSteganography = NeuralSteganographer;
pub type QuantumNoiseShaper = QuantumNoiseInjector;

/// Canonical alias for ZkpAuthenticator.
pub type ZkpAuth = zkp_auth::ZkpAuthenticator;
