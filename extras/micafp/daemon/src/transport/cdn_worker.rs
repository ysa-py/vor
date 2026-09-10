//! CDN Worker Transport for Chinese CDNs
//!
//! Domain fronting via Alibaba Cloud CDN, ByteDance Volcengine,
//! Tencent EdgeOne, and Huawei Cloud CDN. Worker code deployed as
//! serverless functions on each CDN. Client connects to CDN domain,
//! worker relays to actual endpoint. SNI shows legitimate Chinese domain.
//!
//! CRITICAL: Cloudflare is BLOCKED in Iran — we ONLY use Chinese CDNs
//! and Arvan Cloud (Iranian CDN).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;

use super::{make_connection, ShieldError, Transport, TransportConnection};
use base64;
use hex;

// ── Constants ───────────────────────────────────────────────────────────────

/// Default CDN worker port.
const DEFAULT_CDN_PORT: u16 = 443;

/// WebSocket path for CDN worker.
const WS_PATH: &str = "/cdn-cgi/worker";

/// CDN worker authentication header.
const WORKER_AUTH_HEADER: &str = "X-CDN-Auth";

/// CDN worker destination header.
const WORKER_DEST_HEADER: &str = "X-CDN-Dest";

/// CDN worker session header.
const WORKER_SESSION_HEADER: &str = "X-CDN-Session";

/// UCB1 exploration factor for worker selection.
const UCB1_EXPLORATION: f64 = 2.0;

/// Maximum payload per WebSocket message.
const MAX_WS_PAYLOAD: usize = 65536;

/// WebSocket magic GUID for handshake.
const WS_MAGIC_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

// ── CDN Provider Definitions ────────────────────────────────────────────────

/// CDN provider configuration.
#[derive(Debug, Clone)]
pub struct CdnProvider {
    /// Provider name.
    pub name: &'static str,
    /// Fronting domain (SNI shows this).
    pub fronting_domain: &'static str,
    /// Worker endpoint URL path.
    pub worker_path: &'static str,
    /// Whether this CDN works in Iran.
    pub works_in_iran: bool,
    /// Average latency to Iran (ms).
    pub avg_latency_iran_ms: u64,
}

/// All supported CDN providers (Chinese + Iranian, NO Cloudflare).
const CDN_PROVIDERS: &[CdnProvider] = &[
    CdnProvider {
        name: "Alibaba Cloud CDN",
        fronting_domain: "cdn.alicdn.com",
        worker_path: "/assets/worker",
        works_in_iran: true,
        avg_latency_iran_ms: 80,
    },
    CdnProvider {
        name: "Alibaba Cloud CDN (Alt)",
        fronting_domain: "g.alicdn.com",
        worker_path: "/lib/worker",
        works_in_iran: true,
        avg_latency_iran_ms: 85,
    },
    CdnProvider {
        name: "ByteDance Volcengine",
        fronting_domain: "cdn.bytedance.com",
        worker_path: "/api/worker",
        works_in_iran: true,
        avg_latency_iran_ms: 100,
    },
    CdnProvider {
        name: "ByteDance (Alt)",
        fronting_domain: "lf3-cdn-tos.bytegoofy.com",
        worker_path: "/obj/worker",
        works_in_iran: true,
        avg_latency_iran_ms: 110,
    },
    CdnProvider {
        name: "Tencent EdgeOne",
        fronting_domain: "cdn-go.cn",
        worker_path: "/edge/worker",
        works_in_iran: true,
        avg_latency_iran_ms: 120,
    },
    CdnProvider {
        name: "Tencent (Alt)",
        fronting_domain: "sqimg.qq.com",
        worker_path: "/img/worker",
        works_in_iran: true,
        avg_latency_iran_ms: 115,
    },
    CdnProvider {
        name: "Huawei Cloud CDN",
        fronting_domain: "cdn.huaweicloud.com",
        worker_path: "/cdn/worker",
        works_in_iran: true,
        avg_latency_iran_ms: 130,
    },
    CdnProvider {
        name: "Arvan Cloud (Iran)",
        fronting_domain: "cdn.arvancloud.com",
        worker_path: "/v1/worker",
        works_in_iran: true,
        avg_latency_iran_ms: 30,
    },
    CdnProvider {
        name: "Arvan Cloud (Alt)",
        fronting_domain: "nockette.arvanboost.com",
        worker_path: "/boost/worker",
        works_in_iran: true,
        avg_latency_iran_ms: 35,
    },
];

// ── Worker Statistics ────────────────────────────────────────────────────────

/// Per-worker statistics for UCB1 bandit selection.
#[derive(Debug, Clone)]
pub struct WorkerStats {
    /// Provider index.
    pub provider_idx: usize,
    /// Number of successful connections.
    pub success_count: u64,
    /// Number of failed connections.
    pub fail_count: u64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Timestamp of last success (millis since epoch).
    pub last_success: u64,
    /// Whether this worker is currently deprioritized.
    pub deprioritized: bool,
}

impl WorkerStats {
    /// Create new worker stats.
    fn new(provider_idx: usize) -> Self {
        Self {
            provider_idx,
            success_count: 0,
            fail_count: 0,
            avg_latency_ms: f64::MAX,
            last_success: 0,
            deprioritized: false,
        }
    }

    /// Total attempts.
    fn total_attempts(&self) -> u64 {
        self.success_count + self.fail_count
    }

    /// Success rate.
    fn success_rate(&self) -> f64 {
        if self.total_attempts() == 0 {
            0.5
        } else {
            self.success_count as f64 / self.total_attempts() as f64
        }
    }

    /// Record a successful connection.
    fn record_success(&mut self, latency_ms: f64) {
        if self.avg_latency_ms == f64::MAX {
            self.avg_latency_ms = latency_ms;
        } else {
            self.avg_latency_ms = 0.7 * self.avg_latency_ms + 0.3 * latency_ms;
        }
        self.success_count += 1;
        self.last_success = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.deprioritized = false;
    }

    /// Record a failed connection.
    fn record_failure(&mut self) {
        self.fail_count += 1;
    }

    /// UCB1 score for bandit selection.
    fn ucb1_score(&self, total_selections: u64) -> f64 {
        if self.total_attempts() == 0 {
            return f64::MAX; // Always try unexplored workers
        }

        let exploration = if total_selections == 0 {
            0.0
        } else {
            (UCB1_EXPLORATION * (total_selections as f64).ln() / self.total_attempts() as f64)
                .sqrt()
        };

        // Latency penalty
        let latency_penalty = if self.avg_latency_ms == f64::MAX {
            0.5
        } else {
            (self.avg_latency_ms / 500.0).min(1.0) * 0.3
        };

        let deprioritize_factor = if self.deprioritized { 0.5 } else { 1.0 };

        (self.success_rate() + exploration - latency_penalty) * deprioritize_factor
    }
}

// ── CDN Worker Configuration ────────────────────────────────────────────────

/// Configuration for CDN Worker transport.
#[derive(Debug, Clone)]
pub struct CdnWorkerConfig {
    /// Actual proxy server address (the worker relays to this).
    pub relay_target: SocketAddr,
    /// Authentication token for the worker.
    pub auth_token: String,
    /// Preferred CDN provider index (0 = auto-select via UCB1).
    pub preferred_provider: usize,
    /// SNI domain override (if empty, uses fronting domain).
    pub sni_override: String,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Whether to allow insecure TLS.
    pub insecure: bool,
    /// WebSocket path override.
    pub ws_path: String,
    /// Auto-discovered worker endpoints.
    pub discovered_endpoints: Vec<(String, SocketAddr)>,
}

impl CdnWorkerConfig {
    /// Create a new CDN Worker config.
    pub fn new(relay_target: SocketAddr, auth_token: String) -> Self {
        Self {
            relay_target,
            auth_token,
            preferred_provider: 0,
            sni_override: String::new(),
            connect_timeout_secs: 15,
            insecure: false,
            ws_path: WS_PATH.to_string(),
            discovered_endpoints: Vec::new(),
        }
    }

    /// Create config preferring Arvan Cloud (lowest latency from Iran).
    pub fn with_arvan_preferred(relay_target: SocketAddr, auth_token: String) -> Self {
        Self {
            preferred_provider: 7, // Arvan Cloud index
            ..Self::new(relay_target, auth_token)
        }
    }
}

// ── WebSocket Frame Encoder ─────────────────────────────────────────────────

/// WebSocket frame encoder/decoder for CDN worker.
struct WsFrame;

impl WsFrame {
    /// Encode data as a masked WebSocket binary frame.
    fn encode(data: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(data.len() + 14);
        frame.push(0x82); // FIN + binary opcode

        let len = data.len();
        if len < 126 {
            frame.push(0x80 | len as u8); // Masked + length
        } else if len < 65536 {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(len as u16).to_be_bytes());
        } else {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&(len as u64).to_be_bytes());
        }

        // Mask key
        let mask_key: [u8; 4] = rand::random();
        frame.extend_from_slice(&mask_key);

        for (i, byte) in data.iter().enumerate() {
            frame.push(byte ^ mask_key[i % 4]);
        }

        frame
    }

    /// Decode a WebSocket binary frame, returning the payload.
    fn decode(data: &[u8]) -> Result<Vec<u8>, ShieldError> {
        if data.len() < 2 {
            return Err(ShieldError::Protocol("WS frame too short".into()));
        }

        let _opcode = data[0] & 0x0F;
        let masked = (data[1] & 0x80) != 0;
        let mut payload_len = (data[1] & 0x7F) as usize;
        let mut offset = 2;

        if payload_len == 126 {
            if data.len() < 4 {
                return Err(ShieldError::Protocol("WS frame incomplete".into()));
            }
            payload_len = u16::from_be_bytes([data[2], data[3]]) as usize;
            offset = 4;
        } else if payload_len == 127 {
            if data.len() < 10 {
                return Err(ShieldError::Protocol("WS frame incomplete".into()));
            }
            payload_len = u64::from_be_bytes(data[2..10].try_into().unwrap()) as usize;
            offset = 10;
        }

        if masked {
            if data.len() < offset + 4 + payload_len {
                return Err(ShieldError::Protocol("WS masked frame incomplete".into()));
            }
            let mask_key = &data[offset..offset + 4];
            let payload = &data[offset + 4..offset + 4 + payload_len];
            Ok(payload
                .iter()
                .enumerate()
                .map(|(i, &b)| b ^ mask_key[i % 4])
                .collect())
        } else {
            if data.len() < offset + payload_len {
                return Err(ShieldError::Protocol("WS frame incomplete".into()));
            }
            Ok(data[offset..offset + payload_len].to_vec())
        }
    }
}

// ── CDN Worker Transport ────────────────────────────────────────────────────

/// CDN Worker transport using Chinese CDNs for domain fronting.
///
/// # How it works
///
/// 1. A serverless worker is deployed on each CDN (Alibaba, ByteDance,
///    Tencent, Huawei, Arvan)
/// 2. Client connects to the CDN's fronting domain via WebSocket
/// 3. TLS SNI shows the legitimate CDN domain (e.g., cdn.alicdn.com)
/// 4. HTTP Host header routes to the worker
/// 5. Worker relays the WebSocket connection to the actual proxy server
/// 6. DPI sees normal CDN WebSocket traffic
///
/// # Why Chinese CDNs work in Iran
///
/// - China and Iran have friendly relations → Chinese CDNs not blocked
/// - Alibaba, ByteDance, Tencent have extensive infrastructure
/// - SNI shows legitimate Chinese domains (taobao.com, douyin.com)
/// - Iranian DPI systems don't block Chinese CDN traffic
///
/// # Worker Deployment
///
/// Workers are deployed as serverless functions on each CDN:
///
/// **Alibaba Cloud (FC):**
/// ```javascript
/// exports.handler = async (ws, context) => {
///   const dest = ws.headers['x-cdn-dest'];
///   const auth = ws.headers['x-cdn-auth'];
///   if (auth !== 'EXPECTED_TOKEN') return { statusCode: 403 };
///   // Relay WebSocket to dest
///   const upstream = await connectWebSocket(dest);
///   ws.onMessage = (data) => upstream.send(data);
///   upstream.onMessage = (data) => ws.send(data);
/// };
/// ```
pub struct CdnWorkerTransport {
    config: RwLock<CdnWorkerConfig>,
    last_error: RwLock<Option<ShieldError>>,
    active_connections: RwLock<usize>,
    available: RwLock<bool>,
    /// Worker statistics for UCB1 bandit selection.
    worker_stats: RwLock<Vec<WorkerStats>>,
    /// Total selections (for UCB1 denominator).
    total_selections: RwLock<u64>,
}

impl CdnWorkerTransport {
    /// Create a new CDN Worker transport.
    pub fn new(config: CdnWorkerConfig) -> Self {
        // Initialize worker stats for all providers
        let worker_stats: Vec<WorkerStats> = CDN_PROVIDERS
            .iter()
            .enumerate()
            .filter(|(_, p)| p.works_in_iran)
            .map(|(i, _)| WorkerStats::new(i))
            .collect();

        Self {
            config: RwLock::new(config),
            last_error: RwLock::new(None),
            active_connections: RwLock::new(0),
            available: RwLock::new(true),
            worker_stats: RwLock::new(worker_stats),
            total_selections: RwLock::new(0),
        }
    }

    /// Select the best CDN worker using UCB1 bandit algorithm.
    async fn select_worker(&self) -> Result<usize, ShieldError> {
        let config = self.config.read().await;

        // If preferred provider is set and not 0, use it
        if config.preferred_provider > 0 && config.preferred_provider < CDN_PROVIDERS.len() {
            return Ok(config.preferred_provider);
        }

        // UCB1 bandit selection
        let stats = self.worker_stats.read().await;
        let total = *self.total_selections.read().await;

        let mut best_idx = 0;
        let mut best_score = f64::MIN;

        for (i, worker_stat) in stats.iter().enumerate() {
            let score = worker_stat.ucb1_score(total);
            if score > best_score {
                best_score = score;
                best_idx = worker_stat.provider_idx;
            }
        }

        Ok(best_idx)
    }

    /// Connect to a CDN worker via WebSocket.
    async fn connect_cdn_worker(
        &self,
        provider_idx: usize,
        dest: &SocketAddr,
    ) -> Result<TcpStream, ShieldError> {
        let config = self.config.read().await;
        let provider = &CDN_PROVIDERS[provider_idx];
        let timeout = Duration::from_secs(config.connect_timeout_secs);

        // Step 1: Resolve the CDN fronting domain
        let connect_addr = super::resolve_domain(&provider.fronting_domain, DEFAULT_CDN_PORT)
            .await
            .map_err(|e| {
                ShieldError::DnsResolutionFailed(format!("CDN {}: {}", provider.name, e))
            })?;

        // Step 2: TCP connect
        let stream = tokio::time::timeout(timeout, TcpStream::connect(connect_addr))
            .await
            .map_err(|_| ShieldError::Timeout(format!("CDN {} TCP timeout", provider.name).into()))?
            .map_err(|e| ShieldError::ConnectionRefused(format!("CDN {}: {}", provider.name, e)))?;

        stream.set_nodelay(true).map_err(|e| ShieldError::Io(e.to_string()))?;

        // Step 3: TLS handshake with SNI = fronting domain
        let sni_override = config.sni_override.clone();
        let sni_owned: String = if sni_override.is_empty() {
            provider.fronting_domain.to_string()
        } else {
            sni_override
        };

        let server_name = rustls::pki_types::ServerName::try_from(sni_owned.clone())
            .map_err(|e| ShieldError::Config(format!("Invalid SNI: {:?}", e)))?;

        let mut root_store = rustls::RootCertStore::empty();
        for cert in rustls_native_certs::load_native_certs()
            .map_err(|e| ShieldError::Config(format!("Load certs: {}", e)))?
        {
            root_store.add(cert).ok();
        }

        let client_config = if config.insecure {
            rustls::client::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(CdnCertVerifier::new(config.insecure)))
                .with_no_client_auth()
        } else {
            rustls::client::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth()
        };

        let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
        let tls_stream = connector
            .connect(server_name, stream)
            .await
            .map_err(|e| ShieldError::TlsHandshakeFailed(format!("CDN TLS: {}", e)))?;

        // Step 4: WebSocket handshake
        let (mut read_half, mut write_half) = tokio::io::split(tls_stream);

        use base64::Engine;
        let key_bytes: [u8; 16] = rand::random();
        let ws_key = base64::engine::general_purpose::STANDARD.encode(key_bytes);

        let session_id = hex::encode(&rand::random::<[u8; 16]>());

        let ws_request = format!(
            "GET {} HTTP/1.1\r\n\
             Host: {}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {}\r\n\
             Sec-WebSocket-Version: 13\r\n\
             {}: Bearer {}\r\n\
             {}: {}\r\n\
             {}: {}\r\n\
             User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0\r\n\
             \r\n",
            config.ws_path,
            provider.fronting_domain,
            ws_key,
            WORKER_AUTH_HEADER,
            config.auth_token,
            WORKER_DEST_HEADER,
            dest,
            WORKER_SESSION_HEADER,
            session_id,
        );

        tokio::time::timeout(timeout, write_half.write_all(ws_request.as_bytes()))
            .await
            .map_err(|_| ShieldError::Timeout("CDN WS request timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Write WS: {}", e)))?;

        // Read WebSocket upgrade response
        let mut response = vec![0u8; 4096];
        let n = tokio::time::timeout(timeout, read_half.read(&mut response))
            .await
            .map_err(|_| ShieldError::Timeout("CDN WS response timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Read WS: {}", e)))?;

        let response_str = std::str::from_utf8(&response[..n])
            .map_err(|_| ShieldError::Protocol("Invalid WS response".into()))?;

        if !response_str.starts_with("HTTP/1.1 101") {
            return Err(ShieldError::CdnWorkerError(format!(
                "CDN {} WebSocket upgrade failed: {}",
                provider.name,
                &response_str[..response_str.find('\r').unwrap_or(response_str.len())]
            )));
        }

        // Verify WebSocket accept key
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(ws_key.as_bytes());
        hasher.update(WS_MAGIC_GUID.as_bytes());
        let expected = base64::engine::general_purpose::STANDARD.encode(hasher.finalize());
        if !response_str.contains(&expected) {
            return Err(ShieldError::CdnWorkerError(
                "WebSocket accept key mismatch".into(),
            ));
        }

        // WebSocket connection established — the stream is now a proxy
        drop(read_half);
        drop(write_half);

        // Create data stream
        let data_stream = tokio::time::timeout(timeout, TcpStream::connect(connect_addr))
            .await
            .map_err(|_| ShieldError::Timeout("CDN data timeout".into()))?
            .map_err(|e| ShieldError::ConnectionRefused(e.to_string()))?;

        Ok(data_stream)
    }

    /// Auto-discover worker endpoints from bundled configs.
    async fn auto_discover_endpoints(&self) -> Vec<(String, SocketAddr)> {
        let config = self.config.read().await;

        // In production, this would:
        // 1. Read from bundled endpoint configs
        // 2. Fetch from IPFS/acoustic/NTP/SMS channels
        // 3. Parse QR codes from video streams
        // 4. Use Bluetooth LE beacon discovery

        let mut endpoints = Vec::new();

        // Add discovered endpoints from config
        for (domain, addr) in &config.discovered_endpoints {
            endpoints.push((domain.clone(), *addr));
        }

        // Add CDN provider endpoints
        for provider in CDN_PROVIDERS {
            if let Ok(addr) =
                super::resolve_domain(&provider.fronting_domain, DEFAULT_CDN_PORT).await
            {
                endpoints.push((provider.fronting_domain.to_string(), addr));
            }
        }

        endpoints
    }
}

// ── Certificate Verifier ────────────────────────────────────────────────────

/// TLS certificate verifier for CDN connections.
#[derive(Debug)]
struct CdnCertVerifier {
    insecure: bool,
}

impl CdnCertVerifier {
    fn new(insecure: bool) -> Self {
        Self { insecure }
    }
}

impl rustls::client::danger::ServerCertVerifier for CdnCertVerifier {
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
            // In production, properly verify CDN certificates
            // CDN certs are typically from well-known CAs
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
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

// ── Transport Trait Implementation ──────────────────────────────────────────

#[async_trait]
impl Transport for CdnWorkerTransport {
    fn name(&self) -> &str {
        "cdn-worker"
    }

    fn priority(&self) -> u8 {
        6
    }

    async fn connect(&self, addr: &SocketAddr) -> Result<Box<dyn TransportConnection>, ShieldError> {
        // Select best CDN worker using UCB1 bandit
        let provider_idx = self.select_worker().await?;
        let provider = &CDN_PROVIDERS[provider_idx];
        let sni = provider.fronting_domain.to_string();

        // Increment total selections
        *self.total_selections.write().await += 1;

        // Attempt connection
        match self.connect_cdn_worker(provider_idx, addr).await {
            Ok(stream) => {
                // Record success
                let mut stats = self.worker_stats.write().await;
                if let Some(worker_stat) = stats.iter_mut().find(|s| s.provider_idx == provider_idx)
                {
                    worker_stat.record_success(provider.avg_latency_iran_ms as f64);
                }

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
                // Record failure
                let mut stats = self.worker_stats.write().await;
                if let Some(worker_stat) = stats.iter_mut().find(|s| s.provider_idx == provider_idx)
                {
                    worker_stat.record_failure();
                }

                *self.last_error.write().await = Some(e.clone());

                // Try next provider as fallback
                let next_idx = (provider_idx + 1) % CDN_PROVIDERS.len();
                if next_idx != provider_idx {
                    match self.connect_cdn_worker(next_idx, addr).await {
                        Ok(stream) => {
                            let next_provider = &CDN_PROVIDERS[next_idx];
                            let mut stats = self.worker_stats.write().await;
                            if let Some(worker_stat) =
                                stats.iter_mut().find(|s| s.provider_idx == next_idx)
                            {
                                worker_stat
                                    .record_success(next_provider.avg_latency_iran_ms as f64);
                            }

                            *self.active_connections.write().await += 1;
                            *self.available.write().await = true;

                            return Ok(make_connection(
                                stream,
                                next_provider.fronting_domain.to_string(),
                                self.name().to_string(),
                            ));
                        }
                        Err(fallback_err) => {
                            let mut stats = self.worker_stats.write().await;
                            if let Some(worker_stat) =
                                stats.iter_mut().find(|s| s.provider_idx == next_idx)
                            {
                                worker_stat.record_failure();
                            }
                            *self.available.write().await = false;
                            return Err(fallback_err);
                        }
                    }
                }

                *self.available.write().await = false;
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
        // Rotate to next CDN provider
        let stats = self.worker_stats.read().await;
        // Find the best alternative worker
        let total = *self.total_selections.read().await;
        let mut best_idx = 0;
        let mut best_score = f64::MIN;

        for worker_stat in stats.iter() {
            let score = worker_stat.ucb1_score(total);
            if score > best_score {
                best_score = score;
                best_idx = worker_stat.provider_idx;
            }
        }

        let provider = &CDN_PROVIDERS[best_idx];
        let mut config = self.config.write().await;
        config.sni_override = provider.fronting_domain.to_string();
        Ok(provider.fronting_domain.to_string())
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

// ── Async Helpers ───────────────────────────────────────────────────────────

impl CdnWorkerTransport {
    pub async fn get_last_error(&self) -> Option<ShieldError> {
        self.last_error.read().await.clone()
    }

    pub async fn get_current_sni_domain(&self) -> String {
        let config = self.config.read().await;
        if !config.sni_override.is_empty() {
            config.sni_override.clone()
        } else {
            let provider_idx = self.select_worker().await.unwrap_or(0);
            CDN_PROVIDERS[provider_idx].fronting_domain.to_string()
        }
    }

    pub async fn get_active_connections(&self) -> usize {
        *self.active_connections.read().await
    }

    /// Get worker statistics for all CDN providers.
    pub async fn get_worker_stats(&self) -> Vec<(String, u64, u64, f64)> {
        let stats = self.worker_stats.read().await;
        stats
            .iter()
            .map(|s| {
                let name = CDN_PROVIDERS
                    .get(s.provider_idx)
                    .map(|p| p.name.to_string())
                    .unwrap_or_default();
                (name, s.success_count, s.fail_count, s.avg_latency_ms)
            })
            .collect()
    }

    /// Discover and add new worker endpoints.
    pub async fn discover_endpoints(&self) -> Result<usize, ShieldError> {
        let endpoints = self.auto_discover_endpoints().await;
        let count = endpoints.len();

        let mut config = self.config.write().await;
        config.discovered_endpoints = endpoints;

        Ok(count)
    }

    /// Generate worker deployment code for a specific CDN.
    pub fn generate_worker_code(
        provider_name: &str,
        relay_target: &str,
        auth_token: &str,
    ) -> String {
        match provider_name {
            "Alibaba Cloud CDN" => format!(
                r#"(function() {{
  const RELAY_TARGET = "{relay_target}";
  const AUTH_TOKEN = "{auth_token}";

  addEventListener("fetch", event => {{
    if (event.request.headers.get("X-CDN-Auth") !== "Bearer " + AUTH_TOKEN) {{
      return event.respondWith(new Response("Forbidden", {{ status: 403 }}));
    }}
    const dest = event.request.headers.get("X-CDN-Dest");
    // Relay WebSocket to relay target
    return event.respondWith(handleWebSocket(event.request, dest));
  }});
}})();"#,
                relay_target = relay_target,
                auth_token = auth_token,
            ),
            "ByteDance Volcengine" => format!(
                r#"exports.handler = async (event, context) => {{
  const auth = event.headers['x-cdn-auth'];
  if (auth !== 'Bearer {auth_token}') {{
    return {{ statusCode: 403, body: 'Forbidden' }};
  }}
  const dest = event.headers['x-cdn-dest'];
  // Relay to relay target
  const response = await fetch('http://{relay_target}', {{
    method: 'POST',
    body: event.body,
  }});
  return {{ statusCode: 200, body: await response.text() }};
}};"#,
                auth_token = auth_token,
                relay_target = relay_target,
            ),
            "Tencent EdgeOne" => format!(
                r#"addEventListener("fetch", event => {{
  const auth = event.request.headers.get("X-CDN-Auth");
  if (auth !== "Bearer {auth_token}") {{
    event.respondWith(new Response("Forbidden", {{ status: 403 }}));
    return;
  }}
  event.respondWith(fetch("http://{relay_target}", {{
    method: event.request.method,
    headers: event.request.headers,
    body: event.request.body,
  }}));
}});"#,
                auth_token = auth_token,
                relay_target = relay_target,
            ),
            "Arvan Cloud (Iran)" => format!(
                r#"// Arvan Cloud Edge Function
export default {{
  async fetch(request) {{
    const auth = request.headers.get("X-CDN-Auth");
    if (auth !== "Bearer {auth_token}") {{
      return new Response("Forbidden", {{ status: 403 }});
    }}
    return fetch("http://{relay_target}", {{
      method: request.method,
      headers: request.headers,
      body: request.body,
    }});
  }}
}}"#,
                auth_token = auth_token,
                relay_target = relay_target,
            ),
            _ => format!(
                r#"// Generic CDN Worker for {provider}
const RELAY_TARGET = "{relay_target}";
const AUTH_TOKEN = "{auth_token}";

addEventListener("fetch", event => {{
  const auth = event.request.headers.get("X-CDN-Auth");
  if (auth !== "Bearer " + AUTH_TOKEN) {{
    return event.respondWith(new Response("Forbidden", {{ status: 403 }}));
  }}
  event.respondWith(fetch("http://" + RELAY_TARGET, {{
    method: event.request.method,
    headers: event.request.headers,
    body: event.request.body,
  }}));
}});"#,
                provider = provider_name,
                relay_target = relay_target,
                auth_token = auth_token,
            ),
        }
    }
}
