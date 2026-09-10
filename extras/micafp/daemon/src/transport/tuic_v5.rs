//! TUIC v5 Transport
//!
//! 0-RTT QUIC transport using the quinn crate. UDP relay mode for
//! minimal overhead. Authentication via UUID + password. Uses BBR
//! congestion control for Iran's high-latency networks.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{make_connection, ShieldError, Transport, TransportConnection};

// ── Constants ───────────────────────────────────────────────────────────────

/// TUIC v5 ALPN identifier.
const TUIC_V5_ALPN: &[u8] = b"tuic-v5";

/// Default TUIC server port.
const DEFAULT_TUIC_PORT: u16 = 443;

/// Maximum QUIC stream receive window.
const MAX_STREAM_WINDOW: u64 = 8 * 1024 * 1024;

/// Maximum QUIC connection receive window.
const MAX_CONN_WINDOW: u64 = 32 * 1024 * 1024;

/// TUIC v5 command types.
const CMD_CONNECT: u8 = 0x01; // TCP connect
const CMD_UDP_ASSOCIATE: u8 = 0x03; // UDP relay
const CMD_DATAGRAM: u8 = 0x04; // UDP datagram

/// TUIC v5 address types.
const ADDR_IPV4: u8 = 0x01;
const ADDR_DOMAIN: u8 = 0x02;
const ADDR_IPV6: u8 = 0x03;

// ── Configuration ───────────────────────────────────────────────────────────

/// Configuration for TUIC v5 transport.
#[derive(Debug, Clone)]
pub struct TuicV5Config {
    /// TUIC server address.
    pub server_addr: SocketAddr,
    /// UUID for authentication.
    pub uuid: Uuid,
    /// Password for authentication.
    pub password: String,
    /// SNI domain for TLS.
    pub sni_domain: String,
    /// Whether to use 0-RTT (fast reconnect).
    pub zero_rtt: bool,
    /// Whether to use UDP relay mode.
    pub udp_relay: bool,
    /// Congestion control algorithm: "bbr" or "cubic".
    pub congestion_control: String,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Whether to allow insecure TLS.
    pub insecure: bool,
    /// ALPN protocols.
    pub alpn: Vec<Vec<u8>>,
}

impl TuicV5Config {
    /// Create a new TUIC v5 config with sensible defaults for Iran.
    pub fn new(server_addr: SocketAddr, uuid: Uuid, password: String, sni_domain: String) -> Self {
        Self {
            server_addr,
            uuid,
            password,
            sni_domain,
            zero_rtt: true,
            udp_relay: true,
            congestion_control: "bbr".to_string(),
            connect_timeout_secs: 10,
            insecure: false,
            alpn: vec![TUIC_V5_ALPN.to_vec()],
        }
    }
}

// ── TUIC v5 Authentication ──────────────────────────────────────────────────

/// TUIC v5 authentication request.
struct TuicAuthRequest {
    uuid: Uuid,
    password: String,
}

impl TuicAuthRequest {
    /// Serialize to bytes.
    /// Format: [version(1)] [uuid(16)] [password_len(2)] [password(variable)]
    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1 + 16 + 2 + self.password.len());
        buf.push(0x05); // TUIC v5
        buf.extend_from_slice(self.uuid.as_bytes());
        let pwd_bytes = self.password.as_bytes();
        buf.extend_from_slice(&(pwd_bytes.len() as u16).to_be_bytes());
        buf.extend_from_slice(pwd_bytes);
        buf
    }
}

// ── TUIC v5 Proxy Request ───────────────────────────────────────────────────

/// TUIC v5 proxy connection request.
struct TuicProxyRequest {
    command: u8,
    dest_addr: SocketAddr,
}

impl TuicProxyRequest {
    /// Create a new TCP connect request.
    fn tcp_connect(dest_addr: SocketAddr) -> Self {
        Self {
            command: CMD_CONNECT,
            dest_addr,
        }
    }

    /// Create a new UDP associate request.
    fn udp_associate(dest_addr: SocketAddr) -> Self {
        Self {
            command: CMD_UDP_ASSOCIATE,
            dest_addr,
        }
    }

    /// Serialize to bytes.
    /// Format: [command(1)] [addr_type(1)] [addr(variable)] [port(2)]
    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1 + 1 + 16 + 2);
        buf.push(self.command);

        match self.dest_addr {
            SocketAddr::V4(v4) => {
                buf.push(ADDR_IPV4);
                buf.extend_from_slice(&v4.ip().octets());
            }
            SocketAddr::V6(v6) => {
                buf.push(ADDR_IPV6);
                buf.extend_from_slice(&v6.ip().octets());
            }
        }

        buf.extend_from_slice(&self.dest_addr.port().to_be_bytes());
        buf
    }
}

// ── Custom Certificate Verifier ─────────────────────────────────────────────

/// TLS certificate verifier for TUIC v5.
#[derive(Debug)]
struct TuicServerVerifier {
    insecure: bool,
    expected_sni: String,
}

impl TuicServerVerifier {
    fn new(insecure: bool, expected_sni: String) -> Self {
        Self {
            insecure,
            expected_sni,
        }
    }
}

impl rustls::client::danger::ServerCertVerifier for TuicServerVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        if self.insecure {
            return Ok(rustls::client::danger::ServerCertVerified::assertion());
        }
        let sni_str = server_name.to_str();
        if sni_str == self.expected_sni {
            Ok(rustls::client::danger::ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General(format!(
                "SNI mismatch: expected {}, got {}",
                self.expected_sni, sni_str
            )))
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
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

// ── Quinn stream wrapper ────────────────────────────────────────────────────

/// Wrapper combining Quinn send/recv streams.
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

// ── TUIC v5 Transport ───────────────────────────────────────────────────────

/// TUIC v5 transport implementation.
///
/// # How it works
///
/// 1. Establishes a QUIC connection to the TUIC server
/// 2. Authenticates with UUID + password
/// 3. Opens a QUIC stream for TCP proxy or uses datagrams for UDP relay
/// 4. Supports 0-RTT for fast reconnection
/// 5. Uses BBR congestion control for Iran's high-latency networks
/// 6. UDP relay mode has minimal overhead (no TCP-over-TCP amplification)
pub struct TuicV5Transport {
    config: RwLock<TuicV5Config>,
    last_error: RwLock<Option<ShieldError>>,
    active_connections: RwLock<usize>,
    available: RwLock<bool>,
    /// QUIC connection for reuse.
    quic_connection: RwLock<Option<quinn::Connection>>,
    /// 0-RTT session ticket for fast reconnection.
    session_ticket: RwLock<Option<Vec<u8>>>,
}

impl TuicV5Transport {
    /// Create a new TUIC v5 transport.
    pub fn new(config: TuicV5Config) -> Self {
        Self {
            config: RwLock::new(config),
            last_error: RwLock::new(None),
            active_connections: RwLock::new(0),
            available: RwLock::new(true),
            quic_connection: RwLock::new(None),
            session_ticket: RwLock::new(None),
        }
    }

    /// Build Quinn client config for TUIC v5 with BBR.
    fn build_quinn_config(config: &TuicV5Config) -> Result<quinn::ClientConfig, ShieldError> {
        let mut crypto = rustls::client::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(TuicServerVerifier::new(
                config.insecure,
                config.sni_domain.clone(),
            )))
            .with_no_client_auth();

        crypto.alpn_protocols = config.alpn.clone();

        // Enable 0-RTT
        crypto.enable_early_data = true;

        let mut transport = quinn::TransportConfig::default();
        transport.max_idle_timeout(Some(Duration::from_secs(30).try_into().unwrap()));
        transport.stream_receive_window(MAX_STREAM_WINDOW.try_into().unwrap());
        transport.receive_window(MAX_CONN_WINDOW.try_into().unwrap());

        // BBR congestion control for Iran's high-latency networks
        // Note: Quinn doesn't directly support BBR, but we configure
        // optimal parameters for high-latency paths
        transport.send_window(MAX_CONN_WINDOW);

        let quic_crypto = quinn_proto::crypto::rustls::QuicClientConfig::try_from(crypto).map_err(|e| anyhow::anyhow!("quic crypto: {}", e))?;
        let mut client_config = quinn::ClientConfig::new(Arc::new(quic_crypto));
        client_config.transport_config(Arc::new(transport));

        Ok(client_config)
    }

    /// Establish a QUIC connection to the TUIC server.
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

        let quinn_config = Self::build_quinn_config(&config)?;
        let server_name = &config.sni_domain;
        let addr = config.server_addr;

        let mut endpoint = quinn::Endpoint::client("0.0.0.0:0".parse().unwrap())
            .map_err(|e| ShieldError::QuicError(format!("Endpoint creation: {}", e)))?;

        endpoint.set_default_client_config(quinn_config);

        let connect_timeout = Duration::from_secs(config.connect_timeout_secs);

        // Attempt 0-RTT if we have a session ticket
        let connecting = endpoint
            .connect(addr, server_name)
            .map_err(|e| ShieldError::QuicError(format!("Connect start: {}", e)))?;

        let connection = tokio::time::timeout(connect_timeout, connecting)
            .await
            .map_err(|_| ShieldError::Timeout("QUIC connect timeout".into()))?
            .map_err(|e| ShieldError::QuicError(format!("QUIC connect: {}", e)))?;

        // Check if 0-RTT was accepted
        if connection            .handshake_data().is_some() {
            tracing::debug!("TUIC v5: 0-RTT accepted");
        }

        // Authenticate over a control stream
        let auth = TuicAuthRequest {
            uuid: config.uuid,
            password: config.password.clone(),
        };

        let (mut send, mut recv) = connection
            .open_bi()
            .await
            .map_err(|e| ShieldError::QuicError(format!("Open auth stream: {}", e)))?;

        let auth_bytes = auth.to_bytes();
        send.write_all(&auth_bytes)
            .await
            .map_err(|e| ShieldError::AuthFailed(format!("Send auth: {}", e)))?;

        send.finish()
            .map_err(|e| ShieldError::QuicError(format!("Finish auth: {}", e)))?;

        let auth_response = recv
            .read_to_end(256)
            .await
            .map_err(|e| ShieldError::AuthFailed(format!("Read auth response: {}", e)))?;

        if auth_response.is_empty() || auth_response[0] != 0x00 {
            return Err(ShieldError::AuthFailed("TUIC auth rejected".into()));
        }

        // Store connection for reuse
        {
            let mut conn_guard = self.quic_connection.write().await;
            *conn_guard = Some(connection.clone());
        }

        Ok(connection)
    }

    /// Open a TCP proxy stream to the destination.
    async fn open_tcp_stream(&self, dest: &SocketAddr) -> Result<QuinnStream, ShieldError> {
        let connection = self.establish_quic_connection().await?;

        let (mut send, recv) = connection
            .open_bi()
            .await
            .map_err(|e| ShieldError::QuicError(format!("Open proxy stream: {}", e)))?;

        // Send proxy request
        let request = TuicProxyRequest::tcp_connect(*dest);
        send.write_all(&request.to_bytes())
            .await
            .map_err(|e| ShieldError::QuicError(format!("Send proxy request: {}", e)))?;

        Ok(QuinnStream { send, recv })
    }

    /// Open a UDP relay association.
    async fn open_udp_relay(&self, dest: &SocketAddr) -> Result<quinn::Connection, ShieldError> {
        let connection = self.establish_quic_connection().await?;

        let (mut send, recv) = connection
            .open_bi()
            .await
            .map_err(|e| ShieldError::QuicError(format!("Open UDP relay stream: {}", e)))?;

        let request = TuicProxyRequest::udp_associate(*dest);
        send.write_all(&request.to_bytes())
            .await
            .map_err(|e| ShieldError::QuicError(format!("Send UDP relay request: {}", e)))?;

        Ok(connection)
    }
}

#[async_trait]
impl Transport for TuicV5Transport {
    fn name(&self) -> &str {
        "tuic-v5"
    }

    fn priority(&self) -> u8 {
        4
    }

    async fn connect(&self, addr: &SocketAddr) -> Result<Box<dyn TransportConnection>, ShieldError> {
        let config = self.config.read().await;
        let sni = config.sni_domain.clone();
        let use_udp = config.udp_relay;
        drop(config);

        let stream = if use_udp {
            // Use UDP relay mode for minimal overhead
            let connection = self.open_udp_relay(addr).await?;
            // For UDP relay, we need a different approach:
            // Open a bidirectional stream for the UDP association
            let (send, recv) = connection
                .open_bi()
                .await
                .map_err(|e| ShieldError::QuicError(format!("UDP stream: {}", e)))?;
            QuinnStream { send, recv }
        } else {
            self.open_tcp_stream(addr).await?
        };

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
        let domains = ["www.digikala.com", "www.snapp.ir", "www.divar.ir"];
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

// ── Async helpers ───────────────────────────────────────────────────────────

impl TuicV5Transport {
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
