use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::Mutex;
use tracing::{debug, info};

use super::{CensorshipEvent, TelemetryReport};

pub struct TelemetryAggregator {
    events: Arc<Mutex<Vec<CensorshipEvent>>>,
    period_start: Arc<Mutex<u64>>,
    epsilon: f64,
}

impl TelemetryAggregator {
    pub fn new(epsilon: f64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            period_start: Arc::new(Mutex::new(now)),
            epsilon,
        }
    }

    pub async fn record(&self, event: CensorshipEvent) {
        let mut events = self.events.lock().await;
        events.push(event);
        debug!("Telemetry event recorded. Total: {}", events.len());
    }

    pub async fn flush_report(&self) -> Option<TelemetryReport> {
        let mut events = self.events.lock().await;
        if events.is_empty() {
            return None;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let period_start = *self.period_start.lock().await;

        let mut counts: HashMap<String, u64> = HashMap::new();
        let mut total_rtt: u64 = 0;
        let mut successes: u64 = 0;

        for evt in events.iter() {
            *counts.entry(format!("{:?}", evt.event_type)).or_insert(0) += 1;
            total_rtt += evt.rtt_ms as u64;
            if evt.bypass_succeeded {
                successes += 1;
            }
        }

        let n = events.len() as f64;
        let noise_scale = 1.0_f64 / self.epsilon;

        let noisy_counts: HashMap<String, f64> = counts
            .into_iter()
            .map(|(k, v)| (k, (v as f64 + laplace_noise(noise_scale)).max(0.0)))
            .collect();

        let report = TelemetryReport {
            report_id: random_hex(16),
            period_start,
            period_end: now,
            event_counts: noisy_counts,
            bypass_success_rate: successes as f64 / n,
            avg_rtt_ms: total_rtt as f64 / n,
            epsilon: self.epsilon,
        };

        events.clear();
        *self.period_start.lock().await = now;
        info!(report_id = %report.report_id, events = n as u64, "Telemetry report generated");
        Some(report)
    }
}

fn laplace_noise(scale: f64) -> f64 {
    use rand::Rng;
    let u: f64 = rand::thread_rng().gen::<f64>() - 0.5;
    -scale * u.signum() * (1.0 - 2.0 * u.abs()).ln()
}

fn random_hex(bytes: usize) -> String {
    use rand::Rng;
    let b: Vec<u8> = (0..bytes).map(|_| rand::thread_rng().gen()).collect();
    b.iter().map(|x| format!("{x:02x}")).collect()
}
