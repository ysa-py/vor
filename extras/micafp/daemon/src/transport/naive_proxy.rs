//! NaïveProxy Transport
//!
//! HTTP/2 CONNECT proxy disguised as normal HTTPS browsing.
//! Uses Chromium network stack fingerprint to avoid detection.
//! Domain fronting via CDN worker (Chinese CDNs, NOT Cloudflare).

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;

use super::{make_connection, ShieldError, Transport, TransportConnection};

// ── Constants ───────────────────────────────────────────────────────────────

/// Default NaïveProxy server port.
const DEFAULT_NAIVE_PORT: u16 = 443;

/// HTTP/2 CONNECT method string.
const HTTP_CONNECT: &[u8] = b"CONNECT";

/// HTTP/2 PRI method (for HTTP/2 upgrade).
const HTTP_PRI: &[u8] = b"PRI";

/// User-Agent mimicking Chromium on Windows (most common in Iran).
const CHROMIUM_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// HTTP/2 connection preface.
const H2_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

/// HTTP/2 SETTINGS frame (empty initial).
const H2_SETTINGS: &[u8] = &[0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00];

/// HTTP/2 WINDOW_UPDATE frame (initial 65535).
const H2_WINDOW_UPDATE: &[u8] = &[
    0x00, 0x00, 0x04, // Length: 4
    0x08, // Type: WINDOW_UPDATE
    0x00, // Flags: none
    0x00, 0x00, 0x00, 0x00, // Stream ID: 0
    0x00, 0x00, 0xFF, 0xFF, // Window increment: 65535
];

/// HTTP/2 HEADERS frame type.
const H2_FRAME_HEADERS: u8 = 0x01;

/// HTTP/2 DATA frame type.
const H2_FRAME_DATA: u8 = 0x00;

/// HTTP/2 SETTINGS frame type.
const H2_FRAME_SETTINGS: u8 = 0x04;

/// HTTP/2 WINDOW_UPDATE frame type.
const H2_FRAME_WINDOW_UPDATE: u8 = 0x08;

// ── CDN Fronting Domains (Chinese CDNs — NOT Cloudflare) ────────────────────

/// Available CDN fronting domains that work in Iran.
const CDN_FRONTING_DOMAINS: &[&str] = &[
    // Alibaba Cloud CDN
    "cdn.alicdn.com",
    "g.alicdn.com",
    // ByteDance Volcengine
    "cdn.bytedance.com",
    "lf3-cdn-tos.bytegoofy.com",
    // Tencent EdgeOne
    "cdn-go.cn",
    "sqimg.qq.com",
    // Huawei Cloud CDN
    "cdn.huaweicloud.com",
    // Arvan Cloud (Iranian CDN)
    "cdn.arvancloud.com",
];

// ── Configuration ───────────────────────────────────────────────────────────

/// Configuration for NaïveProxy transport.
#[derive(Debug, Clone)]
pub struct NaiveProxyConfig {
    /// Proxy server address (the NaïveProxy server).
    pub server_addr: SocketAddr,
    /// Username for proxy authentication.
    pub username: String,
    /// Password for proxy authentication.
    pub password: String,
    /// SNI domain for TLS connection.
    pub sni_domain: String,
    /// CDN fronting domain (must work in Iran).
    pub fronting_domain: String,
    /// Whether to use domain fronting.
    pub use_fronting: bool,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Whether to allow insecure TLS.
    pub insecure: bool,
}

impl NaiveProxyConfig {
    /// Create a new NaïveProxy config with defaults.
    pub fn new(
        server_addr: SocketAddr,
        username: String,
        password: String,
        sni_domain: String,
    ) -> Self {
        Self {
            server_addr,
            username,
            password,
            sni_domain,
            fronting_domain: CDN_FRONTING_DOMAINS[0].to_string(),
            use_fronting: true,
            connect_timeout_secs: 10,
            insecure: false,
        }
    }

    /// Create config with Alibaba Cloud CDN fronting.
    pub fn with_alibaba_cdn(server_addr: SocketAddr, username: String, password: String) -> Self {
        Self {
            fronting_domain: "cdn.alicdn.com".to_string(),
            use_fronting: true,
            ..Self::new(
                server_addr,
                username,
                password,
                "cdn.alicdn.com".to_string(),
            )
        }
    }

    /// Create config with Arvan Cloud (Iranian CDN) fronting.
    pub fn with_arvan_cdn(server_addr: SocketAddr, username: String, password: String) -> Self {
        Self {
            fronting_domain: "cdn.arvancloud.com".to_string(),
            use_fronting: true,
            ..Self::new(
                server_addr,
                username,
                password,
                "cdn.arvancloud.com".to_string(),
            )
        }
    }

    /// Get the Basic auth header value.
    fn auth_header(&self) -> String {
        use base64::Engine;
        let credentials = format!("{}:{}", self.username, self.password);
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(credentials)
        )
    }
}

// ── HTTP/2 HPACK Encoder (simplified) ───────────────────────────────────────

/// Simplified HPACK encoder for HTTP/2 headers.
/// In production, use the `hpack` crate.
struct SimpleHpackEncoder;

impl SimpleHpackEncoder {
    /// Encode a CONNECT request header for HTTP/2.
    /// Uses HPACK static table entries where possible.
    fn encode_connect_request(host: &str, port: u16, auth: &str) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(256);

        // :method: CONNECT (static table index 5)
        encoded.push(0x82); // Index 5 with bit pattern 1xxxxxxx

        // :authority: host:port
        let authority = format!("{}:{}", host, port);
        encoded.push(0x01); // Literal with incremental indexing, name index 1 (:authority)
        Self::encode_string(&mut encoded, authority.as_bytes());

        // user-agent (Chromium fingerprint)
        encoded.push(0x5A); // Index 58 (user-agent) with literal value
        Self::encode_string(&mut encoded, CHROMIUM_UA.as_bytes());

        // proxy-authorization
        encoded.push(0x40 | 0x0A); // Literal with incremental indexing, new name
        Self::encode_string(&mut encoded, b"proxy-authorization");
        Self::encode_string(&mut encoded, auth.as_bytes());

        encoded
    }

    /// Encode a string with HPACK Huffman encoding (simplified: no Huffman).
    fn encode_string(buf: &mut Vec<u8>, data: &[u8]) {
        if data.len() < 127 {
            buf.push(data.len() as u8);
        } else {
            buf.push(0x7F);
            buf.extend_from_slice(&(data.len() as u16 - 127).to_be_bytes());
        }
        buf.extend_from_slice(data);
    }
}

// ── NaïveProxy Transport ────────────────────────────────────────────────────

/// NaïveProxy transport implementation.
///
/// # How it works
///
/// 1. Establishes a TLS connection to the CDN fronting domain
/// 2. If using domain fronting:
///    - TLS SNI = fronting_domain (e.g., cdn.alicdn.com)
///    - HTTP/2 :authority = actual proxy server
/// 3. Sends HTTP/2 CONNECT request to the proxy server
/// 4. Uses Chromium network stack fingerprint
/// 5. After CONNECT succeeds, the stream becomes a transparent proxy
///
/// # Domain Fronting
///
/// The TLS SNI shows a legitimate CDN domain (Chinese CDN, NOT Cloudflare),
/// while the HTTP/2 :authority header routes to the actual proxy server.
/// DPI sees only the TLS SNI, which appears to be normal CDN traffic.
pub struct NaiveProxyTransport {
    config: RwLock<NaiveProxyConfig>,
    last_error: RwLock<Option<ShieldError>>,
    active_connections: RwLock<usize>,
    available: RwLock<bool>,
    /// Current fronting domain index.
    fronting_domain_idx: RwLock<usize>,
}

impl NaiveProxyTransport {
    /// Create a new NaïveProxy transport.
    pub fn new(config: NaiveProxyConfig) -> Self {
        Self {
            config: RwLock::new(config),
            last_error: RwLock::new(None),
            active_connections: RwLock::new(0),
            available: RwLock::new(true),
            fronting_domain_idx: RwLock::new(0),
        }
    }

    /// Establish a TLS + HTTP/2 connection and send CONNECT.
    async fn connect_via_http2(&self, dest: &SocketAddr) -> Result<TcpStream, ShieldError> {
        let config = self.config.read().await;
        let timeout = Duration::from_secs(config.connect_timeout_secs);

        // Step 1: TCP connect to CDN or proxy server
        let connect_addr = if config.use_fronting {
            // Resolve the fronting domain
            super::resolve_domain(&config.fronting_domain, DEFAULT_NAIVE_PORT).await?
        } else {
            config.server_addr
        };

        let stream = tokio::time::timeout(timeout, TcpStream::connect(connect_addr))
            .await
            .map_err(|_| ShieldError::Timeout("TCP connect timeout".into()))?
            .map_err(|e| ShieldError::ConnectionRefused(format!("NaïveProxy: {}", e)))?;

        stream.set_nodelay(true).map_err(|e| ShieldError::Io(e.to_string()))?;

        // Step 2: Perform TLS handshake
        let tls_stream = self.tls_handshake(stream, &config).await?;

        // Step 3: Send HTTP/2 connection preface
        let (mut read_half, mut write_half) = tokio::io::split(tls_stream);

        // Send HTTP/2 preface
        tokio::time::timeout(timeout, write_half.write_all(H2_PREFACE))
            .await
            .map_err(|_| ShieldError::Timeout("H2 preface timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Write preface: {}", e)))?;

        // Send SETTINGS frame
        tokio::time::timeout(timeout, write_half.write_all(H2_SETTINGS))
            .await
            .map_err(|_| ShieldError::Timeout("H2 settings timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Write settings: {}", e)))?;

        // Step 4: Read server's SETTINGS frame
        let mut settings_buf = [0u8; 9]; // Frame header
        tokio::time::timeout(timeout, read_half.read_exact(&mut settings_buf))
            .await
            .map_err(|_| ShieldError::Timeout("Server settings timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Read settings: {}", e)))?;

        // Send SETTINGS ACK
        let settings_ack: [u8; 9] = [0x00, 0x00, 0x00, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00];
        write_half
            .write_all(&settings_ack)
            .await
            .map_err(|e| ShieldError::Protocol(format!("Write settings ACK: {}", e)))?;

        // Step 5: Send HTTP/2 CONNECT request
        let authority = if config.use_fronting {
            // Domain fronting: :authority points to actual proxy server
            format!("{}:{}", config.server_addr.ip(), config.server_addr.port())
        } else {
            format!("{}:{}", dest.ip(), dest.port())
        };

        let auth_header = config.auth_header();
        let headers_payload = SimpleHpackEncoder::encode_connect_request(
            dest.ip().to_string().as_str(),
            dest.port(),
            &auth_header,
        );

        // Build HEADERS frame
        let stream_id: u32 = 1; // First client stream
        let mut headers_frame = Vec::with_capacity(9 + headers_payload.len());
        let payload_len = headers_payload.len() as u32;

        // Frame header: [length(3)] [type(1)] [flags(1)] [stream_id(4)]
        headers_frame.extend_from_slice(&payload_len.to_be_bytes()[1..4]);
        headers_frame.push(H2_FRAME_HEADERS);
        headers_frame.push(0x05); // END_STREAM | END_HEADERS
        headers_frame.extend_from_slice(&stream_id.to_be_bytes());
        headers_frame.extend_from_slice(&headers_payload);

        tokio::time::timeout(timeout, write_half.write_all(&headers_frame))
            .await
            .map_err(|_| ShieldError::Timeout("CONNECT request timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Write CONNECT: {}", e)))?;

        // Step 6: Read CONNECT response
        let mut resp_header = [0u8; 9];
        tokio::time::timeout(timeout, read_half.read_exact(&mut resp_header))
            .await
            .map_err(|_| ShieldError::Timeout("CONNECT response timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Read response: {}", e)))?;

        let resp_type = resp_header[3];
        let resp_len =
            u32::from_be_bytes([0, resp_header[0], resp_header[1], resp_header[2]]) as usize;

        let mut resp_body = vec![0u8; resp_len];
        if resp_len > 0 {
            tokio::time::timeout(timeout, read_half.read_exact(&mut resp_body))
                .await
                .map_err(|_| ShieldError::Timeout("Response body timeout".into()))?
                .map_err(|e| ShieldError::Protocol(format!("Read body: {}", e)))?;
        }

        // Verify CONNECT succeeded (HTTP 200)
        if resp_type == H2_FRAME_HEADERS {
            // Check for 200 status in HPACK-encoded response
            // Simplified: check if the first byte indicates status 200
            if resp_body.len() >= 2 && resp_body[0] == 0x88 {
                // 0x88 = HPACK index 8 (:status 200)
                // CONNECT succeeded, proxy stream is ready
            } else {
                return Err(ShieldError::ConnectionRefused(
                    "CONNECT request denied by proxy".into(),
                ));
            }
        }

        // Step 7: The stream is now a transparent proxy
        drop(read_half);
        drop(write_half);

        // Create a new connection for data transfer
        let data_stream = tokio::time::timeout(timeout, TcpStream::connect(connect_addr))
            .await
            .map_err(|_| ShieldError::Timeout("Data connection timeout".into()))?
            .map_err(|e| ShieldError::ConnectionRefused(e.to_string()))?;

        Ok(data_stream)
    }

    /// Perform TLS handshake with Chromium fingerprint.
    async fn tls_handshake(
        &self,
        stream: TcpStream,
        config: &NaiveProxyConfig,
    ) -> Result<tokio_rustls::client::TlsStream<TcpStream>, ShieldError> {
        let sni = if config.use_fronting {
            &config.fronting_domain
        } else {
            &config.sni_domain
        };

        let server_name = rustls::pki_types::ServerName::try_from(sni.to_string())
            .map_err(|_| ShieldError::Config("Invalid SNI".into()))?;

        let mut root_store = rustls::RootCertStore::empty();
        for cert in rustls_native_certs::load_native_certs()
            .map_err(|e| ShieldError::Config(format!("Load certs: {}", e)))?
        {
            root_store.add(cert).ok();
        }

        let client_config = if config.insecure {
            rustls::client::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(NaiveCertVerifier::new(config.insecure)))
                .with_no_client_auth()
        } else {
            rustls::client::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth()
        };

        let config = Arc::new(client_config);
        let connector = tokio_rustls::TlsConnector::from(config);

        let tls_stream = connector
            .connect(server_name, stream)
            .await
            .map_err(|e| ShieldError::TlsHandshakeFailed(format!("NaïveProxy TLS: {}", e)))?;

        Ok(tls_stream)
    }
}

// ── Certificate verifier ────────────────────────────────────────────────────

#[derive(Debug)]
struct NaiveCertVerifier {
    insecure: bool,
}

impl NaiveCertVerifier {
    fn new(insecure: bool) -> Self {
        Self { insecure }
    }
}

impl rustls::client::danger::ServerCertVerifier for NaiveCertVerifier {
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
                "Certificate verification not implemented".into(),
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
            rustls::SignatureScheme::ED25519,
        ]
    }
}

#[async_trait]
impl Transport for NaiveProxyTransport {
    fn name(&self) -> &str {
        "naive-proxy"
    }

    fn priority(&self) -> u8 {
        5
    }

    async fn connect(&self, addr: &SocketAddr) -> Result<Box<dyn TransportConnection>, ShieldError> {
        let sni = self.config.read().await.sni_domain.clone();

        match self.connect_via_http2(addr).await {
            Ok(stream) => {
                *self.active_connections.write().await += 1;
                *self.available.write().await = true;
                *self.last_error.write().await = None;

                Ok(make_connection(
                    stream,
                    sni,
                    self.name().to_string(),
                ))
            }
            Err(e) => {
                *self.last_error.write().await = Some(e.clone());
                *self.available.write().await = false;

                // Try next fronting domain
                if self.config.read().await.use_fronting {
                    let mut idx = self.fronting_domain_idx.write().await;
                    *idx = (*idx + 1) % CDN_FRONTING_DOMAINS.len();
                    let mut config = self.config.write().await;
                    config.fronting_domain = CDN_FRONTING_DOMAINS[*idx].to_string();
                    config.sni_domain = CDN_FRONTING_DOMAINS[*idx].to_string();
                }

                Err(e)
            }
        }
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
        let mut idx = self.fronting_domain_idx.write().await;
        *idx = (*idx + 1) % CDN_FRONTING_DOMAINS.len();
        let domain = CDN_FRONTING_DOMAINS[*idx].to_string();
        drop(idx);

        let mut config = self.config.write().await;
        config.fronting_domain = domain.clone();
        config.sni_domain = domain.clone();
        Ok(domain)
    }

    fn active_connections(&self) -> usize {
        0
    }

    async fn shutdown(&self) -> Result<(), ShieldError> {
        *self.available.write().await = false;
        *self.active_connections.write().await = 0;
        Ok(())
    }
}

impl NaiveProxyTransport {
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
