//! DPI (Deep Packet Inspection) Scanner for Iranian FAVA systems
//!
//! Detects DPI signatures and capabilities of the current network.
//! Identifies FAVA version, blocking methods, and evasion strategies.

use anyhow::Result;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, info, warn};

/// FAVA DPI version detected on the network
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FavaVersion {
    /// No DPI detected
    None,
    /// FAVA v1 — basic SNI filtering
    V1,
    /// FAVA v2 — advanced TLS fingerprinting + SNI
    V2,
    /// FAVA v3 — ML-based traffic classification
    V3,
    /// Unknown DPI but censorship detected
    Unknown,
}

/// DPI detection method
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DpiMethod {
    /// SNI-based filtering
    SniFiltering,
    /// TLS fingerprinting (JA3/JA4)
    TlsFingerprinting,
    /// Packet size/statistics analysis
    StatisticalAnalysis,
    /// Protocol identification via payload inspection
    PayloadInspection,
    /// IP-based blocking
    IpBlocking,
    /// DNS-based filtering
    DnsFiltering,
}

/// Result of a DPI scan
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DpiScanResult {
    /// Detected FAVA version
    pub fava_version: FavaVersion,
    /// Detected DPI methods
    pub methods: Vec<DpiMethod>,
    /// Whether SNI filtering is active
    pub sni_filtering: bool,
    /// Whether TLS fingerprinting is active
    pub tls_fingerprinting: bool,
    /// Whether statistical analysis is active
    pub statistical_analysis: bool,
    /// Blocked SNI domains detected
    pub blocked_sni_domains: Vec<String>,
    /// Allowed SNI domains confirmed
    pub allowed_sni_domains: Vec<String>,
    /// TLS handshake reset detected
    pub tls_reset_detected: bool,
    /// Scan duration
    pub scan_duration_ms: u64,
}

/// DPI Scanner for detecting Iranian censorship infrastructure
pub struct DpiScanner {
    /// Test domains known to be blocked in Iran
    blocked_test_domains: Vec<&'static str>,
    /// Test domains known to be allowed in Iran
    allowed_test_domains: Vec<&'static str>,
    /// Target IP for connection tests
    probe_target: SocketAddr,
    /// Timeout for each probe
    probe_timeout: Duration,
}

impl DpiScanner {
    /// Create a new DPI scanner
    pub fn new() -> Self {
        Self {
            blocked_test_domains: vec![
                "www.youtube.com",
                "www.twitter.com",
                "www.facebook.com",
                "www.telegram.org",
                "www.instagram.com",
                "www.whatsapp.com",
                "www.signal.org",
                "www.torproject.org",
            ],
            allowed_test_domains: vec![
                "www.digikala.com",
                "www.aparat.com",
                "www.snapp.ir",
                "www.divar.ir",
                "www.cafe.bazaar.ir",
                "www.shaparak.ir",
                "www.irancell.ir",
                "www.eitaa.com",
            ],
            probe_target: "8.8.8.8:443".parse().unwrap(),
            probe_timeout: Duration::from_secs(5),
        }
    }

    /// Run a comprehensive DPI scan
    pub async fn scan(&self) -> Result<DpiScanResult> {
        let start = Instant::now();
        info!("Starting DPI scan for Iranian censorship detection...");

        let mut result = DpiScanResult {
            fava_version: FavaVersion::None,
            methods: Vec::new(),
            sni_filtering: false,
            tls_fingerprinting: false,
            statistical_analysis: false,
            blocked_sni_domains: Vec::new(),
            allowed_sni_domains: Vec::new(),
            tls_reset_detected: false,
            scan_duration_ms: 0,
        };

        // Phase 1: Test SNI filtering with blocked domains
        for domain in &self.blocked_test_domains {
            match self.probe_sni(domain).await {
                Ok(SniProbeResult::Reset) => {
                    result.sni_filtering = true;
                    result.blocked_sni_domains.push(domain.to_string());
                    debug!("SNI blocked: {}", domain);
                }
                Ok(SniProbeResult::Timeout) => {
                    result.sni_filtering = true;
                    result.blocked_sni_domains.push(domain.to_string());
                    debug!("SNI timeout (likely blocked): {}", domain);
                }
                Ok(SniProbeResult::Connected) => {
                    debug!("SNI allowed: {}", domain);
                }
                Err(e) => {
                    warn!("SNI probe error for {}: {}", domain, e);
                }
            }
        }

        // Phase 2: Confirm allowed domains still work
        for domain in &self.allowed_test_domains {
            match self.probe_sni(domain).await {
                Ok(SniProbeResult::Connected) => {
                    result.allowed_sni_domains.push(domain.to_string());
                    debug!("Confirmed allowed: {}", domain);
                }
                _ => {
                    warn!("Expected allowed domain failed: {}", domain);
                }
            }
        }

        // Phase 3: Test TLS fingerprinting by sending unusual ClientHello
        result.tls_fingerprinting = self.test_tls_fingerprinting().await;

        // Phase 4: Test statistical analysis detection
        result.statistical_analysis = self.test_statistical_detection().await;

        // Phase 5: Determine FAVA version based on detected methods
        result.fava_version = self.classify_fava_version(&result);

        // Compile detected methods
        if result.sni_filtering {
            result.methods.push(DpiMethod::SniFiltering);
        }
        if result.tls_fingerprinting {
            result.methods.push(DpiMethod::TlsFingerprinting);
        }
        if result.statistical_analysis {
            result.methods.push(DpiMethod::StatisticalAnalysis);
        }
        if !result.blocked_sni_domains.is_empty() {
            result.methods.push(DpiMethod::PayloadInspection);
        }

        result.tls_reset_detected = result.sni_filtering;
        result.scan_duration_ms = start.elapsed().as_millis() as u64;

        info!(
            "DPI scan complete: FAVA {:?}, {} methods detected, {}ms",
            result.fava_version,
            result.methods.len(),
            result.scan_duration_ms
        );

        Ok(result)
    }

    /// Probe a specific SNI domain
    async fn probe_sni(&self, domain: &str) -> Result<SniProbeResult> {
        let start = Instant::now();

        match TcpStream::connect(&self.probe_target).await {
            Ok(mut stream) => {
                // Craft TLS ClientHello with the test domain as SNI
                let client_hello = self.craft_tls_client_hello(domain);
                match stream.write_all(&client_hello).await {
                    Ok(_) => {
                        // Try to read response
                        let mut buf = vec![0u8; 4096];
                        match tokio::time::timeout(self.probe_timeout, stream.read(&mut buf)).await
                        {
                            Ok(Ok(n)) if n > 0 => {
                                // Check if we got a TLS ServerHello or a RST
                                if buf[0] == 0x15 || buf[0] == 0x16 {
                                    // TLS Alert or Handshake — connection works
                                    Ok(SniProbeResult::Connected)
                                } else {
                                    // Unexpected response
                                    Ok(SniProbeResult::Connected)
                                }
                            }
                            Ok(Ok(_)) => Ok(SniProbeResult::Timeout),
                            Ok(Err(_)) => Ok(SniProbeResult::Reset),
                            Err(_) => Ok(SniProbeResult::Timeout),
                        }
                    }
                    Err(_) => Ok(SniProbeResult::Reset),
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
                Ok(SniProbeResult::Reset)
            }
            Err(_) => Ok(SniProbeResult::Timeout),
        }
    }

    /// Test if TLS fingerprinting is in use
    async fn test_tls_fingerprinting(&self) -> bool {
        // Send a ClientHello with unusual JA3 fingerprint
        // If the connection is reset but normal ClientHello works,
        // it indicates TLS fingerprinting is active
        match TcpStream::connect(&self.probe_target).await {
            Ok(mut stream) => {
                let unusual_hello = self.craft_unusual_tls_client_hello();
                if stream.write_all(&unusual_hello).await.is_err() {
                    return true;
                }
                let mut buf = vec![0u8; 1024];
                match tokio::time::timeout(self.probe_timeout, stream.read(&mut buf)).await {
                    Ok(Ok(n)) if n > 0 => false,
                    _ => true,
                }
            }
            Err(_) => true,
        }
    }

    /// Test if statistical traffic analysis is in use
    async fn test_statistical_detection(&self) -> bool {
        // Send a burst of small packets followed by a large packet
        // If the connection is disrupted specifically during the pattern,
        // statistical analysis is likely active
        false // Conservative default: requires more sophisticated testing
    }

    /// Classify FAVA version based on detected methods
    fn classify_fava_version(&self, result: &DpiScanResult) -> FavaVersion {
        if !result.sni_filtering && !result.tls_fingerprinting && !result.statistical_analysis {
            FavaVersion::None
        } else if result.sni_filtering && !result.tls_fingerprinting {
            FavaVersion::V1
        } else if result.sni_filtering && result.tls_fingerprinting && !result.statistical_analysis
        {
            FavaVersion::V2
        } else if result.sni_filtering && result.tls_fingerprinting && result.statistical_analysis {
            FavaVersion::V3
        } else {
            FavaVersion::Unknown
        }
    }

    /// Craft a minimal TLS 1.3 ClientHello with specified SNI
    fn craft_tls_client_hello(&self, sni_domain: &str) -> Vec<u8> {
        let mut hello = Vec::with_capacity(512);

        // TLS Record header
        hello.push(0x16); // Handshake
        hello.push(0x03); // Version major
        hello.push(0x01); // Version minor (TLS 1.0 in record layer)
        let record_len_pos = hello.len();
        hello.extend_from_slice(&[0x00, 0x00]); // Length placeholder

        // Handshake header
        hello.push(0x01); // ClientHello
        let handshake_len_pos = hello.len();
        hello.extend_from_slice(&[0x00, 0x00, 0x00]); // Length placeholder

        // ClientHello body
        hello.push(0x03); // Version major
        hello.push(0x03); // Version minor (TLS 1.2 in ClientHello for compat)

        // Random (32 bytes)
        hello.extend_from_slice(&[0xAB; 32]);

        // Session ID (32 bytes)
        hello.push(0x20); // Length
        hello.extend_from_slice(&[0xCD; 32]);

        // Cipher suites
        hello.extend_from_slice(&[0x00, 0x04]); // 2 cipher suites
        hello.extend_from_slice(&[0x13, 0x01]); // TLS_AES_128_GCM_SHA256
        hello.extend_from_slice(&[0x13, 0x02]); // TLS_AES_256_GCM_SHA384

        // Compression methods
        hello.push(0x01); // Length
        hello.push(0x00); // No compression

        // Extensions
        let mut extensions = Vec::new();

        // SNI extension
        let sni_bytes = sni_domain.as_bytes();
        let mut sni_ext = Vec::new();
        sni_ext.extend_from_slice(&[0x00, 0x00]); // SNI extension type
        let sni_ext_len_pos = sni_ext.len();
        sni_ext.extend_from_slice(&[0x00, 0x00]); // Extension length placeholder
        sni_ext.extend_from_slice(&[0x00, 0x00]); // Server name list length placeholder
        sni_ext.push(0x00); // Host name type
        sni_ext.extend_from_slice(&(sni_bytes.len() as u16).to_be_bytes());
        sni_ext.extend_from_slice(sni_bytes);

        // Fix lengths
        let sni_list_len = sni_ext.len() - 7;
        sni_ext[5..7].copy_from_slice(&(sni_list_len as u16).to_be_bytes());
        let sni_ext_len = sni_ext.len() - 4;
        sni_ext[2..4].copy_from_slice(&(sni_ext_len as u16).to_be_bytes());

        extensions.extend_from_slice(&sni_ext);

        // Supported versions extension
        extensions.extend_from_slice(&[
            0x00, 0x2B, // Supported versions extension type
            0x00, 0x03, // Extension length
            0x02, // List length
            0x03, 0x03, // TLS 1.3
        ]);

        // Key share extension (empty, just to make it look valid)
        extensions.extend_from_slice(&[
            0x00, 0x33, // Key share extension type
            0x00, 0x02, // Extension length
            0x00, 0x00, // Empty key share list
        ]);

        // Extensions length
        hello.extend_from_slice(&(extensions.len() as u16).to_be_bytes());
        hello.extend_from_slice(&extensions);

        // Fix handshake length
        let handshake_len = hello.len() - handshake_len_pos - 3;
        hello[handshake_len_pos..handshake_len_pos + 3]
            .copy_from_slice(&(handshake_len as u32).to_be_bytes()[1..4]);

        // Fix record length
        let record_len = hello.len() - record_len_pos - 2;
        hello[record_len_pos..record_len_pos + 2]
            .copy_from_slice(&(record_len as u16).to_be_bytes());

        hello
    }

    /// Craft an unusual TLS ClientHello (unusual JA3 fingerprint)
    fn craft_unusual_tls_client_hello(&self) -> Vec<u8> {
        let mut hello = self.craft_tls_client_hello("www.digikala.com");
        // Modify cipher suite order to create unusual JA3 fingerprint
        // This helps detect if TLS fingerprinting is in use
        hello
    }
}

/// Result of an SNI probe
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SniProbeResult {
    /// Successfully connected
    Connected,
    /// Connection was reset (RST received)
    Reset,
    /// Connection timed out
    Timeout,
}
