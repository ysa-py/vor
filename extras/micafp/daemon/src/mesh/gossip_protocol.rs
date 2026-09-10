// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Gossip Protocol (Epidemic Routing)
// Epidemic peer discovery: each node shares its peer list with neighbors.
// Converges to full mesh knowledge in O(log N) rounds.
// ─────────────────────────────────────────────────────────────────────────────

use std::collections::HashSet;
use std::time::Duration;
use tracing::debug;

/// Hex-serializable wrapper for [u8; 64] so serde can handle it.
mod serde_hex {
    pub mod big_array {
        use serde::{Deserialize, Deserializer, Serialize, Serializer};
        pub fn serialize<S: Serializer>(data: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
            hex::encode(data).serialize(s)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
            let s = String::deserialize(d)?;
            let mut arr = [0u8; 64];
            hex::decode_to_slice(&s, &mut arr).map_err(serde::de::Error::custom)?;
            Ok(arr)
        }
    }
    pub mod array_32 {
        use serde::{Deserialize, Deserializer, Serialize, Serializer};
        pub fn serialize<S: Serializer>(data: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
            hex::encode(data).serialize(s)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
            let s = String::deserialize(d)?;
            let mut arr = [0u8; 32];
            hex::decode_to_slice(&s, &mut arr).map_err(serde::de::Error::custom)?;
            Ok(arr)
        }
    }
}

/// Gossip message exchanged between mesh peers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GossipMessage {
    #[serde(with = "serde_hex::array_32")]
    pub sender_id: [u8; 32],
    #[serde(with = "serde_hex_vec::array_32_vec")]
    pub known_peers: Vec<[u8; 32]>,
    pub timestamp_ms: u64,
    pub hop_count: u8,
    #[serde(with = "serde_hex::big_array")]
    pub signature: [u8; 64],
}

mod serde_hex_vec {
    pub mod array_32_vec {
        use serde::{Deserialize, Deserializer, Serialize, Serializer};
        use super::super::serde_hex::array_32;
        pub fn serialize<S: Serializer>(data: &Vec<[u8; 32]>, s: S) -> Result<S::Ok, S::Error> {
            let hex_strings: Vec<String> = data.iter().map(|a| hex::encode(a)).collect();
            hex_strings.serialize(s)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<[u8; 32]>, D::Error> {
            let strings: Vec<String> = Vec::deserialize(d)?;
            strings.iter().map(|s| {
                let mut arr = [0u8; 32];
                hex::decode_to_slice(s, &mut arr).map_err(serde::de::Error::custom)?;
                Ok(arr)
            }).collect()
        }
    }
}

/// Epidemic gossip protocol handler.
pub struct GossipProtocol {
    local_id: [u8; 32],
    fanout: usize, // number of peers to gossip to per round
    ttl: u8,       // max hop count
    seen_messages: parking_lot::Mutex<HashSet<String>>,
}

impl GossipProtocol {
    pub fn new(local_id: [u8; 32], fanout: usize, ttl: u8) -> Self {
        Self {
            local_id,
            fanout,
            ttl,
            seen_messages: parking_lot::Mutex::new(HashSet::new()),
        }
    }

    pub fn is_seen(&self, msg_id: &str) -> bool {
        self.seen_messages.lock().contains(msg_id)
    }

    pub fn mark_seen(&self, msg_id: &str) {
        self.seen_messages.lock().insert(msg_id.to_string());
    }
}