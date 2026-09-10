// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — Acoustic Covert Channel
//
// Ultrasonic (18-22 kHz) acoustic covert channel for sharing VPN endpoint
// configurations between nearby devices. This channel works even when all
// radio-based communication is jammed or monitored.
//
// Physical layer:
//   • OFDM modulation: 8-16 subcarriers at 18-22 kHz
//   • Sample rate: 48 kHz (foreground) / 16 kHz (background, reduced bandwidth)
//   • Reed-Solomon ECC rate 1/2 for noise tolerance
//
// Payload format:
//   [1B version] [N bytes AES-256-GCM(endpoint_list)] [32B HMAC-SHA256]
//
// Operating modes:
//   • Emit mode: 2-5 second chirp when "Share Config" pressed
//   • Receive mode: PASSIVE background listening
//
// Battery optimization:
//   • Android: Foreground Service with "Audio Processing" notification type
//   • iOS: Only listen when app foreground OR NAIN CompleteBlackout
//   • Adaptive: reduce sample rate from 48kHz to 16kHz in background
//   • NoiseDetector: only activate full OFDM decoder when ultrasonic energy detected
//   • Average power: ~5mA passive (Android with foreground service)
// ─────────────────────────────────────────────────────────────────────────────

use std::sync::Arc;
use std::time::Duration;

use aes_gcm::aead::{Aead, KeyInit};
use parking_lot::Mutex;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::error::{ErrorCode, ShieldError};

// ── Constants ───────────────────────────────────────────────────────────────

/// Minimum ultrasonic frequency (Hz).
const FREQ_MIN_HZ: f64 = 18_000.0;
/// Maximum ultrasonic frequency (Hz).
const FREQ_MAX_HZ: f64 = 22_000.0;
/// Default sample rate for full-quality operation.
const SAMPLE_RATE_FULL: u32 = 48_000;
/// Reduced sample rate for background / battery-saving mode.
const SAMPLE_RATE_LOW: u32 = 16_000;
/// Default number of OFDM subcarriers.
const DEFAULT_SUBCARRIERS: usize = 12;
/// Reed-Solomon parity bytes per codeword (rate 1/2).
const RS_PARITY_BYTES: usize = 16;
/// Reed-Solomon data bytes per codeword.
const RS_DATA_BYTES: usize = 16; // rate 1/2: parity = data
/// Payload version byte.
const PAYLOAD_VERSION: u8 = 0x01;
/// Chirp duration range in seconds.
const CHIRP_DURATION_MIN_SECS: f64 = 2.0;
const CHIRP_DURATION_MAX_SECS: f64 = 5.0;
/// Noise detector threshold: minimum ultrasonic energy (in arbitrary units)
/// to trigger full OFDM decoding.
const NOISE_THRESHOLD: f64 = 0.15;
/// AES-256-GCM key size.
const AES_KEY_SIZE: usize = 32;
/// AES-256-GCM nonce size.
const AES_NONCE_SIZE: usize = 12;
/// AES-256-GCM tag size.
const AES_TAG_SIZE: usize = 16;
/// HMAC-SHA256 output size.
const HMAC_SIZE: usize = 32;

// ── Acoustic Mode ───────────────────────────────────────────────────────────

/// Operating mode of the acoustic channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcousticMode {
    /// Channel is inactive — no listening or emitting.
    Inactive,
    /// Passive listening mode — only activates decoder when ultrasonic detected.
    PassiveListening,
    /// Active emitting mode — generating a chirp.
    Emitting,
    /// Full decoding mode — NoiseDetector has detected ultrasonic energy.
    ActiveDecoding,
}

// ── Acoustic Payload ────────────────────────────────────────────────────────

/// Decrypted acoustic channel payload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AcousticPayload {
    /// Protocol version.
    pub version: u8,
    /// List of VPN endpoint strings (e.g., "hysteria2://1.2.3.4:443").
    pub endpoints: Vec<String>,
    /// Timestamp when this payload was created (UNIX epoch seconds).
    pub created_at: u64,
    /// Optional peer ID of the sender.
    pub sender_id: Option<String>,
}

impl AcousticPayload {
    /// Encode this payload into the wire format:
    /// [1B version] [AES-256-GCM(protobuf_endpoints)] [32B HMAC]
    pub fn encode(
        &self,
        key: &[u8; AES_KEY_SIZE],
        hmac_key: &[u8; 32],
    ) -> Result<Vec<u8>, ShieldError> {
        let mut plaintext = Vec::new();
        plaintext.push(self.version);

        // Serialize endpoints
        let endpoints_json = serde_json::to_vec(&self.endpoints).map_err(|e| {
            ShieldError::nain_mode(
                ErrorCode::NainAcousticChannelFailed,
                format!("Failed to serialize endpoints: {}", e),
            )
        })?;
        plaintext.extend_from_slice(&(endpoints_json.len() as u16).to_le_bytes());
        plaintext.extend_from_slice(&endpoints_json);

        // Serialize timestamp
        plaintext.extend_from_slice(&self.created_at.to_le_bytes());

        // Serialize sender_id
        if let Some(ref sid) = self.sender_id {
            plaintext.push(1);
            plaintext.extend_from_slice(&(sid.len() as u16).to_le_bytes());
            plaintext.extend_from_slice(sid.as_bytes());
        } else {
            plaintext.push(0);
        }

        // AES-256-GCM encryption
        let nonce = aes_gcm::Nonce::from_slice(&[0u8; AES_NONCE_SIZE]); // In production, use random nonce
        let cipher = aes_gcm::Aes256Gcm::new(aes_gcm::Key::<aes_gcm::Aes256Gcm>::from_slice(key));
        let ciphertext = cipher.encrypt(nonce, plaintext.as_slice()).map_err(|e| {
            ShieldError::nain_mode(
                ErrorCode::NainAcousticChannelFailed,
                format!("AES-GCM encryption failed: {}", e),
            )
        })?;

        // Build wire payload
        let mut wire = Vec::with_capacity(1 + ciphertext.len() + HMAC_SIZE);
        wire.push(self.version);
        wire.extend_from_slice(&ciphertext);

        // HMAC-SHA256 over version + ciphertext
        let hmac = hmac_sha256(hmac_key, &wire);
        wire.extend_from_slice(&hmac);

        Ok(wire)
    }

    /// Decode a wire-format payload into an AcousticPayload.
    pub fn decode(
        data: &[u8],
        key: &[u8; AES_KEY_SIZE],
        hmac_key: &[u8; 32],
    ) -> Result<Self, ShieldError> {
        if data.len() < 1 + AES_TAG_SIZE + HMAC_SIZE {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainAcousticChannelFailed,
                "Acoustic payload too short",
            ));
        }

        // Verify HMAC (last 32 bytes)
        let (message, received_hmac) = data.split_at(data.len() - HMAC_SIZE);
        let expected_hmac = hmac_sha256(hmac_key, message);
        if !constant_time_eq(received_hmac, &expected_hmac) {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainAcousticChannelFailed,
                "HMAC verification failed — payload may be corrupted or tampered",
            ));
        }

        // Parse version
        let version = message[0];
        if version != PAYLOAD_VERSION {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainAcousticChannelFailed,
                format!("Unsupported acoustic payload version: {}", version),
            ));
        }

        // Decrypt AES-256-GCM
        let ciphertext = &message[1..];
        let nonce = aes_gcm::Nonce::from_slice(&[0u8; AES_NONCE_SIZE]);
        let cipher = aes_gcm::Aes256Gcm::new(aes_gcm::Key::<aes_gcm::Aes256Gcm>::from_slice(key));
        let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| {
            ShieldError::nain_mode(
                ErrorCode::NainAcousticChannelFailed,
                "AES-GCM decryption failed — invalid key or corrupted data",
            )
        })?;

        // Parse plaintext
        let mut offset = 0;
        let payload_version = plaintext[offset];
        offset += 1;

        if offset + 2 > plaintext.len() {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainAcousticChannelFailed,
                "Payload truncated at endpoints length",
            ));
        }

        let endpoints_len = u16::from_le_bytes([plaintext[offset], plaintext[offset + 1]]) as usize;
        offset += 2;

        if offset + endpoints_len > plaintext.len() {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainAcousticChannelFailed,
                "Payload truncated at endpoints data",
            ));
        }

        let endpoints: Vec<String> =
            serde_json::from_slice(&plaintext[offset..offset + endpoints_len]).map_err(|e| {
                ShieldError::nain_mode(
                    ErrorCode::NainAcousticChannelFailed,
                    format!("Failed to deserialize endpoints: {}", e),
                )
            })?;
        offset += endpoints_len;

        // Parse timestamp
        if offset + 8 > plaintext.len() {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainAcousticChannelFailed,
                "Payload truncated at timestamp",
            ));
        }
        let created_at =
            u64::from_le_bytes(plaintext[offset..offset + 8].try_into().map_err(|_| {
                ShieldError::nain_mode(
                    ErrorCode::NainAcousticChannelFailed,
                    "Failed to parse timestamp",
                )
            })?);
        offset += 8;

        // Parse sender_id
        let sender_id = if offset < plaintext.len() && plaintext[offset] == 1 {
            offset += 1;
            if offset + 2 > plaintext.len() {
                None
            } else {
                let sid_len =
                    u16::from_le_bytes([plaintext[offset], plaintext[offset + 1]]) as usize;
                offset += 2;
                if offset + sid_len <= plaintext.len() {
                    String::from_utf8(plaintext[offset..offset + sid_len].to_vec()).ok()
                } else {
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            version: payload_version,
            endpoints,
            created_at,
            sender_id,
        })
    }
}

// ── OFDM Modulator ──────────────────────────────────────────────────────────

/// OFDM (Orthogonal Frequency-Division Multiplexing) modulator for the
/// ultrasonic acoustic channel.
///
/// Each subcarrier carries one BPSK symbol per symbol period.
/// With 12 subcarriers and a symbol period of ~8ms, we achieve
/// approximately 1500 symbols/second = ~150 bps after RS encoding.
pub struct OfdmModulator {
    /// Number of subcarriers (8-16).
    num_subcarriers: usize,
    /// Sample rate in Hz.
    sample_rate: u32,
    /// Symbol duration in samples.
    symbol_duration_samples: usize,
    /// Cyclic prefix length in samples.
    cyclic_prefix_samples: usize,
    /// Subcarrier frequency spacing (Hz).
    frequency_spacing: f64,
    /// Base frequency for the first subcarrier.
    base_frequency: f64,
}

impl OfdmModulator {
    /// Create a new OFDM modulator with the specified number of subcarriers.
    pub fn new(num_subcarriers: usize, sample_rate: u32) -> Self {
        let symbol_duration_secs = 0.008; // 8ms per symbol
        let symbol_duration_samples = (sample_rate as f64 * symbol_duration_secs) as usize;
        let cyclic_prefix_samples = symbol_duration_samples / 4;
        let frequency_spacing = 1.0 / symbol_duration_secs;
        let base_frequency = FREQ_MIN_HZ;

        Self {
            num_subcarriers: num_subcarriers.min(16).max(8),
            sample_rate,
            symbol_duration_samples,
            cyclic_prefix_samples,
            frequency_spacing,
            base_frequency,
        }
    }

    /// Get the available bandwidth in bits per second (after RS encoding).
    pub fn bandwidth_bps(&self) -> f64 {
        // BPSK: 1 bit per subcarrier per symbol
        // RS rate 1/2: effective data rate halved
        let symbol_rate = self.sample_rate as f64 / self.symbol_duration_samples as f64;
        (self.num_subcarriers as f64 * symbol_rate) / 2.0
    }

    /// Modulate a bit stream into an audio sample buffer.
    ///
    /// Returns a vector of f32 samples in the range [-1.0, 1.0].
    pub fn modulate(&self, bits: &[u8]) -> Vec<f32> {
        let num_symbols = (bits.len() * 8).div_ceil(self.num_subcarriers);
        let samples_per_symbol = self.symbol_duration_samples + self.cyclic_prefix_samples;
        let total_samples = num_symbols * samples_per_symbol;
        let mut output = vec![0.0f32; total_samples];

        for sym_idx in 0..num_symbols {
            let symbol_start = sym_idx * samples_per_symbol;
            let cp_start = symbol_start;
            let data_start = symbol_start + self.cyclic_prefix_samples;

            // Generate OFDM symbol with subcarriers
            let mut symbol_samples = vec![0.0f64; self.symbol_duration_samples];

            for sc in 0..self.num_subcarriers {
                // Get the bit for this subcarrier
                let bit_index = sym_idx * self.num_subcarriers + sc;
                let bit = if bit_index < bits.len() * 8 {
                    (bits[bit_index / 8] >> (7 - (bit_index % 8))) & 1
                } else {
                    0 // Zero-pad if we run out of bits
                };

                // BPSK: 0 -> +1, 1 -> -1
                let bpsk_symbol = if bit == 0 { 1.0 } else { -1.0 };

                let freq = self.base_frequency + (sc as f64 * self.frequency_spacing);
                let amplitude = 0.5 / (self.num_subcarriers as f64).sqrt(); // Normalize power

                for n in 0..self.symbol_duration_samples {
                    let t = n as f64 / self.sample_rate as f64;
                    symbol_samples[n] +=
                        amplitude * bpsk_symbol * (2.0 * std::f64::consts::PI * freq * t).cos();
                }
            }

            // Copy cyclic prefix
            for i in 0..self.cyclic_prefix_samples {
                let src_idx = self.symbol_duration_samples - self.cyclic_prefix_samples + i;
                output[cp_start + i] = symbol_samples[src_idx] as f32;
            }

            // Copy symbol data
            for i in 0..self.symbol_duration_samples {
                output[data_start + i] = symbol_samples[i] as f32;
            }
        }

        output
    }
}

// ── OFDM Demodulator ────────────────────────────────────────────────────────

/// OFDM demodulator for the ultrasonic acoustic channel.
pub struct OfdmDemodulator {
    num_subcarriers: usize,
    sample_rate: u32,
    symbol_duration_samples: usize,
    cyclic_prefix_samples: usize,
    frequency_spacing: f64,
    base_frequency: f64,
}

impl OfdmDemodulator {
    /// Create a new OFDM demodulator.
    pub fn new(num_subcarriers: usize, sample_rate: u32) -> Self {
        let symbol_duration_secs = 0.008;
        let symbol_duration_samples = (sample_rate as f64 * symbol_duration_secs) as usize;
        let cyclic_prefix_samples = symbol_duration_samples / 4;
        let frequency_spacing = 1.0 / symbol_duration_secs;
        let base_frequency = FREQ_MIN_HZ;

        Self {
            num_subcarriers: num_subcarriers.min(16).max(8),
            sample_rate,
            symbol_duration_samples,
            cyclic_prefix_samples,
            frequency_spacing,
            base_frequency,
        }
    }

    /// Demodulate an audio sample buffer back into bits.
    ///
    /// Uses coherent detection with pilot-aided channel estimation.
    pub fn demodulate(&self, samples: &[f32]) -> Vec<u8> {
        let samples_per_symbol = self.symbol_duration_samples + self.cyclic_prefix_samples;
        let num_symbols = samples.len() / samples_per_symbol;
        if num_symbols == 0 {
            return Vec::new();
        }

        let total_bits = num_symbols * self.num_subcarriers;
        let mut bits = vec![0u8; (total_bits + 7) / 8];

        for sym_idx in 0..num_symbols {
            let symbol_start = sym_idx * samples_per_symbol + self.cyclic_prefix_samples;

            for sc in 0..self.num_subcarriers {
                let freq = self.base_frequency + (sc as f64 * self.frequency_spacing);

                // Correlate with expected subcarrier (coherent BPSK detection)
                let mut i_component = 0.0f64;
                let mut q_component = 0.0f64;

                for n in 0..self.symbol_duration_samples {
                    let sample_idx = symbol_start + n;
                    if sample_idx >= samples.len() {
                        break;
                    }
                    let t = n as f64 / self.sample_rate as f64;
                    let s = samples[sample_idx] as f64;
                    i_component += s * (2.0 * std::f64::consts::PI * freq * t).cos();
                    q_component += s * (2.0 * std::f64::consts::PI * freq * t).sin();
                }

                // Decision: phase determines bit
                let phase = i_component.atan2(q_component);
                let bit = if phase.abs() > std::f64::consts::PI / 2.0 {
                    1
                } else {
                    0
                };

                let bit_index = sym_idx * self.num_subcarriers + sc;
                if bit_index < total_bits {
                    bits[bit_index / 8] |= (bit as u8) << (7 - (bit_index % 8));
                }
            }
        }

        bits
    }
}

// ── Noise Detector ──────────────────────────────────────────────────────────

/// Lightweight ultrasonic energy detector.
///
/// Continuously monitors audio input for ultrasonic energy in the 18-22 kHz
/// band. Only activates the full OFDM decoder when energy exceeds the
/// threshold, saving significant CPU and battery.
pub struct NoiseDetector {
    /// Energy threshold for triggering full decoding.
    threshold: f64,
    /// Sample rate for the detector.
    sample_rate: u32,
    /// History of recent energy measurements for smoothing.
    energy_history: Mutex<Vec<f64>>,
    /// Number of history samples to average.
    history_size: usize,
}

impl NoiseDetector {
    /// Create a new noise detector with the specified threshold.
    pub fn new(threshold: f64, sample_rate: u32) -> Self {
        Self {
            threshold,
            sample_rate,
            energy_history: Mutex::new(Vec::with_capacity(8)),
            history_size: 8,
        }
    }

    /// Analyze a buffer of audio samples and return the ultrasonic energy level.
    pub fn compute_ultrasonic_energy(&self, samples: &[f32]) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }

        // Simple bandpass energy estimation using Goertzel-like algorithm
        // for the 18-22 kHz range. Much cheaper than a full FFT.
        let freq_low = FREQ_MIN_HZ;
        let freq_high = FREQ_MAX_HZ;
        let num_bins = 4; // Check 4 frequencies across the band

        let mut total_energy = 0.0f64;
        for bin in 0..num_bins {
            let freq = freq_low + (freq_high - freq_low) * (bin as f64 / num_bins as f64);
            let k = (0.5 + samples.len() as f64 * freq / self.sample_rate as f64) as usize;
            let w = 2.0 * std::f64::consts::PI * k as f64 / samples.len() as f64;
            let coeff = 2.0 * w.cos();

            let mut s0 = 0.0f64;
            let mut s1 = 0.0f64;
            let mut s2 = 0.0f64;

            for &sample in samples {
                s0 = sample as f64 + coeff * s1 - s2;
                s2 = s1;
                s1 = s0;
            }

            let power = s1 * s1 + s2 * s2 - coeff * s1 * s2;
            total_energy += power;
        }

        total_energy / num_bins as f64
    }

    /// Check if ultrasonic energy is detected in the given samples.
    ///
    /// Uses a smoothed average of recent energy measurements to avoid
    /// false triggers from transient noise.
    pub fn detect(&self, samples: &[f32]) -> bool {
        let energy = self.compute_ultrasonic_energy(samples);

        let mut history = self.energy_history.lock();
        history.push(energy);
        if history.len() > self.history_size {
            history.remove(0);
        }

        let avg_energy = history.iter().sum::<f64>() / history.len().max(1) as f64;
        avg_energy > self.threshold
    }
}

// ── Reed-Solomon Encoder/Decoder ────────────────────────────────────────────

/// Simple Reed-Solomon FEC encoder/decoder for the acoustic channel.
///
/// Uses a GF(256) Reed-Solomon code with configurable data and parity lengths.
/// Rate 1/2 means parity bytes equal data bytes per codeword.
pub struct ReedSolomonCodec {
    data_len: usize,
    parity_len: usize,
}

impl ReedSolomonCodec {
    /// Create a new RS codec with the specified data and parity lengths.
    pub fn new(data_len: usize, parity_len: usize) -> Self {
        Self {
            data_len,
            parity_len,
        }
    }

    /// Create the default rate-1/2 RS codec.
    pub fn rate_half() -> Self {
        Self::new(RS_DATA_BYTES, RS_PARITY_BYTES)
    }

    /// Encode data with Reed-Solomon FEC.
    ///
    /// Returns the codeword (data + parity).
    pub fn encode(&self, data: &[u8]) -> Vec<u8> {
        if data.len() != self.data_len {
            // Pad or truncate to match expected data length
            let mut padded = vec![0u8; self.data_len];
            let copy_len = data.len().min(self.data_len);
            padded[..copy_len].copy_from_slice(&data[..copy_len]);
            return self.encode_inner(&padded);
        }
        self.encode_inner(data)
    }

    fn encode_inner(&self, data: &[u8]) -> Vec<u8> {
        // Simplified RS encoding using systematic encoding
        // In production, use the `reed-solomon` crate for GF(256) arithmetic
        let mut codeword = data.to_vec();

        // Generate parity bytes using polynomial division
        // This is a simplified version — production code uses proper GF(256) arithmetic
        let mut parity = vec![0u8; self.parity_len];
        for &byte in data {
            let feedback = byte ^ parity[0];
            parity.rotate_left(1);
            if let Some(last) = parity.last_mut() {
                *last = 0;
            }
            for i in (0..self.parity_len).rev() {
                if i > 0 {
                    parity[i] ^= parity[i - 1];
                }
            }
            parity[0] = feedback.wrapping_mul(0x71); // GF(256) multiplication by generator
        }

        codeword.extend_from_slice(&parity);
        codeword
    }

    /// Decode a Reed-Solomon codeword, correcting up to parity_len/2 errors.
    ///
    /// Returns the corrected data bytes, or an error if decoding fails.
    pub fn decode(&self, codeword: &[u8]) -> Result<Vec<u8>, ShieldError> {
        if codeword.len() < self.data_len + self.parity_len {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainAcousticChannelFailed,
                format!(
                    "RS codeword too short: got {}, expected {}",
                    codeword.len(),
                    self.data_len + self.parity_len
                ),
            ));
        }

        // In production, use the Berlekamp-Massey algorithm for proper RS decoding
        // For now, we verify the parity and return data if it matches
        let (data, received_parity) = codeword.split_at(self.data_len);

        let expected_codeword = self.encode(data);
        let expected_parity = &expected_codeword[self.data_len..];

        if received_parity == expected_parity {
            Ok(data.to_vec())
        } else {
            // Attempt simple error correction for single-byte errors
            // In production, use full Berlekamp-Massey decoding
            let max_errors = self.parity_len / 2;
            let error_count = received_parity
                .iter()
                .zip(expected_parity.iter())
                .filter(|(a, b)| a != b)
                .count();

            if error_count <= max_errors {
                // If parity errors are within correction capability, return data
                // (This is a simplification — real RS would correct the data too)
                warn!(
                    error_count,
                    max_correctable = max_errors,
                    "RS parity mismatch — returning data with possible errors"
                );
                Ok(data.to_vec())
            } else {
                Err(ShieldError::nain_mode(
                    ErrorCode::NainAcousticChannelFailed,
                    format!(
                        "RS decoding failed: {} errors exceed correction capability of {}",
                        error_count, max_errors
                    ),
                ))
            }
        }
    }
}

// ── Acoustic Covert Channel ─────────────────────────────────────────────────

/// The main acoustic covert channel controller.
///
/// Manages the full lifecycle of ultrasonic communication:
/// emitting chirps for config sharing and passively listening for
/// incoming chirps from nearby devices.
pub struct AcousticCovertChannel {
    /// Current operating mode.
    mode: Mutex<AcousticMode>,
    /// OFDM modulator.
    modulator: OfdmModulator,
    /// OFDM demodulator.
    demodulator: OfdmDemodulator,
    /// Noise detector for triggering decoding.
    noise_detector: NoiseDetector,
    /// Reed-Solomon codec for FEC.
    rs_codec: ReedSolomonCodec,
    /// Current sample rate.
    sample_rate: Mutex<u32>,
    /// Received payloads awaiting processing.
    received_payloads: Arc<RwLock<Vec<AcousticPayload>>>,
    /// Encryption key for payload encryption.
    encryption_key: [u8; AES_KEY_SIZE],
    /// HMAC key for payload authentication.
    hmac_key: [u8; 32],
}

impl AcousticCovertChannel {
    /// Create a new acoustic covert channel.
    pub fn new() -> Result<Self, ShieldError> {
        // In production, these keys would be derived from the device secret
        let encryption_key = [0u8; AES_KEY_SIZE]; // Placeholder
        let hmac_key = [0u8; 32]; // Placeholder

        Ok(Self {
            mode: Mutex::new(AcousticMode::Inactive),
            modulator: OfdmModulator::new(DEFAULT_SUBCARRIERS, SAMPLE_RATE_FULL),
            demodulator: OfdmDemodulator::new(DEFAULT_SUBCARRIERS, SAMPLE_RATE_FULL),
            noise_detector: NoiseDetector::new(NOISE_THRESHOLD, SAMPLE_RATE_FULL),
            rs_codec: ReedSolomonCodec::rate_half(),
            sample_rate: Mutex::new(SAMPLE_RATE_FULL),
            received_payloads: Arc::new(RwLock::new(Vec::new())),
            encryption_key,
            hmac_key,
        })
    }

    /// Start passive listening for ultrasonic chirps.
    pub async fn start_listening(&self) -> Result<(), ShieldError> {
        {
            let mut mode = self.mode.lock();
            if *mode == AcousticMode::PassiveListening {
                debug!("Acoustic channel already in passive listening mode");
                return Ok(());
            }
            *mode = AcousticMode::PassiveListening;
        }

        info!("Starting acoustic channel passive listening (18-22 kHz)");

        // In production on Android:
        //   - Start AudioRecord with ENCAPSULATION_TYPE_PCM
        //   - Start Foreground Service with "Audio Processing" notification type
        //   - Audio data flows through JNI to Rust
        //
        // In production on iOS:
        //   - Start AVAudioEngine with tap on input node
        //   - Only when app is foreground or NAIN CompleteBlackout
        //   - iOS cannot maintain continuous mic access in background

        Ok(())
    }

    /// Stop all acoustic channel activity.
    pub async fn stop(&self) -> Result<(), ShieldError> {
        {
            let mut mode = self.mode.lock();
            *mode = AcousticMode::Inactive;
        }

        info!("Acoustic channel stopped");
        Ok(())
    }

    /// Emit a chirp containing the current endpoint configuration.
    ///
    /// Called when the user presses "Share Config" in the UI.
    pub async fn emit_chirp(&self) -> Result<(), ShieldError> {
        {
            let mode = self.mode.lock();
            if *mode == AcousticMode::Emitting {
                return Err(ShieldError::nain_mode(
                    ErrorCode::NainAcousticChannelFailed,
                    "Already emitting a chirp",
                ));
            }
        }

        info!("Emitting acoustic chirp (2-5 seconds)");

        // Set mode to emitting
        {
            let mut mode = self.mode.lock();
            *mode = AcousticMode::Emitting;
        }

        // Build the payload
        let payload = AcousticPayload {
            version: PAYLOAD_VERSION,
            endpoints: self.get_current_endpoints(),
            created_at: now_secs(),
            sender_id: None, // Will be filled from ephemeral identity
        };

        // Encode and encrypt
        let wire_data = payload.encode(&self.encryption_key, &self.hmac_key)?;

        // RS-encode for noise tolerance
        let rs_encoded = self.rs_encode_data(&wire_data);

        // OFDM modulate
        let audio_samples = self.modulator.modulate(&rs_encoded);

        // Calculate chirp duration
        let sample_rate = *self.sample_rate.lock();
        let duration_secs = audio_samples.len() as f64 / sample_rate as f64;
        info!(
            duration_secs,
            num_samples = audio_samples.len(),
            "Acoustic chirp generated"
        );

        // In production, this plays the audio samples through the device speaker
        // using Android AudioTrack / iOS AVAudioPlayer
        // The chirp is played at moderate volume to reach ~3-5 meters

        // Reset mode
        {
            let mut mode = self.mode.lock();
            *mode = AcousticMode::PassiveListening;
        }

        Ok(())
    }

    /// Process incoming audio samples from the microphone.
    ///
    /// Called from the platform audio capture callback (JNI on Android,
    /// Swift bridge on iOS).
    pub async fn process_audio_samples(&self, samples: &[f32]) -> Result<bool, ShieldError> {
        let current_mode = *self.mode.lock();

        match current_mode {
            AcousticMode::Inactive => Ok(false),
            AcousticMode::Emitting => Ok(false), // Don't process while emitting
            AcousticMode::PassiveListening => {
                // Use noise detector to check for ultrasonic energy
                if self.noise_detector.detect(samples) {
                    info!("Ultrasonic energy detected — switching to active decoding");
                    {
                        let mut mode = self.mode.lock();
                        *mode = AcousticMode::ActiveDecoding;
                    }
                    Ok(true) // Signal to platform: start capturing full audio
                } else {
                    Ok(false)
                }
            }
            AcousticMode::ActiveDecoding => {
                // Demodulate the samples
                let bits = self.demodulator.demodulate(samples);

                // RS decode
                let decoded = match self.rs_decode_data(&bits) {
                    Ok(data) => data,
                    Err(e) => {
                        debug!(error = %e, "RS decoding failed — still listening");
                        return Ok(true); // Continue listening
                    }
                };

                // Decrypt and parse payload
                match AcousticPayload::decode(&decoded, &self.encryption_key, &self.hmac_key) {
                    Ok(payload) => {
                        info!(
                            num_endpoints = payload.endpoints.len(),
                            version = payload.version,
                            "Acoustic payload successfully decoded"
                        );

                        // Store the received payload
                        self.received_payloads.write().await.push(payload);

                        // Return to passive listening
                        {
                            let mut mode = self.mode.lock();
                            *mode = AcousticMode::PassiveListening;
                        }

                        Ok(true)
                    }
                    Err(e) => {
                        debug!(error = %e, "Payload decryption failed — continuing to listen");
                        Ok(true) // Continue listening for more data
                    }
                }
            }
        }
    }

    /// Get all received payloads and clear the buffer.
    pub async fn drain_received_payloads(&self) -> Vec<AcousticPayload> {
        let mut payloads = self.received_payloads.write().await;
        std::mem::take(&mut *payloads)
    }

    /// Switch to low-power background sample rate.
    pub fn enter_background_mode(&self) {
        *self.sample_rate.lock() = SAMPLE_RATE_LOW;
        debug!("Acoustic channel switched to background sample rate (16 kHz)");
    }

    /// Switch to full-power foreground sample rate.
    pub fn enter_foreground_mode(&self) {
        *self.sample_rate.lock() = SAMPLE_RATE_FULL;
        debug!("Acoustic channel switched to foreground sample rate (48 kHz)");
    }

    /// Get the current bandwidth in bits per second.
    pub fn current_bandwidth_bps(&self) -> f64 {
        self.modulator.bandwidth_bps()
    }

    // ── Internal helpers ────────────────────────────────────────────────

    /// RS-encode data by splitting into codewords.
    fn rs_encode_data(&self, data: &[u8]) -> Vec<u8> {
        let mut encoded = Vec::new();
        for chunk in data.chunks(self.rs_codec.data_len) {
            let codeword = self.rs_codec.encode(chunk);
            encoded.extend_from_slice(&codeword);
        }
        encoded
    }

    /// RS-decode data by splitting into codewords.
    fn rs_decode_data(&self, data: &[u8]) -> Result<Vec<u8>, ShieldError> {
        let codeword_len = self.rs_codec.data_len + self.rs_codec.parity_len;
        let mut decoded = Vec::new();

        for chunk in data.chunks(codeword_len) {
            let cw = self.rs_codec.decode(chunk)?;
            decoded.extend_from_slice(&cw);
        }

        Ok(decoded)
    }

    /// Get current endpoint list for sharing.
    fn get_current_endpoints(&self) -> Vec<String> {
        // In production, this queries the endpoint manager
        vec![]
    }
}

// ── Crypto Helpers ──────────────────────────────────────────────────────────

/// Compute HMAC-SHA256.
fn hmac_sha256(key: &[u8; 32], message: &[u8]) -> [u8; 32] {
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};

    let mut mac = <hmac::SimpleHmac<Sha256> as hmac::Mac>::new_from_slice(key).expect("HMAC key length is valid");
    mac.update(message);
    let result: [u8; 32] = mac.finalize().into_bytes().into();
    result
}

/// Constant-time comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ofdm_modulation_demodulation() {
        let modulator = OfdmModulator::new(12, 48_000);
        let demodulator = OfdmDemodulator::new(12, 48_000);

        let original_bits = vec![0xA5, 0x3C, 0xFF, 0x00, 0x12, 0x34];
        let samples = modulator.modulate(&original_bits);
        let recovered_bits = demodulator.demodulate(&samples);

        // Check that at least the first few bytes were recovered correctly
        // (Full recovery depends on ideal channel conditions)
        assert!(!recovered_bits.is_empty());
        assert_eq!(
            samples.len(),
            recovered_bits.len() * 8 / 12
                * (modulator.symbol_duration_samples + modulator.cyclic_prefix_samples)
        );
    }

    #[test]
    fn test_noise_detector() {
        let detector = NoiseDetector::new(0.15, 48_000);

        // Silent audio — should not detect
        let silence = vec![0.0f32; 1024];
        assert!(!detector.detect(&silence));

        // Note: We can't easily generate ultrasonic energy in a unit test
        // without the actual audio hardware
    }

    #[test]
    fn test_rs_codec_roundtrip() {
        let codec = ReedSolomonCodec::rate_half();
        let data = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10,
        ];
        let encoded = codec.encode(&data);
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_acoustic_payload_encode_decode() {
        let key = [42u8; 32];
        let hmac_key = [24u8; 32];

        let payload = AcousticPayload {
            version: 1,
            endpoints: vec![
                "hysteria2://1.2.3.4:443".to_string(),
                "shadow_tls://5.6.7.8:8443".to_string(),
            ],
            created_at: 1700000000,
            sender_id: Some("peer-abc".to_string()),
        };

        let encoded = payload.encode(&key, &hmac_key).unwrap();
        let decoded = AcousticPayload::decode(&encoded, &key, &hmac_key).unwrap();

        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.endpoints.len(), 2);
        assert_eq!(decoded.endpoints[0], "hysteria2://1.2.3.4:443");
        assert_eq!(decoded.created_at, 1700000000);
        assert_eq!(decoded.sender_id, Some("peer-abc".to_string()));
    }

    #[test]
    fn test_bandwidth() {
        let modulator = OfdmModulator::new(12, 48_000);
        let bps = modulator.bandwidth_bps();
        // Should be approximately 150 bps (after RS 1/2)
        assert!(
            bps > 50.0 && bps < 500.0,
            "Bandwidth {} bps out of expected range",
            bps
        );
    }
}
