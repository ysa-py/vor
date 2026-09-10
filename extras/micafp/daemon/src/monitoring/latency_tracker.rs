// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Latency Tracker
// Per-transport P50/P95/P99 latency tracking with rolling window.
// ─────────────────────────────────────────────────────────────────────────────

use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

const WINDOW_SIZE: usize = 1000;

/// Tracks RTT samples per transport and computes percentile latencies.
pub struct LatencyTracker {
    samples: Mutex<HashMap<String, VecDeque<f64>>>,
}

impl LatencyTracker {
    pub fn new() -> Self {
        Self {
            samples: Mutex::new(HashMap::new()),
        }
    }

    /// Record a new RTT sample (milliseconds) for a transport.
    pub fn record(&self, transport: &str, rtt_ms: f64) {
        let mut map = self.samples.lock();
        let deque = map
            .entry(transport.to_string())
            .or_insert_with(VecDeque::new);
        if deque.len() >= WINDOW_SIZE {
            deque.pop_front();
        }
        deque.push_back(rtt_ms);
    }

    /// Compute P50, P95, P99 for a transport. Returns None if no data.
    pub fn percentiles(&self, transport: &str) -> Option<(f64, f64, f64)> {
        let map = self.samples.lock();
        let deque = map.get(transport)?;
        if deque.is_empty() {
            return None;
        }

        let mut sorted: Vec<f64> = deque.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = sorted.len();

        let p50 = sorted[n / 2];
        let p95 = sorted[(n as f64 * 0.95) as usize];
        let p99 = sorted[(n as f64 * 0.99) as usize];

        Some((p50, p95, p99))
    }

    /// Get the best transport by P50 latency.
    pub fn best_transport(&self) -> Option<String> {
        let map = self.samples.lock();
        map.keys()
            .filter_map(|t| {
                let deque = map.get(t)?;
                if deque.is_empty() {
                    return None;
                }
                let avg: f64 = deque.iter().sum::<f64>() / deque.len() as f64;
                Some((t.clone(), avg))
            })
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(t, _)| t)
    }
}

impl Default for LatencyTracker {
    fn default() -> Self {
        Self::new()
    }
}
