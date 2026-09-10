//! Network Assessor — comprehensive network capability assessment for Iran
//!
//! Combines DPI scanning, adaptive port scanning, and DNS scanning results
//! to provide a complete assessment of the current network environment,
//! calculate multi-signal confidence scores, detect network state shifts,
//! and recommend the best transport strategy.
//!
//! ## Physical Bearer & Full-Severance Notice
//! When international BGP / physical routes are severed (NetworkState::DomesticOnly),
//! software protocols cannot magically route packets out of country without physical egress.
//! In this state, the engine halts battery-draining international retries, surfaces the state
//! explicitly, and routes via domestic reverse relays, Iranian CDNs (Arvan), or local mesh nodes.

use anyhow::Result;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

use super::dns_scanner::{DnsScanResult, DnsScanner};
use super::dpi_scanner::{DpiScanResult, DpiScanner, FavaVersion};
use super::port_scanner::{PortScanResult, PortScanner, PortStatus, ScanMode};

/// Connectivity and severance state of the network
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum NetworkState {
    /// Normal connectivity: both international and domestic hosts are reachable
    #[default]
    Normal,
    /// Degraded connectivity: significant throttling, packet loss, or active DPI manipulation
    Degraded,
    /// Domestic only (National Information Network / NIN): international bearer is severed,
    /// but domestic Iranian hosts are reachable. Software VPNs cannot restore international
    /// routing; traffic must fall back to domestic/intranet reverse relays.
    DomesticOnly,
    /// Full severance: both domestic and international probes fail completely.
    FullSeverance,
}

/// Overall network assessment
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkAssessment {
    /// DPI scan results
    pub dpi_result: DpiScanResult,
    /// DNS scan results
    pub dns_result: DnsScanResult,
    /// Port scan results
    pub port_results: Vec<PortScanResult>,
    /// Whether the network is heavily censored
    pub heavily_censored: bool,
    /// Recommended transport priority list
    pub recommended_transports: Vec<String>,
    /// Whether TLS fragmentation is recommended
    pub tls_fragmentation_recommended: bool,
    /// Whether domain fronting is required
    pub domain_fronting_required: bool,
    /// Whether covert channels are needed
    pub covert_channels_needed: bool,
    /// Assessment timestamp
    pub timestamp: String,
    /// Overall risk level (0-100, higher = more censored)
    pub censorship_risk_score: u8,
    /// Scan mode used for this assessment
    #[serde(default)]
    pub scan_mode: Option<ScanMode>,
    /// Detected connectivity & severance state
    #[serde(default)]
    pub network_state: Option<NetworkState>,
    /// Statistical confidence score (0-100)
    #[serde(default = "default_confidence")]
    pub confidence_score: u8,
    /// Incremental scan generation number
    #[serde(default)]
    pub generation: u64,
    /// Whether domestic Iranian probe targets were verified reachable
    #[serde(default = "default_true")]
    pub domestic_probe_success: bool,
    /// Whether international probe targets were verified reachable
    #[serde(default = "default_true")]
    pub international_probe_success: bool,
    /// Whether this assessment detected a major state shift compared to previous scan
    #[serde(default)]
    pub state_shift_detected: bool,
}

fn default_confidence() -> u8 {
    95
}

fn default_true() -> bool {
    true
}

/// Max entries to retain in assessment history for change detection
const HISTORY_BUFFER_CAPACITY: usize = 16;

/// Network Assessor
pub struct NetworkAssessor {
    /// Target host for international port scanning
    probe_host: String,
    /// Target host for domestic verification
    domestic_probe_host: String,
    /// Default scan mode
    scan_mode: ScanMode,
    /// Generation counter
    generation_counter: AtomicU64,
    /// Assessment history ring buffer for change detection
    history: Arc<RwLock<VecDeque<NetworkAssessment>>>,
}

impl NetworkAssessor {
    /// Create a new network assessor
    pub fn new() -> Self {
        Self {
            probe_host: "8.8.8.8".to_string(),
            domestic_probe_host: "185.143.232.1".to_string(), // Iranian Anycast / Aparat CDN
            scan_mode: ScanMode::Adaptive,
            generation_counter: AtomicU64::new(1),
            history: Arc::new(RwLock::new(VecDeque::with_capacity(
                HISTORY_BUFFER_CAPACITY,
            ))),
        }
    }

    /// Create with specific scan mode
    pub fn new_with_mode(scan_mode: ScanMode) -> Self {
        Self {
            probe_host: "8.8.8.8".to_string(),
            domestic_probe_host: "185.143.232.1".to_string(),
            scan_mode,
            generation_counter: AtomicU64::new(1),
            history: Arc::new(RwLock::new(VecDeque::with_capacity(
                HISTORY_BUFFER_CAPACITY,
            ))),
        }
    }

    /// Run a full network assessment in current mode
    pub async fn assess(&self) -> Result<NetworkAssessment> {
        self.assess_with_mode(self.scan_mode).await
    }

    /// Run a full network assessment with an explicit scan mode
    pub async fn assess_with_mode(&self, mode: ScanMode) -> Result<NetworkAssessment> {
        let gen = self.generation_counter.fetch_add(1, Ordering::SeqCst);
        info!(
            "Starting comprehensive network assessment (Mode: {:?}, Gen: {}) for Iranian censorship...",
            mode, gen
        );

        let intl_port_scanner = PortScanner::new_with_mode(&self.probe_host, mode);
        let domestic_port_scanner =
            PortScanner::new_with_mode(&self.domestic_probe_host, ScanMode::Static);

        // Run multi-vector scans in parallel for speed and minimum latency
        let mut dpi_scanner = DpiScanner::new();
        let mut dns_scanner = DnsScanner::new();
        let (dpi_result, dns_result, port_results, domestic_ports) = tokio::join!(
            dpi_scanner.scan(),
            dns_scanner.scan(),
            intl_port_scanner.scan_vpn_ports(),
            domestic_port_scanner.scan_range(443, 443),
        );

        let dpi_result = dpi_result?;
        let dns_result = dns_result?;
        let port_results = port_results?;
        let domestic_results = domestic_ports.unwrap_or_default();

        let domestic_probe_success = domestic_results
            .iter()
            .any(|p| p.status == PortStatus::Open)
            || !dpi_result.allowed_sni_domains.is_empty();

        let open_intl_ports = port_results
            .iter()
            .filter(|p| p.status == PortStatus::Open)
            .count();
        let international_probe_success =
            open_intl_ports > 0 || !dns_result.clean_servers.is_empty();

        // Detect network state: Normal, Degraded, DomesticOnly, FullSeverance
        let network_state = match (domestic_probe_success, international_probe_success) {
            (true, true) => {
                if dpi_result.fava_version == FavaVersion::V3
                    || dns_result.injection_detected
                    || open_intl_ports < 2
                {
                    NetworkState::Degraded
                } else {
                    NetworkState::Normal
                }
            }
            (true, false) => {
                // International probes failed, domestic works -> National Information Network / Blackout
                warn!("International bearer appears severed. Domestic Iranian network remains active.");
                NetworkState::DomesticOnly
            }
            (false, false) => {
                warn!("Both domestic and international probes failing. Full network severance or offline.");
                NetworkState::FullSeverance
            }
            (false, true) => NetworkState::Degraded,
        };

        // Calculate censorship risk score (0-100)
        let risk_score =
            self.calculate_risk_score(&dpi_result, &dns_result, &port_results, network_state);

        // Multi-signal statistical confidence score
        let confidence_score =
            self.calculate_confidence(&dpi_result, &dns_result, &port_results, mode);

        // Determine recommendations
        let heavily_censored = risk_score > 70 || network_state == NetworkState::DomesticOnly;
        let tls_fragmentation_recommended =
            dpi_result.sni_filtering || network_state == NetworkState::Degraded;
        let domain_fronting_required =
            dpi_result.tls_fingerprinting || network_state == NetworkState::DomesticOnly;
        let covert_channels_needed = dpi_result.fava_version == FavaVersion::V3
            || (dns_result.injection_detected && open_intl_ports < 3)
            || network_state == NetworkState::DomesticOnly;

        let recommended_transports =
            self.recommend_transports(&dpi_result, &dns_result, &port_results, network_state);

        // Check for state shift compared to previous assessment
        let state_shift_detected = {
            let hist = self.history.read();
            if let Some(last) = hist.back() {
                last.network_state != Some(network_state)
                    || (last.censorship_risk_score as i16 - risk_score as i16).abs() > 20
            } else {
                false
            }
        };

        if state_shift_detected {
            info!("Major network profile shift detected! Transitioning circumvention recommendations.");
        }

        let assessment = NetworkAssessment {
            dpi_result,
            dns_result,
            port_results,
            heavily_censored,
            recommended_transports,
            tls_fragmentation_recommended,
            domain_fronting_required,
            covert_channels_needed,
            timestamp: chrono::Utc::now().to_rfc3339(),
            censorship_risk_score: risk_score,
            scan_mode: Some(mode),
            network_state: Some(network_state),
            confidence_score,
            generation: gen,
            domestic_probe_success,
            international_probe_success,
            state_shift_detected,
        };

        // Save to ring buffer history
        {
            let mut hist = self.history.write();
            if hist.len() >= HISTORY_BUFFER_CAPACITY {
                hist.pop_front();
            }
            hist.push_back(assessment.clone());
        }

        info!(
            "Assessment complete (Gen {}): State={:?}, Risk={}, Conf={}% Transports={:?}",
            gen,
            network_state,
            assessment.censorship_risk_score,
            assessment.confidence_score,
            assessment.recommended_transports
        );

        Ok(assessment)
    }

    /// Retrieve recent assessment history
    pub fn get_history(&self) -> Vec<NetworkAssessment> {
        self.history.read().iter().cloned().collect()
    }

    /// Calculate censorship risk score (0-100)
    fn calculate_risk_score(
        &self,
        dpi: &DpiScanResult,
        dns: &DnsScanResult,
        ports: &[PortScanResult],
        state: NetworkState,
    ) -> u8 {
        if state == NetworkState::DomesticOnly {
            return 100;
        }

        let mut score: u8 = 0;

        // DPI factors (0-40 points)
        match dpi.fava_version {
            FavaVersion::None => score += 0,
            FavaVersion::V1 => score += 15,
            FavaVersion::V2 => score += 25,
            FavaVersion::V3 => score += 40,
            FavaVersion::Unknown => score += 20,
        }

        // DNS factors (0-30 points)
        if dns.injection_detected {
            score += 15;
        }
        if dns.doh_required {
            score += 10;
        }
        if dns.poisoned_domains.len() > 3 {
            score += 5;
        }

        // Port factors (0-30 points)
        let open_ports = ports
            .iter()
            .filter(|p| p.status == PortStatus::Open)
            .count();
        let blocked_ports = ports
            .iter()
            .filter(|p| p.status == PortStatus::Blocked)
            .count();

        if blocked_ports > 5 {
            score += 20;
        } else if blocked_ports > 2 {
            score += 10;
        }

        if open_ports < 3 {
            score += 10;
        }

        score.min(100)
    }

    /// Calculate statistical confidence score based on probe coverage and response quality
    fn calculate_confidence(
        &self,
        dpi: &DpiScanResult,
        dns: &DnsScanResult,
        ports: &[PortScanResult],
        mode: ScanMode,
    ) -> u8 {
        let mut conf: u8 = 75;

        // More ports tested -> higher confidence
        if ports.len() >= 12 {
            conf += 10;
        } else if ports.len() >= 6 {
            conf += 5;
        }

        // DPI scan validation
        if !dpi.allowed_sni_domains.is_empty()
            && (!dpi.blocked_sni_domains.is_empty() || dpi.methods.is_empty())
        {
            conf += 8;
        }

        // DNS test servers coverage
        if dns.clean_servers.len() + dns.injected_servers.len() >= 4 {
            conf += 5;
        }

        if mode == ScanMode::Deep {
            conf += 2;
        }

        conf.min(100)
    }

    /// Recommend transport protocols based on scan results and network state
    fn recommend_transports(
        &self,
        dpi: &DpiScanResult,
        dns: &DnsScanResult,
        ports: &[PortScanResult],
        state: NetworkState,
    ) -> Vec<String> {
        let mut transports = Vec::new();
        let port_443_open = ports
            .iter()
            .any(|p| p.port == 443 && p.status == PortStatus::Open);

        // In domestic-only / blackout state, prioritize domestic CDN reverse relays and Chinese CDNs
        if state == NetworkState::DomesticOnly {
            transports.push("arvan-cdn".to_string());
            transports.push("intranet-reverse-relay".to_string());
            transports.push("alibaba-cdn".to_string());
            transports.push("tencent-cdn".to_string());
            transports.push("snowflake".to_string());
            transports.push("p2p-mesh".to_string());
            return transports;
        }

        // Always recommend Arvan CDN first (Iranian CDN, never blocked)
        transports.push("arvan-cdn".to_string());

        if dpi.sni_filtering {
            // SNI filtering detected — use Shadow TLS v3 and XTLS-Reality
            transports.push("shadow-tls-v3".to_string());
            transports.push("xtls-reality".to_string());
        }

        if port_443_open {
            transports.push("hysteria2".to_string());
            transports.push("naiveproxy".to_string());
        }

        if dpi.tls_fingerprinting {
            // TLS fingerprinting detected — need more sophisticated evasion
            transports.push("xtls-reality".to_string());
        }

        // Chinese CDN workers (not blocked in Iran)
        transports.push("alibaba-cdn".to_string());
        transports.push("bytedance-cdn".to_string());
        transports.push("tencent-cdn".to_string());

        // QUIC-based transports
        transports.push("tuic-v5".to_string());
        transports.push("webtransport".to_string());

        // Covert channels for extreme censorship
        if dpi.fava_version == FavaVersion::V3 {
            transports.push("doq-tunnel".to_string());
            transports.push("mqtt-ws".to_string());
            transports.push("ntp-covert".to_string());
            transports.push("icmp-tunnel".to_string());
        }

        // P2P overlays as last resort
        transports.push("yggdrasil".to_string());
        transports.push("i2p-overlay".to_string());

        // Deduplicate while preserving order
        let mut seen = std::collections::HashSet::new();
        transports.retain(|t| seen.insert(t.clone()));

        transports
    }
}
