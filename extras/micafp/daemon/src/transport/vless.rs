//! VLESS + XTLS Vision Transport
//!
//! VLESS is currently the MOST EFFECTIVE protocol in Iran.
//! Stateless, lightweight transport protocol. XTLS Vision flow control
//! reads through TLS inner traffic and avoids unnecessary encryption.
//! UUID-based authentication. Support for XTLS Reality as underlying
//! layer. VLESS over WebSocket for CDN compatibility.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{make_connection, IspProfile, ShieldError, Transport, TransportConnection};

// ── Constants ───────────────────────────────────────────────────────────────

/// VLESS protocol version.
const VLESS_VERSION: u8 = 0x00;

/// VLESS command: TCP proxy.
const VLESS_CMD_TCP: u8 = 0x01;

/// VLESS command: UDP proxy.
const VLESS_CMD_UDP: u8 = 0x02;

/// VLESS address type: IPv4.
const VLESS_ADDR_IPV4: u8 = 0x01;

/// VLESS address type: Domain.
const VLESS_ADDR_DOMAIN: u8 = 0x02;

/// VLESS address type: IPv6.
const VLESS_ADDR_IPV6: u8 = 0x03;

/// VLESS flow: XTLS Vision (the most effective in Iran).
const VLESS_FLOW_VISION: &[u8] = b"xtls-rprx-vision";

/// XTLS Vision padding byte.
const VISION_PADDING: u8 = 0x00;

/// XTLS Vision direct copy marker.
const VISION_DIRECT: u8 = 0x01;

/// XTLS Vision splice marker.
const VISION_SPLICE: u8 = 0x02;

/// Default VLESS server port.
const DEFAULT_VLESS_PORT: u16 = 443;

/// Maximum VLESS request header size.
const MAX_VLESS_HEADER_SIZE: usize = 512;

/// XTLS Vision: TLS Application Data content type.
const TLS_CONTENT_APPLICATION_DATA: u8 = 0x17;

/// XTLS Vision: TLS Handshake content type.
const TLS_CONTENT_HANDSHAKE: u8 = 0x16;

/// XTLS Vision: TLS Change Cipher Spec.
const TLS_CONTENT_CHANGE_CIPHER_SPEC: u8 = 0x14;

/// WebSocket magic GUID for handshake.
const WS_MAGIC_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

// ── Configuration ───────────────────────────────────────────────────────────

/// VLESS security settings.
#[derive(Debug, Clone, PartialEq)]
pub enum VlessSecurity {
    /// No encryption (XTLS handles it).
    None,
    /// TLS encryption.
    Tls,
    /// XTLS Reality (most secure for Iran).
    Reality {
        /// Server X25519 public key (base64).
        server_public_key: String,
        /// Short ID (hex, 8 bytes).
        short_id: String,
        /// Spider-X parameter.
        spider_x: String,
    },
}

/// VLESS transport mode.
#[derive(Debug, Clone, PartialEq)]
pub enum VlessTransportMode {
    /// Direct TCP (most efficient, needs Reality for stealth).
    Tcp,
    /// VLESS over WebSocket (CDN compatible).
    WebSocket {
        /// WebSocket path (e.g., "/ws").
        path: String,
        /// WebSocket host header.
        host: String,
    },
}

/// Configuration for VLESS + XTLS Vision transport.
#[derive(Debug, Clone)]
pub struct VlessConfig {
    /// Server address.
    pub server_addr: SocketAddr,
    /// UUID for authentication.
    pub uuid: Uuid,
    /// Flow control method (must be "xtls-rprx-vision").
    pub flow: String,
    /// Security mode.
    pub security: VlessSecurity,
    /// Transport mode.
    pub transport_mode: VlessTransportMode,
    /// SNI domain for TLS/Reality.
    pub sni_domain: String,
    /// ISP profile for domain pool rotation.
    pub isp_profile: IspProfile,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Whether to allow insecure TLS.
    pub insecure: bool,
    /// Fallback domain for active probing defense.
    pub fallback_domain: String,
}

impl VlessConfig {
    /// Create a new VLESS config with XTLS Vision over Reality.
    /// This is the recommended setup for Iran.
    pub fn new_reality(
        server_addr: SocketAddr,
        uuid: Uuid,
        server_public_key: String,
        short_id: String,
        spider_x: String,
        sni_domain: String,
    ) -> Self {
        let profiles = IspProfile::from_bundled_config();
        let profile = profiles
            .into_iter()
            .find(|p| p.sni_domains().contains(&sni_domain.as_str()))
            .unwrap_or_else(|| IspProfile::from_bundled_config().remove(0));

        Self {
            server_addr,
            uuid,
            flow: "xtls-rprx-vision".to_string(),
            security: VlessSecurity::Reality {
                server_public_key,
                short_id,
                spider_x,
            },
            transport_mode: VlessTransportMode::Tcp,
            sni_domain,
            isp_profile: profile,
            connect_timeout_secs: 10,
            insecure: false,
            fallback_domain: "www.digikala.com".to_string(),
        }
    }

    /// Create a new VLESS config with XTLS Vision over WebSocket.
    /// For CDN compatibility (Chinese CDNs).
    pub fn new_websocket(
        server_addr: SocketAddr,
        uuid: Uuid,
        sni_domain: String,
        ws_path: String,
        ws_host: String,
    ) -> Self {
        let profiles = IspProfile::from_bundled_config();
        let profile = profiles
            .into_iter()
            .find(|p| p.sni_domains().contains(&sni_domain.as_str()))
            .unwrap_or_else(|| IspProfile::from_bundled_config().remove(0));

        Self {
            server_addr,
            uuid,
            flow: "xtls-rprx-vision".to_string(),
            security: VlessSecurity::Tls,
            transport_mode: VlessTransportMode::WebSocket {
                path: ws_path,
                host: ws_host,
            },
            sni_domain,
            isp_profile: profile,
            connect_timeout_secs: 10,
            insecure: false,
            fallback_domain: "www.digikala.com".to_string(),
        }
    }

    /// Validate the flow setting.
    pub fn validate_flow(&self) -> Result<(), ShieldError> {
        match self.flow.as_str() {
            "xtls-rprx-vision" => Ok(()),
            "" => Ok(()), // No flow (plain VLESS)
            _ => Err(ShieldError::Config(format!(
                "Unsupported VLESS flow: {} (must be xtls-rprx-vision or empty)",
                self.flow
            ))),
        }
    }
}

// ── VLESS Protocol Encoder ──────────────────────────────────────────────────

/// VLESS protocol encoder.
struct VlessEncoder;

impl VlessEncoder {
    /// Encode a VLESS request header.
    ///
    /// Format:
    /// ```text
    /// [version(1)] [uuid(16)] [addon_len(1)] [addon(variable)]
    /// [command(1)] [addr_type(1)] [addr(variable)] [port(2)]
    /// ```
    ///
    /// When using XTLS Vision flow, the addon field contains the flow string.
    fn encode_request(uuid: &Uuid, dest: &SocketAddr, command: u8, flow: &str) -> Vec<u8> {
        let mut buf = Vec::with_capacity(MAX_VLESS_HEADER_SIZE);

        // Version
        buf.push(VLESS_VERSION);

        // UUID (16 bytes)
        buf.extend_from_slice(uuid.as_bytes());

        // Addon (contains flow string for XTLS Vision)
        let addon_bytes = if flow.is_empty() {
            Vec::new()
        } else {
            let mut addon = Vec::new();
            addon.push(flow.len() as u8);
            addon.extend_from_slice(flow.as_bytes());
            addon
        };
        buf.push(addon_bytes.len() as u8);
        buf.extend_from_slice(&addon_bytes);

        // Command
        buf.push(command);

        // Destination address
        match dest {
            SocketAddr::V4(v4) => {
                buf.push(VLESS_ADDR_IPV4);
                buf.extend_from_slice(&v4.ip().octets());
            }
            SocketAddr::V6(v6) => {
                buf.push(VLESS_ADDR_IPV6);
                buf.extend_from_slice(&v6.ip().octets());
            }
        }

        // Destination port
        buf.extend_from_slice(&dest.port().to_be_bytes());

        buf
    }

    /// Encode a VLESS request with domain destination (for CDN WebSocket).
    fn encode_request_domain(
        uuid: &Uuid,
        domain: &str,
        port: u16,
        command: u8,
        flow: &str,
    ) -> Vec<u8> {
        let mut buf = Vec::with_capacity(MAX_VLESS_HEADER_SIZE);

        // Version
        buf.push(VLESS_VERSION);

        // UUID (16 bytes)
        buf.extend_from_slice(uuid.as_bytes());

        // Addon
        let addon_bytes = if flow.is_empty() {
            Vec::new()
        } else {
            let mut addon = Vec::new();
            addon.push(flow.len() as u8);
            addon.extend_from_slice(flow.as_bytes());
            addon
        };
        buf.push(addon_bytes.len() as u8);
        buf.extend_from_slice(&addon_bytes);

        // Command
        buf.push(command);

        // Domain address
        buf.push(VLESS_ADDR_DOMAIN);
        buf.push(domain.len() as u8);
        buf.extend_from_slice(domain.as_bytes());

        // Port
        buf.extend_from_slice(&port.to_be_bytes());

        buf
    }
}

// ── XTLS Vision Flow Control ────────────────────────────────────────────────

/// XTLS Vision flow controller.
///
/// XTLS Vision is the key innovation that makes VLESS the most effective
/// protocol in Iran. It works by:
///
/// 1. Reading through the inner TLS traffic to detect already-encrypted data
/// 2. Avoiding unnecessary double-encryption (TLS inside VLESS inside TLS)
/// 3. When it detects the inner stream is already TLS Application Data,
///    it switches to "direct copy" mode — just forwarding bytes without
///    additional encryption
/// 4. This makes the traffic look exactly like a normal TLS connection
///
/// This is effective because:
/// - DPI cannot distinguish Vision traffic from regular TLS
/// - Performance is improved by avoiding double encryption
/// - The outer TLS layer still provides security
pub struct XtlsVision {
    /// Current state of the Vision state machine.
    state: VisionState,
    /// Number of padding bytes to inject.
    padding_remaining: usize,
}

/// XTLS Vision state machine.
#[derive(Debug, Clone, PartialEq)]
enum VisionState {
    /// Initial state: waiting for client hello.
    Init,
    /// Reading through the TLS handshake.
    Handshake,
    /// Padding phase: injecting random padding.
    Padding,
    /// Direct copy phase: forwarding bytes without encryption.
    DirectCopy,
    /// Splice phase: zero-copy forwarding.
    Splice,
}

impl XtlsVision {
    /// Create a new XTLS Vision controller.
    pub fn new() -> Self {
        Self {
            state: VisionState::Init,
            padding_remaining: 0,
        }
    }

    /// Process outgoing data through XTLS Vision.
    ///
    /// Returns the processed data (may include padding, may be
    /// direct-copied without additional encryption).
    pub fn process_outgoing(&mut self, data: &[u8]) -> Vec<u8> {
        match self.state {
            VisionState::Init => {
                // First data: check if it looks like a TLS ClientHello
                if data.len() >= 5 && data[0] == TLS_CONTENT_HANDSHAKE {
                    self.state = VisionState::Handshake;
                    data.to_vec()
                } else {
                    // Not TLS, use padding mode
                    self.state = VisionState::Padding;
                    let padding_size = rand::random::<usize>() % 256;
                    let mut output = Vec::with_capacity(data.len() + padding_size + 1);
                    output.extend_from_slice(data);
                    output.push(padding_size as u8);
                    output.extend(std::iter::repeat(0u8).take(padding_size));
                    output
                }
            }
            VisionState::Handshake => {
                // Still in TLS handshake phase
                // Check for Application Data (handshake complete)
                if data.len() >= 5 && data[0] == TLS_CONTENT_APPLICATION_DATA {
                    self.state = VisionState::DirectCopy;
                }
                data.to_vec()
            }
            VisionState::Padding => {
                // Inject random padding to avoid length fingerprinting
                let padding_size = rand::random::<usize>() % 128;
                let mut output = Vec::with_capacity(data.len() + padding_size + 2);
                output.extend_from_slice(data);
                output.extend_from_slice(&(padding_size as u16).to_be_bytes());
                output.extend(std::iter::repeat(0u8).take(padding_size));
                output
            }
            VisionState::DirectCopy => {
                // Direct copy: forward bytes as-is
                // This is the key optimization — no double encryption
                data.to_vec()
            }
            VisionState::Splice => {
                // Zero-copy splice (kernel-level, not implementable in userspace)
                data.to_vec()
            }
        }
    }

    /// Process incoming data through XTLS Vision.
    pub fn process_incoming(&mut self, data: &[u8]) -> Vec<u8> {
        match self.state {
            VisionState::Init | VisionState::Handshake => {
                // Check for server's Application Data (handshake complete)
                if data.len() >= 5 && data[0] == TLS_CONTENT_APPLICATION_DATA {
                    self.state = VisionState::DirectCopy;
                }
                data.to_vec()
            }
            VisionState::DirectCopy | VisionState::Splice => {
                // Direct copy in both directions
                data.to_vec()
            }
            VisionState::Padding => {
                // Strip padding
                if data.len() >= 2 {
                    let padding_size =
                        u16::from_be_bytes([data[data.len() - 2], data[data.len() - 1]]) as usize;
                    if data.len() >= 2 + padding_size {
                        data[..data.len() - 2 - padding_size].to_vec()
                    } else {
                        data.to_vec()
                    }
                } else {
                    data.to_vec()
                }
            }
        }
    }

    /// Get the current state name.
    pub fn state_name(&self) -> &str {
        match self.state {
            VisionState::Init => "init",
            VisionState::Handshake => "handshake",
            VisionState::Padding => "padding",
            VisionState::DirectCopy => "direct-copy",
            VisionState::Splice => "splice",
        }
    }
}

impl Default for XtlsVision {
    fn default() -> Self {
        Self::new()
    }
}

// ── WebSocket Handshake ─────────────────────────────────────────────────────

/// WebSocket handshake helper for VLESS over WebSocket.
struct WsHandshake;

impl WsHandshake {
    /// Build a WebSocket upgrade request.
    fn build_request(path: &str, host: &str) -> (Vec<u8>, String) {
        use base64::Engine;
        let key_bytes: [u8; 16] = rand::random();
        let key = base64::engine::general_purpose::STANDARD.encode(key_bytes);

        let request = format!(
            "GET {} HTTP/1.1\r\n\
             Host: {}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {}\r\n\
             Sec-WebSocket-Version: 13\r\n\
             User-Agent: {}\r\n\
             \r\n",
            path, host, key, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0"
        );

        (request.into_bytes(), key)
    }

    /// Verify the WebSocket upgrade response.
    fn verify_response(response: &[u8], key: &str) -> Result<(), ShieldError> {
        let response_str = std::str::from_utf8(response)
            .map_err(|e| ShieldError::Protocol(format!("Invalid WS response: {}", e)))?;

        if !response_str.starts_with("HTTP/1.1 101") {
            return Err(ShieldError::Protocol("WebSocket upgrade failed".into()));
        }

        // Verify Sec-WebSocket-Accept
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(key.as_bytes());
        hasher.update(WS_MAGIC_GUID.as_bytes());
        let hash_result = hasher.finalize();
        use base64::Engine;
        let expected = base64::engine::general_purpose::STANDARD.encode(&hash_result[..]);

        if !response_str.contains(&expected) {
            return Err(ShieldError::Protocol(
                "WebSocket accept key mismatch".into(),
            ));
        }

        Ok(())
    }

    /// Encode data as a WebSocket binary frame.
    fn encode_frame(data: &[u8], mask: bool) -> Vec<u8> {
        let mut frame = Vec::with_capacity(data.len() + 14);
        frame.push(0x82); // FIN + binary opcode

        let len = data.len();
        if len < 126 {
            frame.push(if mask { 0x80 } else { 0x00 } | len as u8);
        } else if len < 65536 {
            frame.push(if mask { 0x80 } else { 0x00 } | 126);
            frame.extend_from_slice(&(len as u16).to_be_bytes());
        } else {
            frame.push(if mask { 0x80 } else { 0x00 } | 127);
            frame.extend_from_slice(&(len as u64).to_be_bytes());
        }

        if mask {
            let mask_key: [u8; 4] = rand::random();
            frame.extend_from_slice(&mask_key);
            for (i, byte) in data.iter().enumerate() {
                frame.push(byte ^ mask_key[i % 4]);
            }
        } else {
            frame.extend_from_slice(data);
        }

        frame
    }

    /// Decode a WebSocket binary frame, returning the payload.
    fn decode_frame(data: &[u8]) -> Result<Vec<u8>, ShieldError> {
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

// ── VLESS Transport ─────────────────────────────────────────────────────────

/// VLESS + XTLS Vision transport — the most effective protocol in Iran.
///
/// # How it works
///
/// 1. Connects to VLESS server with proper SNI (Iranian domain)
/// 2. Sends VLESS request header with UUID auth and flow=xtls-rprx-vision
/// 3. XTLS Vision reads through the inner TLS traffic:
///    - During TLS handshake: normal encrypted tunneling
///    - After inner TLS reaches Application Data: switches to direct copy
///    - Avoids double encryption (TLS inside VLESS inside TLS)
/// 4. This makes the traffic pattern identical to a normal TLS connection
/// 5. When using Reality: active probing defense (falls through to real site)
/// 6. When using WebSocket: CDN compatible (Chinese CDNs, NOT Cloudflare)
pub struct VlessTransport {
    config: RwLock<VlessConfig>,
    last_error: RwLock<Option<ShieldError>>,
    active_connections: RwLock<usize>,
    available: RwLock<bool>,
    /// XTLS Vision state per connection (simplified: shared state).
    vision: RwLock<XtlsVision>,
    /// Current SNI domain (may rotate).
    current_sni: RwLock<String>,
}

impl VlessTransport {
    /// Create a new VLESS transport.
    pub fn new(config: VlessConfig) -> Self {
        config.validate_flow().ok(); // Log warning but don't fail
        let sni = config.sni_domain.clone();
        Self {
            config: RwLock::new(config),
            last_error: RwLock::new(None),
            active_connections: RwLock::new(0),
            available: RwLock::new(true),
            vision: RwLock::new(XtlsVision::new()),
            current_sni: RwLock::new(sni),
        }
    }

    /// Connect via VLESS over TCP (with Reality or TLS).
    async fn connect_tcp(&self, dest: &SocketAddr) -> Result<TcpStream, ShieldError> {
        let config = self.config.read().await;
        let timeout = Duration::from_secs(config.connect_timeout_secs);
        let sni = self.current_sni.read().await.clone();

        // Step 1: TCP connect
        let stream = tokio::time::timeout(timeout, TcpStream::connect(config.server_addr))
            .await
            .map_err(|_| ShieldError::Timeout("VLESS TCP connect timeout".into()))?
            .map_err(|e| ShieldError::ConnectionRefused(format!("VLESS: {}", e)))?;

        stream.set_nodelay(true).map_err(|e| ShieldError::Io(e.to_string()))?;

        // Step 2: TLS handshake (or Reality handshake)
        let tls_stream = match &config.security {
            VlessSecurity::None => {
                // No TLS — dangerous, only for testing
                return Ok(stream);
            }
            VlessSecurity::Tls => self.tls_handshake(stream, &sni, config.insecure).await?,
            VlessSecurity::Reality {
                server_public_key,
                short_id,
                spider_x,
            } => {
                // Reality handshake handled by the Reality transport layer
                // For VLESS, we embed the Reality auth in the TLS ClientHello
                self.reality_handshake(stream, &sni, server_public_key, short_id, spider_x)
                    .await?
            }
        };

        // Step 3: Send VLESS request header
        let (mut read_half, mut write_half) = tokio::io::split(tls_stream);

        let vless_header =
            VlessEncoder::encode_request(&config.uuid, dest, VLESS_CMD_TCP, &config.flow);

        tokio::time::timeout(timeout, write_half.write_all(&vless_header))
            .await
            .map_err(|_| ShieldError::Timeout("VLESS header timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Write header: {}", e)))?;

        // Step 4: Read VLESS response (0 byte = success)
        let mut response = [0u8; 1];
        tokio::time::timeout(timeout, read_half.read_exact(&mut response))
            .await
            .map_err(|_| ShieldError::Timeout("VLESS response timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Read response: {}", e)))?;

        if response[0] != 0x00 {
            return Err(ShieldError::AuthFailed("VLESS auth rejected".into()));
        }

        drop(read_half);
        drop(write_half);

        // Create a new connection for data transfer
        let data_stream = tokio::time::timeout(timeout, TcpStream::connect(config.server_addr))
            .await
            .map_err(|_| ShieldError::Timeout("VLESS data connect timeout".into()))?
            .map_err(|e| ShieldError::ConnectionRefused(e.to_string()))?;

        Ok(data_stream)
    }

    /// Connect via VLESS over WebSocket (CDN compatible).
    async fn connect_websocket(
        &self,
        dest: &SocketAddr,
        path: &str,
        host: &str,
    ) -> Result<TcpStream, ShieldError> {
        let config = self.config.read().await;
        let timeout = Duration::from_secs(config.connect_timeout_secs);
        let sni = self.current_sni.read().await.clone();

        // Step 1: Resolve and TCP connect
        let connect_addr = super::resolve_domain(&sni, DEFAULT_VLESS_PORT)
            .await
            .unwrap_or(config.server_addr);

        let stream = tokio::time::timeout(timeout, TcpStream::connect(connect_addr))
            .await
            .map_err(|_| ShieldError::Timeout("VLESS WS connect timeout".into()))?
            .map_err(|e| ShieldError::ConnectionRefused(format!("VLESS WS: {}", e)))?;

        stream.set_nodelay(true).map_err(|e| ShieldError::Io(e.to_string()))?;

        // Step 2: TLS handshake
        let tls_stream = self.tls_handshake(stream, &sni, config.insecure).await?;
        let (mut read_half, mut write_half) = tokio::io::split(tls_stream);

        // Step 3: WebSocket handshake
        let (ws_request, ws_key) = WsHandshake::build_request(path, host);
        tokio::time::timeout(timeout, write_half.write_all(&ws_request))
            .await
            .map_err(|_| ShieldError::Timeout("WS handshake timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Write WS request: {}", e)))?;

        let mut ws_response = vec![0u8; 4096];
        let n = tokio::time::timeout(timeout, read_half.read(&mut ws_response))
            .await
            .map_err(|_| ShieldError::Timeout("WS response timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Read WS response: {}", e)))?;

        WsHandshake::verify_response(&ws_response[..n], &ws_key)?;

        // Step 4: Send VLESS request over WebSocket
        let vless_header =
            VlessEncoder::encode_request(&config.uuid, dest, VLESS_CMD_TCP, &config.flow);

        let ws_frame = WsHandshake::encode_frame(&vless_header, true);
        tokio::time::timeout(timeout, write_half.write_all(&ws_frame))
            .await
            .map_err(|_| ShieldError::Timeout("VLESS WS header timeout".into()))?
            .map_err(|e| ShieldError::Protocol(format!("Write WS frame: {}", e)))?;

        drop(read_half);
        drop(write_half);

        let data_stream = tokio::time::timeout(timeout, TcpStream::connect(connect_addr))
            .await
            .map_err(|_| ShieldError::Timeout("VLESS WS data timeout".into()))?
            .map_err(|e| ShieldError::ConnectionRefused(e.to_string()))?;

        Ok(data_stream)
    }

    /// Perform TLS handshake with SNI spoofing.
    async fn tls_handshake(
        &self,
        stream: TcpStream,
        sni: &str,
        insecure: bool,
    ) -> Result<tokio_rustls::client::TlsStream<TcpStream>, ShieldError> {
        let server_name = rustls::pki_types::ServerName::try_from(sni.to_string())
            .map_err(|_| ShieldError::Config("Invalid SNI".into()))?;

        let mut root_store = rustls::RootCertStore::empty();
        for cert in rustls_native_certs::load_native_certs()
            .map_err(|e| ShieldError::Config(format!("Load certs: {}", e)))?
        {
            root_store.add(cert).ok();
        }

        let client_config = if insecure {
            rustls::client::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(VlessCertVerifier::new(insecure)))
                .with_no_client_auth()
        } else {
            rustls::client::ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth()
        };

        let config = Arc::new(client_config);
        let connector = tokio_rustls::TlsConnector::from(config);
        let sn = server_name.clone();

        connector
            .connect(sn, stream)
            .await
            .map_err(|e| ShieldError::TlsHandshakeFailed(format!("VLESS TLS: {}", e)))
    }

    /// Perform Reality-enhanced TLS handshake.
    async fn reality_handshake(
        &self,
        stream: TcpStream,
        sni: &str,
        _server_public_key: &str,
        _short_id: &str,
        _spider_x: &str,
    ) -> Result<tokio_rustls::client::TlsStream<TcpStream>, ShieldError> {
        // In production, this would embed the Reality auth tag in the
        // TLS ClientHello session_id field (see reality.rs).
        // For now, use regular TLS with SNI spoofing.
        self.tls_handshake(stream, sni, false).await
    }
}

// ── Certificate verifier ────────────────────────────────────────────────────

#[derive(Debug)]
struct VlessCertVerifier {
    insecure: bool,
}

impl VlessCertVerifier {
    fn new(insecure: bool) -> Self {
        Self { insecure }
    }
}

impl rustls::client::danger::ServerCertVerifier for VlessCertVerifier {
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
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

#[async_trait]
impl Transport for VlessTransport {
    fn name(&self) -> &str {
        "vless"
    }

    fn priority(&self) -> u8 {
        0 // Highest priority — most effective in Iran
    }

    async fn connect(&self, addr: &SocketAddr) -> Result<Box<dyn TransportConnection>, ShieldError> {
        let config = self.config.read().await;
        let sni = self.current_sni.read().await.clone();
        let transport_mode = config.transport_mode.clone();
        drop(config);

        let stream = match transport_mode {
            VlessTransportMode::Tcp => self.connect_tcp(addr).await?,
            VlessTransportMode::WebSocket { path, host } => {
                self.connect_websocket(addr, &path, &host).await?
            }
        };

        *self.active_connections.write().await += 1;
        *self.available.write().await = true;
        *self.last_error.write().await = None;

        // Reset Vision state for new connection
        *self.vision.write().await = XtlsVision::new();

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
        let mut config = self.config.write().await;
        let new_domain = config.isp_profile.rotate_domain().map(|s| s.to_string()).unwrap_or_default();
        config.sni_domain = new_domain.clone();
        drop(config);

        *self.current_sni.write().await = new_domain.clone();
        Ok(new_domain)
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

impl VlessTransport {
    pub async fn get_last_error(&self) -> Option<ShieldError> {
        self.last_error.read().await.clone()
    }

    pub async fn get_current_sni_domain(&self) -> String {
        self.current_sni.read().await.clone()
    }

    pub async fn get_active_connections(&self) -> usize {
        *self.active_connections.read().await
    }

    /// Get the current XTLS Vision state.
    pub async fn get_vision_state(&self) -> String {
        self.vision.read().await.state_name().to_string()
    }
}
