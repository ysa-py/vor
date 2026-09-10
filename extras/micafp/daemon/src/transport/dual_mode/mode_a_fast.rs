// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Mode A: Parallel Multipath Transport (Fast)
//
// Mode A: High Throughput / Low Latency Multipath QUIC Transport
//   • N concurrent independent paths (default: 3, max: 5)
//   • Single encryption layer (ChaCha20-Poly1305 or AES-256-GCM) — NO nested layers
//   • Zero-copy packet manipulation via `bytes::Bytes`
//   • Adaptive path scheduler with live quality metrics
//   • BBR / CUBIC congestion controller per path
//   • Dynamic Path MTU Discovery (PMTUD)
//
// RATIONALE:
//   Mode A is optimized for video streaming, real-time gaming, and bulk file
//   transfers where maximum throughput and low jitter are primary constraints.
//   It trades onion-style multi-hop anonymity for line-rate speed.
// ─────────────────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use bytes::{BufMut, Bytes, BytesMut};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

use super::scheduler::{
    AdaptiveMultipathScheduler, CongestionAlgorithm, CongestionState, PathMetrics, PathScheduler,
};

/// Configuration for Mode A Parallel Multipath Transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeAFastConfig {
    /// Number of concurrent paths (default: 3, max: 5).
    pub path_count: usize,
    /// Target remote relay endpoints.
    pub relay_endpoints: Vec<SocketAddr>,
    /// Crypto cipher choice (ChaCha20-Poly1305 or AES-256-GCM).
    pub cipher_suite: CipherSuite,
    /// Pre-shared or derived path session keys (32 bytes each).
    #[serde(skip)]
    pub session_keys: Vec<[u8; 32]>,
    /// Congestion control algorithm.
    pub congestion_algorithm: CongestionAlgorithm,
    /// Initial Path MTU (e.g. 1420).
    pub initial_pmtu: u16,
    /// Minimum allowed MTU.
    pub min_pmtu: u16,
    /// Enable dynamic PMTUD probing.
    pub enable_pmtud: bool,
    /// Flow affinity rebalancing interval.
    pub rebalance_interval_ms: u64,
}

impl Default for ModeAFastConfig {
    fn default() -> Self {
        Self {
            path_count: 3,
            relay_endpoints: vec![
                "198.51.100.1:443".parse().unwrap(),
                "198.51.100.2:443".parse().unwrap(),
                "198.51.100.3:443".parse().unwrap(),
            ],
            cipher_suite: CipherSuite::ChaCha20Poly1305,
            session_keys: vec![[0x42; 32], [0x43; 32], [0x44; 32]],
            congestion_algorithm: CongestionAlgorithm::Bbr,
            initial_pmtu: 1420,
            min_pmtu: 1280,
            enable_pmtud: true,
            rebalance_interval_ms: 1000,
        }
    }
}

/// Supported single-layer AEAD ciphers for Mode A.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CipherSuite {
    ChaCha20Poly1305,
    Aes256Gcm,
}

/// Dynamic Path MTU Discovery (PMTUD) state per path.
#[derive(Debug, Clone)]
pub struct PmtudController {
    pub current_pmtu: u16,
    pub min_pmtu: u16,
    pub max_pmtu: u16,
    pub probe_sequence: u32,
    pub last_probe_sent: Instant,
    pub probe_confirmed: bool,
}

impl PmtudController {
    pub fn new(initial: u16, min_pmtu: u16) -> Self {
        Self {
            current_pmtu: initial,
            min_pmtu,
            max_pmtu: 1500,
            probe_sequence: 0,
            last_probe_sent: Instant::now(),
            probe_confirmed: true,
        }
    }

    /// On packet too big ICMP/feedback received, adjust MTU down.
    pub fn handle_packet_too_big(&mut self, advertised_mtu: Option<u16>) -> u16 {
        if let Some(adv) = advertised_mtu {
            self.current_pmtu = adv.clamp(self.min_pmtu, self.current_pmtu);
        } else {
            // Step-down search
            let step = if self.current_pmtu > 1420 { 1420 } else { 1280 };
            self.current_pmtu = step.max(self.min_pmtu);
        }
        self.probe_confirmed = false;
        self.current_pmtu
    }

    /// Periodic probe to check if a larger MTU has become available.
    pub fn generate_probe_size(&mut self) -> Option<u16> {
        if self.current_pmtu < self.max_pmtu
            && self.last_probe_sent.elapsed() > Duration::from_secs(60)
        {
            self.last_probe_sent = Instant::now();
            self.probe_sequence += 1;
            Some((self.current_pmtu + 64).min(self.max_pmtu))
        } else {
            None
        }
    }
}

/// Represents an individual QUIC / UDP sub-path.
pub struct MultipathChannel {
    pub path_id: usize,
    pub endpoint: SocketAddr,
    cipher: ChaCha20Poly1305,
    nonce_counter: AtomicU64,
    pub pmtud: RwLock<PmtudController>,
    pub tx_packets: AtomicU64,
    pub rx_packets: AtomicU64,
    pub tx_bytes: AtomicU64,
    pub rx_bytes: AtomicU64,
}

impl MultipathChannel {
    pub fn new(
        path_id: usize,
        endpoint: SocketAddr,
        key_bytes: &[u8; 32],
        initial_pmtu: u16,
        min_pmtu: u16,
    ) -> Self {
        let key = Key::from_slice(key_bytes);
        let cipher = ChaCha20Poly1305::new(key);
        Self {
            path_id,
            endpoint,
            cipher,
            nonce_counter: AtomicU64::new(1),
            pmtud: RwLock::new(PmtudController::new(initial_pmtu, min_pmtu)),
            tx_packets: AtomicU64::new(0),
            rx_packets: AtomicU64::new(0),
            tx_bytes: AtomicU64::new(0),
            rx_bytes: AtomicU64::new(0),
        }
    }

    /// Single-layer encryption for outgoing packet using zero-copy Bytes.
    /// Wire format: [4-byte PathId | 8-byte Nonce | 4-byte StreamHint | Ciphertext + 16-byte Poly1305 Tag]
    pub fn encrypt_single_layer(&self, plaintext: &[u8], stream_hint: u32) -> Result<Bytes> {
        let nonce_val = self.nonce_counter.fetch_add(1, Ordering::SeqCst);
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..12].copy_from_slice(&nonce_val.to_be_bytes());
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Associated Data: path_id (4 bytes) + stream_hint (4 bytes)
        let mut aad = [0u8; 8];
        aad[0..4].copy_from_slice(&(self.path_id as u32).to_be_bytes());
        aad[4..8].copy_from_slice(&stream_hint.to_be_bytes());

        let payload = Payload {
            msg: plaintext,
            aad: &aad,
        };

        let ciphertext = self
            .cipher
            .encrypt(nonce, payload)
            .map_err(|e| anyhow::anyhow!("Mode A encryption failed: {:?}", e))?;

        // Zero-copy assembly into BytesMut
        let mut out = BytesMut::with_capacity(4 + 8 + 4 + ciphertext.len());
        out.put_u32(self.path_id as u32);
        out.put_u64(nonce_val);
        out.put_u32(stream_hint);
        out.put_slice(&ciphertext);

        self.tx_packets.fetch_add(1, Ordering::Relaxed);
        self.tx_bytes.fetch_add(out.len() as u64, Ordering::Relaxed);

        Ok(out.freeze())
    }

    /// Single-layer decapsulation for incoming packet.
    pub fn decrypt_single_layer(&self, mut packet: Bytes) -> Result<(Bytes, u32)> {
        if packet.len() < 4 + 8 + 4 + 16 {
            bail!("Mode A packet too short for single-layer decapsulation");
        }

        let path_id = u32::from_be_bytes(packet[0..4].try_into().unwrap()) as usize;
        if path_id != self.path_id {
            bail!(
                "Path ID mismatch: expected {}, got {}",
                self.path_id,
                path_id
            );
        }

        let nonce_val = u64::from_be_bytes(packet[4..12].try_into().unwrap());
        let stream_hint = u32::from_be_bytes(packet[12..16].try_into().unwrap());
        let ciphertext = &packet[16..];

        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..12].copy_from_slice(&nonce_val.to_be_bytes());
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut aad = [0u8; 8];
        aad[0..4].copy_from_slice(&(self.path_id as u32).to_be_bytes());
        aad[4..8].copy_from_slice(&stream_hint.to_be_bytes());

        let payload = Payload {
            msg: ciphertext,
            aad: &aad,
        };

        let plaintext = self.cipher.decrypt(nonce, payload).map_err(|e| {
            anyhow::anyhow!("Mode A authentication tag verification failed: {:?}", e)
        })?;

        self.rx_packets.fetch_add(1, Ordering::Relaxed);
        self.rx_bytes
            .fetch_add(packet.len() as u64, Ordering::Relaxed);

        Ok((Bytes::from(plaintext), stream_hint))
    }
}

/// Mode A parallel multipath transport engine.
pub struct ModeAFastEngine {
    config: ModeAFastConfig,
    scheduler: Arc<AdaptiveMultipathScheduler>,
    channels: Arc<RwLock<HashMap<usize, Arc<MultipathChannel>>>>,
    is_running: AtomicBool,
}

impl ModeAFastEngine {
    pub fn new(config: ModeAFastConfig) -> Result<Self> {
        let path_count = config.path_count.clamp(1, 5);
        let scheduler = Arc::new(AdaptiveMultipathScheduler::new(path_count));
        let channels = Arc::new(RwLock::new(HashMap::new()));

        let mut engine = Self {
            config,
            scheduler,
            channels,
            is_running: AtomicBool::new(false),
        };

        engine.init_channels()?;
        Ok(engine)
    }

    fn init_channels(&mut self) -> Result<()> {
        let count = self
            .config
            .path_count
            .min(self.config.relay_endpoints.len());
        let mut ch_map = self.channels.write();

        for i in 0..count {
            let endpoint = self.config.relay_endpoints[i];
            let key = if i < self.config.session_keys.len() {
                self.config.session_keys[i]
            } else {
                [0x55 + (i as u8); 32]
            };

            let channel = Arc::new(MultipathChannel::new(
                i,
                endpoint,
                &key,
                self.config.initial_pmtu,
                self.config.min_pmtu,
            ));
            ch_map.insert(i, channel);

            let mut metrics = PathMetrics::default();
            metrics.path_id = i;
            metrics.remote_addr = endpoint;
            metrics.rtt_ms = 20.0 + (i as f64 * 5.0); // Initial baseline estimation
            metrics.pmtu = self.config.initial_pmtu;
            self.scheduler.register_path(metrics);
        }

        info!(
            "Mode A initialized with {} parallel independent QUIC/UDP paths",
            count
        );
        Ok(())
    }

    /// Encapsulate a TUN plaintext packet across the optimal scheduled path.
    pub async fn dispatch_tun_packet(
        &self,
        packet: Bytes,
        flow_hint: Option<u64>,
    ) -> Result<(usize, Bytes)> {
        let chosen_path_id = self
            .scheduler
            .schedule_packet(packet.len(), flow_hint)
            .context("All Mode A paths are congested or unavailable")?;

        let channel = {
            let ch_map = self.channels.read();
            ch_map
                .get(&chosen_path_id)
                .cloned()
                .context("Scheduled path channel not found")?
        };

        let stream_hint = flow_hint.unwrap_or(0) as u32;
        let encrypted = channel.encrypt_single_layer(&packet, stream_hint)?;
        self.scheduler
            .on_packet_sent(chosen_path_id, encrypted.len());

        Ok((chosen_path_id, encrypted))
    }

    /// Process an incoming single-layer encrypted datagram.
    pub async fn handle_incoming_datagram(&self, path_id: usize, datagram: Bytes) -> Result<Bytes> {
        let channel = {
            let ch_map = self.channels.read();
            ch_map
                .get(&path_id)
                .cloned()
                .context("Incoming path channel not found")?
        };

        let (plaintext, _stream_hint) = channel.decrypt_single_layer(datagram)?;
        Ok(plaintext)
    }

    /// Notify engine of path loss or RTT sample.
    pub fn record_path_ack(&self, path_id: usize, rtt: Duration) {
        self.scheduler.on_packet_ack(path_id, rtt);
    }

    pub fn record_path_loss(&self, path_id: usize) {
        self.scheduler.on_packet_loss(path_id);
    }

    pub fn scheduler(&self) -> Arc<AdaptiveMultipathScheduler> {
        self.scheduler.clone()
    }
}
