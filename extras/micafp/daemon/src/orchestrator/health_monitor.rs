// Health Monitor — probes transports and reports scores.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::net::TcpStream;
use tracing::{debug, warn};

/// Score for a single transport probe.
#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub transport: String,
    pub latency_ms: u64,
    pub packet_loss: f64,
    pub success: bool,
}

pub struct HealthMonitor {
    probe_timeout: Duration,
    history: HashMap<String, Vec<ProbeResult>>,
}

impl HealthMonitor {
    pub fn new(probe_timeout: Duration) -> Self {
        Self {
            probe_timeout,
            history: HashMap::new(),
        }
    }

    /// Probe a TCP endpoint and return a result.
    pub async fn probe(&mut self, transport: &str, addr: &str) -> ProbeResult {
        let start = Instant::now();
        let success = tokio::time::timeout(self.probe_timeout, TcpStream::connect(addr))
            .await
            .map(|r| r.is_ok())
            .unwrap_or(false);

        let latency_ms = start.elapsed().as_millis() as u64;

        let result = ProbeResult {
            transport: transport.to_string(),
            latency_ms,
            packet_loss: if success { 0.0 } else { 1.0 },
            success,
        };

        self.history
            .entry(transport.to_string())
            .or_default()
            .push(result.clone());

        if !success {
            warn!(transport, addr, "Probe failed");
        } else {
            debug!(transport, latency_ms, "Probe OK");
        }

        result
    }

    /// Compute a health score (0.0–1.0) from recent probe history.
    pub fn health_score(&self, transport: &str) -> f64 {
        let history = match self.history.get(transport) {
            Some(h) => h,
            None => return 0.5, // Unknown — neutral score
        };

        let recent: Vec<&ProbeResult> = history.iter().rev().take(10).collect();
        if recent.is_empty() {
            return 0.5;
        }

        let success_rate = recent.iter().filter(|r| r.success).count() as f64 / recent.len() as f64;
        let avg_latency = recent
            .iter()
            .filter(|r| r.success)
            .map(|r| r.latency_ms as f64)
            .sum::<f64>()
            / recent.len().max(1) as f64;

        let latency_score = (1.0 - (avg_latency / 2000.0).min(1.0)).max(0.0);
        success_rate * 0.7 + latency_score * 0.3
    }
}
