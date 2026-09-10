use std::collections::HashMap;

pub struct RelaySelection {
    relay_scores: HashMap<String, f64>,
}

impl RelaySelection {
    pub fn new() -> Self {
        Self {
            relay_scores: HashMap::new(),
        }
    }

    pub fn score_relay(
        &mut self,
        id: &str,
        latency_ms: f64,
        uptime_pct: f64,
        bandwidth_kbps: f64,
    ) -> f64 {
        let score = (1000.0 / latency_ms.max(1.0)) * 0.4
            + uptime_pct * 0.3
            + (bandwidth_kbps / 1000.0) * 0.3;
        self.relay_scores.insert(id.to_string(), score);
        score
    }

    pub fn select_best_relay(&self) -> Option<String> {
        self.relay_scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(k, _)| k.clone())
    }
}
