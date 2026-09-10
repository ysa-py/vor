// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — NAIN Auto-Detection
//
// Automatically detects when Iran's National Intranet (NAIN) is activated
// by probing international and domestic endpoints. Uses adaptive intervals
// based on battery PowerMode to minimise power consumption.
//
// Detection strategy:
//   • Probe international: 8.8.8.8:53, 1.1.1.1:53, 9.9.9.9:53
//     (TCP connect + DNS query over TCP)
//   • Probe domestic: Arvan CDN IPs, sntp.ir:123, shaparak.ir:443
//   • if domestic_ok && !international_ok => NainOnly
//   • if !domestic_ok && !international_ok => CompleteBlackout
//   • else => FullInternet
//
// Battery optimization:
//   • PowerMode intervals: Performance=30s, Normal=60s, Save=120s, Critical=300s
//   • Screen-off multiplier: 2x interval
//   • Coalesces with NTP covert channel probes to minimise CPU wakeups
// ─────────────────────────────────────────────────────────────────────────────

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::{NainState, NainStatus};
use crate::battery::adaptive_duty::PowerMode;
use crate::error::{ErrorCode, ShieldError};

// ── Probe Configuration ─────────────────────────────────────────────────────

/// International endpoints to probe. These are well-known DNS resolvers
/// that should always be reachable under normal internet conditions.
const INTERNATIONAL_PROBES: &[&str] = &[
    "8.8.8.8:53", // Google DNS
    "1.1.1.1:53", // Cloudflare DNS
    "9.9.9.9:53", // Quad9 DNS
];

/// Domestic (Iranian) endpoints to probe. These should be reachable
/// even under NAIN mode but NOT during a complete blackout.
const DOMESTIC_PROBES: &[&str] = &[
    "185.143.234.42:443",  // Arvan Cloud (domestic CDN)
    "5.200.200.200:443",   // Shaparak payment gateway
    "217.218.155.155:443", // sntp.ir / domestic NTP
];

/// Timeout for each individual TCP connection probe.
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for DNS query over established TCP connection.
const DNS_QUERY_TIMEOUT: Duration = Duration::from_secs(3);

/// DNS query for "." root (simplest valid DNS query).
const DNS_ROOT_QUERY: &[u8] = &[
    0x00, 0x0D, // Length prefix: 13 bytes
    0x12, 0x34, // Transaction ID
    0x01, 0x00, // Flags: standard query
    0x00, 0x01, // Questions: 1
    0x00, 0x00, // Answer RRs: 0
    0x00, 0x00, // Authority RRs: 0
    0x00, 0x00, // Additional RRs: 0
    0x00, // Root label
    0x00, 0x01, // Type: A
    0x00, 0x01, // Class: IN
];

// ── PowerMode-based interval table ──────────────────────────────────────────

/// Base detection interval per PowerMode, matching battery adaptive_duty.rs.
const INTERVAL_PERFORMANCE: Duration = Duration::from_secs(30);
const INTERVAL_NORMAL: Duration = Duration::from_secs(60);
const INTERVAL_SAVE: Duration = Duration::from_secs(120);
const INTERVAL_CRITICAL: Duration = Duration::from_secs(300);

/// Screen-off multiplier (2x per battery subsystem spec).
const SCREEN_OFF_MULTIPLIER: f64 = 2.0;

// ── Detection Result ────────────────────────────────────────────────────────

/// Result of a single NAIN detection cycle.
#[derive(Debug, Clone)]
pub struct DetectionResult {
    /// The determined NAIN network status.
    pub status: NainStatus,
    /// Number of international probes that succeeded.
    pub international_ok_count: u8,
    /// Total number of international probes attempted.
    pub international_total: u8,
    /// Number of domestic probes that succeeded.
    pub domestic_ok_count: u8,
    /// Total number of domestic probes attempted.
    pub domestic_total: u8,
    /// Round-trip latency of the fastest successful international probe.
    pub international_latency: Option<Duration>,
    /// Round-trip latency of the fastest successful domestic probe.
    pub domestic_latency: Option<Duration>,
    /// Timestamp of this detection cycle.
    pub timestamp: Instant,
    /// Whether NTP covert channel probes were coalesced.
    pub ntp_coalesced: bool,
}

impl DetectionResult {
    /// Returns true if international connectivity is confirmed.
    pub fn international_ok(&self) -> bool {
        self.international_ok_count > 0
    }

    /// Returns true if domestic connectivity is confirmed.
    pub fn domestic_ok(&self) -> bool {
        self.domestic_ok_count > 0
    }
}

// ── NAIN Detector ───────────────────────────────────────────────────────────

/// Probes international and domestic endpoints to detect NAIN activation.
///
/// The detector uses TCP connection attempts and simple DNS queries to
/// determine which parts of the internet are currently reachable. It adapts
/// its probe interval based on battery PowerMode and screen state.
///
/// Probes are coalesced with NTP covert channel probes to minimise
/// CPU wakeups and conserve battery.
pub struct NainDetector {
    /// Shared NAIN state for reading battery info and writing results.
    state: Arc<RwLock<NainState>>,
    /// Most recent detection result.
    last_result: parking_lot::Mutex<Option<DetectionResult>>,
    /// Whether the detector is currently running a detection cycle.
    detecting: Arc<std::sync::atomic::AtomicBool>,
    /// Whether to coalesce the next detection cycle with NTP probes.
    ntp_coalesce_pending: Arc<std::sync::atomic::AtomicBool>,
}

impl NainDetector {
    /// Create a new NAIN detector with shared state access.
    pub fn new(state: Arc<RwLock<NainState>>) -> Result<Self, ShieldError> {
        Ok(Self {
            state,
            last_result: parking_lot::Mutex::new(None),
            detecting: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            ntp_coalesce_pending: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Run a single detection cycle and return the determined status.
    ///
    /// This method is safe to call concurrently — if a detection cycle is
    /// already in progress, it returns the last known status.
    pub async fn detect(&self) -> NainStatus {
        // Prevent concurrent detection cycles
        if self
            .detecting
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::Acquire,
                std::sync::atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            debug!("NAIN detection already in progress — returning last known status");
            return self.last_status();
        }

        let coalesced = self
            .ntp_coalesce_pending
            .swap(false, std::sync::atomic::Ordering::AcqRel);

        let result = self.run_detection_cycle(coalesced).await;

        let status = result.status;
        *self.last_result.lock() = Some(result);

        self.detecting
            .store(false, std::sync::atomic::Ordering::Release);

        // Update shared state with detection metrics
        {
            let mut state = self.state.write().await;
            state.confirmation_count = state.confirmation_count.saturating_add(1);
        }

        status
    }

    /// Get the last known NAIN status without running a new detection cycle.
    pub fn last_status(&self) -> NainStatus {
        self.last_result
            .lock()
            .as_ref()
            .map(|r| r.status)
            .unwrap_or(NainStatus::FullInternet)
    }

    /// Get a clone of the last detection result.
    pub fn last_result(&self) -> Option<DetectionResult> {
        self.last_result.lock().clone()
    }

    /// Calculate the adaptive detection interval based on PowerMode + screen state.
    ///
    /// PowerMode intervals:
    ///   - Performance: 30s
    ///   - Normal: 60s
    ///   - Save: 120s
    ///   - Critical: 300s
    ///
    /// Screen-off multiplier: 2x
    pub fn adaptive_interval(&self) -> Duration {
        let state = self.state.try_read();
        match state {
            Ok(s) => {
                let base = match s.power_mode {
                    0 => INTERVAL_PERFORMANCE,
                    1 => INTERVAL_NORMAL,
                    2 => INTERVAL_SAVE,
                    _ => INTERVAL_CRITICAL,
                };

                let mut effective = base;

                // Screen-off: 2x interval
                if !s.screen_on {
                    effective = effective.mul_f64(SCREEN_OFF_MULTIPLIER);
                }

                // Charging overrides: back to base if charging
                if s.is_charging {
                    effective = base;
                }

                effective
            }
            Err(_) => INTERVAL_NORMAL,
        }
    }

    /// Request that the next detection cycle coalesce with NTP probes.
    pub fn request_ntp_coalesce(&self) {
        self.ntp_coalesce_pending
            .store(true, std::sync::atomic::Ordering::Release);
    }

    // ── Internal detection logic ────────────────────────────────────────

    /// Execute the full detection cycle: probe international and domestic.
    async fn run_detection_cycle(&self, coalesce_ntp: bool) -> DetectionResult {
        let now = Instant::now();

        if coalesce_ntp {
            debug!("Running NAIN detection with NTP covert channel coalescing");
        }

        // Run international and domestic probes concurrently
        let (intl_results, domestic_results) =
            tokio::join!(self.probe_international(), self.probe_domestic(),);

        // Aggregate results
        let international_ok_count = intl_results.iter().filter(|(ok, _)| *ok).count() as u8;
        let domestic_ok_count = domestic_results.iter().filter(|(ok, _)| *ok).count() as u8;
        let international_total = intl_results.len() as u8;
        let domestic_total = domestic_results.len() as u8;

        // Find fastest latencies
        let international_latency = intl_results
            .iter()
            .filter(|(ok, _)| *ok)
            .filter_map(|(_, lat)| *lat)
            .min();
        let domestic_latency = domestic_results
            .iter()
            .filter(|(ok, _)| *ok)
            .filter_map(|(_, lat)| *lat)
            .min();

        let international_ok = international_ok_count > 0;
        let domestic_ok = domestic_ok_count > 0;

        // Determine NAIN status based on probe results
        let status = match (international_ok, domestic_ok) {
            (true, _) => NainStatus::FullInternet,
            (false, true) => NainStatus::NainOnly,
            (false, false) => NainStatus::CompleteBlackout,
        };

        info!(
            ?status,
            intl_ok = international_ok_count,
            intl_total = international_total,
            domestic_ok = domestic_ok_count,
            domestic_total = domestic_total,
            ntp_coalesced = coalesce_ntp,
            "NAIN detection cycle completed"
        );

        DetectionResult {
            status,
            international_ok_count,
            international_total,
            domestic_ok_count,
            domestic_total,
            international_latency,
            domestic_latency,
            timestamp: now,
            ntp_coalesced: coalesce_ntp,
        }
    }

    /// Probe all international endpoints concurrently.
    async fn probe_international(&self) -> Vec<(bool, Option<Duration>)> {
        let mut handles = Vec::with_capacity(INTERNATIONAL_PROBES.len());

        for addr in INTERNATIONAL_PROBES {
            let addr = addr.to_string();
            handles.push(tokio::spawn(async move { probe_tcp_with_dns(&addr).await }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!(error = %e, "International probe task panicked");
                    results.push((false, None));
                }
            }
        }

        results
    }

    /// Probe all domestic endpoints concurrently.
    async fn probe_domestic(&self) -> Vec<(bool, Option<Duration>)> {
        let mut handles = Vec::with_capacity(DOMESTIC_PROBES.len());

        for addr in DOMESTIC_PROBES {
            let addr = addr.to_string();
            handles.push(tokio::spawn(async move { probe_tcp_connect(&addr).await }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!(error = %e, "Domestic probe task panicked");
                    results.push((false, None));
                }
            }
        }

        results
    }
}

// ── Probe functions ─────────────────────────────────────────────────────────

/// Probe an endpoint by establishing a TCP connection and sending a DNS query.
///
/// Returns (success, latency) tuple.
async fn probe_tcp_with_dns(addr: &str) -> (bool, Option<Duration>) {
    let start = Instant::now();

    match TcpStream::connect(addr).await {
        Ok(mut stream) => {
            let _ = stream.set_nodelay(true);

            if stream.write_all(DNS_ROOT_QUERY).await.is_err() {
                debug!(addr, "Failed to send DNS query — treating as unreachable");
                return (false, None);
            }

            let mut response_buf = [0u8; 2];
            match tokio::time::timeout(DNS_QUERY_TIMEOUT, stream.read_exact(&mut response_buf))
                .await
            {
                Ok(Ok(_)) => {
                    let latency = start.elapsed();
                    debug!(
                        addr,
                        ?latency,
                        "DNS query response received — endpoint is reachable"
                    );
                    (true, Some(latency))
                }
                Ok(Err(e)) => {
                    debug!(addr, error = %e, "DNS query read failed");
                    (false, None)
                }
                Err(_) => {
                    debug!(addr, "DNS query timed out");
                    (false, None)
                }
            }
        }
        Err(e) => {
            debug!(addr, error = %e, "TCP connect failed");
            (false, None)
        }
    }
}

/// Probe an endpoint by establishing a TCP connection only.
///
/// Returns (success, latency) tuple.
async fn probe_tcp_connect(addr: &str) -> (bool, Option<Duration>) {
    let start = Instant::now();
    match tokio::time::timeout(PROBE_TIMEOUT, TcpStream::connect(addr)).await {
        Ok(Ok(_stream)) => {
            let latency = start.elapsed();
            debug!(addr, ?latency, "TCP connect succeeded");
            (true, Some(latency))
        }
        Ok(Err(e)) => {
            debug!(addr, error = %e, "TCP connect refused/failed");
            (false, None)
        }
        Err(_) => {
            debug!(addr, "TCP connect timed out");
            (false, None)
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_result_status_logic() {
        let result = DetectionResult {
            status: NainStatus::NainOnly,
            international_ok_count: 0,
            international_total: 3,
            domestic_ok_count: 2,
            domestic_total: 3,
            international_latency: None,
            domestic_latency: Some(Duration::from_millis(45)),
            timestamp: Instant::now(),
            ntp_coalesced: false,
        };

        assert!(!result.international_ok());
        assert!(result.domestic_ok());
        assert_eq!(result.status, NainStatus::NainOnly);
    }

    #[test]
    fn status_classification_rules() {
        let cases = vec![
            (true, true, NainStatus::FullInternet),
            (true, false, NainStatus::FullInternet),
            (false, true, NainStatus::NainOnly),
            (false, false, NainStatus::CompleteBlackout),
        ];

        for (intl, domestic, expected) in cases {
            let status = match (intl, domestic) {
                (true, _) => NainStatus::FullInternet,
                (false, true) => NainStatus::NainOnly,
                (false, false) => NainStatus::CompleteBlackout,
            };
            assert_eq!(status, expected);
        }
    }

    #[test]
    fn power_mode_intervals() {
        // Verify the interval table matches the specification
        assert_eq!(INTERVAL_PERFORMANCE, Duration::from_secs(30));
        assert_eq!(INTERVAL_NORMAL, Duration::from_secs(60));
        assert_eq!(INTERVAL_SAVE, Duration::from_secs(120));
        assert_eq!(INTERVAL_CRITICAL, Duration::from_secs(300));
    }

    #[test]
    fn screen_off_multiplier() {
        assert!((SCREEN_OFF_MULTIPLIER - 2.0).abs() < f64::EPSILON);
    }
}
