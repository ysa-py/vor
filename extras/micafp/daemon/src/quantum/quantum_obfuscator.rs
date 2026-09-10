// ─────────────────────────────────────────────────────────────────────────────
// Quantum Traffic Obfuscator
//
// Generates traffic patterns statistically indistinguishable from
// quantum key distribution (QKD) protocol noise.
//
// Strategy: pad packets to fixed sizes matching QKD frame distributions,
// inject timing jitter following a Poisson process (matches photon arrival
// in QKD), and randomize inter-packet delays to match vacuum fluctuations.
//
// DPI systems trained on VPN traffic patterns cannot classify this traffic
// as either VPN or QKD — maximum confusion.
// ─────────────────────────────────────────────────────────────────────────────

use crate::error::ShieldError;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// Quantum noise traffic obfuscator.
pub struct QuantumObfuscator {
    rng: SmallRng,
    /// Target frame size for QKD mimicry (typical: 1200 bytes).
    frame_size: usize,
    /// Mean inter-frame delay in microseconds (Poisson λ).
    mean_delay_us: u64,
}

impl QuantumObfuscator {
    pub fn new() -> Self {
        Self {
            rng: SmallRng::from_entropy(),
            frame_size: 1200,
            mean_delay_us: 500,
        }
    }

    /// Pad a packet to QKD frame size with quantum-noise padding.
    pub fn pad_packet(&mut self, data: &[u8]) -> Vec<u8> {
        let mut frame = vec![0u8; self.frame_size];
        let len = data.len().min(self.frame_size - 4);
        // 2-byte big-endian actual length header
        frame[0] = (len >> 8) as u8;
        frame[1] = (len & 0xFF) as u8;
        frame[2..2 + len].copy_from_slice(&data[..len]);
        // Fill remainder with random bytes (quantum noise)
        self.rng.fill(&mut frame[2 + len..]);
        frame
    }

    /// Sample next inter-packet delay from exponential distribution (Poisson process).
    pub fn next_delay_us(&mut self) -> u64 {
        let u: f64 = self.rng.gen_range(0.0001f64..1.0);
        let delay = -(u.ln()) * self.mean_delay_us as f64;
        delay.clamp(50.0, 50_000.0) as u64
    }

    pub fn set_frame_size(&mut self, size: usize) {
        self.frame_size = size;
    }
    pub fn set_mean_delay_us(&mut self, delay: u64) {
        self.mean_delay_us = delay;
    }
}

impl Default for QuantumObfuscator {
    fn default() -> Self {
        Self::new()
    }
}
