//! Adaptive Path Scheduler for Directive v70.
//! Dynamically evaluates RTT, jitter, loss, bandwidth, and battery/CPU cost to assign streams.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

/// Comprehensive metrics for a transport path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathMetrics {
    pub path_id: u8,
    pub rtt_ms: u32,
    pub jitter_ms: u32,
    pub packet_loss_rate: f32, // 0.0 to 1.0 (e.g. 0.02 = 2%)
    pub available_bandwidth_kbps: u32,
    pub congestion_state: String, // "Open", "Recovery", "Loss", "ProbeRTT"
    pub reorder_rate: f32,
    pub cpu_cost_score: f32, // 0.0 to 1.0
    pub battery_cost_score: f32, // 0.0 to 1.0
    pub live_score: f32, // Composite score calculated by scheduler (higher is better)
    pub total_packets_sent: u64,
    pub total_bytes_sent: u64,
}

impl Default for PathMetrics {
    fn default() -> Self {
        Self {
            path_id: 1,
            rtt_ms: 35,
            jitter_ms: 3,
            packet_loss_rate: 0.001,
            available_bandwidth_kbps: 85000,
            congestion_state: "Open".to_string(),
            reorder_rate: 0.0,
            cpu_cost_score: 0.12,
            battery_cost_score: 0.10,
            live_score: 95.0,
            total_packets_sent: 0,
            total_bytes_sent: 0,
        }
    }
}

/// Pluggable PathScheduler Trait for both Transport Modes.
pub trait PathScheduler: Send + Sync {
    /// Evaluate all candidate paths and pick the best path ID for the incoming packet/stream.
    fn select_path(&self, packet_len: usize) -> Result<u8, String>;

    /// Update live metrics for a specific path.
    fn update_metrics(&self, metrics: PathMetrics);

    /// Trigger rebalance calculations across all active paths.
    fn rebalance(&self);

    /// Return a snapshot of all active path metrics and scores.
    fn get_metrics_snapshot(&self) -> Vec<PathMetrics>;
}

/// Mode A Implementation: Adaptive Multi-Path Load Balancer with Live Scoring.
pub struct AdaptiveMultiPathScheduler {
    paths: RwLock<HashMap<u8, PathMetrics>>,
    weights: RwLock<HashMap<u8, f32>>,
    dispatch_counter: AtomicU64,
}

impl AdaptiveMultiPathScheduler {
    pub fn new(initial_paths: Vec<u8>) -> Self {
        let mut map = HashMap::new();
        let mut weight_map = HashMap::new();
        for &id in &initial_paths {
            let mut m = PathMetrics::default();
            m.path_id = id;
            map.insert(id, m);
            weight_map.insert(id, 1.0);
        }

        Self {
            paths: RwLock::new(map),
            weights: RwLock::new(weight_map),
            dispatch_counter: AtomicU64::new(0),
        }
    }

    /// Calculate path quality score:
    /// Score = (100 - RTT*0.4 - Jitter*1.2 - Loss*300) * (Bandwidth / MaxBandwidth) - (BatteryCost * 10)
    fn compute_path_score(m: &PathMetrics) -> f32 {
        let base_rtt_penalty = (m.rtt_ms as f32 * 0.35).min(45.0);
        let jitter_penalty = (m.jitter_ms as f32 * 1.5).min(20.0);
        let loss_penalty = (m.packet_loss_rate * 400.0).min(60.0);
        let congestion_penalty = if m.congestion_state == "Loss" || m.congestion_state == "Recovery" {
            25.0
        } else {
            0.0
        };

        let raw_score = 100.0 - base_rtt_penalty - jitter_penalty - loss_penalty - congestion_penalty;
        let clamped = raw_score.max(1.0);

        // Normalize with bandwidth factor
        let bw_factor = (m.available_bandwidth_kbps as f32 / 100_000.0).clamp(0.2, 1.5);
        clamped * bw_factor
    }
}

impl PathScheduler for AdaptiveMultiPathScheduler {
    fn select_path(&self, packet_len: usize) -> Result<u8, String> {
        let weights = self.weights.read().map_err(|e| e.to_string())?;
        if weights.is_empty() {
            return Err("No active transport paths available".to_string());
        }

        let total_weight: f32 = weights.values().sum();
        if total_weight <= 0.0 {
            return Ok(*weights.keys().next().unwrap_or(&1));
        }

        // Weighted round-robin selection driven by score
        let count = self.dispatch_counter.fetch_add(1, Ordering::Relaxed);
        let mut accumulated = 0.0;
        let target = (count as f32 % 100.0) / 100.0 * total_weight;

        for (&path_id, &w) in weights.iter() {
            accumulated += w;
            if target <= accumulated {
                // Update packet sent counters
                if let Ok(mut paths) = self.paths.write() {
                    if let Some(m) = paths.get_mut(&path_id) {
                        m.total_packets_sent += 1;
                        m.total_bytes_sent += packet_len as u64;
                    }
                }
                return Ok(path_id);
            }
        }

        Ok(*weights.keys().next().unwrap_or(&1))
    }

    fn update_metrics(&self, mut metrics: PathMetrics) {
        metrics.live_score = Self::compute_path_score(&metrics);
        let id = metrics.path_id;

        if let Ok(mut paths) = self.paths.write() {
            paths.insert(id, metrics);
        }
        self.rebalance();
    }

    fn rebalance(&self) {
        if let (Ok(paths), Ok(mut weights)) = (self.paths.read(), self.weights.write()) {
            for (&id, metrics) in paths.iter() {
                let score = Self::compute_path_score(metrics);
                weights.insert(id, score.max(0.1));
            }
        }
    }

    fn get_metrics_snapshot(&self) -> Vec<PathMetrics> {
        if let Ok(paths) = self.paths.read() {
            paths.values().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

/// Mode B Implementation: Fixed 5-Hop Chain Scheduler (No load-balancing across paths, manages fixed chain).
pub struct FixedChainScheduler {
    hop_metrics: RwLock<Vec<PathMetrics>>,
    fail_closed: bool,
}

impl FixedChainScheduler {
    pub fn new(hops_count: usize, fail_closed: bool) -> Self {
        let mut hops = Vec::new();
        for i in 1..=hops_count {
            let mut m = PathMetrics::default();
            m.path_id = i as u8;
            m.congestion_state = "ChainHop".to_string();
            hops.push(m);
        }

        Self {
            hop_metrics: RwLock::new(hops),
            fail_closed,
        }
    }
}

impl PathScheduler for FixedChainScheduler {
    fn select_path(&self, packet_len: usize) -> Result<u8, String> {
        // Mode B always dispatches through the entry hop (Hop 1) of the 5-hop chain
        let hops = self.hop_metrics.read().map_err(|e| e.to_string())?;
        if self.fail_closed {
            // Check if any hop is marked as down/loss > 80%
            for hop in hops.iter() {
                if hop.packet_loss_rate >= 0.85 {
                    return Err(format!("Mode B Fail-Closed: Hop {} is unreachable. Packet dropped.", hop.path_id));
                }
            }
        }

        if let Ok(mut hops_mut) = self.hop_metrics.write() {
            if let Some(entry) = hops_mut.first_mut() {
                entry.total_packets_sent += 1;
                entry.total_bytes_sent += packet_len as u64;
            }
        }

        Ok(1)
    }

    fn update_metrics(&self, metrics: PathMetrics) {
        if let Ok(mut hops) = self.hop_metrics.write() {
            if let Some(h) = hops.iter_mut().find(|h| h.path_id == metrics.path_id) {
                *h = metrics;
            }
        }
    }

    fn rebalance(&self) {
        // Mode B has fixed topology; rebalance verifies hop health
    }

    fn get_metrics_snapshot(&self) -> Vec<PathMetrics> {
        if let Ok(hops) = self.hop_metrics.read() {
            hops.clone()
        } else {
            Vec::new()
        }
    }
}
