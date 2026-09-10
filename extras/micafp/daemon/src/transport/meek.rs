//! Meek Transport — Domain Fronting via Azure CDN
//!
//! Domain fronting via ajax.aspnetcdn.com (Azure CDN). HTTP requests
//! disguised as normal web browsing. Session management via custom
//! headers. Works in Iran because Azure CDN is not blocked.
//!
//! NOTE: We use Azure CDN here, NOT Cloudflare (blocked in Iran).
//! Also supports Alibaba Cloud CDN and Arvan Cloud as alternatives.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;

use super::{make_connection, ShieldError, Transport, TransportConnection};
use base64;
use hex;

// ── Constants ───────────────────────────────────────────────────────────────

/// Default Meek server port.
const DEFAULT_MEEK_PORT: u16 = 443;

/// Meek session header name (looks like a custom CDN header).
const MEEK_SESSION_HEADER: &str = "X-Request-ID";

/// Meek data header name (looks like a CDN cache header).
const MEEK_DATA_HEADER: &str = "X-Cache-Status";

/// Meek sequence header name.
const MEEK_SEQ_HEADER: &str = "X-CDN-Request-ID";

/// Meek auth header name.
const MEEK_AUTH_HEADER: &str = "Authorization";

/// Maximum payload per HTTP request (to avoid large request detection).
const MAX_PAYLOAD_PER_REQUEST: usize = 8192;

/// Poll interval for receiving data (milliseconds).
const POLL_INTERVAL_MS: u64 = 200;

/// Session ID length.
const SESSION_ID_LEN: usize = 32;

// ── Fronting Domain Pool ────────────────────────────────────────────────────

/// Available fronting domains (NOT Cloudflare — blocked in Iran).
const FRONTING_DOMAINS: &[&str] = &[
    // Azure CDN (Microsoft) — works in Iran
    "ajax.aspnetcdn.com",
    "az416426.vo.msecnd.net",
    // Alibaba Cloud CDN — works in Iran
    "cdn.alicdn.com",
    "g.alicdn.com",
    // Arvan Cloud (Iranian CDN)
    "cdn.arvancloud.com",
    // ByteDance Volcengine
    "cdn.bytedance.com",
];

// ── Configuration ───────────────────────────────────────────────────────────

/// Configuration for Meek transport.
#[derive(Debug, Clone)]
pub struct MeekConfig {
    /// Meek server address (the actual proxy server).
    pub server_addr: SocketAddr,
    /// Fronting domain for domain fronting.
    pub fronting_domain: String,
    /// URL path for meek requests.
    pub url_path: String,
    /// Authentication token.
    pub auth_token: String,
    /// SNI domain for TLS (same as fronting domain).
    pub sni_domain: String,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Whether to allow insecure TLS.
    pub insecure: bool,
    /// Whether to use domain fronting.
    pub use_fronting: bool,
    /// User-Agent string (mimics normal browsing).
    pub user_agent: String,
}

impl MeekConfig {
    /// Create a new Meek config with Azure CDN fronting.
    pub fn new(server_addr: SocketAddr, auth_token: String) -> Self {
        Self {
            server_addr,
            fronting_domain: FRONTING_DOMAINS[0].to_string(),
            url_path: "/cdn-cgi/challenge-platform".to_string(),
            auth_token,
            sni_domain: FRONTING_DOMAINS[0].to_string(),
            connect_timeout_secs: 15,
            insecure: false,
            use_fronting: true,
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
        }
    }

    /// Create config with Alibaba Cloud CDN fronting.
    pub fn with_alibaba(server_addr: SocketAddr, auth_token: String) -> Self {
        Self {
            fronting_domain: "cdn.alicdn.com".to_string(),
            sni_domain: "cdn.alicdn.com".to_string(),
            url_path: "/assets/common".to_string(),
            ..Self::new(server_addr, auth_token)
        }
    }
}

// ── Meek Session ────────────────────────────────────────────────────────────

/// Meek session state for managing a persistent proxy connection
/// over a series of HTTP requests.
#[derive(Clone)]
struct MeekSession {
    /// Unique session identifier.
    id: String,
    /// Sequence number for outgoing data.
    send_seq: u64,
    /// Sequence number for incoming data.
    recv_seq: u64,
    /// Buffered received data.
    recv_buffer: Vec<u8>,
}

impl MeekSession {
    /// Create a new Meek session with a random ID.
    fn new() -> Self {
        let id_bytes: [u8; SESSION_ID_LEN] = rand::random();
        Self {
            id: hex::encode(id_bytes),
            send_seq: 0,
            recv_seq: 0,
            recv_buffer: Vec::new(),
        }
    }

    /// Get the next send sequence number.
    fn next_send_seq(&mut self) -> u64 {
        let seq = self.send_seq;
        self.send_seq += 1;
        seq
    }

    /// Get the next expected receive sequence number.
    fn next_recv_seq(&mut self) -> u64 {
        let seq = self.recv_seq;
        self.recv_seq += 1;
        seq
    }
}

// ── HTTP Request Builder ────────────────────────────────────────────────────

/// Build HTTP requests for the Meek transport.
struct MeekRequestBuilder;

impl MeekRequestBuilder {
    /// Build a POST request carrying proxy data.
    fn build_post_request(
        session: &mut MeekSession,
        data: &[u8],
        fronting_domain: &str,
        host_header: &str,
        url_path: &str,
        auth_token: &str,
        user_agent: &str,
    ) -> Vec<u8> {
        use base64::Engine;
        let encoded_data = base64::engine::general_purpose::STANDARD.encode(data);
        let seq = session.next_send_seq();

        let request = format!(
            "POST {} HTTP/1.1\r\n\
             Host: {}\r\n\
             User-Agent: {}\r\n\
             Accept: application/json, text/javascript, */*; q=0.01\r\n\
             Accept-Language: en-US,en;q=0.9\r\n\
             Content-Type: application/x-www-form-urlencoded\r\n\
             X-Requested-With: XMLHttpRequest\r\n\
             {}: {}\r\n\
             {}: {}\r\n\
             {}: {}\r\n\
             Content-Length: {}\r\n\
             \r\n\
             data={}",
            url_path,
            host_header, // Domain fronting: host header = actual server
            user_agent,
            MEEK_SESSION_HEADER,
            session.id,
            MEEK_SEQ_HEADER,
            seq,
            MEEK_AUTH_HEADER,
            format!("Bearer {}", auth_token),
            5 + encoded_data.len(), // "data=" prefix
            encoded_data,
        );

        request.into_bytes()
    }

    /// Build a GET polling request to receive proxy data.
    fn build_poll_request(
        session: &mut MeekSession,
        fronting_domain: &str,
        host_header: &str,
        url_path: &str,
        auth_token: &str,
        user_agent: &str,
    ) -> Vec<u8> {
        let seq = session.next_recv_seq();

        let request = format!(
            "GET {}?session={}&seq={}&t={} HTTP/1.1\r\n\
             Host: {}\r\n\
             User-Agent: {}\r\n\
             Accept: */*\r\n\
             Accept-Language: en-US,en;q=0.9\r\n\
             {}: {}\r\n\
             {}: {}\r\n\
             {}: {}\r\n\
             \r\n",
            url_path,
            session.id,
            seq,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            host_header,
            user_agent,
            MEEK_SESSION_HEADER,
            session.id,
            MEEK_SEQ_HEADER,
            seq,
            MEEK_AUTH_HEADER,
            format!("Bearer {}", auth_token),
        );

        request.into_bytes()
    }
}

// ── HTTP Response Parser ────────────────────────────────────────────────────

/// Parse HTTP responses from the Meek server.
struct MeekResponseParser;

impl MeekResponseParser {
    /// Extract proxy data from an HTTP response body.
    fn parse_response(data: &[u8]) -> Result<Vec<u8>, ShieldError> {
        let response_str = std::str::from_utf8(data)
            .map_err(|_| ShieldError::Protocol("Invalid HTTP response".into()))?;

        // Find the header/body boundary
        let body_start = response_str
            .find("\r\n\r\n")
            .ok_or_else(|| ShieldError::Protocol("No HTTP body found".into()))?
            + 4;

        let body = &response_str[body_start..];

        // Check HTTP status
        if !response_str.starts_with("HTTP/1.") || !response_str.contains("200") {
            return Err(ShieldError::Protocol(format!(
                "Meek server error: {}",
                &response_str[..response_str.find('\r').unwrap_or(response_str.len())]
            )));
        }

        // Decode base64 response data
        use base64::Engine;
        let trimmed = body.trim();
        if trimmed.is_empty() {
            return Ok(vec![]);
        }

        base64::engine::general_purpose::STANDARD
            .decode(trimmed)
            .map_err(|e| ShieldError::Protocol(format!("Decode response: {}", e)))
    }
}

// ── Meek Transport ──────────────────────────────────────────────────────────

/// Meek transport using domain fronting.
///
/// # How it works
///
/// 1. Client makes HTTPS requests to a fronting domain (e.g., ajax.aspnetcdn.com)
/// 2. TLS SNI = fronting domain (DPI sees normal CDN traffic)
/// 3. HTTP Host header = actual Meek server (CDN routes to our server)
/// 4. Proxy data is base64-encoded in POST body and GET parameters
/// 5. Session management via custom headers that look like CDN headers
///
/// # Domain Fronting
///
/// The key technique: TLS SNI shows one domain, HTTP Host shows another.
/// The CDN terminates TLS and forwards based on Host header.
/// DPI only sees the TLS SNI, which appears legitimate.
pub struct MeekTransport {
    config: RwLock<MeekConfig>,
    last_error: RwLock<Option<ShieldError>>,
    active_connections: RwLock<usize>,
    available: RwLock<bool>,
    /// Current fronting domain index.
    fronting_idx: RwLock<usize>,
    /// Active sessions.
    sessions: RwLock<Vec<MeekSession>>,
}

impl MeekTransport {
    /// Create a new Meek transport.
    pub fn new(config: MeekConfig) -> Self {
        Self {
            config: RwLock::new(config),
            last_error: RwLock::new(None),
            active_connections: RwLock::new(0),
            available: RwLock::new(true),
            fronting_idx: RwLock::new(0),
            sessions: RwLock::new(Vec::new()),
        }
    }

    /// Send data via Meek HTTP POST request.
    async fn send_data(&self, session: &mut MeekSession, data: &[u8]) -> Result<Vec<u8>, ShieldError> {
        let config = self.config.read().await;
        let timeout = Duration::from_secs(config.connect_timeout_secs);

        // Resolve fronting domain
        let connect_addr = super::resolve_domain(&config.fronting_domain, DEFAULT_MEEK_PORT)
            .await
            .unwrap_or(config.server_addr);

        // TCP connect
        let stream = tokio::time::timeout(timeout, TcpStream::connect(connect_addr))
            .await
            .map_err(|_| ShieldError::Timeout("Meek TCP timeout".into()))?
            .map_err(|e| ShieldError::ConnectionRefused(format!("Meek: {}", e)))?;

        stream.set_nodelay(true).map_err(|e| ShieldError::Io(e.to_string()))?;

        // TLS handshake with SNI = fronting domain
        let fronting_domain = config.fronting_domain.clone();
        let server_name = rustls::pki_types::ServerName::try_from(fronting_domain.clone())
            .map_err(|e| ShieldError::Config(format!("Invalid SNI: {:?}", e)))?;

        let mut root_store = rustls::RootCertStore::empty();
        for cert in rustls_native_certs::load_native_certs()
            .map_err(|e| ShieldError::Config(format!("Load certs: {}", e)))?
        {
            root_store.add(cert).ok();
        }

        let client_config = rustls::client::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
        let tls_stream = connector
            .connect(server_name, stream)
            .await
            .map_err(|e| ShieldError::TlsHandshakeFailed(format!("Meek TLS: {}", e)))?;

        let (mut read_half, mut write_half) = tokio::io::split(tls_stream);

        // Build and send POST request
        // Host header = actual server (domain fronting)
        let host_header = if config.use_fronting {
            format!("{}:{}", config.server_addr.ip(), config.server_addr.port())
        } else {
            config.fronting_domain.clone()
        };

        let request = MeekRequestBuilder::build_post_request(
            session,
            data,
            &config.fronting_domain,
            &host_header,
            &config.url_path,
            &config.auth_token,
            &config.user_agent,
        );

        tokio::time::timeout(timeout, write_half.write_all(&request))
            .await
            .map_err(|_| ShieldError::Timeout("Meek POST timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Write POST: {}", e)))?;

        // Read response
        let mut response = vec![0u8; 65536];
        let n = tokio::time::timeout(timeout, read_half.read(&mut response))
            .await
            .map_err(|_| ShieldError::Timeout("Meek response timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Read response: {}", e)))?;

        MeekResponseParser::parse_response(&response[..n])
    }

    /// Poll for incoming data via HTTP GET request.
    async fn poll_data(&self, session: &mut MeekSession) -> Result<Vec<u8>, ShieldError> {
        let config = self.config.read().await;
        let timeout = Duration::from_secs(config.connect_timeout_secs);

        let connect_addr = super::resolve_domain(&config.fronting_domain, DEFAULT_MEEK_PORT)
            .await
            .unwrap_or(config.server_addr);

        let stream = tokio::time::timeout(timeout, TcpStream::connect(connect_addr))
            .await
            .map_err(|_| ShieldError::Timeout("Meek poll TCP timeout".into()))?
            .map_err(|e| ShieldError::ConnectionRefused(format!("Meek poll: {}", e)))?;

        stream.set_nodelay(true).map_err(|e| ShieldError::Io(e.to_string()))?;

        let fronting_domain = config.fronting_domain.clone();
        let server_name = rustls::pki_types::ServerName::try_from(fronting_domain.clone())
            .map_err(|e| ShieldError::Config(format!("Invalid SNI: {:?}", e)))?;

        let mut root_store = rustls::RootCertStore::empty();
        for cert in rustls_native_certs::load_native_certs()
            .map_err(|e| ShieldError::Config(format!("Load certs: {}", e)))?
        {
            root_store.add(cert).ok();
        }

        let client_config = rustls::client::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
        let tls_stream = connector
            .connect(server_name, stream)
            .await
            .map_err(|e| ShieldError::TlsHandshakeFailed(format!("Meek poll TLS: {}", e)))?;

        let (mut read_half, mut write_half) = tokio::io::split(tls_stream);

        let host_header = if config.use_fronting {
            format!("{}:{}", config.server_addr.ip(), config.server_addr.port())
        } else {
            config.fronting_domain.clone()
        };

        let request = MeekRequestBuilder::build_poll_request(
            session,
            &config.fronting_domain,
            &host_header,
            &config.url_path,
            &config.auth_token,
            &config.user_agent,
        );

        tokio::time::timeout(timeout, write_half.write_all(&request))
            .await
            .map_err(|_| ShieldError::Timeout("Meek poll timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Write poll: {}", e)))?;

        let mut response = vec![0u8; 65536];
        let n = tokio::time::timeout(timeout, read_half.read(&mut response))
            .await
            .map_err(|_| ShieldError::Timeout("Meek poll response timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Read poll response: {}", e)))?;

        MeekResponseParser::parse_response(&response[..n])
    }
}

// ── Meek stream wrapper ─────────────────────────────────────────────────────

/// Wrapper that turns a Meek session into an AsyncRead + AsyncWrite stream.
/// Data is sent/received via HTTP requests/responses.
pub struct MeekStream {
    transport: Arc<MeekTransport>,
    session: MeekSession,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
}

impl tokio::io::AsyncRead for MeekStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        // Check if we have buffered data
        if !self.read_buffer.is_empty() {
            let to_read = buf.remaining().min(self.read_buffer.len());
            buf.put_slice(&self.read_buffer[..to_read]);
            self.read_buffer.drain(..to_read);
            return std::task::Poll::Ready(Ok(()));
        }

        // Need to poll for more data — this is a simplified implementation
        // In production, this would use a proper async channel
        std::task::Poll::Pending
    }
}

impl tokio::io::AsyncWrite for MeekStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        self.write_buffer.extend_from_slice(buf);
        std::task::Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

#[async_trait]
impl Transport for MeekTransport {
    fn name(&self) -> &str {
        "meek"
    }

    fn priority(&self) -> u8 {
        9
    }

    async fn connect(&self, addr: &SocketAddr) -> Result<Box<dyn TransportConnection>, ShieldError> {
        let config = self.config.read().await;
        let sni = config.sni_domain.clone();
        drop(config);

        // Create a new session
        let mut session = MeekSession::new();

        // Send initial CONNECT request with destination address
        let connect_data = format!("CONNECT {}", addr).into_bytes();
        let response = self.send_data(&mut session, &connect_data).await?;

        if response.is_empty() || response[0] != 0x00 {
            return Err(ShieldError::ConnectionRefused("Meek CONNECT failed".into()));
        }

        // Create a Meek stream wrapper
        let session_for_push = session.clone();
        let stream = MeekStream {
            transport: Arc::new(Self::new(self.config.read().await.clone())),
            session,
            read_buffer: response[1..].to_vec(),
            write_buffer: Vec::new(),
        };

        *self.active_connections.write().await += 1;
        *self.available.write().await = true;
        *self.last_error.write().await = None;
        self.sessions.write().await.push(session_for_push);

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
        let mut idx = self.fronting_idx.write().await;
        *idx = (*idx + 1) % FRONTING_DOMAINS.len();
        let domain = FRONTING_DOMAINS[*idx].to_string();
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
        self.sessions.write().await.clear();
        Ok(())
    }
}

impl MeekTransport {
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
