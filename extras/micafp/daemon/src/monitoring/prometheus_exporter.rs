// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Prometheus Metrics Exporter
//
// Exposes real-time VPN metrics in Prometheus text format at /metrics.
// Compatible with Grafana dashboards.
//
// Metrics exported:
//   shield_active_transport          (gauge)  — currently active transport
//   shield_bytes_sent_total          (counter)— total bytes sent through VPN
//   shield_bytes_recv_total          (counter)— total bytes received
//   shield_connection_latency_ms     (histogram)— RTT per transport
//   shield_dpi_detection_probability (gauge)  — AI DPI detection score 0-1
//   shield_core_switches_total       (counter)— total AI-triggered core switches
//   shield_transport_failures_total  (counter)— transport failures per type
//   shield_nain_status               (gauge)  — NAIN (national intranet) status
//   shield_p2p_peers_active          (gauge)  — active P2P relay peers
//   shield_battery_saver_active      (gauge)  — battery saver mode
// ─────────────────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use tokio::net::TcpListener;
use tracing::{error, info};

/// A single Prometheus metric label set.
type Labels = Vec<(String, String)>;

/// Prometheus metric types.
#[derive(Debug, Clone)]
pub enum MetricValue {
    Gauge(f64),
    Counter(f64),
    Histogram {
        buckets: Vec<(f64, u64)>,
        sum: f64,
        count: u64,
    },
}

/// A registered metric entry.
#[derive(Debug, Clone)]
struct Metric {
    help: String,
    metric_type: &'static str,
    values: Vec<(Labels, MetricValue)>,
}

/// Thread-safe Prometheus metrics registry.
#[derive(Debug, Clone)]
pub struct PrometheusExporter {
    registry: Arc<RwLock<HashMap<String, Metric>>>,
    start_time: Instant,
}

impl PrometheusExporter {
    /// Create a new exporter with pre-registered UnifiedShield metrics.
    pub fn new() -> Self {
        let exporter = Self {
            registry: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        };
        exporter.register_default_metrics();
        exporter
    }

    /// Register the default UnifiedShield metric set.
    fn register_default_metrics(&self) {
        let defaults: &[(&str, &str, &str)] = &[
            (
                "shield_active_transport",
                "gauge",
                "Currently active transport index (0=VLESS, 1=ShadowTLS, 2=Reality, ...)",
            ),
            (
                "shield_bytes_sent_total",
                "counter",
                "Total bytes sent through the VPN tunnel",
            ),
            (
                "shield_bytes_recv_total",
                "counter",
                "Total bytes received through the VPN tunnel",
            ),
            (
                "shield_connection_latency_ms",
                "histogram",
                "Round-trip latency per transport in milliseconds",
            ),
            (
                "shield_dpi_detection_probability",
                "gauge",
                "AI-estimated DPI detection probability (0.0 to 1.0)",
            ),
            (
                "shield_core_switches_total",
                "counter",
                "Total number of AI-triggered VPN core switches",
            ),
            (
                "shield_transport_failures_total",
                "counter",
                "Total transport connection failures by transport type",
            ),
            (
                "shield_nain_status",
                "gauge",
                "NAIN status: 0=normal, 1=partial, 2=full_shutdown",
            ),
            (
                "shield_p2p_peers_active",
                "gauge",
                "Number of currently active P2P relay peers",
            ),
            (
                "shield_battery_saver_active",
                "gauge",
                "Battery saver mode: 0=off, 1=on",
            ),
            (
                "shield_uptime_seconds",
                "counter",
                "Daemon uptime in seconds",
            ),
            (
                "shield_post_quantum_kex_total",
                "counter",
                "Total post-quantum key exchanges completed",
            ),
            (
                "shield_ble_mesh_peers",
                "gauge",
                "Active Bluetooth LE mesh peers",
            ),
            (
                "shield_ai_bandit_exploration_rate",
                "gauge",
                "UCB1 bandit current exploration rate",
            ),
        ];

        let mut reg = self.registry.write();
        for (name, typ, help) in defaults {
            reg.insert(
                name.to_string(),
                Metric {
                    help: help.to_string(),
                    metric_type: typ,
                    values: Vec::new(),
                },
            );
        }
    }

    /// Update a gauge metric.
    pub fn set_gauge(&self, name: &str, labels: Labels, value: f64) {
        let mut reg = self.registry.write();
        if let Some(metric) = reg.get_mut(name) {
            if let Some(entry) = metric.values.iter_mut().find(|(l, _)| l == &labels) {
                entry.1 = MetricValue::Gauge(value);
            } else {
                metric.values.push((labels, MetricValue::Gauge(value)));
            }
        }
    }

    /// Increment a counter metric.
    pub fn inc_counter(&self, name: &str, labels: Labels, delta: f64) {
        let mut reg = self.registry.write();
        if let Some(metric) = reg.get_mut(name) {
            if let Some(entry) = metric.values.iter_mut().find(|(l, _)| l == &labels) {
                if let MetricValue::Counter(ref mut v) = entry.1 {
                    *v += delta;
                }
            } else {
                metric.values.push((labels, MetricValue::Counter(delta)));
            }
        }
    }

    /// Observe a histogram value.
    pub fn observe_histogram(&self, name: &str, labels: Labels, value_ms: f64, buckets: &[f64]) {
        let mut reg = self.registry.write();
        if let Some(metric) = reg.get_mut(name) {
            let bucket_counts: Vec<(f64, u64)> = buckets
                .iter()
                .map(|&b| (b, if value_ms <= b { 1 } else { 0 }))
                .collect();

            if let Some(entry) = metric.values.iter_mut().find(|(l, _)| l == &labels) {
                if let MetricValue::Histogram {
                    ref mut buckets,
                    ref mut sum,
                    ref mut count,
                } = entry.1
                {
                    for (i, (_, c)) in bucket_counts.iter().enumerate() {
                        buckets[i].1 += c;
                    }
                    *sum += value_ms;
                    *count += 1;
                }
            } else {
                metric.values.push((
                    labels,
                    MetricValue::Histogram {
                        buckets: bucket_counts,
                        sum: value_ms,
                        count: 1,
                    },
                ));
            }
        }
    }

    /// Render all metrics in Prometheus text format.
    pub fn render(&self) -> String {
        let reg = self.registry.read();
        let mut output = String::with_capacity(4096);

        // Add uptime
        let uptime = self.start_time.elapsed().as_secs_f64();

        for (name, metric) in reg.iter() {
            output.push_str(&format!("# HELP {} {}\n", name, metric.help));
            output.push_str(&format!("# TYPE {} {}\n", name, metric.metric_type));

            // Special case: uptime
            if name == "shield_uptime_seconds" {
                output.push_str(&format!("{} {:.3}\n", name, uptime));
                continue;
            }

            for (labels, value) in &metric.values {
                let label_str = if labels.is_empty() {
                    String::new()
                } else {
                    let pairs: Vec<String> = labels
                        .iter()
                        .map(|(k, v)| format!("{}=\"{}\"", k, v))
                        .collect();
                    format!("{{{}}}", pairs.join(","))
                };

                match value {
                    MetricValue::Gauge(v) | MetricValue::Counter(v) => {
                        output.push_str(&format!("{}{} {:.6}\n", name, label_str, v));
                    }
                    MetricValue::Histogram {
                        buckets,
                        sum,
                        count,
                    } => {
                        for (bound, count_le) in buckets {
                            output.push_str(&format!(
                                "{}_bucket{}{{le=\"{:.1}\"}} {}\n",
                                name,
                                label_str.trim_end_matches('}').trim_start_matches('{'),
                                bound,
                                count_le
                            ));
                        }
                        output.push_str(&format!(
                            "{}_bucket{}{{le=\"+Inf\"}} {}\n",
                            name, label_str, count
                        ));
                        output.push_str(&format!("{}_sum{} {:.3}\n", name, label_str, sum));
                        output.push_str(&format!("{}_count{} {}\n", name, label_str, count));
                    }
                }
            }
        }
        output
    }

    /// Start the HTTP /metrics server.
    pub async fn serve(self, addr: std::net::SocketAddr) {
        let listener = match TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                error!("prometheus: failed to bind {}: {}", addr, e);
                return;
            }
        };
        info!(
            "prometheus: metrics server listening on http://{}/metrics",
            addr
        );

        loop {
            match listener.accept().await {
                Ok((mut stream, peer)) => {
                    let body = self.render();
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(), body
                    );
                    use tokio::io::AsyncWriteExt;
                    let _ = stream.write_all(response.as_bytes()).await;
                }
                Err(e) => {
                    error!("prometheus: accept error: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}

impl Default for PrometheusExporter {
    fn default() -> Self {
        Self::new()
    }
}
