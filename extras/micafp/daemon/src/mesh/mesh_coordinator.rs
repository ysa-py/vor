// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Mesh Coordinator
// Orchestrates all mesh networking channels with automatic failover.
// ─────────────────────────────────────────────────────────────────────────────

use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

use super::{MeshChannel, MeshPeer};

/// Configuration for the mesh coordinator.
#[derive(Debug, Clone)]
pub struct MeshConfig {
    pub max_hop_count: u8,
    pub peer_timeout_secs: u64,
    pub enable_wifi_direct: bool,
    pub enable_wifi_aware: bool,
    pub enable_ble_mesh: bool,
    pub enable_yggdrasil: bool,
    pub enable_i2p: bool,
    pub gossip_interval_secs: u64,
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            max_hop_count: 5,
            peer_timeout_secs: 300,
            enable_wifi_direct: true,
            enable_wifi_aware: true,
            enable_ble_mesh: true,
            enable_yggdrasil: true,
            enable_i2p: false, // disabled by default (slowest)
            gossip_interval_secs: 60,
        }
    }
}

/// Metrics snapshot for the mesh network.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct MeshMetrics {
    pub total_peers: usize,
    pub reachable_peers: usize,
    pub active_channel: Option<MeshChannel>,
    pub avg_hop_count: f64,
    pub messages_relayed: u64,
    pub bytes_relayed: u64,
}

/// The mesh network coordinator. Manages peer discovery, routing,
/// and channel selection across all mesh technologies.
pub struct MeshCoordinator {
    config: MeshConfig,
    peers: Arc<RwLock<Vec<MeshPeer>>>,
    metrics: Arc<RwLock<MeshMetrics>>,
    active_channel: Arc<RwLock<Option<MeshChannel>>>,
}

impl MeshCoordinator {
    pub fn new(config: MeshConfig) -> Self {
        Self {
            config,
            peers: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(MeshMetrics::default())),
            active_channel: Arc::new(RwLock::new(None)),
        }
    }

    /// Start all enabled mesh channels concurrently.
    pub async fn start(&self) {
        info!(
            "mesh: starting coordinator with config: wifi_direct={}, ble={}, yggdrasil={}",
            self.config.enable_wifi_direct,
            self.config.enable_ble_mesh,
            self.config.enable_yggdrasil,
        );

        // In production: spawn WiFi Aware, BLE, Yggdrasil tasks here
        // Each channel updates self.peers when new peers are discovered
        tokio::time::sleep(Duration::from_millis(100)).await;
        info!("mesh: coordinator started");
    }

    /// Select the best available channel to reach a peer.
    pub fn best_channel_for(&self, peer_id: &[u8; 32]) -> Option<MeshChannel> {
        let peers = self.peers.read();
        let peer = peers.iter().find(|p| p.peer_id.as_bytes() == peer_id)?;
        peer.channels.iter().copied().min_by_key(|c| *c as u8) // lower = better priority
    }

    /// Add or refresh a discovered peer.
    pub fn upsert_peer(&self, peer: MeshPeer) {
        let mut peers = self.peers.write();
        if let Some(existing) = peers.iter_mut().find(|p| p.peer_id == peer.peer_id) {
            *existing = peer;
        } else {
            info!("mesh: new peer discovered via {:?}", peer.channels.first());
            peers.push(peer);
        }
        self.update_metrics_internal(&mut self.metrics.write(), &peers);
    }

    /// Evict peers that haven't been seen in `peer_timeout_secs`.
    pub fn evict_stale_peers(&self, now_ms: u64) {
        let timeout_ms = self.config.peer_timeout_secs * 1000;
        let mut peers = self.peers.write();
        let before = peers.len();
        peers.retain(|p| now_ms.saturating_sub(p.last_seen_ms) < timeout_ms);
        let removed = before - peers.len();
        if removed > 0 {
            debug!("mesh: evicted {} stale peers", removed);
        }
    }

    pub fn metrics(&self) -> MeshMetrics {
        self.metrics.read().clone()
    }

    pub fn peer_count(&self) -> usize {
        self.peers.read().len()
    }

    fn update_metrics_internal(&self, metrics: &mut MeshMetrics, peers: &[MeshPeer]) {
        metrics.total_peers = peers.len();
        metrics.reachable_peers = peers.iter().filter(|p| !p.channels.is_empty()).count();
        if !peers.is_empty() {
            metrics.avg_hop_count =
                peers.iter().map(|p| p.hop_count as f64).sum::<f64>() / peers.len() as f64;
        }
    }
}
