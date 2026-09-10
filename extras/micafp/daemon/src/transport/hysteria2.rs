//! Hysteria2 QUIC-based Transport
//!
//! Disguised as HTTP/3 video streaming. Uses "Brutal" congestion control
//! optimized for Iran's lossy, high-latency networks. When probed,
//! returns HTTP/403 mimicking Aparat (Iranian video platform).
//!
//! Uses BLAKE3 for key derivation in Salamander obfuscation mode.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::RwLock;

use super::{make_connection, ShieldError, Transport, TransportConnection};

// ── Constants ───────────────────────────────────────────────────────────────

/// Default Hysteria2 server port.
const DEFAULT_HYSTERIA2_PORT: u16 = 443;

/// QUIC ALPN identifier for Hysteria2.
const HYSTERIA2_ALPN: &[u8] = b"hysteria2";

/// Salamander obfuscation key derivation context.
const SALAMANDER_CONTEXT: &[u8] = b"hysteria2-salamander-v1";

/// Maximum QUIC stream receive window.
const MAX_STREAM_WINDOW: u64 = 8 * 1024 * 1024; // 8 MB

/// Maximum QUIC connection receive window.
const MAX_CONN_WINDOW: u64 = 32 * 1024 * 1024; // 32 MB

/// Default bandwidth for Brutal CC (10 Mbps).
const DEFAULT_BANDWIDTH_MBPS: u64 = 10;

/// Brutal congestion control: minimum sending rate (bytes/sec).
const BRUTAL_MIN_RATE: u64 = 65536; // 64 KB/s

/// Masquerade response: HTTP/403 mimicking Aparat.
const MASQUERADE_HTTP_403: &[u8] = b"HTTP/1.1 403 Forbidden\r\n\
    Server: nginx\r\n\
    Content-Type: text/html\r\n\
    Content-Length: 162\r\n\
    Connection: close\r\n\
    \r\n\
    <html>\r\n\
    <head><title>403 Forbidden</title></head>\r\n\
    <body>\r\n\
    <center><h1>403 Forbidden</h1></center>\r\n\
    <hr><center>nginx</center>\r\n\
    </body>\r\n\
    </html>\r\n";

/// Masquerade TLS certificate common name (Aparat).
const MASQUERADE_CN: &str = "www.aparat.com";

// ── Configuration ───────────────────────────────────────────────────────────

/// Configuration for Hysteria2 transport.
#[derive(Debug, Clone)]
pub struct HysteriaConfig {
    /// Hysteria2 server address.
    pub server_addr: SocketAddr,
    /// Upload bandwidth in Mbps.
    pub bandwidth_up: u64,
    /// Download bandwidth in Mbps.
    pub bandwidth_down: u64,
    /// Authentication password.
    pub auth_password: String,
    /// URL for masquerade responses (when probed).
    /// Default: mimics Aparat video site.
    pub masquerade_url: String,
    /// Salamander obfuscation key (empty = disabled).
    pub obfs_key: String,
    /// SNI domain for TLS (e.g., "www.aparat.com").
    pub sni_domain: String,
    /// Whether to allow insecure TLS (for testing).
    pub insecure: bool,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,
    /// ALPN protocols.
    pub alpn: Vec<Vec<u8>>,
}

impl HysteriaConfig {
    /// Create a new Hysteria2 config with sensible defaults for Iran.
    pub fn new(server_addr: SocketAddr, auth_password: String) -> Self {
        Self {
            server_addr,
            bandwidth_up: DEFAULT_BANDWIDTH_MBPS,
            bandwidth_down: DEFAULT_BANDWIDTH_MBPS,
            auth_password,
            masquerade_url: "https://www.aparat.com".to_string(),
            obfs_key: String::new(),
            sni_domain: "www.aparat.com".to_string(),
            insecure: false,
            connect_timeout_secs: 10,
            alpn: vec![HYSTERIA2_ALPN.to_vec()],
        }
    }

    /// Create config with custom bandwidth for Brutal CC.
    pub fn with_bandwidth(
        server_addr: SocketAddr,
        auth_password: String,
        bandwidth_up_mbps: u64,
        bandwidth_down_mbps: u64,
    ) -> Self {
        Self {
            bandwidth_up: bandwidth_up_mbps,
            bandwidth_down: bandwidth_down_mbps,
            ..Self::new(server_addr, auth_password)
        }
    }
}

// ── Salamander obfuscation ──────────────────────────────────────────────────

/// Salamander obfuscation layer using BLAKE3 for key derivation.
///
/// Salamander adds a lightweight XOR-based stream cipher on top of
/// QUIC traffic to defeat protocol fingerprinting. It's NOT a
/// replacement for TLS — it adds an extra obfuscation layer.
pub struct SalamanderObfs {
    /// BLAKE3-derived key stream.
    key_stream: blake3::OutputReader,
}

impl SalamanderObfs {
    /// Create a new Salamander obfuscator with the given key.
    pub fn new(key: &str) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(SALAMANDER_CONTEXT);
        hasher.update(key.as_bytes());
        let key_stream = hasher.finalize_xof();
        Self { key_stream }
    }

    /// Obfuscate (encrypt) a data buffer in place.
    pub fn obfuscate(&mut self, data: &mut [u8]) {
        let mut key_bytes = [0u8; 64];
        for chunk in data.chunks_mut(64) {
            self.key_stream.fill(&mut key_bytes);
            for (i, byte) in chunk.iter_mut().enumerate() {
                *byte ^= key_bytes[i];
            }
        }
    }

    /// Deobfuscate (decrypt) a data buffer in place.
    /// For XOR-based stream ciphers, this is identical to obfuscation.
    pub fn deobfuscate(&mut self, data: &mut [u8]) {
        self.obfuscate(data);
    }
}

// ── Brutal Congestion Control ───────────────────────────────────────────────

/// Brutal congestion control parameters for Iran's lossy networks.
///
/// Unlike standard congestion control (Cubic, BBR) which reduces sending
/// rate on packet loss, Brutal CC maintains a fixed sending rate based
/// on configured bandwidth. This is critical for Iran where:
/// - Packet loss is frequent and often artificial (DPI)
/// - Standard CC would throttle to unusable speeds
/// - We know our actual bandwidth and should use it aggressively
pub struct BrutalCongestionControl {
    /// Configured send rate in bytes per second.
    send_rate_bps: u64,
    /// Configured receive rate in bytes per second.
    recv_rate_bps: u64,
    /// Current RTT estimate in milliseconds.
    rtt_ms: f64,
    /// Number of packets sent.
    packets_sent: u64,
    /// Number of packets acked.
    packets_acked: u64,
    /// Number of packets lost.
    packets_lost: u64,
}

impl BrutalCongestionControl {
    /// Create a new Brutal CC instance with configured bandwidth.
    pub fn new(bandwidth_up_mbps: u64, bandwidth_down_mbps: u64) -> Self {
        Self {
            send_rate_bps: (bandwidth_up_mbps * 1_000_000 / 8).max(BRUTAL_MIN_RATE),
            recv_rate_bps: (bandwidth_down_mbps * 1_000_000 / 8).max(BRUTAL_MIN_RATE),
            rtt_ms: 200.0, // Initial RTT estimate for Iran
            packets_sent: 0,
            packets_acked: 0,
            packets_lost: 0,
        }
    }

    /// Get the current sending rate in bytes per second.
    /// Brutal CC: always returns the configured rate.
    pub fn send_rate(&self) -> u64 {
        self.send_rate_bps
    }

    /// Get the current receive rate in bytes per second.
    pub fn recv_rate(&self) -> u64 {
        self.recv_rate_bps
    }

    /// Record a packet being sent.
    pub fn on_packet_sent(&mut self) {
        self.packets_sent += 1;
    }

    /// Record a packet being acknowledged.
    pub fn on_packet_acked(&mut self, rtt: Duration) {
        self.packets_acked += 1;
        // Update RTT with exponential moving average
        let rtt_ms = rtt.as_millis() as f64;
        self.rtt_ms = 0.875 * self.rtt_ms + 0.125 * rtt_ms;
    }

    /// Record a packet loss.
    /// Brutal CC: does NOT reduce rate. Just count for stats.
    pub fn on_packet_lost(&mut self) {
        self.packets_lost += 1;
        // Brutal CC: ignore loss, keep sending at configured rate
    }

    /// Get the loss rate (for diagnostics only, not used for rate control).
    pub fn loss_rate(&self) -> f64 {
        if self.packets_sent == 0 {
            0.0
        } else {
            self.packets_lost as f64 / self.packets_sent as f64
        }
    }

    /// Get the current RTT estimate.
    pub fn rtt_ms(&self) -> f64 {
        self.rtt_ms
    }

    /// Update bandwidth settings (e.g., after speed test).
    pub fn update_bandwidth(&mut self, up_mbps: u64, down_mbps: u64) {
        self.send_rate_bps = (up_mbps * 1_000_000 / 8).max(BRUTAL_MIN_RATE);
        self.recv_rate_bps = (down_mbps * 1_000_000 / 8).max(BRUTAL_MIN_RATE);
    }
}

// ── Hysteria2 Authentication ────────────────────────────────────────────────

/// Hysteria2 authentication message.
///
/// Format: [version(1)] [auth_len(2)] [auth_data(variable)] [bandwidth_up(8)] [bandwidth_down(8)]
struct Hysteria2Auth {
    auth_password: String,
    bandwidth_up: u64,
    bandwidth_down: u64,
}

impl Hysteria2Auth {
    /// Create a new auth message.
    fn new(password: &str, bandwidth_up: u64, bandwidth_down: u64) -> Self {
        Self {
            auth_password: password.to_string(),
            bandwidth_up,
            bandwidth_down,
        }
    }

    /// Serialize the auth message to bytes.
    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1 + 2 + self.auth_password.len() + 16);
        buf.push(0x01); // Version
        let auth_bytes = self.auth_password.as_bytes();
        buf.extend_from_slice(&(auth_bytes.len() as u16).to_be_bytes());
        buf.extend_from_slice(auth_bytes);
        buf.extend_from_slice(&self.bandwidth_up.to_be_bytes());
        buf.extend_from_slice(&self.bandwidth_down.to_be_bytes());
        buf
    }

    /// Parse an auth response.
    fn parse_response(data: &[u8]) -> Result<Hysteria2AuthResponse, ShieldError> {
        if data.len() < 2 {
            return Err(ShieldError::Protocol("Auth response too short".into()));
        }

        let status = data[0];
        if status != 0x00 {
            return Err(ShieldError::AuthFailed(format!(
                "Server rejected auth: status {}",
                status
            )));
        }

        let server_up = if data.len() >= 9 {
            u64::from_be_bytes(data[1..9].try_into().unwrap())
        } else {
            0
        };

        let server_down = if data.len() >= 17 {
            u64::from_be_bytes(data[9..17].try_into().unwrap())
        } else {
            0
        };

        Ok(Hysteria2AuthResponse {
            success: true,
            server_bandwidth_up: server_up,
            server_bandwidth_down: server_down,
        })
    }
}

/// Hysteria2 authentication response.
struct Hysteria2AuthResponse {
    success: bool,
    server_bandwidth_up: u64,
    server_bandwidth_down: u64,
}

// ── Hysteria2 Transport ─────────────────────────────────────────────────────

/// Hysteria2 QUIC-based transport disguised as HTTP/3 video streaming.
///
/// # How it works
///
/// 1. Establishes a QUIC connection (HTTP/3) to the Hysteria2 server
/// 2. SNI is set to an Iranian video site (e.g., aparat.com)
/// 3. Uses Brutal congestion control to maintain throughput on lossy networks
/// 4. Authenticates with password + bandwidth parameters
/// 5. Opens bidirectional QUIC streams for proxy data
/// 6. When probed, returns HTTP/403 mimicking Aparat
/// 7. Optional Salamander obfuscation adds XOR layer on top of QUIC
pub struct Hysteria2Transport {
    config: RwLock<HysteriaConfig>,
    last_error: RwLock<Option<ShieldError>>,
    active_connections: RwLock<usize>,
    available: RwLock<bool>,
    /// Brutal congestion control state.
    congestion_control: RwLock<BrutalCongestionControl>,
    /// Salamander obfuscation (if enabled).
    obfs: RwLock<Option<SalamanderObfs>>,
    /// QUIC connection (reused across streams).
    quic_connection: RwLock<Option<quinn::Connection>>,
}

impl Hysteria2Transport {
    /// Create a new Hysteria2 transport.
    pub fn new(config: HysteriaConfig) -> Self {
        let cc = BrutalCongestionControl::new(config.bandwidth_up, config.bandwidth_down);
        let obfs = if config.obfs_key.is_empty() {
            None
        } else {
            Some(SalamanderObfs::new(&config.obfs_key))
        };

        Self {
            config: RwLock::new(config),
            last_error: RwLock::new(None),
            active_connections: RwLock::new(0),
            available: RwLock::new(true),
            congestion_control: RwLock::new(cc),
            obfs: RwLock::new(obfs),
            quic_connection: RwLock::new(None),
        }
    }

    /// Build a Quinn client config for Hysteria2.
    fn build_quinn_config(config: &HysteriaConfig) -> Result<quinn::ClientConfig, ShieldError> {
        let mut crypto = rustls::client::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(HysteriaServerVerifier::new(
                config.insecure,
                config.sni_domain.clone(),
            )))
            .with_no_client_auth();

        // Set ALPN
        crypto.alpn_protocols = config.alpn.clone();

        let mut transport = quinn::TransportConfig::default();
        transport.max_idle_timeout(Some(Duration::from_secs(30).try_into().unwrap()));
        transport.stream_receive_window(MAX_STREAM_WINDOW.try_into().unwrap());
        transport.receive_window(MAX_CONN_WINDOW.try_into().unwrap());

        let quic_crypto = quinn_proto::crypto::rustls::QuicClientConfig::try_from(crypto).map_err(|e| anyhow::anyhow!("quic crypto: {}", e))?;
        let mut client_config = quinn::ClientConfig::new(Arc::new(quic_crypto));
        client_config.transport_config(Arc::new(transport));

        Ok(client_config)
    }

    /// Establish a QUIC connection to the Hysteria2 server.
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

        let connecting = endpoint.connect(addr, server_name)
            .map_err(|e| ShieldError::QuicError(format!("QUIC connect: {}", e)))?;
        let connection = tokio::time::timeout(connect_timeout, connecting)
            .await
            .map_err(|_| ShieldError::Timeout("QUIC connect timeout".into()))?
            .map_err(|e| ShieldError::QuicError(format!("QUIC connect: {}", e)))?;

        // Authenticate
        let auth = Hysteria2Auth::new(
            &config.auth_password,
            config.bandwidth_up,
            config.bandwidth_down,
        );

        // Open a control stream for authentication
        let (mut send, mut recv) = connection
            .open_bi()
            .await
            .map_err(|e| ShieldError::QuicError(format!("Open auth stream: {}", e)))?;

        let auth_bytes = auth.to_bytes();
        send.write_all(&auth_bytes)
            .await
            .map_err(|e| ShieldError::AuthFailed(format!("Send auth: {}", e)))?;

        send.finish()
            .map_err(|e| ShieldError::QuicError(format!("Finish auth stream: {}", e)))?;

        let auth_response = recv
            .read_to_end(256)
            .await
            .map_err(|e| ShieldError::AuthFailed(format!("Read auth response: {}", e)))?;

        let _response = Hysteria2Auth::parse_response(&auth_response)?;

        // Store connection for reuse
        {
            let mut conn_guard = self.quic_connection.write().await;
            *conn_guard = Some(connection.clone());
        }

        Ok(connection)
    }

    /// Open a proxy stream over the QUIC connection.
    async fn open_proxy_stream(&self, dest: &SocketAddr) -> Result<quinn::SendStream, ShieldError> {
        let connection = self.establish_quic_connection().await?;

        // Open a new bidirectional QUIC stream for proxy data
        let (mut send, recv) = connection
            .open_bi()
            .await
            .map_err(|e| ShieldError::QuicError(format!("Open proxy stream: {}", e)))?;

        // Send the destination address header
        // Format: [addr_type(1)] [addr(variable)] [port(2)]
        let dest_str = dest.to_string();
        let dest_bytes = dest_str.as_bytes();
        let mut header = Vec::with_capacity(1 + 1 + dest_bytes.len() + 2);
        header.push(0x03); // Address type: domain
        header.push(dest_bytes.len() as u8);
        header.extend_from_slice(dest_bytes);
        header.extend_from_slice(&dest.port().to_be_bytes());

        // Apply Salamander obfuscation if enabled
        {
            let mut obfs_guard = self.obfs.write().await;
            if let Some(obfs) = obfs_guard.as_mut() {
                obfs.obfuscate(&mut header);
            }
        }

        send.write_all(&header)
            .await
            .map_err(|e| ShieldError::QuicError(format!("Send proxy header: {}", e)))?;

        Ok(send)
    }

    /// Generate a masquerade response (HTTP/403 mimicking Aparat).
    /// Used when a probe connects without valid authentication.
    pub fn generate_masquerade_response() -> Vec<u8> {
        MASQUERADE_HTTP_403.to_vec()
    }
}

// ── Custom certificate verifier ─────────────────────────────────────────────

/// Custom TLS certificate verifier for Hysteria2.
///
/// In production, this would verify the server certificate against
/// the expected SNI domain. For Iran, we may need to accept self-signed
/// certs due to certificate transparency issues.
#[derive(Debug)]
struct HysteriaServerVerifier {
    insecure: bool,
    expected_sni: String,
}

impl HysteriaServerVerifier {
    fn new(insecure: bool, expected_sni: String) -> Self {
        Self {
            insecure,
            expected_sni,
        }
    }
}

impl rustls::client::danger::ServerCertVerifier for HysteriaServerVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::pki_types::CertificateDer<'_>,
        intermediates: &[rustls::pki_types::CertificateDer<'_>],
        server_name: &rustls::pki_types::ServerName<'_>,
        ocsp_response: &[u8],
        now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        if self.insecure {
            // Skip verification (dangerous, but sometimes needed in Iran)
            return Ok(rustls::client::danger::ServerCertVerified::assertion());
        }

        // In production, implement proper certificate verification here.
        // For now, accept all certs from the expected SNI domain.
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
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

// ── Transport trait implementation ──────────────────────────────────────────

#[async_trait]
impl Transport for Hysteria2Transport {
    fn name(&self) -> &str {
        "hysteria2"
    }

    fn priority(&self) -> u8 {
        3
    }

    async fn connect(&self, addr: &SocketAddr) -> Result<Box<dyn TransportConnection>, ShieldError> {
        let sni = self.config.read().await.sni_domain.clone();

        // Establish QUIC connection and authenticate
        let connection = self.establish_quic_connection().await?;

        // Open a proxy stream to the destination
        let (send, recv) = connection
            .open_bi()
            .await
            .map_err(|e| ShieldError::QuicError(format!("Open stream: {}", e)))?;

        // Send destination header
        let dest_str = addr.to_string();
        let dest_bytes = dest_str.as_bytes();
        let mut header = Vec::with_capacity(1 + 1 + dest_bytes.len() + 2);
        header.push(0x03); // Address type: domain string
        header.push(dest_bytes.len() as u8);
        header.extend_from_slice(dest_bytes);
        header.extend_from_slice(&addr.port().to_be_bytes());

        // Create a combined read/write stream from QUIC send/recv
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
        // Cycle through Iranian video/streaming domains
        let domains = ["www.aparat.com", "www.filimo.com", "www.namava.ir"];
        let mut config = self.config.write().await;
        let current = &config.sni_domain;
        let next = domains
            .iter()
            .find(|d| **d != current.as_str())
            .or_else(|| domains.first())
            .unwrap();
        config.sni_domain = next.to_string();
        // Reset QUIC connection to use new SNI
        *self.quic_connection.write().await = None;
        Ok(next.to_string())
    }

    fn active_connections(&self) -> usize {
        0
    }

    async fn shutdown(&self) -> Result<(), ShieldError> {
        *self.available.write().await = false;
        *self.active_connections.write().await = 0;

        // Close QUIC connection
        if let Some(conn) = self.quic_connection.write().await.take() {
            conn.close(0u32.into(), b"shutdown");
        }

        Ok(())
    }
}

// ── Quinn stream wrapper ────────────────────────────────────────────────────

/// Wrapper that combines Quinn send and receive streams into a single
/// AsyncRead + AsyncWrite implementor.
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

// ── Async helpers ───────────────────────────────────────────────────────────

impl Hysteria2Transport {
    /// Get the last error (async version).
    pub async fn get_last_error(&self) -> Option<ShieldError> {
        self.last_error.read().await.clone()
    }

    /// Get the current SNI domain (async version).
    pub async fn get_current_sni_domain(&self) -> String {
        self.config.read().await.sni_domain.clone()
    }

    /// Get the number of active connections (async version).
    pub async fn get_active_connections(&self) -> usize {
        *self.active_connections.read().await
    }

    /// Get congestion control statistics.
    pub async fn get_cc_stats(&self) -> (f64, f64, u64, u64) {
        let cc = self.congestion_control.read().await;
        (cc.rtt_ms(), cc.loss_rate(), cc.send_rate(), cc.recv_rate())
    }

    /// Update bandwidth settings (e.g., after speed test).
    pub async fn update_bandwidth(&self, up_mbps: u64, down_mbps: u64) {
        self.congestion_control
            .write()
            .await
            .update_bandwidth(up_mbps, down_mbps);
        let mut config = self.config.write().await;
        config.bandwidth_up = up_mbps;
        config.bandwidth_down = down_mbps;
    }
}
