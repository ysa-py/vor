//! Configuration specifications for Directive v70 Dual-Mode Transport Core.
//! Defines Mode A (Parallel Multipath) and Mode B (5-Hop Nested Layered) settings.

use serde::{Deserialize, Serialize};

/// Selectable Dual-Mode Transport types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportMode {
    /// Mode A (Default): Parallel Multipath Transport via QUIC over UDP.
    /// Focus: Maximum throughput, minimal latency, single-layer crypto per path.
    #[serde(rename = "mode_a_fast")]
    ModeAFast,

    /// Mode B (Opt-in): 5-Hop Nested Layered Encryption via Sphinx / Noise.
    /// Focus: High anonymity, path-correlation resistance, fail-closed onion routing.
    #[serde(rename = "mode_b_layered")]
    ModeBLayered,
}

impl Default for TransportMode {
    fn default() -> Self {
        TransportMode::ModeAFast
    }
}

/// Congestion control algorithms for Mode A QUIC paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CongestionControl {
    Bbr,
    Cubic,
}

impl Default for CongestionControl {
    fn default() -> Self {
        CongestionControl::Bbr
    }
}

/// Supported symmetric cipher suites for transport encapsulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CipherSuite {
    ChaCha20Poly1305,
    Aes256Gcm,
}

impl Default for CipherSuite {
    fn default() -> Self {
        CipherSuite::ChaCha20Poly1305
    }
}

/// Configuration for a single QUIC path in Mode A.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    pub path_id: u8,
    pub endpoint: String,
    pub region_tag: String,
    pub initial_rtt_ms: u32,
    pub congestion_control: CongestionControl,
    pub cipher: CipherSuite,
    pub max_mtu: u16,
}

/// Geographic node description for Mode B 5-hop circuit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoHopNode {
    pub hop_index: u8, // 1 to 5
    pub node_id: String,
    pub endpoint: String,
    pub region_tag: String, // e.g. "EU-Frankfurt", "SG-Singapore", "US-Virginia", "JP-Tokyo", "CH-Alibaba"
    pub public_key: String,
    pub expected_rtt_ms: u32,
}

/// Main runtime configuration for Dual-Mode Transport Engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualModeConfig {
    pub mode: TransportMode,
    
    // Mode A specific parameters
    pub concurrent_paths: u8, // 1 to 5 (default 3)
    pub mode_a_paths: Vec<PathConfig>,
    pub pmtud_enabled: bool,
    pub scheduler_rebalance_interval_ms: u64,

    // Mode B specific parameters
    pub mode_b_hops: Vec<GeoHopNode>, // Exactly 5 hops
    pub fail_closed: bool, // Default true: Drop packet if any hop drops, no silent fallback
    pub allow_degraded_fallback: bool, // Strict opt-in

    // Shared parameters
    pub zero_copy_buffer_size: usize,
    pub ios_memory_ceiling_mb: u32,
}

impl Default for DualModeConfig {
    fn default() -> Self {
        let default_paths = vec![
            PathConfig {
                path_id: 1,
                endpoint: "relay-de1.unifiedshield.net:443".to_string(),
                region_tag: "EU-Central".to_string(),
                initial_rtt_ms: 45,
                congestion_control: CongestionControl::Bbr,
                cipher: CipherSuite::ChaCha20Poly1305,
                max_mtu: 1420,
            },
            PathConfig {
                path_id: 2,
                endpoint: "relay-sg1.unifiedshield.net:443".to_string(),
                region_tag: "AP-Southeast".to_string(),
                initial_rtt_ms: 68,
                congestion_control: CongestionControl::Bbr,
                cipher: CipherSuite::ChaCha20Poly1305,
                max_mtu: 1420,
            },
            PathConfig {
                path_id: 3,
                endpoint: "relay-hk1.unifiedshield.net:443".to_string(),
                region_tag: "AP-East".to_string(),
                initial_rtt_ms: 55,
                congestion_control: CongestionControl::Bbr,
                cipher: CipherSuite::ChaCha20Poly1305,
                max_mtu: 1420,
            },
        ];

        let default_hops = vec![
            GeoHopNode {
                hop_index: 1,
                node_id: "node-entry-fra".to_string(),
                endpoint: "hop1.unifiedshield.net:8443".to_string(),
                region_tag: "EU-West (Frankfurt)".to_string(),
                public_key: "ed25519_pk_hop1_entry_gateway".to_string(),
                expected_rtt_ms: 42,
            },
            GeoHopNode {
                hop_index: 2,
                node_id: "node-relay-sin".to_string(),
                endpoint: "hop2.unifiedshield.net:8443".to_string(),
                region_tag: "SG-East (Singapore)".to_string(),
                public_key: "ed25519_pk_hop2_mixnet_relay".to_string(),
                expected_rtt_ms: 65,
            },
            GeoHopNode {
                hop_index: 3,
                node_id: "node-core-iad".to_string(),
                endpoint: "hop3.unifiedshield.net:8443".to_string(),
                region_tag: "US-East (Virginia)".to_string(),
                public_key: "ed25519_pk_hop3_mixnet_core".to_string(),
                expected_rtt_ms: 110,
            },
            GeoHopNode {
                hop_index: 4,
                node_id: "node-bridge-nrt".to_string(),
                endpoint: "hop4.unifiedshield.net:8443".to_string(),
                region_tag: "JP-Central (Tokyo)".to_string(),
                public_key: "ed25519_pk_hop4_bridge_relay".to_string(),
                expected_rtt_ms: 88,
            },
            GeoHopNode {
                hop_index: 5,
                node_id: "node-egress-hkg".to_string(),
                endpoint: "hop5.unifiedshield.net:8443".to_string(),
                region_tag: "CH-Alibaba (Hong Kong)".to_string(),
                public_key: "ed25519_pk_hop5_exit_gateway".to_string(),
                expected_rtt_ms: 50,
            },
        ];

        Self {
            mode: TransportMode::ModeAFast,
            concurrent_paths: 3,
            mode_a_paths: default_paths,
            pmtud_enabled: true,
            scheduler_rebalance_interval_ms: 250,
            mode_b_hops: default_hops,
            fail_closed: true,
            allow_degraded_fallback: false,
            zero_copy_buffer_size: 65536,
            ios_memory_ceiling_mb: 35,
        }
    }
}
