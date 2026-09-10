// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Path Scheduler Subsystem (Directive v70)
//
// Pluggable scheduler trait and implementations:
//   1. AdaptiveMultipathScheduler: Multi-path dynamic load balancing for Mode A (Fast)
//   2. FixedHopChainScheduler: Fixed-sequence 5-hop chain dispatch for Mode B (Layered)
// ─────────────────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Live metrics for a single transport path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathMetrics {
    pub path_id: usize,
    pub remote_addr: SocketAddr,
    pub rtt_ms: f64,
    pub rtt_variance_ms: f64,
    pub jitter_ms: f64,
    pub packet_loss_rate: f64, // 0.0 to 1.0
    pub available_bandwidth_kbps: u64,
    pub in_flight_bytes: usize,
    pub reorder_rate: f64, // 0.0 to 1.0
    pub congestion_state: CongestionState,
    pub pmtu: u16,
    pub battery_cost_weight: f64, // 0.0 (efficient) to 1.0 (heavy)
    pub consecutive_failures: u32,
    pub is_alive: bool,
    #[serde(skip, default = "Instant::now")]
    pub last_updated: Instant,
}

impl Default for PathMetrics {
    fn default() -> Self {
        Self {
            path_id: 0,
            remote_addr: "127.0.0.1:443".parse().unwrap(),
            rtt_ms: 25.0,
            rtt_variance_ms: 2.0,
            jitter_ms: 3.0,
            packet_loss_rate: 0.0,
            available_bandwidth_kbps: 50_000,
            in_flight_bytes: 0,
            reorder_rate: 0.0,
            congestion_state: CongestionState::Open,
            pmtu: 1420,
            battery_cost_weight: 0.2,
            consecutive_failures: 0,
            is_alive: true,
            last_updated: Instant::now(),
        }
    }
}

/// Congestion state indicator per path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CongestionState {
    Open,
    Disorder,
    Recovery,
    Loss,
    CongestionWindowLimited,
}

/// Congestion control algorithm selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CongestionAlgorithm {
    Bbr,
    Cubic,
}

/// Dynamic path score used for scheduling decisions. Higher score = preferred.
#[derive(Debug, Clone, Copy)]
pub struct PathScore {
    pub path_id: usize,
    pub composite_score: f64,
    pub latency_penalty: f64,
    pub loss_penalty: f64,
    pub bandwidth_bonus: f64,
    pub battery_penalty: f64,
}

/// Pluggable trait for path scheduling and dispatching packets across network paths.
pub trait PathScheduler: Send + Sync {
    /// Calculate composite quality score for a path.
    fn score_path(&self, metrics: &PathMetrics) -> PathScore;

    /// Select the optimal path ID for an outgoing packet or stream.
    fn schedule_packet(&self, packet_len: usize, flow_hint: Option<u64>) -> Option<usize>;

    /// Update runtime metrics for a specific path.
    fn update_metrics(&self, path_id: usize, updater: Box<dyn FnOnce(&mut PathMetrics) + Send>);

    /// Trigger periodic rebalancing across active paths.
    fn rebalance(&self);

    /// Record a packet transmission event on a path.
    fn on_packet_sent(&self, path_id: usize, bytes: usize);

    /// Record a packet acknowledgment or delivery event.
    fn on_packet_ack(&self, path_id: usize, rtt_sample: Duration);

    /// Record a packet loss event on a path.
    fn on_packet_loss(&self, path_id: usize);

    /// Get current snapshot of all path metrics.
    fn get_all_metrics(&self) -> Vec<PathMetrics>;
}

/// Mode A: Adaptive Multipath Scheduler.
/// Scores paths in real-time based on RTT, loss, jitter, bandwidth, and battery cost.
pub struct AdaptiveMultipathScheduler {
    paths: RwLock<HashMap<usize, PathMetrics>>,
    path_scores: RwLock<HashMap<usize, PathScore>>,
    max_paths: usize,
    round_robin_counter: AtomicUsize,
    total_packets_scheduled: AtomicU64,
    total_rebalances: AtomicU64,
}

impl AdaptiveMultipathScheduler {
    pub fn new(max_paths: usize) -> Self {
        Self {
            paths: RwLock::new(HashMap::new()),
            path_scores: RwLock::new(HashMap::new()),
            max_paths: max_paths.clamp(1, 5),
            round_robin_counter: AtomicUsize::new(0),
            total_packets_scheduled: AtomicU64::new(0),
            total_rebalances: AtomicU64::new(0),
        }
    }

    /// Register a path with initial metrics.
    pub fn register_path(&self, metrics: PathMetrics) {
        let path_id = metrics.path_id;
        let score = self.score_path(&metrics);
        self.paths.write().insert(path_id, metrics);
        self.path_scores.write().insert(path_id, score);
    }

    /// Mark a path as down or failed.
    pub fn mark_path_failed(&self, path_id: usize) {
        if let Some(metrics) = self.paths.write().get_mut(&path_id) {
            metrics.is_alive = false;
            metrics.consecutive_failures += 1;
        }
        self.rebalance();
    }
}

impl PathScheduler for AdaptiveMultipathScheduler {
    fn score_path(&self, metrics: &PathMetrics) -> PathScore {
        if !metrics.is_alive || metrics.consecutive_failures >= 3 {
            return PathScore {
                path_id: metrics.path_id,
                composite_score: 0.0,
                latency_penalty: 1000.0,
                loss_penalty: 1000.0,
                bandwidth_bonus: 0.0,
                battery_penalty: 100.0,
            };
        }

        // Latency penalty: normalize RTT + 2 * jitter
        let effective_rtt = metrics.rtt_ms + (metrics.jitter_ms * 2.0);
        let latency_penalty = (effective_rtt / 20.0).clamp(0.1, 50.0);

        // Loss penalty: exponential penalty for packet loss above 0.5%
        let loss_penalty = if metrics.packet_loss_rate > 0.005 {
            metrics.packet_loss_rate * 200.0
        } else {
            metrics.packet_loss_rate * 20.0
        };

        // Bandwidth bonus: higher available throughput increases score
        let bandwidth_mbps = (metrics.available_bandwidth_kbps as f64) / 1000.0;
        let bandwidth_bonus = (bandwidth_mbps.ln_1p() * 10.0).clamp(0.0, 50.0);

        // Congestion state penalty
        let congestion_penalty = match metrics.congestion_state {
            CongestionState::Open => 0.0,
            CongestionState::Disorder => 5.0,
            CongestionState::Recovery => 15.0,
            CongestionState::Loss => 30.0,
            CongestionState::CongestionWindowLimited => 10.0,
        };

        // Battery penalty
        let battery_penalty = metrics.battery_cost_weight * 5.0;

        // Composite score calculation (higher is better)
        let raw_score = 100.0 + bandwidth_bonus
            - (latency_penalty + loss_penalty + congestion_penalty + battery_penalty);
        let composite_score = raw_score.max(1.0);

        PathScore {
            path_id: metrics.path_id,
            composite_score,
            latency_penalty,
            loss_penalty,
            bandwidth_bonus,
            battery_penalty,
        }
    }

    fn schedule_packet(&self, _packet_len: usize, flow_hint: Option<u64>) -> Option<usize> {
        let scores = self.path_scores.read();
        let active_candidates: Vec<(&usize, &PathScore)> = scores
            .iter()
            .filter(|(_, s)| s.composite_score > 1.0)
            .collect();

        if active_candidates.is_empty() {
            return None;
        }

        self.total_packets_scheduled.fetch_add(1, Ordering::Relaxed);

        // If flow_hint is provided, maintain flow affinity by hashing across top-tier paths
        if let Some(hint) = flow_hint {
            let mut sorted = active_candidates.clone();
            sorted.sort_by(|a, b| {
                b.1.composite_score
                    .partial_cmp(&a.1.composite_score)
                    .unwrap()
            });
            let top_k = sorted.len().min(3);
            let idx = (hint as usize) % top_k;
            return Some(*sorted[idx].0);
        }

        // Weighted probabilistic selection proportional to composite_score
        let total_score: f64 = active_candidates
            .iter()
            .map(|(_, s)| s.composite_score)
            .sum();
        if total_score <= 0.0 {
            return Some(*active_candidates[0].0);
        }

        let ticket = (self.round_robin_counter.fetch_add(1, Ordering::Relaxed) % 1000) as f64
            / 1000.0
            * total_score;
        let mut accum = 0.0;
        for (path_id, score) in &active_candidates {
            accum += score.composite_score;
            if ticket <= accum {
                return Some(**path_id);
            }
        }

        Some(*active_candidates.last().unwrap().0)
    }

    fn update_metrics(&self, path_id: usize, updater: Box<dyn FnOnce(&mut PathMetrics) + Send>) {
        if let Some(metrics) = self.paths.write().get_mut(&path_id) {
            updater(metrics);
            metrics.last_updated = Instant::now();
            let score = self.score_path(metrics);
            self.path_scores.write().insert(path_id, score);
        }
    }

    fn rebalance(&self) {
        let paths = self.paths.read();
        let mut new_scores = HashMap::new();
        for (id, metrics) in paths.iter() {
            new_scores.insert(*id, self.score_path(metrics));
        }
        drop(paths);
        *self.path_scores.write() = new_scores;
        self.total_rebalances.fetch_add(1, Ordering::Relaxed);
    }

    fn on_packet_sent(&self, path_id: usize, bytes: usize) {
        if let Some(metrics) = self.paths.write().get_mut(&path_id) {
            metrics.in_flight_bytes += bytes;
        }
    }

    fn on_packet_ack(&self, path_id: usize, rtt_sample: Duration) {
        let sample_ms = rtt_sample.as_secs_f64() * 1000.0;
        self.update_metrics(
            path_id,
            Box::new(move |m| {
                // EWMA update for RTT and RTT variance
                let alpha = 0.125;
                let beta = 0.25;
                let diff = (sample_ms - m.rtt_ms).abs();
                m.rtt_ms = (1.0 - alpha) * m.rtt_ms + alpha * sample_ms;
                m.rtt_variance_ms = (1.0 - beta) * m.rtt_variance_ms + beta * diff;
                m.jitter_ms = m.rtt_variance_ms;
                m.packet_loss_rate = (m.packet_loss_rate * 0.95).max(0.0);
                m.consecutive_failures = 0;
                m.is_alive = true;
            }),
        );
    }

    fn on_packet_loss(&self, path_id: usize) {
        self.update_metrics(
            path_id,
            Box::new(|m| {
                m.packet_loss_rate = (m.packet_loss_rate * 0.9 + 0.1).min(1.0);
                m.congestion_state = CongestionState::Loss;
            }),
        );
    }

    fn get_all_metrics(&self) -> Vec<PathMetrics> {
        self.paths.read().values().cloned().collect()
    }
}

/// Mode B: Fixed Hop Chain Scheduler.
/// Dispatches packets through a fixed 5-hop onion chain and enforces fail-closed semantics.
pub struct FixedHopChainScheduler {
    hop_chain: RwLock<Vec<PathMetrics>>,
    fail_closed: bool,
    chain_healthy: RwLock<bool>,
}

impl FixedHopChainScheduler {
    pub fn new(hops: Vec<PathMetrics>, fail_closed: bool) -> Self {
        assert_eq!(hops.len(), 5, "Mode B requires exactly 5 hops in chain");
        Self {
            hop_chain: RwLock::new(hops),
            fail_closed,
            chain_healthy: RwLock::new(true),
        }
    }

    pub fn is_chain_healthy(&self) -> bool {
        *self.chain_healthy.read()
    }
}

impl PathScheduler for FixedHopChainScheduler {
    fn score_path(&self, metrics: &PathMetrics) -> PathScore {
        let is_ok = metrics.is_alive && metrics.consecutive_failures == 0;
        PathScore {
            path_id: metrics.path_id,
            composite_score: if is_ok { 100.0 } else { 0.0 },
            latency_penalty: metrics.rtt_ms,
            loss_penalty: metrics.packet_loss_rate * 100.0,
            bandwidth_bonus: 0.0,
            battery_penalty: 0.0,
        }
    }

    fn schedule_packet(&self, _packet_len: usize, _flow_hint: Option<u64>) -> Option<usize> {
        let chain = self.hop_chain.read();
        // In Mode B, all 5 hops must be alive. If any hop fails, fail-closed unless configured
        for hop in chain.iter() {
            if !hop.is_alive || hop.consecutive_failures > 0 {
                if self.fail_closed {
                    *self.chain_healthy.write() = false;
                    return None; // Drop packet - fail closed!
                }
            }
        }
        *self.chain_healthy.write() = true;
        // Always dispatch to ingress Hop 0
        Some(0)
    }

    fn update_metrics(&self, hop_index: usize, updater: Box<dyn FnOnce(&mut PathMetrics) + Send>) {
        let mut chain = self.hop_chain.write();
        if let Some(hop) = chain.get_mut(hop_index) {
            updater(hop);
            hop.last_updated = Instant::now();
        }
    }

    fn rebalance(&self) {
        let chain = self.hop_chain.read();
        let all_healthy = chain
            .iter()
            .all(|h| h.is_alive && h.consecutive_failures == 0);
        *self.chain_healthy.write() = all_healthy;
    }

    fn on_packet_sent(&self, hop_index: usize, bytes: usize) {
        let mut chain = self.hop_chain.write();
        if let Some(hop) = chain.get_mut(hop_index) {
            hop.in_flight_bytes += bytes;
        }
    }

    fn on_packet_ack(&self, hop_index: usize, rtt_sample: Duration) {
        self.update_metrics(
            hop_index,
            Box::new(move |m| {
                m.rtt_ms = rtt_sample.as_secs_f64() * 1000.0;
                m.consecutive_failures = 0;
                m.is_alive = true;
            }),
        );
    }

    fn on_packet_loss(&self, hop_index: usize) {
        self.update_metrics(
            hop_index,
            Box::new(|m| {
                m.consecutive_failures += 1;
                m.packet_loss_rate = 1.0;
                m.is_alive = false;
            }),
        );
        self.rebalance();
    }

    fn get_all_metrics(&self) -> Vec<PathMetrics> {
        self.hop_chain.read().clone()
    }
}
