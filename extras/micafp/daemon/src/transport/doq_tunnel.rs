//! DNS-over-QUIC (DoQ) Covert Channel Transport
//!
//! Encodes proxy traffic as DNS queries/responses using the DoQ format
//! (RFC 9250). Runs on port 853 (standard DoQ port). Bypasses DPI
//! that only monitors HTTP/HTTPS traffic.
//!
//! The key insight: Iranian DPI systems heavily scrutinize HTTP and HTTPS
//! but generally pass DNS traffic (including DoQ on port 853) since
//! DNS is essential infrastructure. By encoding proxy data inside
//! DoQ query/response payloads, we can tunnel through undetected.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::RwLock;

use super::{make_connection, ShieldError, Transport, TransportConnection};

// ── Constants ───────────────────────────────────────────────────────────────

/// Standard DoQ port.
const DOQ_PORT: u16 = 853;

/// DoQ ALPN identifier per RFC 9250.
const DOQ_ALPN: &[u8] = b"doq";

/// Maximum DNS query name length (253 bytes).
const MAX_DNS_NAME_LEN: usize = 253;

/// Maximum DNS record data length (65535 bytes theoretical, practical ~4096).
const MAX_RDATA_LEN: usize = 4096;

/// DNS record type for our covert channel (private use range).
/// Using TYPE65400 (private use per RFC 6895).
const COVERT_QTYPE: u16 = 65400;

/// DNS class: IN (Internet).
const DNS_CLASS_IN: u16 = 1;

/// Base64 alphabet for encoding data in DNS names.
const BASE64URL_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Maximum data per DNS query (accounting for name encoding overhead).
const MAX_DATA_PER_QUERY: usize = 180;

/// Chunk size for splitting large payloads.
const CHUNK_SIZE: usize = 200;

// ── DNS Protocol Structures ─────────────────────────────────────────────────

/// DNS header (12 bytes).
#[derive(Debug, Clone)]
struct DnsHeader {
    id: u16,
    flags: u16,
    qdcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16,
}

impl DnsHeader {
    /// Create a standard query header.
    fn new_query(id: u16) -> Self {
        Self {
            id,
            flags: 0x0100, // Standard query with RD flag
            qdcount: 1,
            ancount: 0,
            nscount: 0,
            arcount: 1, // EDNS0
        }
    }

    /// Create a response header.
    fn new_response(id: u16) -> Self {
        Self {
            id,
            flags: 0x8180, // Standard response with RD and RA
            qdcount: 1,
            ancount: 1,
            nscount: 0,
            arcount: 1,
        }
    }

    /// Serialize to bytes.
    fn to_bytes(&self) -> [u8; 12] {
        let mut buf = [0u8; 12];
        buf[0..2].copy_from_slice(&self.id.to_be_bytes());
        buf[2..4].copy_from_slice(&self.flags.to_be_bytes());
        buf[4..6].copy_from_slice(&self.qdcount.to_be_bytes());
        buf[6..8].copy_from_slice(&self.ancount.to_be_bytes());
        buf[8..10].copy_from_slice(&self.nscount.to_be_bytes());
        buf[10..12].copy_from_slice(&self.arcount.to_be_bytes());
        buf
    }

    /// Parse from bytes.
    fn from_bytes(data: &[u8; 12]) -> Self {
        Self {
            id: u16::from_be_bytes([data[0], data[1]]),
            flags: u16::from_be_bytes([data[2], data[3]]),
            qdcount: u16::from_be_bytes([data[4], data[5]]),
            ancount: u16::from_be_bytes([data[6], data[7]]),
            nscount: u16::from_be_bytes([data[8], data[9]]),
            arcount: u16::from_be_bytes([data[10], data[11]]),
        }
    }
}

// ── DNS Name Encoding ───────────────────────────────────────────────────────

/// Encode arbitrary data as a DNS domain name using base64url.
///
/// Format: <data_base64>.<seq_num>.covert.digikala.com
///
/// The base64url-encoded data appears as subdomain labels,
/// which is indistinguishable from CDN-style subdomain naming.
struct DnsNameEncoder;

impl DnsNameEncoder {
    /// Encode data as a DNS query name.
    fn encode(data: &[u8], seq: u16) -> String {
        let encoded = Self::base64url_encode(data);

        // Split into DNS labels (max 63 bytes each)
        let mut name = String::new();
        let mut chars = encoded.chars().peekable();
        let mut label_count = 0;

        while chars.peek().is_some() && label_count < 5 {
            if !name.is_empty() {
                name.push('.');
            }
            let label: String = chars.by_ref().take(63).collect();
            name.push_str(&label);
            label_count += 1;
        }

        // Add sequence number and cover domain
        name.push_str(&format!(".{}.covert.digikala.com", seq));
        name
    }

    /// Decode data from a DNS query name.
    fn decode(name: &str) -> Result<(Vec<u8>, u16), ShieldError> {
        // Remove trailing dot if present
        let name = name.trim_end_matches('.');

        // Find the cover domain suffix
        let parts: Vec<&str> = name.split('.').collect();
        if parts.len() < 4 {
            return Err(ShieldError::Protocol("Invalid covert DNS name".into()));
        }

        // Extract sequence number (second to last before "covert.digikala.com")
        let seq_idx = parts.len() - 4; // Position of sequence number
        let seq: u16 = parts[seq_idx]
            .parse()
            .map_err(|_| ShieldError::Protocol("Invalid sequence number".into()))?;

        // Extract the encoded data labels
        let data_labels = &parts[..seq_idx];
        let encoded: String = data_labels.join("");

        let data = Self::base64url_decode(&encoded)?;
        Ok((data, seq))
    }

    /// Base64url encode (no padding).
    fn base64url_encode(data: &[u8]) -> String {
        let mut encoded = String::with_capacity((data.len() * 4 + 2) / 3);
        let mut i = 0;

        while i < data.len() {
            let b0 = data[i] as u32;
            let b1 = if i + 1 < data.len() {
                data[i + 1] as u32
            } else {
                0
            };
            let b2 = if i + 2 < data.len() {
                data[i + 2] as u32
            } else {
                0
            };

            encoded.push(BASE64URL_CHARS[((b0 >> 2) & 0x3F) as usize] as char);

            if i + 1 < data.len() {
                encoded.push(BASE64URL_CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
                if i + 2 < data.len() {
                    encoded
                        .push(BASE64URL_CHARS[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
                    encoded.push(BASE64URL_CHARS[(b2 & 0x3F) as usize] as char);
                } else {
                    encoded.push(BASE64URL_CHARS[((b1 & 0x0F) << 2) as usize] as char);
                }
            } else {
                encoded.push(BASE64URL_CHARS[((b0 & 0x03) << 4) as usize] as char);
            }

            i += 3;
        }

        encoded
    }

    /// Base64url decode.
    fn base64url_decode(encoded: &str) -> Result<Vec<u8>, ShieldError> {
        let mut result = Vec::with_capacity(encoded.len() * 3 / 4);
        let mut acc: u32 = 0;
        let mut bits: u32 = 0;

        for ch in encoded.chars() {
            let val = BASE64URL_CHARS
                .iter()
                .position(|&c| c as char == ch)
                .ok_or_else(|| ShieldError::Protocol("Invalid base64 char".into()))?;

            acc = (acc << 6) | val as u32;
            bits += 6;

            if bits >= 8 {
                bits -= 8;
                result.push((acc >> bits) as u8);
            }
        }

        Ok(result)
    }
}

// ── DNS Query Builder ───────────────────────────────────────────────────────

/// Build a DNS query packet carrying covert data.
struct DnsQueryBuilder;

impl DnsQueryBuilder {
    /// Build a DNS query packet with covert data in the query name.
    fn build_query(data: &[u8], seq: u16) -> Vec<u8> {
        let id = rand::random::<u16>();
        let header = DnsHeader::new_query(id);

        let mut packet = Vec::with_capacity(512);
        packet.extend_from_slice(&header.to_bytes());

        // Encode data as DNS name
        let name = DnsNameEncoder::encode(data, seq);

        // Write DNS name in wire format
        for label in name.split('.') {
            let label_bytes = label.as_bytes();
            packet.push(label_bytes.len() as u8);
            packet.extend_from_slice(label_bytes);
        }
        packet.push(0x00); // Root label

        // Query type (covert) and class
        packet.extend_from_slice(&COVERT_QTYPE.to_be_bytes());
        packet.extend_from_slice(&DNS_CLASS_IN.to_be_bytes());

        // EDNS0 OPT record (standard for DNS queries)
        packet.push(0x00); // Root name
        packet.extend_from_slice(&[0x00, 0x29]); // Type: OPT
        packet.extend_from_slice(&[0x10, 0x00]); // UDP payload size: 4096
        packet.extend_from_slice(&[0x00]); // Extended RCODE
        packet.extend_from_slice(&[0x00]); // EDNS version
        packet.extend_from_slice(&[0x80, 0x00]); // Flags: DO bit
        packet.extend_from_slice(&[0x00, 0x00]); // No options

        packet
    }

    /// Build a DNS response packet with covert data in the answer section.
    fn build_response(query_id: u16, data: &[u8], seq: u16) -> Vec<u8> {
        let header = DnsHeader::new_response(query_id);

        let mut packet = Vec::with_capacity(4096);
        packet.extend_from_slice(&header.to_bytes());

        // Question section (repeat the query name)
        let name = DnsNameEncoder::encode(&[], seq); // Empty data for question
        for label in name.split('.') {
            let label_bytes = label.as_bytes();
            packet.push(label_bytes.len() as u8);
            packet.extend_from_slice(label_bytes);
        }
        packet.push(0x00);
        packet.extend_from_slice(&COVERT_QTYPE.to_be_bytes());
        packet.extend_from_slice(&DNS_CLASS_IN.to_be_bytes());

        // Answer section
        // Name pointer (compress to question name)
        packet.extend_from_slice(&[0xC0, 0x0C]); // Pointer to offset 12
        packet.extend_from_slice(&COVERT_QTYPE.to_be_bytes());
        packet.extend_from_slice(&DNS_CLASS_IN.to_be_bytes());
        packet.extend_from_slice(&300u32.to_be_bytes()); // TTL: 5 minutes
        packet.extend_from_slice(&(data.len() as u16).to_be_bytes());
        packet.extend_from_slice(data);

        // EDNS0
        packet.push(0x00);
        packet.extend_from_slice(&[0x00, 0x29]);
        packet.extend_from_slice(&[0x10, 0x00]);
        packet.extend_from_slice(&[0x00, 0x00, 0x80, 0x00, 0x00, 0x00]);

        packet
    }
}

// ── Configuration ───────────────────────────────────────────────────────────

/// Configuration for DoQ tunnel transport.
#[derive(Debug, Clone)]
pub struct DoqTunnelConfig {
    /// DoQ server address.
    pub server_addr: SocketAddr,
    /// SNI domain for QUIC TLS (must be a legitimate DoQ resolver domain).
    pub sni_domain: String,
    /// Authentication key (shared secret).
    pub auth_key: String,
    /// Cover domain for DNS names.
    pub cover_domain: String,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Whether to allow insecure TLS.
    pub insecure: bool,
}

impl DoqTunnelConfig {
    /// Create a new DoQ tunnel config with defaults.
    pub fn new(server_addr: SocketAddr, auth_key: String) -> Self {
        Self {
            server_addr,
            sni_domain: "dns.digikala.com".to_string(),
            auth_key,
            cover_domain: "covert.digikala.com".to_string(),
            connect_timeout_secs: 10,
            insecure: false,
        }
    }
}

// ── Quinn stream wrapper ────────────────────────────────────────────────────

struct QuinnStream {
    send: quinn::SendStream,
    recv: quinn::RecvStream,
}

impl tokio::io::AsyncRead for QuinnStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.recv).poll_read(cx, buf)
    }
}

impl tokio::io::AsyncWrite for QuinnStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::pin::Pin::new(&mut self.send).poll_write(cx, buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.send).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.send).poll_shutdown(cx)
    }
}

// ── Certificate verifier ────────────────────────────────────────────────────

#[derive(Debug)]
struct DoqCertVerifier {
    insecure: bool,
}

impl DoqCertVerifier {
    fn new(insecure: bool) -> Self {
        Self { insecure }
    }
}

impl rustls::client::danger::ServerCertVerifier for DoqCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        if self.insecure {
            Ok(rustls::client::danger::ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(
                "Cert verification not implemented".into(),
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
        ]
    }
}

// ── DoQ Tunnel Transport ────────────────────────────────────────────────────

/// DNS-over-QUIC covert channel transport.
///
/// # How it works
///
/// 1. Establishes a QUIC connection to port 853 (standard DoQ port)
/// 2. Sends proxy data encoded as DNS queries
/// 3. Receives proxy data decoded from DNS responses
/// 4. DPI sees only standard DoQ traffic (port 853, ALPN "doq")
/// 5. DNS query names contain base64url-encoded data as subdomain labels
/// 6. DNS responses carry data in TXT-like record payloads
///
/// # Why this works in Iran
///
/// - Iran's DPI primarily monitors HTTP (port 80) and HTTPS (port 443)
/// - DoQ (port 853) is increasingly used by legitimate DNS resolvers
/// - Blocking DoQ would break DNS resolution for many applications
/// - The covert DNS queries look like normal subdomain lookups
pub struct DoqTunnelTransport {
    config: RwLock<DoqTunnelConfig>,
    last_error: RwLock<Option<ShieldError>>,
    active_connections: RwLock<usize>,
    available: RwLock<bool>,
    /// QUIC connection for reuse.
    quic_connection: RwLock<Option<quinn::Connection>>,
    /// Current sequence number for DNS queries.
    sequence: RwLock<u16>,
}

impl DoqTunnelTransport {
    /// Create a new DoQ tunnel transport.
    pub fn new(config: DoqTunnelConfig) -> Self {
        Self {
            config: RwLock::new(config),
            last_error: RwLock::new(None),
            active_connections: RwLock::new(0),
            available: RwLock::new(true),
            quic_connection: RwLock::new(None),
            sequence: RwLock::new(0),
        }
    }

    /// Establish a QUIC connection to the DoQ server.
    async fn establish_quic_connection(&self) -> Result<quinn::Connection, ShieldError> {
        let config = self.config.read().await;

        // Check for existing connection
        {
            let conn_guard = self.quic_connection.read().await;
            if let Some(conn) = conn_guard.as_ref() {
                if conn.close_reason().is_none() {
                    return Ok(conn.clone());
                }
            }
        }

        let mut crypto = rustls::client::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(DoqCertVerifier::new(config.insecure)))
            .with_no_client_auth();

        crypto.alpn_protocols = vec![DOQ_ALPN.to_vec()];

        let mut transport = quinn::TransportConfig::default();
        transport.max_idle_timeout(Some(Duration::from_secs(30).try_into().unwrap()));
        transport.stream_receive_window(MAX_RDATA_LEN.try_into().unwrap());

        let quic_crypto = quinn_proto::crypto::rustls::QuicClientConfig::try_from(crypto).map_err(|e| anyhow::anyhow!("quic crypto: {}", e))?;
        let mut client_config = quinn::ClientConfig::new(Arc::new(quic_crypto));
        client_config.transport_config(Arc::new(transport));

        let server_name = &config.sni_domain;
        let addr = config.server_addr;

        let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse().unwrap())
            .map_err(|e| ShieldError::QuicError(format!("Endpoint: {}", e)))?;

        endpoint.set_default_client_config(client_config);

        let timeout = Duration::from_secs(config.connect_timeout_secs);

        let connecting = endpoint.connect(addr, server_name)
            .map_err(|e| ShieldError::QuicError(format!("DoQ connect: {}", e)))?;
        let connection = tokio::time::timeout(timeout, connecting)
            .await
            .map_err(|_| ShieldError::Timeout("DoQ QUIC timeout".into()))?
            .map_err(|e| ShieldError::QuicError(format!("DoQ connect: {}", e)))?;

        // Store for reuse
        *self.quic_connection.write().await = Some(connection.clone());

        Ok(connection)
    }

    /// Send proxy data encoded as a DNS query and receive the response.
    async fn send_dns_query(&self, data: &[u8]) -> Result<Vec<u8>, ShieldError> {
        let connection = self.establish_quic_connection().await?;

        // Get next sequence number
        let seq = {
            let mut seq_guard = self.sequence.write().await;
            *seq_guard = seq_guard.wrapping_add(1);
            *seq_guard
        };

        // Build DNS query with covert data
        let query_packet = DnsQueryBuilder::build_query(data, seq);

        // DoQ: send query as a QUIC stream
        // Per RFC 9250, each DNS query is sent on a separate QUIC stream
        let (mut send, mut recv) = connection
            .open_bi()
            .await
            .map_err(|e| ShieldError::QuicError(format!("DoQ stream: {}", e)))?;

        // DoQ requires a 2-byte length prefix
        let len = query_packet.len() as u16;
        send.write_all(&len.to_be_bytes())
            .await
            .map_err(|e| ShieldError::QuicError(format!("DoQ length: {}", e)))?;

        send.write_all(&query_packet)
            .await
            .map_err(|e| ShieldError::QuicError(format!("DoQ query: {}", e)))?;

        send.finish()
            .map_err(|e| ShieldError::QuicError(format!("DoQ finish: {}", e)))?;

        // Read response with length prefix
        let mut len_buf = [0u8; 2];
        recv.read_exact(&mut len_buf)
            .await
            .map_err(|e| ShieldError::QuicError(format!("DoQ response length: {}", e)))?;

        let resp_len = u16::from_be_bytes(len_buf) as usize;
        let mut resp_buf = vec![0u8; resp_len];
        recv.read_exact(&mut resp_buf)
            .await
            .map_err(|e| ShieldError::QuicError(format!("DoQ response body: {}", e)))?;

        // Parse DNS response and extract covert data
        if resp_buf.len() < 12 {
            return Err(ShieldError::Protocol("DoQ response too short".into()));
        }

        // The answer section contains the covert data in RDATA
        // Skip header (12) and question section to find answer
        let mut offset = 12;

        // Skip question section
        while offset < resp_buf.len() && resp_buf[offset] != 0 {
            if resp_buf[offset] & 0xC0 == 0xC0 {
                offset += 2; // Pointer
                break;
            }
            let label_len = resp_buf[offset] as usize;
            offset += 1 + label_len;
        }
        offset += 5; // Root label + QTYPE(2) + QCLASS(2)

        // Parse answer section
        if offset + 12 <= resp_buf.len() {
            // Skip name (could be pointer)
            if resp_buf[offset] & 0xC0 == 0xC0 {
                offset += 2;
            } else {
                while offset < resp_buf.len() && resp_buf[offset] != 0 {
                    offset += 1 + resp_buf[offset] as usize;
                }
                offset += 1;
            }

            offset += 8; // TYPE(2) + CLASS(2) + TTL(4)

            if offset + 2 <= resp_buf.len() {
                let rdlength =
                    u16::from_be_bytes([resp_buf[offset], resp_buf[offset + 1]]) as usize;
                offset += 2;

                if offset + rdlength <= resp_buf.len() {
                    return Ok(resp_buf[offset..offset + rdlength].to_vec());
                }
            }
        }

        Ok(vec![])
    }

    /// Send proxy data through DoQ tunnel, splitting into chunks.
    async fn tunnel_data(&self, data: &[u8]) -> Result<Vec<u8>, ShieldError> {
        if data.len() <= MAX_DATA_PER_QUERY {
            return self.send_dns_query(data).await;
        }

        // Split into chunks and send sequentially
        let mut result = Vec::new();
        for chunk in data.chunks(MAX_DATA_PER_QUERY) {
            let response = self.send_dns_query(chunk).await?;
            result.extend_from_slice(&response);
        }

        Ok(result)
    }
}

#[async_trait]
impl Transport for DoqTunnelTransport {
    fn name(&self) -> &str {
        "doq-tunnel"
    }

    fn priority(&self) -> u8 {
        7
    }

    async fn connect(&self, addr: &SocketAddr) -> Result<Box<dyn TransportConnection>, ShieldError> {
        let sni = self.config.read().await.sni_domain.clone();

        // Establish QUIC connection
        let connection = self.establish_quic_connection().await?;

        // Send initial connection request with destination address
        let dest_bytes = addr.to_string().into_bytes();
        let response = self.tunnel_data(&dest_bytes).await?;

        if response.is_empty() || response[0] != 0x00 {
            return Err(ShieldError::AuthFailed("DoQ tunnel auth failed".into()));
        }

        // Open a persistent bidirectional stream for data
        let (send, recv) = connection
            .open_bi()
            .await
            .map_err(|e| ShieldError::QuicError(format!("DoQ data stream: {}", e)))?;

        let stream = QuinnStream { send, recv };

        *self.active_connections.write().await += 1;
        *self.available.write().await = true;
        *self.last_error.write().await = None;

        Ok(make_connection(
            stream,
            sni,
            self.name().to_string(),
        ))
    }

    async fn is_available(&self) -> bool {
        *self.available.read().await
    }

    fn last_error(&self) -> Option<&ShieldError> {
        None
    }

    fn current_sni_domain(&self) -> &str {
        ""
    }

    async fn rotate_sni_domain(&self) -> Result<String, ShieldError> {
        let domains = ["dns.digikala.com", "dns.snapp.ir", "dns.divar.ir"];
        let mut config = self.config.write().await;
        let current = &config.sni_domain;
        let next = domains
            .iter()
            .find(|d| **d != current.as_str())
            .or_else(|| domains.first())
            .unwrap();
        config.sni_domain = next.to_string();
        *self.quic_connection.write().await = None;
        Ok(next.to_string())
    }

    fn active_connections(&self) -> usize {
        0
    }

    async fn shutdown(&self) -> Result<(), ShieldError> {
        *self.available.write().await = false;
        *self.active_connections.write().await = 0;
        if let Some(conn) = self.quic_connection.write().await.take() {
            conn.close(0u32.into(), b"shutdown");
        }
        Ok(())
    }
}

impl DoqTunnelTransport {
    pub async fn get_last_error(&self) -> Option<ShieldError> {
        self.last_error.read().await.clone()
    }

    pub async fn get_current_sni_domain(&self) -> String {
        self.config.read().await.sni_domain.clone()
    }

    pub async fn get_active_connections(&self) -> usize {
        *self.active_connections.read().await
    }
}
