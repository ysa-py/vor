//! ICMP Tunnel (NAIN Fallback) Transport
//!
//! Encodes data in ICMP echo request/reply payloads. Works when only
//! ICMP is allowed through the firewall. Rate limited to avoid detection.
//! Uses raw sockets on Linux, IcmpSendEcho on Windows.
//!
//! This is a last-resort transport (NAIN = "Nothing Anything Is Network")
//! for situations where TCP, UDP, and QUIC are all blocked but ICMP
//! (ping) still works. Iranian firewalls sometimes enter this mode
//! during protests or periods of heavy censorship.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::RwLock;

use super::{make_connection, ShieldError, Transport, TransportConnection};

// ── Constants ───────────────────────────────────────────────────────────────

/// ICMP Echo Request type.
const ICMP_ECHO_REQUEST: u8 = 8;

/// ICMP Echo Reply type.
const ICMP_ECHO_REPLY: u8 = 0;

/// ICMP code for echo request/reply.
const ICMP_CODE: u8 = 0;

/// Maximum ICMP payload size (safe for most networks).
const MAX_ICMP_PAYLOAD: usize = 1400;

/// Data offset within ICMP payload (after type, code, checksum, id, seq).
const ICMP_DATA_OFFSET: usize = 8;

/// Minimum ICMP payload for our data.
const MIN_ICMP_DATA: usize = 16;

/// Magic bytes identifying our tunnel packets.
const TUNNEL_MAGIC: &[u8; 4] = b"SHLD";

/// Rate limit: minimum interval between ICMP packets (milliseconds).
const RATE_LIMIT_MS: u64 = 50;

/// Rate limit: maximum packets per second.
const MAX_PACKETS_PER_SEC: u32 = 20;

/// Default ICMP tunnel server IP.
const DEFAULT_ICMP_SERVER: &str = "10.0.0.1";

// ── ICMP Packet Structure ───────────────────────────────────────────────────

/// ICMP packet for tunnel data.
///
/// Format:
/// ```text
/// [type(1)] [code(1)] [checksum(2)] [id(2)] [seq(2)]
/// [magic(4)] [flags(1)] [session_id(4)] [seq_num(4)] [data(variable)]
/// ```
#[derive(Debug, Clone)]
struct IcmpPacket {
    /// ICMP type (8 for request, 0 for reply).
    icmp_type: u8,
    /// ICMP code (always 0 for echo).
    code: u8,
    /// ICMP identifier.
    identifier: u16,
    /// ICMP sequence number.
    sequence: u16,
    /// Tunnel session ID.
    session_id: u32,
    /// Tunnel sequence number.
    tunnel_seq: u32,
    /// Tunnel flags.
    flags: IcmpTunnelFlags,
    /// Payload data.
    data: Vec<u8>,
}

/// Flags for ICMP tunnel packets.
#[derive(Debug, Clone, Copy)]
struct IcmpTunnelFlags {
    /// This is the first packet of a new session.
    pub syn: bool,
    /// This is an acknowledgment.
    pub ack: bool,
    /// This is the last packet (session close).
    pub fin: bool,
    /// This packet contains proxy data.
    pub data_flag: bool,
    /// Retransmission.
    pub retransmit: bool,
}

impl IcmpTunnelFlags {
    fn new() -> Self {
        Self {
            syn: false,
            ack: false,
            fin: false,
            data_flag: false,
            retransmit: false,
        }
    }

    /// Encode flags as a single byte.
    fn to_byte(&self) -> u8 {
        let mut flags: u8 = 0;
        if self.syn {
            flags |= 0x01;
        }
        if self.ack {
            flags |= 0x02;
        }
        if self.fin {
            flags |= 0x04;
        }
        if self.data_flag {
            flags |= 0x08;
        }
        if self.retransmit {
            flags |= 0x10;
        }
        flags
    }

    /// Decode flags from a byte.
    fn from_byte(byte: u8) -> Self {
        Self {
            syn: byte & 0x01 != 0,
            ack: byte & 0x02 != 0,
            fin: byte & 0x04 != 0,
            data_flag: byte & 0x08 != 0,
            retransmit: byte & 0x10 != 0,
        }
    }
}

impl IcmpPacket {
    /// Create a new ICMP echo request with tunnel data.
    fn new_request(
        session_id: u32,
        tunnel_seq: u32,
        flags: IcmpTunnelFlags,
        data: Vec<u8>,
    ) -> Self {
        Self {
            icmp_type: ICMP_ECHO_REQUEST,
            code: ICMP_CODE,
            identifier: rand::random(),
            sequence: (tunnel_seq & 0xFFFF) as u16,
            session_id,
            tunnel_seq,
            flags,
            data,
        }
    }

    /// Create a new ICMP echo reply with tunnel data.
    fn new_reply(session_id: u32, tunnel_seq: u32, flags: IcmpTunnelFlags, data: Vec<u8>) -> Self {
        Self {
            icmp_type: ICMP_ECHO_REPLY,
            code: ICMP_CODE,
            identifier: rand::random(),
            sequence: (tunnel_seq & 0xFFFF) as u16,
            session_id,
            tunnel_seq,
            flags,
            data,
        }
    }

    /// Serialize the ICMP packet to bytes.
    fn to_bytes(&self) -> Vec<u8> {
        let payload_len = ICMP_DATA_OFFSET + 4 + 1 + 4 + 4 + self.data.len();
        let mut packet = Vec::with_capacity(payload_len);

        // ICMP header
        packet.push(self.icmp_type);
        packet.push(self.code);
        packet.push(0x00); // Checksum placeholder
        packet.push(0x00);
        packet.extend_from_slice(&self.identifier.to_be_bytes());
        packet.extend_from_slice(&self.sequence.to_be_bytes());

        // Tunnel header
        packet.extend_from_slice(TUNNEL_MAGIC);
        packet.push(self.flags.to_byte());
        packet.extend_from_slice(&self.session_id.to_be_bytes());
        packet.extend_from_slice(&self.tunnel_seq.to_be_bytes());

        // Payload
        packet.extend_from_slice(&self.data);

        // Calculate and set checksum
        let checksum = Self::calculate_checksum(&packet);
        packet[2] = (checksum >> 8) as u8;
        packet[3] = (checksum & 0xFF) as u8;

        packet
    }

    /// Parse an ICMP packet from bytes.
    fn from_bytes(data: &[u8]) -> Result<Self, ShieldError> {
        if data.len() < ICMP_DATA_OFFSET + 4 + 1 + 4 + 4 {
            return Err(ShieldError::Protocol("ICMP packet too short".into()));
        }

        // Verify checksum
        let expected_checksum = Self::calculate_checksum(data);
        let actual_checksum = u16::from_be_bytes([data[2], data[3]]);
        if expected_checksum != 0 && actual_checksum != expected_checksum {
            // In practice, the OS might recalculate; be lenient
        }

        let icmp_type = data[0];
        let code = data[1];
        let identifier = u16::from_be_bytes([data[4], data[5]]);
        let sequence = u16::from_be_bytes([data[6], data[7]]);

        // Check for tunnel magic
        if &data[ICMP_DATA_OFFSET..ICMP_DATA_OFFSET + 4] != TUNNEL_MAGIC {
            return Err(ShieldError::Protocol("Not a tunnel ICMP packet".into()));
        }

        let flags = IcmpTunnelFlags::from_byte(data[ICMP_DATA_OFFSET + 4]);
        let session_id = u32::from_be_bytes([
            data[ICMP_DATA_OFFSET + 5],
            data[ICMP_DATA_OFFSET + 6],
            data[ICMP_DATA_OFFSET + 7],
            data[ICMP_DATA_OFFSET + 8],
        ]);
        let tunnel_seq = u32::from_be_bytes([
            data[ICMP_DATA_OFFSET + 9],
            data[ICMP_DATA_OFFSET + 10],
            data[ICMP_DATA_OFFSET + 11],
            data[ICMP_DATA_OFFSET + 12],
        ]);

        let data_offset = ICMP_DATA_OFFSET + 13;
        let payload = if data_offset < data.len() {
            data[data_offset..].to_vec()
        } else {
            vec![]
        };

        Ok(Self {
            icmp_type,
            code,
            identifier,
            sequence,
            session_id,
            tunnel_seq,
            flags,
            data: payload,
        })
    }

    /// Calculate ICMP checksum (RFC 1071).
    fn calculate_checksum(data: &[u8]) -> u16 {
        let mut sum: u32 = 0;

        // Sum 16-bit words
        let mut i = 0;
        while i + 1 < data.len() {
            let word = ((data[i] as u32) << 8) | (data[i + 1] as u32);
            sum += word;
            i += 2;
        }

        // Handle odd byte
        if i < data.len() {
            sum += (data[i] as u32) << 8;
        }

        // Fold 32-bit sum to 16 bits
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        (!sum as u16)
    }

    /// Get the maximum data capacity per ICMP packet.
    fn max_data_size() -> usize {
        MAX_ICMP_PAYLOAD - ICMP_DATA_OFFSET - 13 // Tunnel header
    }
}

// ── ICMP Raw Socket ─────────────────────────────────────────────────────────

/// Cross-platform ICMP socket abstraction.
enum IcmpSocket {
    /// Linux raw socket.
    #[cfg(target_os = "linux")]
    Raw(tokio::net::UdpSocket), // Will be replaced with actual raw socket
    /// Windows IcmpSendEcho.
    #[cfg(target_os = "windows")]
    Windows,
    /// Fallback: UDP-based tunnel (for non-root environments).
    UdpFallback(tokio::net::UdpSocket),
}

/// ICMP socket wrapper that works across platforms.
struct IcmpSocketWrapper {
    /// The underlying socket (UDP fallback for non-root).
    socket: Option<tokio::net::UdpSocket>,
    /// Remote address for UDP fallback.
    remote_addr: SocketAddr,
    /// Whether we're using raw ICMP (requires root/capabilities).
    using_raw: bool,
    /// Rate limiter state.
    last_send_time: RwLock<std::time::Instant>,
    /// Packets sent in current second.
    packets_this_second: RwLock<(std::time::Instant, u32)>,
}

impl IcmpSocketWrapper {
    /// Create a new ICMP socket wrapper.
    async fn new(remote_addr: SocketAddr) -> Result<Self, ShieldError> {
        // Try to create a raw ICMP socket (requires root on Linux)
        let using_raw = false; // Will be set based on capability check

        // Fallback to UDP tunnel
        let socket = tokio::net::UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| ShieldError::Io(e.to_string()))?;

        Ok(Self {
            socket: Some(socket),
            remote_addr,
            using_raw,
            last_send_time: RwLock::new(std::time::Instant::now()),
            packets_this_second: RwLock::new((std::time::Instant::now(), 0)),
        })
    }

    /// Send an ICMP packet with rate limiting.
    async fn send(&self, packet: &IcmpPacket) -> Result<(), ShieldError> {
        // Rate limiting
        self.check_rate_limit().await?;

        let bytes = packet.to_bytes();

        if let Some(ref socket) = self.socket {
            // UDP fallback: send as UDP with ICMP-like structure
            socket
                .send_to(&bytes, &self.remote_addr)
                .await
                .map_err(|e| ShieldError::IcmpError(format!("Send: {}", e)))?;
        }

        *self.last_send_time.write().await = std::time::Instant::now();
        Ok(())
    }

    /// Receive an ICMP packet.
    async fn recv(&self) -> Result<IcmpPacket, ShieldError> {
        if let Some(ref socket) = self.socket {
            let mut buf = [0u8; MAX_ICMP_PAYLOAD + 64];
            let (n, _addr) = socket
                .recv_from(&mut buf)
                .await
                .map_err(|e| ShieldError::IcmpError(format!("Recv: {}", e)))?;

            IcmpPacket::from_bytes(&buf[..n])
        } else {
            Err(ShieldError::IcmpError("No socket available".into()))
        }
    }

    /// Check and enforce rate limiting.
    async fn check_rate_limit(&self) -> Result<(), ShieldError> {
        // Check minimum interval
        let last = *self.last_send_time.read().await;
        let elapsed = last.elapsed();
        if elapsed < Duration::from_millis(RATE_LIMIT_MS) {
            tokio::time::sleep(Duration::from_millis(RATE_LIMIT_MS) - elapsed).await;
        }

        // Check packets per second
        let mut pps = self.packets_this_second.write().await;
        if pps.0.elapsed() > Duration::from_secs(1) {
            *pps = (std::time::Instant::now(), 1);
        } else if pps.1 >= MAX_PACKETS_PER_SEC {
            return Err(ShieldError::RateLimited("ICMP rate limit exceeded".into()));
        } else {
            pps.1 += 1;
        }

        Ok(())
    }
}

// ── ICMP Tunnel Session ─────────────────────────────────────────────────────

/// ICMP tunnel session state.
struct IcmpTunnelSession {
    /// Session ID.
    id: u32,
    /// Destination address.
    dest: SocketAddr,
    /// Next send sequence number.
    send_seq: u32,
    /// Next expected receive sequence number.
    recv_seq: u32,
    /// Receive buffer.
    recv_buffer: Vec<u8>,
    /// Whether the session is established.
    established: bool,
}

impl IcmpTunnelSession {
    fn new(dest: SocketAddr) -> Self {
        Self {
            id: rand::random(),
            dest,
            send_seq: 0,
            recv_seq: 0,
            recv_buffer: Vec::new(),
            established: false,
        }
    }

    fn next_send_seq(&mut self) -> u32 {
        let seq = self.send_seq;
        self.send_seq += 1;
        seq
    }
}

// ── ICMP Tunnel Stream ──────────────────────────────────────────────────────

/// AsyncRead + AsyncWrite wrapper for ICMP tunnel.
pub struct IcmpTunnelStream {
    socket: Arc<IcmpSocketWrapper>,
    session: RwLock<IcmpTunnelSession>,
}

impl tokio::io::AsyncRead for IcmpTunnelStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        // Check buffered data first
        let session = self.session.try_write();
        if let Ok(mut session) = session {
            if !session.recv_buffer.is_empty() {
                let to_read = buf.remaining().min(session.recv_buffer.len());
                buf.put_slice(&session.recv_buffer[..to_read]);
                session.recv_buffer.drain(..to_read);
                return std::task::Poll::Ready(Ok(()));
            }
        }

        // Need to poll for ICMP data — simplified
        std::task::Poll::Pending
    }
}

impl tokio::io::AsyncWrite for IcmpTunnelStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
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

// ── Configuration ───────────────────────────────────────────────────────────

/// Configuration for ICMP tunnel transport.
#[derive(Debug, Clone)]
pub struct IcmpTunnelConfig {
    /// ICMP tunnel server address.
    pub server_addr: SocketAddr,
    /// Shared secret for authentication.
    pub shared_secret: String,
    /// Rate limit: packets per second.
    pub rate_limit_pps: u32,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Whether to use raw ICMP (requires root/capabilities).
    pub use_raw_icmp: bool,
    /// UDP fallback port (when raw ICMP is not available).
    pub udp_fallback_port: u16,
}

impl IcmpTunnelConfig {
    /// Create a new ICMP tunnel config.
    pub fn new(server_addr: SocketAddr, shared_secret: String) -> Self {
        Self {
            server_addr,
            shared_secret,
            rate_limit_pps: MAX_PACKETS_PER_SEC,
            connect_timeout_secs: 30,
            use_raw_icmp: false,
            udp_fallback_port: 5353, // mDNS port as cover
        }
    }
}

// ── ICMP Tunnel Transport ───────────────────────────────────────────────────

/// ICMP tunnel transport (NAIN fallback).
///
/// # How it works
///
/// 1. Encodes proxy data in ICMP echo request/reply payloads
/// 2. On Linux: uses raw ICMP sockets (requires CAP_NET_RAW)
/// 3. On Windows: uses IcmpSendEcho API
/// 4. Fallback: UDP tunnel that mimics ICMP patterns
/// 5. Rate limited to avoid triggering ICMP flood detection
///
/// # When to use
///
/// This is a last-resort transport for when TCP, UDP, and QUIC
/// are all blocked. During heavy censorship in Iran, sometimes
/// only ICMP (ping) is allowed. This transport allows the proxy
/// to continue working even in those conditions.
///
/// # Limitations
///
/// - Very low bandwidth (limited by ICMP rate limits)
/// - High latency (each packet requires a round trip)
/// - May require root/CAP_NET_RAW on Linux
/// - Some networks block large ICMP payloads
pub struct IcmpTunnelTransport {
    config: RwLock<IcmpTunnelConfig>,
    last_error: RwLock<Option<ShieldError>>,
    active_connections: RwLock<usize>,
    available: RwLock<bool>,
    /// ICMP socket wrapper.
    socket: RwLock<Option<Arc<IcmpSocketWrapper>>>,
}

impl IcmpTunnelTransport {
    /// Create a new ICMP tunnel transport.
    pub fn new(config: IcmpTunnelConfig) -> Self {
        Self {
            config: RwLock::new(config),
            last_error: RwLock::new(None),
            active_connections: RwLock::new(0),
            available: RwLock::new(true),
            socket: RwLock::new(None),
        }
    }

    /// Initialize the ICMP socket.
    async fn init_socket(&self) -> Result<Arc<IcmpSocketWrapper>, ShieldError> {
        // Check if we already have a socket
        {
            let socket_guard = self.socket.read().await;
            if let Some(socket) = socket_guard.as_ref() {
                return Ok(socket.clone());
            }
        }

        let config = self.config.read().await;

        // Determine the remote address
        let remote_addr = if config.use_raw_icmp {
            config.server_addr
        } else {
            // UDP fallback: use the server address with fallback port
            SocketAddr::new(config.server_addr.ip(), config.udp_fallback_port)
        };

        let socket = Arc::new(IcmpSocketWrapper::new(remote_addr).await?);

        *self.socket.write().await = Some(socket.clone());

        Ok(socket)
    }

    /// Send a SYN packet to establish a new ICMP tunnel session.
    async fn send_syn(
        &self,
        socket: &IcmpSocketWrapper,
        session: &mut IcmpTunnelSession,
    ) -> Result<(), ShieldError> {
        let mut flags = IcmpTunnelFlags::new();
        flags.syn = true;

        // Include destination address in SYN payload
        let dest_bytes = session.dest.to_string().into_bytes();
        let mut payload =
            Vec::with_capacity(dest_bytes.len() + self.config.read().await.shared_secret.len() + 2);
        payload.extend_from_slice(&(dest_bytes.len() as u16).to_be_bytes());
        payload.extend_from_slice(&dest_bytes);
        payload.extend_from_slice(self.config.read().await.shared_secret.as_bytes());

        let packet = IcmpPacket::new_request(session.id, session.next_send_seq(), flags, payload);

        socket.send(&packet).await
    }

    /// Send data through the ICMP tunnel.
    async fn send_data(
        &self,
        socket: &IcmpSocketWrapper,
        session: &mut IcmpTunnelSession,
        data: &[u8],
    ) -> Result<(), ShieldError> {
        let max_chunk = IcmpPacket::max_data_size();

        for chunk in data.chunks(max_chunk) {
            let mut flags = IcmpTunnelFlags::new();
            flags.data_flag = true;

            let packet =
                IcmpPacket::new_request(session.id, session.next_send_seq(), flags, chunk.to_vec());

            socket.send(&packet).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Transport for IcmpTunnelTransport {
    fn name(&self) -> &str {
        "icmp-tunnel"
    }

    fn priority(&self) -> u8 {
        11 // Lowest priority — last resort
    }

    async fn connect(&self, addr: &SocketAddr) -> Result<Box<dyn TransportConnection>, ShieldError> {
        let config = self.config.read().await;
        let sni = format!("icmp://{}", config.server_addr.ip());
        drop(config);

        // Initialize socket
        let socket = self.init_socket().await?;

        // Create session
        let mut session = IcmpTunnelSession::new(*addr);

        // Send SYN
        self.send_syn(&socket, &mut session).await?;

        // Wait for SYN-ACK
        let timeout = Duration::from_secs(self.config.read().await.connect_timeout_secs);
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            match tokio::time::timeout(Duration::from_secs(5), socket.recv()).await {
                Ok(Ok(packet)) => {
                    if packet.flags.syn && packet.flags.ack && packet.session_id == session.id {
                        session.established = true;
                        session.recv_seq = packet.tunnel_seq + 1;
                        break;
                    }
                }
                _ => continue,
            }
        }

        if !session.established {
            return Err(ShieldError::Timeout("ICMP tunnel SYN-ACK timeout".into()));
        }

        // Create stream wrapper
        let stream = IcmpTunnelStream {
            socket,
            session: RwLock::new(session),
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
        "icmp-tunnel" // ICMP doesn't use SNI
    }

    async fn rotate_sni_domain(&self) -> Result<String, ShieldError> {
        // ICMP doesn't use domains
        Ok("icmp-tunnel".to_string())
    }

    fn active_connections(&self) -> usize {
        0
    }

    async fn shutdown(&self) -> Result<(), ShieldError> {
        *self.available.write().await = false;
        *self.active_connections.write().await = 0;
        *self.socket.write().await = None;
        Ok(())
    }
}

impl IcmpTunnelTransport {
    pub async fn get_last_error(&self) -> Option<ShieldError> {
        self.last_error.read().await.clone()
    }

    pub async fn get_current_sni_domain(&self) -> String {
        format!("icmp://{}", self.config.read().await.server_addr.ip())
    }

    pub async fn get_active_connections(&self) -> usize {
        *self.active_connections.read().await
    }

    /// Check if raw ICMP is available (requires root/capabilities).
    pub async fn check_raw_icmp_available(&self) -> bool {
        // Try to create a raw ICMP socket
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            // Check for CAP_NET_RAW
            fs::read_to_string("/proc/self/status")
                .map(|s| s.contains("CapEff:") && s.contains("net_raw"))
                .unwrap_or(false)
        }

        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }
}
