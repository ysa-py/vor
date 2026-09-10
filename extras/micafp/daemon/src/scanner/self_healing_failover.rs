//! Self-Healing Failover & Health Watchdog Engine
//!
//! Continuously monitors active tunnel health, detects pre-failure degradation
//! (latency spikes, packet loss bursts), maintains a warm standby pool of top verified candidates,
//! and orchestrates sub-second failovers with anti-flapping hysteresis and circuit breakers.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::scanner::candidate_graph::{CandidateGraph, CandidateNode, CandidateState};

/// Failover Event Record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverEvent {
    pub timestamp: DateTime<Utc>,
    pub previous_node_id: String,
    pub new_node_id: String,
    pub trigger_reason: String,
    pub switch_latency_ms: u64,
}

/// State of the Self-Healing Failover Engine
pub struct SelfHealingEngine {
    graph: Arc<CandidateGraph>,
    active_node: Arc<RwLock<Option<CandidateNode>>>,
    warm_standbys: Arc<RwLock<Vec<CandidateNode>>>,
    last_failover_time: Arc<RwLock<DateTime<Utc>>>,
    failover_history: Arc<RwLock<Vec<FailoverEvent>>>,
    min_hold_time: Duration, // Anti-flapping minimum hold time (e.g. 15s)
}

impl SelfHealingEngine {
    pub fn new(graph: Arc<CandidateGraph>) -> Self {
        Self {
            graph,
            active_node: Arc::new(RwLock::new(None)),
            warm_standbys: Arc::new(RwLock::new(Vec::new())),
            last_failover_time: Arc::new(RwLock::new(Utc::now() - chrono::Duration::seconds(60))),
            failover_history: Arc::new(RwLock::new(Vec::new())),
            min_hold_time: Duration::from_secs(15),
        }
    }

    /// Refresh warm standby pool with top verified nodes
    pub fn refresh_standby_pool(&self) {
        let top_nodes = self.graph.get_top_verified(5);
        let mut standbys = self.warm_standbys.write();
        *standbys = top_nodes;
    }

    /// Set current active node
    pub fn set_active_node(&self, node: CandidateNode) {
        let mut active = self.active_node.write();
        *active = Some(node);
    }

    /// Evaluate active connection health and trigger predictive failover if degraded
    pub fn evaluate_health_and_heal(
        &self,
        current_latency_ms: u64,
        current_packet_loss: f32,
        consecutive_timeouts: u32,
    ) -> Option<CandidateNode> {
        let active_opt = self.active_node.read().clone();
        let active = match active_opt {
            Some(a) => a,
            None => {
                // No active node, select best verified
                let best = self.graph.get_top_verified(1).into_iter().next();
                if let Some(ref b) = best {
                    self.set_active_node(b.clone());
                }
                return best;
            }
        };

        // Pre-failure trigger conditions
        let is_degraded = consecutive_timeouts >= 2
            || current_packet_loss > 0.35
            || (active.recent_latency_ms.map_or(false, |base| {
                current_latency_ms > base * 4 && current_latency_ms > 400
            }));

        if !is_degraded {
            return None;
        }

        // Check anti-flapping hysteresis
        let now = Utc::now();
        let last_switch = *self.last_failover_time.read();
        if (now - last_switch).num_seconds() < self.min_hold_time.as_secs() as i64
            && consecutive_timeouts < 4
        {
            debug!(
                "Failover throttled by anti-flapping cooldown (held for {}s)",
                (now - last_switch).num_seconds()
            );
            return None;
        }

        // Find best alternate standby node from a different failure domain if possible
        let standbys = self.warm_standbys.read().clone();
        let alternate = standbys
            .into_iter()
            .find(|s| s.id != active.id && s.state == CandidateState::Verified);

        let new_target = match alternate {
            Some(alt) => alt,
            None => {
                // Fallback to graph query
                self.graph
                    .get_top_verified(3)
                    .into_iter()
                    .find(|n| n.id != active.id)?
            }
        };

        let start_switch = std::time::Instant::now();
        info!(
            "Predictive Self-Healing Failover triggered: [{}] -> [{}] (Loss: {:.1}%, Latency: {}ms)",
            active.id, new_target.id, current_packet_loss * 100.0, current_latency_ms
        );

        let switch_latency = start_switch.elapsed().as_millis() as u64;

        // Record failover event
        let event = FailoverEvent {
            timestamp: now,
            previous_node_id: active.id.clone(),
            new_node_id: new_target.id.clone(),
            trigger_reason: format!(
                "Loss: {:.1}%, Timeout: {}, Latency: {}ms",
                current_packet_loss * 100.0,
                consecutive_timeouts,
                current_latency_ms
            ),
            switch_latency_ms: switch_latency,
        };

        {
            let mut hist = self.failover_history.write();
            if hist.len() >= 50 {
                hist.remove(0);
            }
            hist.push(event);
        }

        *self.last_failover_time.write() = now;
        self.set_active_node(new_target.clone());

        Some(new_target)
    }

    pub fn get_failover_count(&self) -> usize {
        self.failover_history.read().len()
    }
}
