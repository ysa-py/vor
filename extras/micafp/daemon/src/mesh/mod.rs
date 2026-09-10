pub mod gossip_protocol;
pub mod mesh_coordinator;
pub mod mesh_crypto;
pub mod topology_manager;
pub use gossip_protocol::GossipProtocol;
pub use mesh_coordinator::MeshCoordinator;
pub use topology_manager::TopologyManager;

// ── Shared mesh types ─────────────────────────────────────────────────────────

pub use mesh_crypto::MeshSessionKey;

/// Communication channel type for mesh networking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, serde::Serialize, serde::Deserialize)]
pub enum MeshChannel {
    BleAdvertising,
    WifiAware,
    WifiDirect,
    Yggdrasil,
    I2pOverlay,
}

/// A peer node in the mesh network.
#[derive(Debug, Clone)]
pub struct MeshPeer {
    pub peer_id: String,
    pub channels: Vec<MeshChannel>,
    pub hop_count: u8,
    pub rtt_ms: Option<u32>,
    pub last_seen_ms: u64,
}
