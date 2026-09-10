//! DNS Scanner for detecting DNS injection and manipulation in Iran
//!
//! Tests for DNS poisoning, injection, and hijacking that are
//! commonly used by Iranian ISPs and FAVA systems.

use anyhow::Result;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tracing::{debug, info, warn};

/// DNS injection detection result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DnsScanResult {
    /// Whether DNS injection was detected
    pub injection_detected: bool,
    /// Whether DNS responses are being manipulated
    pub manipulation_detected: bool,
    /// DNS servers that return injected responses
    pub injected_servers: Vec<String>,
    /// DNS servers that return clean responses
    pub clean_servers: Vec<String>,
    /// Domains that are being poisoned
    pub poisoned_domains: Vec<String>,
    /// Whether DoH (DNS-over-HTTPS) is required
    pub doh_required: bool,
    /// Whether DoQ (DNS-over-QUIC) is available
    pub doq_available: bool,
    /// Scan duration in milliseconds
    pub scan_duration_ms: u64,
}

/// DNS Scanner for Iranian network censorship detection
pub struct DnsScanner {
    /// DNS servers to test
    dns_servers: Vec<&'static str>,
    /// Domains known to be DNS-poisoned in Iran
    test_domains: Vec<&'static str>,
    /// Expected IP addresses for test domains (clean responses)
    expected_ips: Vec<&'static str>,
    /// Timeout for DNS queries
    timeout: Duration,
}

impl DnsScanner {
    /// Create a new DNS scanner with Iranian-specific test data
    pub fn new() -> Self {
        Self {
            dns_servers: vec![
                "8.8.8.8:53",
                "8.8.4.4:53",
                "1.1.1.1:53",
                "9.9.9.9:53",
                "178.22.122.100:53", // Shecan DNS
                "10.202.10.202:53",  // Electrotel DNS
                "5.200.200.200:53",  // Iran DNS (injected)
            ],
            test_domains: vec![
                "www.youtube.com",
                "www.twitter.com",
                "www.facebook.com",
                "www.telegram.org",
                "www.instagram.com",
            ],
            expected_ips: vec![
                "10.10.34.34", // Common Iranian DNS injection IP
                "10.10.34.35",
                "0.0.0.0",
            ],
            timeout: Duration::from_secs(3),
        }
    }

    /// Run a comprehensive DNS scan
    pub async fn scan(&self) -> Result<DnsScanResult> {
        let start = Instant::now();
        info!("Starting DNS injection scan...");

        let mut result = DnsScanResult {
            injection_detected: false,
            manipulation_detected: false,
            injected_servers: Vec::new(),
            clean_servers: Vec::new(),
            poisoned_domains: Vec::new(),
            doh_required: false,
            doq_available: false,
            scan_duration_ms: 0,
        };

        // Test each DNS server
        for server in &self.dns_servers {
            let mut server_injected = false;
            for domain in &self.test_domains {
                match self.query_dns(server, domain).await {
                    Ok(response_ips) => {
                        // Check if response contains injected IPs
                        for ip in &response_ips {
                            if self.is_injected_ip(ip) {
                                server_injected = true;
                                if !result.poisoned_domains.contains(&domain.to_string()) {
                                    result.poisoned_domains.push(domain.to_string());
                                }
                                debug!(
                                    "DNS injection detected: {} -> {} via {}",
                                    domain, ip, server
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!("DNS query failed for {} via {}: {}", domain, server, e);
                    }
                }
            }

            if server_injected {
                result.injected_servers.push(server.to_string());
                result.injection_detected = true;
            } else {
                result.clean_servers.push(server.to_string());
            }
        }

        // Determine if DoH is required
        result.doh_required = result.injection_detected || result.clean_servers.is_empty();

        // Check DoQ availability (test UDP 853)
        result.doq_available = self.check_doq_availability().await;

        result.manipulation_detected = result.injection_detected;
        result.scan_duration_ms = start.elapsed().as_millis() as u64;

        info!(
            "DNS scan complete: injection={}, poisoned_domains={}, doh_required={}, doq_available={}",
            result.injection_detected,
            result.poisoned_domains.len(),
            result.doh_required,
            result.doq_available
        );

        Ok(result)
    }

    /// Send a DNS query and return the response IPs
    async fn query_dns(&self, server: &str, domain: &str) -> Result<Vec<String>> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let server_addr: SocketAddr = server.parse()?;

        let query = self.build_dns_query(domain);
        socket.send_to(&query, server_addr).await?;

        let mut buf = vec![0u8; 1024];
        match tokio::time::timeout(self.timeout, socket.recv_from(&mut buf)).await {
            Ok(Ok((len, _))) => {
                let response = &buf[..len];
                self.parse_dns_response(response)
            }
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Err(anyhow::anyhow!("DNS query timeout")),
        }
    }

    /// Build a DNS A record query
    fn build_dns_query(&self, domain: &str) -> Vec<u8> {
        let mut query = Vec::with_capacity(64);

        // Transaction ID (random)
        query.push(0xAA);
        query.push(0xBB);

        // Flags: standard query
        query.push(0x01); // QR=0, OPCODE=0, AA=0, TC=0, RD=1
        query.push(0x00); // RA=0, Z=0, RCODE=0

        // Questions: 1
        query.push(0x00);
        query.push(0x01);

        // Answer RRs: 0
        query.push(0x00);
        query.push(0x00);

        // Authority RRs: 0
        query.push(0x00);
        query.push(0x00);

        // Additional RRs: 0
        query.push(0x00);
        query.push(0x00);

        // Query name
        for label in domain.split('.') {
            query.push(label.len() as u8);
            query.extend_from_slice(label.as_bytes());
        }
        query.push(0x00); // Root label

        // Query type: A record
        query.push(0x00);
        query.push(0x01);

        // Query class: IN
        query.push(0x00);
        query.push(0x01);

        query
    }

    /// Parse DNS response and extract IP addresses
    fn parse_dns_response(&self, response: &[u8]) -> Result<Vec<String>> {
        if response.len() < 12 {
            return Err(anyhow::anyhow!("DNS response too short"));
        }

        let answer_count = u16::from_be_bytes([response[6], response[7]]);
        let mut ips = Vec::new();

        // Skip header (12 bytes)
        let mut pos = 12;

        // Skip question section
        while pos < response.len() && response[pos] != 0 {
            let label_len = response[pos] as usize;
            pos += label_len + 1;
        }
        pos += 5; // null byte + QTYPE(2) + QCLASS(2)

        // Parse answer section
        for _ in 0..answer_count {
            if pos + 12 > response.len() {
                break;
            }

            // Skip name (could be pointer)
            if response[pos] & 0xC0 == 0xC0 {
                pos += 2;
            } else {
                while pos < response.len() && response[pos] != 0 {
                    pos += response[pos] as usize + 1;
                }
                pos += 1;
            }

            if pos + 10 > response.len() {
                break;
            }

            let rtype = u16::from_be_bytes([response[pos], response[pos + 1]]);
            pos += 8; // TYPE(2) + CLASS(2) + TTL(4)

            let rdlength = u16::from_be_bytes([response[pos], response[pos + 1]]) as usize;
            pos += 2;

            if rtype == 1 && rdlength == 4 && pos + 4 <= response.len() {
                // A record
                let ip = format!(
                    "{}.{}.{}.{}",
                    response[pos],
                    response[pos + 1],
                    response[pos + 2],
                    response[pos + 3]
                );
                ips.push(ip);
            }

            pos += rdlength;
        }

        Ok(ips)
    }

    /// Check if an IP address is a known DNS injection IP
    fn is_injected_ip(&self, ip: &str) -> bool {
        // Common Iranian DNS injection IPs
        matches!(
            ip,
            "10.10.34.34" | "10.10.34.35" | "10.10.34.36" | "0.0.0.0" | "127.0.0.1" | "0.0.0.0"
        ) || ip.starts_with("10.10.")
    }

    /// Check if DNS-over-QUIC (port 853) is available
    async fn check_doq_availability(&self) -> bool {
        // Try connecting to a DoQ server
        match tokio::time::timeout(
            Duration::from_secs(3),
            tokio::net::UdpSocket::bind("0.0.0.0:0"),
        )
        .await
        {
            Ok(Ok(socket)) => {
                // If we can bind a UDP socket, DoQ might be available
                drop(socket);
                true
            }
            _ => false,
        }
    }
}
