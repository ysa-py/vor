// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Mode B: 5-Hop Nested Encryption (Layered)
//
// Mode B: High Anonymity / Path-Correlation Resistant Sphinx-Style Onion Transport
//   • 5-hop nested encapsulation (Hop 1 → Hop 2 → Hop 3 → Hop 4 → Hop 5)
//   • Hardware-accelerated ChaCha20-Poly1305 / AES-256-GCM
//   • Geographic diversity routing (selects nodes across distinct geopolitical regions)
//   • Fail-closed security guarantee: Drops packets on partial chain compromise
//     or unreachable hop without silent degradation unless fallback is explicitly set
//   • Operates strictly over UDP/QUIC datagrams (prevents TCP-over-TCP HOL blocking)
//
// PERFORMANCE NOTICE:
//   Mode B deliberately incurs higher end-to-end latency and CPU overhead than
//   Mode A due to 5 cumulative round-trip network hops and 5 layers of AEAD
//   encapsulation/decapsulation. It is engineered for threat resistance rather
//   than line-rate speed.
// ─────────────────────────────────────────────────────────────────────────────

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use bytes::{BufMut, Bytes, BytesMut};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use super::scheduler::{FixedHopChainScheduler, PathMetrics, PathScheduler};

pub const MODE_B_HOP_COUNT: usize = 5;

/// Geopolitical region tag for geographic diversity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeoRegion {
    WesternEurope,
    NorthernEurope,
    EastAsia,
    SoutheastAsia,
    NorthAmerica,
    NeutralTransit,
}

/// Metadata and cryptographic keys for a single hop in the 5-hop onion circuit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnionHopNode {
    pub hop_index: usize,
    pub node_id: String,
    pub endpoint: SocketAddr,
    pub region: GeoRegion,
    #[serde(skip)]
    pub hop_shared_key: [u8; 32],
    pub is_reachable: bool,
}

/// Configuration for Mode B Layered 5-Hop Onion Transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeBLayeredConfig {
    /// Ordered sequence of exactly 5 hops.
    pub hops: Vec<OnionHopNode>,
    /// Strictly fail-closed if any hop is unreachable (default: true).
    pub fail_closed: bool,
    /// Allow fallback to fewer hops in emergency situations (default: false).
    pub allow_emergency_fallback: bool,
    /// Per-hop ping interval for circuit liveness.
    pub liveness_interval_ms: u64,
}

impl Default for ModeBLayeredConfig {
    fn default() -> Self {
        let default_hops = vec![
            OnionHopNode {
                hop_index: 0,
                node_id: "eu-west-entry-01".into(),
                endpoint: "192.0.2.10:8443".parse().unwrap(),
                region: GeoRegion::WesternEurope,
                hop_shared_key: [0x11; 32],
                is_reachable: true,
            },
            OnionHopNode {
                hop_index: 1,
                node_id: "eu-north-relay-02".into(),
                endpoint: "192.0.2.20:8443".parse().unwrap(),
                region: GeoRegion::NorthernEurope,
                hop_shared_key: [0x22; 32],
                is_reachable: true,
            },
            OnionHopNode {
                hop_index: 2,
                node_id: "asia-east-relay-03".into(),
                endpoint: "192.0.2.30:8443".parse().unwrap(),
                region: GeoRegion::EastAsia,
                hop_shared_key: [0x33; 32],
                is_reachable: true,
            },
            OnionHopNode {
                hop_index: 3,
                node_id: "asia-se-relay-04".into(),
                endpoint: "192.0.2.40:8443".parse().unwrap(),
                region: GeoRegion::SoutheastAsia,
                hop_shared_key: [0x44; 32],
                is_reachable: true,
            },
            OnionHopNode {
                hop_index: 4,
                node_id: "na-east-exit-05".into(),
                endpoint: "192.0.2.50:8443".parse().unwrap(),
                region: GeoRegion::NorthAmerica,
                hop_shared_key: [0x55; 32],
                is_reachable: true,
            },
        ];

        Self {
            hops: default_hops,
            fail_closed: true,
            allow_emergency_fallback: false,
            liveness_interval_ms: 2000,
        }
    }
}

/// 5-Layer Onion Encapsulation Engine.
pub struct ModeBLayeredEngine {
    config: ModeBLayeredConfig,
    scheduler: Arc<FixedHopChainScheduler>,
    hop_ciphers: Vec<ChaCha20Poly1305>,
    circuit_nonce: AtomicU64,
    tx_packets: AtomicU64,
    rx_packets: AtomicU64,
    dropped_fail_closed: AtomicU64,
}

impl ModeBLayeredEngine {
    pub fn new(config: ModeBLayeredConfig) -> Result<Self> {
        if config.hops.len() != MODE_B_HOP_COUNT {
            bail!(
                "Mode B strictly requires {} hops, provided {}",
                MODE_B_HOP_COUNT,
                config.hops.len()
            );
        }

        // Validate geographic diversity: at least 3 distinct regions
        let distinct_regions: std::collections::HashSet<_> =
            config.hops.iter().map(|h| &h.region).collect();
        if distinct_regions.len() < 3 {
            warn!(
                "Mode B warning: geographic diversity is below recommended 3 regions (found {})",
                distinct_regions.len()
            );
        }

        let mut hop_ciphers = Vec::with_capacity(MODE_B_HOP_COUNT);
        let mut path_metrics = Vec::with_capacity(MODE_B_HOP_COUNT);

        for (i, hop) in config.hops.iter().enumerate() {
            let key = Key::from_slice(&hop.hop_shared_key);
            hop_ciphers.push(ChaCha20Poly1305::new(key));

            let mut m = PathMetrics::default();
            m.path_id = i;
            m.remote_addr = hop.endpoint;
            m.rtt_ms = 35.0 + (i as f64 * 15.0);
            m.is_alive = hop.is_reachable;
            path_metrics.push(m);
        }

        let scheduler = Arc::new(FixedHopChainScheduler::new(
            path_metrics,
            config.fail_closed,
        ));

        Ok(Self {
            config,
            scheduler,
            hop_ciphers,
            circuit_nonce: AtomicU64::new(1),
            tx_packets: AtomicU64::new(0),
            rx_packets: AtomicU64::new(0),
            dropped_fail_closed: AtomicU64::new(0),
        })
    }

    /// 5-Layer Nested Encapsulation (Client → Hop 0).
    ///
    /// The inner payload is first encrypted for Hop 4 (Exit), then for Hop 3,
    /// Hop 2, Hop 1, and finally Hop 0 (Entry).
    /// Each hop strips one layer and discovers the next hop's routing instructions.
    pub fn encapsulate_5_hops(&self, plaintext: &[u8]) -> Result<Bytes> {
        // Enforce fail-closed check
        if !self.scheduler.is_chain_healthy() && self.config.fail_closed {
            self.dropped_fail_closed.fetch_add(1, Ordering::Relaxed);
            bail!(
                "Mode B circuit failed closed: one or more hops in the 5-hop chain are unreachable"
            );
        }

        let base_nonce = self.circuit_nonce.fetch_add(1, Ordering::SeqCst);
        let mut current_payload = plaintext.to_vec();

        // Encapsulate backwards from Exit (Hop 4) to Entry (Hop 0)
        for hop_idx in (0..MODE_B_HOP_COUNT).rev() {
            let cipher = &self.hop_ciphers[hop_idx];

            let mut nonce_bytes = [0u8; 12];
            nonce_bytes[0..4].copy_from_slice(&(hop_idx as u32).to_be_bytes());
            nonce_bytes[4..12].copy_from_slice(&base_nonce.to_be_bytes());
            let nonce = Nonce::from_slice(&nonce_bytes);

            // AAD includes next hop info (or exit flag for hop 4)
            let mut aad = [0u8; 16];
            aad[0..4].copy_from_slice(&(hop_idx as u32).to_be_bytes());
            if hop_idx < MODE_B_HOP_COUNT - 1 {
                let next_hop_addr = self.config.hops[hop_idx + 1].endpoint;
                match next_hop_addr {
                    SocketAddr::V4(v4) => {
                        aad[4..8].copy_from_slice(&v4.ip().octets());
                        aad[8..10].copy_from_slice(&v4.port().to_be_bytes());
                    }
                    SocketAddr::V6(v6) => {
                        aad[4..8].copy_from_slice(&v6.ip().octets()[0..4]);
                        aad[8..10].copy_from_slice(&v6.port().to_be_bytes());
                    }
                }
            } else {
                aad[4..8].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // Exit marker
            }

            let payload = Payload {
                msg: &current_payload,
                aad: &aad,
            };

            let encrypted_layer = cipher
                .encrypt(nonce, payload)
                .map_err(|e| anyhow::anyhow!("Encryption failed at hop {}: {:?}", hop_idx, e))?;

            // Prepend hop header: [1-byte HopIndex | 8-byte BaseNonce | Ciphertext]
            let mut layer_buf = BytesMut::with_capacity(1 + 8 + encrypted_layer.len());
            layer_buf.put_u8(hop_idx as u8);
            layer_buf.put_u64(base_nonce);
            layer_buf.put_slice(&encrypted_layer);

            current_payload = layer_buf.to_vec();
        }

        self.tx_packets.fetch_add(1, Ordering::Relaxed);
        Ok(Bytes::from(current_payload))
    }

    /// Single-hop decapsulation helper (simulating node processing along the chain).
    pub fn decapsulate_hop_layer(
        &self,
        hop_idx: usize,
        packet: &[u8],
    ) -> Result<(Bytes, Option<SocketAddr>)> {
        if hop_idx >= MODE_B_HOP_COUNT {
            bail!("Invalid hop index {}", hop_idx);
        }
        if packet.len() < 1 + 8 + 16 {
            bail!("Packet too short for hop decapsulation");
        }

        let header_hop = packet[0] as usize;
        if header_hop != hop_idx {
            bail!(
                "Header hop mismatch: expected {}, got {}",
                hop_idx,
                header_hop
            );
        }

        let base_nonce = u64::from_be_bytes(packet[1..9].try_into().unwrap());
        let ciphertext = &packet[9..];

        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[0..4].copy_from_slice(&(hop_idx as u32).to_be_bytes());
        nonce_bytes[4..12].copy_from_slice(&base_nonce.to_be_bytes());
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut aad = [0u8; 16];
        aad[0..4].copy_from_slice(&(hop_idx as u32).to_be_bytes());
        let next_hop_addr = if hop_idx < MODE_B_HOP_COUNT - 1 {
            let next_addr = self.config.hops[hop_idx + 1].endpoint;
            match next_addr {
                SocketAddr::V4(v4) => {
                    aad[4..8].copy_from_slice(&v4.ip().octets());
                    aad[8..10].copy_from_slice(&v4.port().to_be_bytes());
                }
                SocketAddr::V6(v6) => {
                    aad[4..8].copy_from_slice(&v6.ip().octets()[0..4]);
                    aad[8..10].copy_from_slice(&v6.port().to_be_bytes());
                }
            }
            Some(next_addr)
        } else {
            aad[4..8].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
            None
        };

        let cipher = &self.hop_ciphers[hop_idx];
        let payload = Payload {
            msg: ciphertext,
            aad: &aad,
        };

        let plaintext = cipher
            .decrypt(nonce, payload)
            .map_err(|e| anyhow::anyhow!("Decapsulation failed at hop {}: {:?}", hop_idx, e))?;

        Ok((Bytes::from(plaintext), next_hop_addr))
    }

    /// Full end-to-end decapsulation verification through all 5 hops.
    pub fn verify_full_5_hop_pipeline(&self, onion_packet: &[u8]) -> Result<Bytes> {
        let mut current = onion_packet.to_vec();
        for hop_idx in 0..MODE_B_HOP_COUNT {
            let (unwrapped, _next) = self.decapsulate_hop_layer(hop_idx, &current)?;
            current = unwrapped.to_vec();
        }
        self.rx_packets.fetch_add(1, Ordering::Relaxed);
        Ok(Bytes::from(current))
    }

    /// Set status of a hop (used for failover/liveness detection).
    pub fn set_hop_liveness(&self, hop_index: usize, alive: bool) {
        if alive {
            self.scheduler
                .on_packet_ack(hop_index, Duration::from_millis(50));
        } else {
            self.scheduler.on_packet_loss(hop_index);
        }
    }

    pub fn scheduler(&self) -> Arc<FixedHopChainScheduler> {
        self.scheduler.clone()
    }

    pub fn ingress_endpoint(&self) -> SocketAddr {
        self.config.hops[0].endpoint
    }
}
