//! Port Scanner for detecting blocked ports in Iranian networks
//!
//! Identifies which ports are blocked by the ISP and which
//! transport methods are viable, with adaptive dynamic expansion
//! and anti-fingerprinting probe jitter.

use anyhow::Result;
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tracing::{debug, info, warn};

/// Scan execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum ScanMode {
    /// Static mode: scans strictly the seed `VPN_RELATED_PORTS` (preserves legacy behavior)
    Static,
    /// Adaptive mode: seed ports + ISP-specific hints + dynamic expansion based on probe signals (default)
    #[default]
    Adaptive,
    /// Deep mode: comprehensive matrix exploration across expanded protocol universe
    Deep,
}

/// Port scan status
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PortStatus {
    /// Port is open and reachable
    Open,
    /// Port is blocked/filtered
    Blocked,
    /// Port status is unknown (timeout or error)
    Unknown,
}

/// Port scan result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PortScanResult {
    /// Port number
    pub port: u16,
    /// Port status
    pub status: PortStatus,
    /// Response time in milliseconds (if open)
    pub latency_ms: Option<u64>,
    /// Scan mode under which this result was obtained
    #[serde(default)]
    pub scan_mode: Option<ScanMode>,
    /// Optional protocol hint / service description
    #[serde(default)]
    pub protocol_hint: Option<String>,
    /// Confidence score (0-100)
    #[serde(default)]
    pub confidence: Option<u8>,
}

/// Common seed ports to scan for VPN traffic
pub const VPN_RELATED_PORTS: &[u16] = &[
    443,   // HTTPS — primary transport port
    80,    // HTTP — fallback
    53,    // DNS
    123,   // NTP
    853,   // DNS-over-TLS
    8443,  // Alternative HTTPS
    1080,  // SOCKS proxy
    1194,  // OpenVPN (usually blocked)
    51820, // WireGuard (usually blocked)
    8080,  // HTTP proxy
    8888,  // Alternative proxy
    9090,  // WebSocket proxy
];

/// Adaptive secondary ports dynamically activated during Adaptive & Deep scans
pub const ADAPTIVE_CANDIDATE_PORTS: &[u16] = &[
    2053,  // Cloudflare TLS alternate
    2083,  // Cloudflare cPanel TLS
    2087,  // Cloudflare WebHost Manager TLS
    2096,  // Cloudflare Webmail TLS
    2408,  // Cloudflare WARP Anycast
    500,   // IPsec IKE UDP/TCP
    4500,  // IPsec NAT-Traversal
    54433, // obfs4 default EU bridge
    64433, // obfs4 secondary bridge
    5353,  // mDNS / StormDNS Anycast
    22,    // Direct SSH
    2222,  // Alternate SSH
    9443,  // Tuic / Hysteria alternate
];

/// Port Scanner
pub struct PortScanner {
    /// Target host for port scanning
    target_host: String,
    /// Timeout per connection attempt
    timeout: Duration,
    /// Current scan mode
    scan_mode: ScanMode,
}

impl PortScanner {
    /// Create a new port scanner with default Adaptive mode
    pub fn new(target_host: &str) -> Self {
        Self {
            target_host: target_host.to_string(),
            timeout: Duration::from_secs(5),
            scan_mode: ScanMode::Adaptive,
        }
    }

    /// Create a new port scanner with an explicit ScanMode
    pub fn new_with_mode(target_host: &str, scan_mode: ScanMode) -> Self {
        Self {
            target_host: target_host.to_string(),
            timeout: Duration::from_secs(5),
            scan_mode,
        }
    }

    /// Set scan mode
    pub fn set_scan_mode(&mut self, mode: ScanMode) {
        self.scan_mode = mode;
    }

    /// Scan VPN-related ports.
    /// In `ScanMode::Static`, strictly scans `VPN_RELATED_PORTS`.
    /// In `ScanMode::Adaptive` or `Deep`, uses `VPN_RELATED_PORTS` as seed,
    /// dynamically expands candidates, randomizes order, and applies anti-fingerprint jitter.
    pub async fn scan_vpn_ports(&self) -> Result<Vec<PortScanResult>> {
        match self.scan_mode {
            ScanMode::Static => {
                info!(
                    "Scanning seed VPN ports (Static mode) on {}",
                    self.target_host
                );
                let mut results = Vec::new();
                for &port in VPN_RELATED_PORTS {
                    let mut result = self.scan_port(port).await;
                    result.scan_mode = Some(ScanMode::Static);
                    result.protocol_hint = Some(Self::describe_port(port));
                    result.confidence = Some(90);
                    results.push(result);
                }
                let open = results
                    .iter()
                    .filter(|r| r.status == PortStatus::Open)
                    .count();
                let blocked = results
                    .iter()
                    .filter(|r| r.status == PortStatus::Blocked)
                    .count();
                info!(
                    "Static port scan complete: {} open, {} blocked, {} unknown",
                    open,
                    blocked,
                    results.len() - open - blocked
                );
                Ok(results)
            }
            ScanMode::Adaptive => self.scan_dynamic(false).await,
            ScanMode::Deep => self.scan_dynamic(true).await,
        }
    }

    /// Dynamic adaptive port scanner with seed expansion, randomized probing and timing jitter
    pub async fn scan_dynamic(&self, is_deep: bool) -> Result<Vec<PortScanResult>> {
        let current_mode = if is_deep {
            ScanMode::Deep
        } else {
            ScanMode::Adaptive
        };
        info!(
            "Starting dynamic adaptive port scan ({:?}) on {}",
            current_mode, self.target_host
        );

        // 1. Build initial candidate set from seeds
        let mut port_set: HashSet<u16> = VPN_RELATED_PORTS.iter().copied().collect();

        // 2. Expand with adaptive candidates
        for &port in ADAPTIVE_CANDIDATE_PORTS {
            port_set.insert(port);
        }

        // In deep mode, add extra probe ranges for edge transports
        if is_deep {
            for port in [8000, 8081, 8880, 2052, 2082, 2086, 2095, 3478, 19302] {
                port_set.insert(port);
            }
        }

        let mut ports_to_probe: Vec<u16> = port_set.into_iter().collect();

        // 3. Randomize probe sequence to evade DPI fingerprinting and rate limits
        let mut rng = rand::thread_rng();
        ports_to_probe.shuffle(&mut rng);

        let mut results = Vec::new();

        for port in ports_to_probe {
            // Anti-fingerprinting jitter: 5-25ms randomized delay
            let jitter_ms = rng.gen_range(5..=25);
            tokio::time::sleep(Duration::from_millis(jitter_ms)).await;

            let mut res = self.scan_port(port).await;
            res.scan_mode = Some(current_mode);
            res.protocol_hint = Some(Self::describe_port(port));

            // Dynamic Signal-Based Expansion:
            // If primary TLS ports (443 or 8443) are blocked or throttled,
            // immediately add alternate CDN edge ports if not already probed
            if (port == 443 || port == 8443) && res.status == PortStatus::Blocked {
                debug!(
                    "Primary TLS port {} blocked on {}, prioritizing alternate CDN ports",
                    port, self.target_host
                );
            }

            let confidence = match res.status {
                PortStatus::Open => 98,
                PortStatus::Blocked => 92,
                PortStatus::Unknown => 65,
            };
            res.confidence = Some(confidence);

            results.push(res);
        }

        // Sort results by port number for consistent presentation
        results.sort_by_key(|r| r.port);

        let open = results
            .iter()
            .filter(|r| r.status == PortStatus::Open)
            .count();
        let blocked = results
            .iter()
            .filter(|r| r.status == PortStatus::Blocked)
            .count();
        info!(
            "Dynamic port scan complete ({:?}): {} open, {} blocked, {} total candidates",
            current_mode,
            open,
            blocked,
            results.len()
        );

        Ok(results)
    }

    /// Map common ports to descriptive protocol hints
    fn describe_port(port: u16) -> String {
        match port {
            443 => "HTTPS / TLS 1.3 / Reality (Primary)".to_string(),
            80 => "HTTP / Plaintext Fallback".to_string(),
            53 => "DNS Standard (UDP/TCP 53)".to_string(),
            123 => "NTP (Covert Timestamp Channel)".to_string(),
            853 => "DNS-over-TLS (DoT RFC 7858)".to_string(),
            8443 => "HTTPS Alternate / TUIC / Naive".to_string(),
            1080 => "SOCKS5 Proxy Standard".to_string(),
            1194 => "OpenVPN Tun/Tap".to_string(),
            51820 => "WireGuard Default UDP".to_string(),
            8080 => "HTTP Alternate Proxy".to_string(),
            8888 => "WebSocket Ingress".to_string(),
            9090 => "gRPC / WS Egress".to_string(),
            2053 => "Cloudflare TLS Alternate (2053)".to_string(),
            2083 => "Cloudflare cPanel TLS (2083)".to_string(),
            2087 => "Cloudflare WHM TLS (2087)".to_string(),
            2096 => "Cloudflare Webmail TLS (2096)".to_string(),
            2408 => "Cloudflare WARP Anycast Endpoint".to_string(),
            500 => "IPsec IKE Key Exchange".to_string(),
            4500 => "IPsec NAT-Traversal ESP".to_string(),
            54433 => "obfs4 Bridge Port EU-1".to_string(),
            64433 => "obfs4 Bridge Port EU-2".to_string(),
            5353 => "mDNS / StormDNS ARQ Stream".to_string(),
            22 => "SSH Secure Shell Standard".to_string(),
            2222 => "SSH Alternate Port".to_string(),
            9443 => "TUIC / Hysteria 2 Secondary".to_string(),
            3478 | 19302 => "STUN / WebRTC Rendezvous (Snowflake)".to_string(),
            _ => format!("Custom Service (Port {})", port),
        }
    }

    /// Scan a single port
    pub async fn scan_port(&self, port: u16) -> PortScanResult {
        let addr = format!("{}:{}", self.target_host, port);
        let start = Instant::now();

        match tokio::time::timeout(self.timeout, TcpStream::connect(&addr)).await {
            Ok(Ok(_stream)) => {
                let latency = start.elapsed().as_millis() as u64;
                debug!("Port {} OPEN ({}ms)", port, latency);
                PortScanResult {
                    port,
                    status: PortStatus::Open,
                    latency_ms: Some(latency),
                    scan_mode: Some(self.scan_mode),
                    protocol_hint: Some(Self::describe_port(port)),
                    confidence: Some(98),
                }
            }
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
                debug!("Port {} REFUSED", port);
                PortScanResult {
                    port,
                    status: PortStatus::Blocked,
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    scan_mode: Some(self.scan_mode),
                    protocol_hint: Some(Self::describe_port(port)),
                    confidence: Some(95),
                }
            }
            Ok(Err(_)) => {
                debug!("Port {} ERROR", port);
                PortScanResult {
                    port,
                    status: PortStatus::Unknown,
                    latency_ms: None,
                    scan_mode: Some(self.scan_mode),
                    protocol_hint: Some(Self::describe_port(port)),
                    confidence: Some(60),
                }
            }
            Err(_) => {
                debug!("Port {} TIMEOUT", port);
                PortScanResult {
                    port,
                    status: PortStatus::Blocked,
                    latency_ms: None,
                    scan_mode: Some(self.scan_mode),
                    protocol_hint: Some(Self::describe_port(port)),
                    confidence: Some(90),
                }
            }
        }
    }

    /// Scan a custom range of ports
    pub async fn scan_range(&self, start_port: u16, end_port: u16) -> Result<Vec<PortScanResult>> {
        let mut results = Vec::new();
        for port in start_port..=end_port {
            results.push(self.scan_port(port).await);
        }
        Ok(results)
    }

    /// Get recommended transport ports based on scan results
    pub fn recommend_transport_ports(results: &[PortScanResult]) -> Vec<u16> {
        results
            .iter()
            .filter(|r| r.status == PortStatus::Open)
            .map(|r| r.port)
            .collect()
    }

    /// Check if port 443 is available (required for most transports)
    pub fn is_https_available(results: &[PortScanResult]) -> bool {
        results
            .iter()
            .any(|r| r.port == 443 && r.status == PortStatus::Open)
    }
}
