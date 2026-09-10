// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Mesh Topology Manager
// Tracks the mesh graph and computes shortest relay paths using Dijkstra.
// ─────────────────────────────────────────────────────────────────────────────

use parking_lot::RwLock;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::Arc;

type PeerId = [u8; 32];

/// An edge in the mesh topology graph.
#[derive(Debug, Clone)]
struct Edge {
    to: PeerId,
    weight_ms: u32, // estimated RTT in ms
}

/// Manages the mesh topology graph and route computation.
pub struct TopologyManager {
    graph: Arc<RwLock<HashMap<PeerId, Vec<Edge>>>>,
}

impl TopologyManager {
    pub fn new() -> Self {
        Self {
            graph: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add or update an edge between two peers.
    pub fn update_edge(&self, from: PeerId, to: PeerId, rtt_ms: u32) {
        let mut graph = self.graph.write();
        graph.entry(from).or_default().retain(|e| e.to != to);
        graph.entry(from).or_default().push(Edge {
            to,
            weight_ms: rtt_ms,
        });
        // Undirected
        graph.entry(to).or_default().retain(|e| e.to != from);
        graph.entry(to).or_default().push(Edge {
            to: from,
            weight_ms: rtt_ms,
        });
    }

    /// Remove a peer from the topology.
    pub fn remove_peer(&self, peer_id: &PeerId) {
        let mut graph = self.graph.write();
        graph.remove(peer_id);
        for edges in graph.values_mut() {
            edges.retain(|e| &e.to != peer_id);
        }
    }

    /// Compute shortest path (by RTT) from `src` to `dst` using Dijkstra.
    /// Returns the list of hop peer IDs, or None if unreachable.
    pub fn shortest_path(&self, src: &PeerId, dst: &PeerId) -> Option<Vec<PeerId>> {
        let graph = self.graph.read();

        let mut dist: HashMap<PeerId, u32> = HashMap::new();
        let mut prev: HashMap<PeerId, PeerId> = HashMap::new();
        let mut heap: BinaryHeap<Reverse<(u32, PeerId)>> = BinaryHeap::new();

        dist.insert(*src, 0);
        heap.push(Reverse((0, *src)));

        while let Some(Reverse((d, u))) = heap.pop() {
            if &u == dst {
                // Reconstruct path
                let mut path = vec![u];
                let mut cur = u;
                while let Some(&p) = prev.get(&cur) {
                    path.push(p);
                    cur = p;
                }
                path.reverse();
                return Some(path);
            }
            if d > *dist.get(&u).unwrap_or(&u32::MAX) {
                continue;
            }
            if let Some(edges) = graph.get(&u) {
                for edge in edges {
                    let next_d = d.saturating_add(edge.weight_ms);
                    if next_d < *dist.get(&edge.to).unwrap_or(&u32::MAX) {
                        dist.insert(edge.to, next_d);
                        prev.insert(edge.to, u);
                        heap.push(Reverse((next_d, edge.to)));
                    }
                }
            }
        }
        None
    }

    pub fn node_count(&self) -> usize {
        self.graph.read().len()
    }
}

impl Default for TopologyManager {
    fn default() -> Self {
        Self::new()
    }
}
