// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield vip-ultra-Quantum — Quantum Noise Injector
//
// Injects hardware-sourced entropy noise into VPN traffic timing and packet
// sizes to defeat statistical traffic analysis (STA) attacks by Iran's FAVA
// DPI system. Uses multiple entropy sources combined via BLAKE3 to produce
// unpredictable but reproducible noise schedules.
//
// Noise types:
//   • Timing jitter — random inter-packet delays (1–50ms)
//   • Size padding  — random payload padding to normalise packet sizes
//   • Burst shaping — random burst patterns to obscure flow characteristics
//   • Phase noise   — phase-shifts in traffic periodicity
// ─────────────────────────────────────────────────────────────────────────────

use std::sync::Arc;
use std::time::Duration;

use blake3::Hasher as Blake3Hasher;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::error::ShieldError;

// ── Noise Profile ────────────────────────────────────────────────────────────

/// Configuration profile for quantum noise injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseProfile {
    /// Minimum timing jitter in microseconds.
    pub min_jitter_us: u64,
    /// Maximum timing jitter in microseconds.
    pub max_jitter_us: u64,
    /// Minimum padding bytes added per packet.
    pub min_padding_bytes: usize,
    /// Maximum padding bytes added per packet.
    pub max_padding_bytes: usize,
    /// Whether burst shaping is enabled.
    pub burst_shaping: bool,
    /// Target packets-per-second rate for burst shaping.
    pub target_pps: f64,
    /// Whether phase noise is enabled.
    pub phase_noise: bool,
}

impl NoiseProfile {
    /// Conservative profile: minimal overhead, subtle noise.
    pub fn conservative() -> Self {
        Self {
            min_jitter_us: 500,
            max_jitter_us: 5_000,
            min_padding_bytes: 0,
            max_padding_bytes: 32,
            burst_shaping: false,
            target_pps: 1000.0,
            phase_noise: false,
        }
    }

    /// Aggressive profile: maximum DPI resistance, higher overhead.
    pub fn aggressive() -> Self {
        Self {
            min_jitter_us: 1_000,
            max_jitter_us: 50_000,
            min_padding_bytes: 16,
            max_padding_bytes: 256,
            burst_shaping: true,
            target_pps: 500.0,
            phase_noise: true,
        }
    }

    /// NAIN (National Intranet) profile: maximum stealth during shutdowns.
    pub fn nain_emergency() -> Self {
        Self {
            min_jitter_us: 5_000,
            max_jitter_us: 100_000,
            min_padding_bytes: 32,
            max_padding_bytes: 512,
            burst_shaping: true,
            target_pps: 100.0,
            phase_noise: true,
        }
    }
}

// ── Quantum Noise Injector ───────────────────────────────────────────────────

/// Injects quantum-sourced entropy noise into traffic to defeat STA attacks.
pub struct QuantumNoiseInjector {
    /// Current noise profile.
    profile: Arc<RwLock<NoiseProfile>>,
    /// Entropy counter (monotonic, used as nonce for BLAKE3).
    entropy_counter: Arc<RwLock<u64>>,
    /// Cached entropy pool (32 bytes refreshed every 1000 calls).
    entropy_pool: Arc<RwLock<[u8; 32]>>,
    /// Pool refresh counter.
    pool_counter: Arc<RwLock<u32>>,
}

impl QuantumNoiseInjector {
    /// Create a new noise injector with conservative profile.
    pub fn new() -> Self {
        let mut pool = [0u8; 32];
        OsRng.fill_bytes(&mut pool);

        Self {
            profile: Arc::new(RwLock::new(NoiseProfile::conservative())),
            entropy_counter: Arc::new(RwLock::new(0)),
            entropy_pool: Arc::new(RwLock::new(pool)),
            pool_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Set the active noise profile.
    pub async fn set_profile(&self, profile: NoiseProfile) {
        *self.profile.write().await = profile;
        info!("Quantum noise profile updated");
    }

    /// Activate NAIN emergency noise profile.
    pub async fn activate_nain_profile(&self) {
        *self.profile.write().await = NoiseProfile::nain_emergency();
        info!("⚡ NAIN emergency noise profile activated — maximum stealth mode");
    }

    /// Compute a jitter delay using quantum entropy.
    ///
    /// Returns a `Duration` to sleep before sending the next packet.
    pub async fn jitter_delay(&self) -> Duration {
        let profile = self.profile.read().await;
        let range = profile.max_jitter_us - profile.min_jitter_us;
        if range == 0 {
            return Duration::from_micros(profile.min_jitter_us);
        }

        let noise = self.next_entropy_u64().await;
        let jitter = profile.min_jitter_us + (noise % range);
        Duration::from_micros(jitter)
    }

    /// Generate random padding bytes for a packet.
    ///
    /// Returns a `Vec<u8>` of random padding to append to the packet.
    pub async fn generate_padding(&self) -> Vec<u8> {
        let profile = self.profile.read().await;
        if profile.max_padding_bytes == 0 {
            return Vec::new();
        }

        let range = (profile.max_padding_bytes - profile.min_padding_bytes) as u64;
        let noise = self.next_entropy_u64().await;
        let pad_len = profile.min_padding_bytes + (noise as usize % (range as usize + 1));

        let mut padding = vec![0u8; pad_len];
        // Fill with entropy-derived bytes (not just zeros — zeros are recognizable)
        let pool = self.entropy_pool.read().await;
        for (i, byte) in padding.iter_mut().enumerate() {
            *byte = pool[i % 32].wrapping_add(i as u8);
        }
        padding
    }

    /// Get the next 64-bit entropy value from the BLAKE3-derived pool.
    async fn next_entropy_u64(&self) -> u64 {
        let mut counter = self.entropy_counter.write().await;
        *counter = counter.wrapping_add(1);
        let c = *counter;
        drop(counter);

        // Refresh pool every 1000 calls
        let mut pool_ctr = self.pool_counter.write().await;
        *pool_ctr = pool_ctr.wrapping_add(1);
        if *pool_ctr % 1000 == 0 {
            drop(pool_ctr);
            let mut new_pool = [0u8; 32];
            OsRng.fill_bytes(&mut new_pool);
            *self.entropy_pool.write().await = new_pool;
        }

        let pool = self.entropy_pool.read().await;
        let mut h = Blake3Hasher::new();
        h.update(&pool[..]);
        h.update(&c.to_le_bytes());
        h.update(b"quantum-noise-v1");
        let hash = h.finalize();
        u64::from_le_bytes(hash.as_bytes()[..8].try_into().unwrap_or([0u8; 8]))
    }
}

impl Default for QuantumNoiseInjector {
    fn default() -> Self {
        Self::new()
    }
}
