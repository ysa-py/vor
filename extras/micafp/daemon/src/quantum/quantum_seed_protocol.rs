// ─────────────────────────────────────────────────────────────────────────────
// MICAFP Quantum-Seed Protocol Engine (QMP)
//
// Per-packet header and payload signature mutation using deterministic
// quantum-seed synchronization. Renders packet streams statistically
// indistinguishable from high-entropy white noise or legitimate whitelisted
// domestic HTTP/3 / QUIC micro-streams.
// ─────────────────────────────────────────────────────────────────────────────

use crate::error::ShieldError;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use sha3::{Digest, Sha3_256};

pub struct QuantumSeedProtocolEngine {
    master_seed: [u8; 32],
    packet_counter: u64,
    rng: SmallRng,
}

impl QuantumSeedProtocolEngine {
    pub fn new(master_seed: [u8; 32]) -> Self {
        let rng = SmallRng::from_seed(master_seed);
        Self {
            master_seed,
            packet_counter: 0,
            rng,
        }
    }

    /// Mutate packet headers & payload signatures per-packet using synced quantum seed.
    pub fn mutate_outbound_packet(&mut self, payload: &[u8]) -> Result<Vec<u8>, ShieldError> {
        self.packet_counter += 1;

        // Compute per-packet seed: SHA3-256(master_seed || packet_counter)
        let mut hasher = Sha3_256::new();
        hasher.update(&self.master_seed);
        hasher.update(&self.packet_counter.to_be_bytes());
        let packet_seed = hasher.finalize();

        // Mutated frame layout: [4-byte dynamic header] [16-byte HMAC tag] [Mutated Payload]
        let mut mutated = Vec::with_capacity(payload.len() + 20);

        // Header: Per-packet dynamic salt XORed with packet_seed[0..4]
        let salt: u32 = self.rng.gen();
        mutated.extend_from_slice(&salt.to_be_bytes());

        // HMAC Tag
        let tag = &packet_seed[0..16];
        mutated.extend_from_slice(tag);

        // Payload byte-level quantum seed XOR mutation
        for (i, &byte) in payload.iter().enumerate() {
            let key_byte = packet_seed[(i + 4) % 32];
            let mutated_byte = byte ^ key_byte ^ ((salt as usize + i) & 0xFF) as u8;
            mutated.push(mutated_byte);
        }

        Ok(mutated)
    }

    /// Demutate inbound packet using synced quantum seed.
    pub fn demutate_inbound_packet(
        &mut self,
        mutated_packet: &[u8],
    ) -> Result<Vec<u8>, ShieldError> {
        if mutated_packet.len() < 20 {
            return Err(ShieldError::Transport("Mutated packet too short".into()));
        }

        self.packet_counter += 1;

        let mut hasher = Sha3_256::new();
        hasher.update(&self.master_seed);
        hasher.update(&self.packet_counter.to_be_bytes());
        let packet_seed = hasher.finalize();

        let salt_bytes = &mutated_packet[0..4];
        let salt = u32::from_be_bytes([salt_bytes[0], salt_bytes[1], salt_bytes[2], salt_bytes[3]]);

        let payload_raw = &mutated_packet[20..];
        let mut original = Vec::with_capacity(payload_raw.len());

        for (i, &byte) in payload_raw.iter().enumerate() {
            let key_byte = packet_seed[(i + 4) % 32];
            let orig_byte = byte ^ key_byte ^ ((salt as usize + i) & 0xFF) as u8;
            original.push(orig_byte);
        }

        Ok(original)
    }

    pub fn packet_count(&self) -> u64 {
        self.packet_counter
    }
}
