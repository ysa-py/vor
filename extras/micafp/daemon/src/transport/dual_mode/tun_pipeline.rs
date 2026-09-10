// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Zero-Copy TUN Pipeline & Dispatcher (Directive v70)
//
// Bridges OS-level TUN virtual interface with Dual-Mode Transport:
//   • Zero-copy memory handoff using `bytes::Bytes` & `bytes::BytesMut`
//   • Support for Unix file descriptors (`AsyncFd` / `RawFd`) and memory channels
//   • Runtime selectable Dual-Mode switching between Mode A (Fast) & Mode B (Layered)
//   • Compatible with Desktop Tokio multi-thread and iOS single-thread runtime
// ─────────────────────────────────────────────────────────────────────────────

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use bytes::{Bytes, BytesMut};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

use super::mode_a_fast::{ModeAFastConfig, ModeAFastEngine};
use super::mode_b_layered::{ModeBLayeredConfig, ModeBLayeredEngine};

/// Active transport mode selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DualTransportMode {
    /// Mode A: Parallel Multipath Transport (High throughput / low latency).
    Fast,
    /// Mode B: 5-Hop Nested Onion Encryption (High anonymity / mixnet style).
    Layered,
}

impl Default for DualTransportMode {
    fn default() -> Self {
        DualTransportMode::Fast
    }
}

/// Unified configuration for Dual-Mode Transport Core.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualModeCoreConfig {
    pub active_mode: DualTransportMode,
    pub fast_config: ModeAFastConfig,
    pub layered_config: ModeBLayeredConfig,
    pub tun_mtu: u16,
    pub queue_depth: usize,
}

impl Default for DualModeCoreConfig {
    fn default() -> Self {
        Self {
            active_mode: DualTransportMode::Fast,
            fast_config: ModeAFastConfig::default(),
            layered_config: ModeBLayeredConfig::default(),
            tun_mtu: 1420,
            queue_depth: 4096,
        }
    }
}

/// Runtime engine managing zero-copy TUN ingestion and dual-mode dispatch.
pub struct DualModeCoreEngine {
    config: RwLock<DualModeCoreConfig>,
    mode_a_engine: Arc<ModeAFastEngine>,
    mode_b_engine: Arc<ModeBLayeredEngine>,
    is_running: AtomicBool,
    tun_tx_bytes: AtomicU64,
    tun_rx_bytes: AtomicU64,
    tun_tx_packets: AtomicU64,
    tun_rx_packets: AtomicU64,
}

impl DualModeCoreEngine {
    pub fn new(config: DualModeCoreConfig) -> Result<Self> {
        let mode_a = Arc::new(ModeAFastEngine::new(config.fast_config.clone())?);
        let mode_b = Arc::new(ModeBLayeredEngine::new(config.layered_config.clone())?);

        Ok(Self {
            config: RwLock::new(config),
            mode_a_engine: mode_a,
            mode_b_engine: mode_b,
            is_running: AtomicBool::new(false),
            tun_tx_bytes: AtomicU64::new(0),
            tun_rx_bytes: AtomicU64::new(0),
            tun_tx_packets: AtomicU64::new(0),
            tun_rx_packets: AtomicU64::new(0),
        })
    }

    /// Switch active transport mode dynamically at runtime without restarting service.
    pub fn set_transport_mode(&self, new_mode: DualTransportMode) {
        let mut cfg = self.config.write();
        let prev = cfg.active_mode;
        cfg.active_mode = new_mode;
        info!("Dual-Mode Transport switched: {:?} -> {:?}", prev, new_mode);
    }

    pub fn current_mode(&self) -> DualTransportMode {
        self.config.read().active_mode
    }

    /// Ingest a zero-copy packet from TUN interface and dispatch through active mode.
    pub async fn process_outbound_tun_packet(
        &self,
        packet: Bytes,
        flow_hint: Option<u64>,
    ) -> Result<(SocketAddr, Bytes)> {
        let mode = self.current_mode();
        self.tun_tx_packets.fetch_add(1, Ordering::Relaxed);
        self.tun_tx_bytes
            .fetch_add(packet.len() as u64, Ordering::Relaxed);

        match mode {
            DualTransportMode::Fast => {
                let (path_id, encrypted) = self
                    .mode_a_engine
                    .dispatch_tun_packet(packet, flow_hint)
                    .await?;
                let endpoint = self
                    .config
                    .read()
                    .fast_config
                    .relay_endpoints
                    .get(path_id)
                    .cloned()
                    .unwrap_or_else(|| "127.0.0.1:443".parse().unwrap());
                Ok((endpoint, encrypted))
            }
            DualTransportMode::Layered => {
                let onion_payload = self.mode_b_engine.encapsulate_5_hops(&packet)?;
                let ingress = self.mode_b_engine.ingress_endpoint();
                Ok((ingress, onion_payload))
            }
        }
    }

    /// Process incoming encrypted wire payload and emit zero-copy plaintext for TUN write.
    pub async fn process_inbound_wire_packet(
        &self,
        path_or_hop_id: usize,
        wire_packet: Bytes,
    ) -> Result<Bytes> {
        let mode = self.current_mode();
        self.tun_rx_packets.fetch_add(1, Ordering::Relaxed);
        self.tun_rx_bytes
            .fetch_add(wire_packet.len() as u64, Ordering::Relaxed);

        match mode {
            DualTransportMode::Fast => {
                self.mode_a_engine
                    .handle_incoming_datagram(path_or_hop_id, wire_packet)
                    .await
            }
            DualTransportMode::Layered => {
                self.mode_b_engine.verify_full_5_hop_pipeline(&wire_packet)
            }
        }
    }

    pub fn mode_a(&self) -> Arc<ModeAFastEngine> {
        self.mode_a_engine.clone()
    }

    pub fn mode_b(&self) -> Arc<ModeBLayeredEngine> {
        self.mode_b_engine.clone()
    }

    /// Get cumulative statistics snapshot.
    pub fn get_telemetry_snapshot(&self) -> DualModeStatsSnapshot {
        DualModeStatsSnapshot {
            active_mode: self.current_mode(),
            tun_tx_bytes: self.tun_tx_bytes.load(Ordering::Relaxed),
            tun_rx_bytes: self.tun_rx_bytes.load(Ordering::Relaxed),
            tun_tx_packets: self.tun_tx_packets.load(Ordering::Relaxed),
            tun_rx_packets: self.tun_rx_packets.load(Ordering::Relaxed),
        }
    }
}

/// Telemetry stats snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualModeStatsSnapshot {
    pub active_mode: DualTransportMode,
    pub tun_tx_bytes: u64,
    pub tun_rx_bytes: u64,
    pub tun_tx_packets: u64,
    pub tun_rx_packets: u64,
}
