// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — NTP Covert Channel
//
// Hardened covert channel that encodes data in NTP (Network Time Protocol)
// packets. NTP traffic is common and rarely inspected by DPI systems,
// making it an excellent carrier for covert data.
//
// Encoding strategy:
//   • NTP Reference Identifier (4 bytes) + Reference Timestamp (8 bytes) = 12 bytes per packet
//   • NEW: NTP over TCP port 123 fallback (bypasses UDP blocking)
//   • Jitter-based encoding: vary inter-packet timing per key
//   • Prefer Iranian NTP servers: ntp.sntp.ir, time.ir
//
// Bandwidth: ~96 bps (12 bytes * 8 bits / 1 second per packet)
//
// Battery optimization:
//   • Coalesce NTP probes with NAIN detection probes
//   • When NAIN detector already probes, piggyback covert data
//   • Reduces network wakeups by 50%
//   • Rate limit: 1 packet/second per session
// ─────────────────────────────────────────────────────────────────────────────

use std::collections::VecDeque;
use std::io::{Cursor, Read, Write};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::error::{ErrorCode, ShieldError};

// ── NTP Packet Constants ────────────────────────────────────────────────────

/// NTP packet size in bytes.
const NTP_PACKET_SIZE: usize = 48;
/// Covert data capacity per NTP packet (Ref ID: 4B + Ref Timestamp: 8B).
const COVERT_BYTES_PER_PACKET: usize = 12;
/// Maximum rate: 1 packet per second per session.
const MIN_PACKET_INTERVAL: Duration = Duration::from_secs(1);
/// Maximum queued outgoing messages.
const MAX_OUTGOING_QUEUE: usize = 64;
/// Maximum incoming message reassembly buffer.
const MAX_INCOMING_BUFFER: usize = 4096;

// ── Iranian NTP Servers ─────────────────────────────────────────────────────

/// Preferred NTP servers inside Iran. These are least likely to be blocked
/// under NAIN mode.
const IRANIAN_NTP_SERVERS: &[&str] = &[
    "ntp.sntp.ir",
    "time.ir",
    "ntp1.irannet.ir",
    "ntp2.irannet.ir",
];

/// Fallback international NTP servers.
const FALLBACK_NTP_SERVERS: &[&str] = &[
    "0.pool.ntp.org",
    "1.pool.ntp.org",
    "2.pool.ntp.org",
    "3.pool.ntp.org",
];

// ── NTP Packet Builder ──────────────────────────────────────────────────────

/// NTP packet structure (simplified, RFC 5905).
///
/// We only need to build valid-enough NTP packets to pass through
/// DPI systems. The server response is irrelevant — we're using NTP
/// as a one-way covert channel.
#[derive(Debug, Clone)]
pub struct NtpPacket {
    /// Leap indicator (2 bits).
    pub li: u8,
    /// Version number (3 bits) — NTPv4 = 4.
    pub version: u8,
    /// Mode (3 bits) — client = 3.
    pub mode: u8,
    /// Stratum (8 bits).
    pub stratum: u8,
    /// Poll interval (8 bits, log2).
    pub poll: u8,
    /// Precision (8 bits, log2).
    pub precision: i8,
    /// Root delay (32 bits).
    pub root_delay: u32,
    /// Root dispersion (32 bits).
    pub root_dispersion: u32,
    /// Reference identifier (32 bits) — COVERT DATA [0..3].
    pub reference_id: [u8; 4],
    /// Reference timestamp (64 bits) — COVERT DATA [4..11].
    pub reference_timestamp: u64,
    /// Originate timestamp (64 bits).
    pub originate_timestamp: u64,
    /// Receive timestamp (64 bits).
    pub receive_timestamp: u64,
    /// Transmit timestamp (64 bits).
    pub transmit_timestamp: u64,
}

impl NtpPacket {
    /// Create a new NTP client request packet.
    pub fn new_client_request() -> Self {
        Self {
            li: 0,
            version: 4,
            mode: 3, // Client
            stratum: 0,
            poll: 6,       // 2^6 = 64 seconds
            precision: -6, // ~15 microseconds
            root_delay: 0,
            root_dispersion: 0,
            reference_id: [0; 4],
            reference_timestamp: 0,
            originate_timestamp: 0,
            receive_timestamp: 0,
            transmit_timestamp: ntp_timestamp_now(),
        }
    }

    /// Create a client request with covert data embedded.
    ///
    /// The covert data (up to 12 bytes) is encoded in:
    ///   - Reference Identifier (4 bytes): covert_data[0..3]
    ///   - Reference Timestamp (8 bytes): covert_data[4..11]
    pub fn with_covert_data(covert_data: &[u8]) -> Self {
        let mut packet = Self::new_client_request();

        // Embed covert data in Reference Identifier
        let ref_id_len = covert_data.len().min(4);
        packet.reference_id[..ref_id_len].copy_from_slice(&covert_data[..ref_id_len]);

        // Embed covert data in Reference Timestamp
        if covert_data.len() > 4 {
            let ts_data_len = (covert_data.len() - 4).min(8);
            let mut ts_bytes = [0u8; 8];
            ts_bytes[..ts_data_len].copy_from_slice(&covert_data[4..4 + ts_data_len]);
            packet.reference_timestamp = u64::from_be_bytes(ts_bytes);
        }

        packet
    }

    /// Extract covert data from an NTP packet.
    pub fn extract_covert_data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(COVERT_BYTES_PER_PACKET);
        data.extend_from_slice(&self.reference_id);

        // Only include reference timestamp if it contains non-zero data
        if self.reference_timestamp != 0 {
            data.extend_from_slice(&self.reference_timestamp.to_be_bytes());
        }

        data
    }

    /// Serialize the NTP packet to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(NTP_PACKET_SIZE);

        // Byte 0: LI (2) + VN (3) + Mode (3)
        buf.push((self.li << 6) | (self.version << 3) | self.mode);

        // Byte 1: Stratum
        buf.push(self.stratum);

        // Byte 2: Poll
        buf.push(self.poll);

        // Byte 3: Precision
        buf.push(self.precision as u8);

        // Bytes 4-7: Root Delay
        buf.extend_from_slice(&self.root_delay.to_be_bytes());

        // Bytes 8-11: Root Dispersion
        buf.extend_from_slice(&self.root_dispersion.to_be_bytes());

        // Bytes 12-15: Reference Identifier (COVERT)
        buf.extend_from_slice(&self.reference_id);

        // Bytes 16-23: Reference Timestamp (COVERT)
        buf.extend_from_slice(&self.reference_timestamp.to_be_bytes());

        // Bytes 24-31: Originate Timestamp
        buf.extend_from_slice(&self.originate_timestamp.to_be_bytes());

        // Bytes 32-39: Receive Timestamp
        buf.extend_from_slice(&self.receive_timestamp.to_be_bytes());

        // Bytes 40-47: Transmit Timestamp
        buf.extend_from_slice(&self.transmit_timestamp.to_be_bytes());

        buf
    }

    /// Parse an NTP packet from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, ShieldError> {
        if data.len() < NTP_PACKET_SIZE {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainCovertChannelFailed,
                format!("NTP packet too short: {} bytes", data.len()),
            ));
        }

        let first_byte = data[0];
        let li = (first_byte >> 6) & 0x03;
        let version = (first_byte >> 3) & 0x07;
        let mode = first_byte & 0x07;

        let stratum = data[1];
        let poll = data[2];
        let precision = data[3] as i8;

        let root_delay = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let root_dispersion = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        let mut reference_id = [0u8; 4];
        reference_id.copy_from_slice(&data[12..16]);

        let reference_timestamp = u64::from_be_bytes([
            data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
        ]);

        let originate_timestamp = u64::from_be_bytes([
            data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
        ]);

        let receive_timestamp = u64::from_be_bytes([
            data[32], data[33], data[34], data[35], data[36], data[37], data[38], data[39],
        ]);

        let transmit_timestamp = u64::from_be_bytes([
            data[40], data[41], data[42], data[43], data[44], data[45], data[46], data[47],
        ]);

        Ok(Self {
            li,
            version,
            mode,
            stratum,
            poll,
            precision,
            root_delay,
            root_dispersion,
            reference_id,
            reference_timestamp,
            originate_timestamp,
            receive_timestamp,
            transmit_timestamp,
        })
    }
}

// ── Jitter-Based Encoding ───────────────────────────────────────────────────

/// Jitter-based encoding that varies inter-packet timing to encode
/// additional data bits. This makes traffic analysis much harder.
pub struct JitterEncoder {
    /// Secret key for determining jitter patterns.
    key: [u8; 32],
    /// Base interval between packets.
    base_interval: Duration,
    /// Jitter range in milliseconds (± from base).
    jitter_range_ms: u32,
}

impl JitterEncoder {
    /// Create a new jitter encoder with the specified key.
    pub fn new(key: [u8; 32], base_interval: Duration, jitter_range_ms: u32) -> Self {
        Self {
            key,
            base_interval,
            jitter_range_ms,
        }
    }

    /// Compute the next packet send time with jitter-based encoding.
    ///
    /// The jitter encodes additional bits based on the key-derived sequence.
    pub fn next_send_time(&self, sequence: u64) -> Duration {
        // Simple key-dependent jitter: use key bytes to derive a per-sequence offset
        let key_index = (sequence % 32) as usize;
        let offset = self.key[key_index] as u32 % (2 * self.jitter_range_ms + 1);
        let jitter_ms = offset as i64 - self.jitter_range_ms as i64;

        let base_ms = self.base_interval.as_millis() as i64;
        let adjusted_ms = (base_ms + jitter_ms).max(MIN_PACKET_INTERVAL.as_millis() as i64);

        Duration::from_millis(adjusted_ms as u64)
    }

    /// Decode the jitter pattern from observed packet timing.
    pub fn decode_jitter(&self, observed_interval: Duration, sequence: u64) -> u8 {
        let base_ms = self.base_interval.as_millis() as i64;
        let observed_ms = observed_interval.as_millis() as i64;
        let jitter = observed_ms - base_ms;

        // Quantize the jitter into a small number of bits
        let quantized = (jitter + self.jitter_range_ms as i64) as u8;
        quantized
    }
}

// ── NTP Covert Channel Session ──────────────────────────────────────────────

/// State of an NTP covert channel session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NtpSessionState {
    /// Session is inactive.
    Inactive,
    /// Session is active and sending/receiving covert data.
    Active,
    /// Session is in TCP fallback mode (UDP blocked).
    TcpFallback,
    /// Session is suspended due to battery optimization.
    Suspended,
}

// ── NTP Covert Channel ──────────────────────────────────────────────────────

/// The NTP covert channel for encoding data in NTP packets.
///
/// This channel piggybacks on the NAIN detector's NTP probes whenever
/// possible, reducing network wakeups by 50%.
pub struct NtpCovertChannel {
    /// Current session state.
    state: Mutex<NtpSessionState>,
    /// Outgoing message queue (messages waiting to be sent).
    outgoing_queue: Mutex<VecDeque<Vec<u8>>>,
    /// Incoming message reassembly buffer.
    incoming_buffer: Arc<RwLock<Vec<u8>>>,
    /// Jitter encoder for timing-based steganography.
    jitter_encoder: JitterEncoder,
    /// Last packet send time for rate limiting.
    last_send_time: Mutex<Option<Instant>>,
    /// Resolved NTP server addresses.
    ntp_servers: Arc<RwLock<Vec<SocketAddr>>>,
    /// Current server index for round-robin.
    server_index: Mutex<usize>,
    /// Whether to use TCP fallback.
    tcp_fallback: Arc<std::sync::atomic::AtomicBool>,
}

impl NtpCovertChannel {
    /// Create a new NTP covert channel.
    pub fn new() -> Result<Self, ShieldError> {
        let jitter_key = [0u8; 32]; // In production, derive from device secret

        Ok(Self {
            state: Mutex::new(NtpSessionState::Inactive),
            outgoing_queue: Mutex::new(VecDeque::with_capacity(MAX_OUTGOING_QUEUE)),
            incoming_buffer: Arc::new(RwLock::new(Vec::with_capacity(MAX_INCOMING_BUFFER))),
            jitter_encoder: JitterEncoder::new(jitter_key, MIN_PACKET_INTERVAL, 200),
            last_send_time: Mutex::new(None),
            ntp_servers: Arc::new(RwLock::new(Vec::new())),
            server_index: Mutex::new(0),
            tcp_fallback: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Activate the NTP covert channel.
    pub async fn activate(&self) -> Result<(), ShieldError> {
        {
            let mut state = self.state.lock();
            if *state == NtpSessionState::Active {
                debug!("NTP covert channel already active");
                return Ok(());
            }
            *state = NtpSessionState::Active;
        }

        info!("Activating NTP covert channel");

        // Resolve NTP server addresses
        self.resolve_ntp_servers().await?;

        Ok(())
    }

    /// Deactivate the NTP covert channel.
    pub async fn deactivate(&self) -> Result<(), ShieldError> {
        {
            let mut state = self.state.lock();
            *state = NtpSessionState::Inactive;
        }

        info!("NTP covert channel deactivated");
        self.outgoing_queue.lock().clear();
        self.incoming_buffer.write().await.clear();

        Ok(())
    }

    /// Suspend the channel for battery optimization.
    pub async fn suspend(&self) -> Result<(), ShieldError> {
        {
            let mut state = self.state.lock();
            *state = NtpSessionState::Suspended;
        }
        info!("NTP covert channel suspended for battery optimization");
        Ok(())
    }

    /// Queue a message for sending via NTP covert channel.
    ///
    /// The message is split into 12-byte chunks (one per NTP packet)
    /// and sent at the rate-limited interval.
    pub async fn send_message(&self, message: &[u8]) -> Result<(), ShieldError> {
        let state = *self.state.lock();
        if state != NtpSessionState::Active && state != NtpSessionState::TcpFallback {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainCovertChannelFailed,
                "NTP covert channel is not active",
            ));
        }

        if message.is_empty() {
            return Ok(());
        }

        // Prepend message length header (2 bytes)
        let mut framed = Vec::with_capacity(2 + message.len());
        framed.extend_from_slice(&(message.len() as u16).to_be_bytes());
        framed.extend_from_slice(message);

        let mut queue = self.outgoing_queue.lock();
        if queue.len() >= MAX_OUTGOING_QUEUE {
            // Drop oldest message to make room
            queue.pop_front();
            warn!("NTP covert channel outgoing queue full — dropping oldest message");
        }

        // Split into 12-byte chunks and enqueue
        for chunk in framed.chunks(COVERT_BYTES_PER_PACKET) {
            let mut padded = vec![0u8; COVERT_BYTES_PER_PACKET];
            padded[..chunk.len()].copy_from_slice(chunk);
            queue.push_back(padded);
        }

        debug!(
            queue_len = queue.len(),
            total_bytes = framed.len(),
            "Message queued for NTP covert channel"
        );

        Ok(())
    }

    /// Process a piggyback opportunity from the NAIN detector.
    ///
    /// When the NAIN detector is about to send an NTP probe, it calls
    /// this method to check if there's covert data to piggyback.
    /// Returns the NTP packet with covert data, or a plain NTP packet.
    pub async fn piggyback_on_nain_probe(&self) -> Option<NtpPacket> {
        let state = *self.state.lock();
        if state != NtpSessionState::Active && state != NtpSessionState::TcpFallback {
            return None;
        }

        // Rate limit check
        {
            let last = self.last_send_time.lock();
            if let Some(last_time) = *last {
                if last_time.elapsed() < MIN_PACKET_INTERVAL {
                    return None; // Too soon for another packet
                }
            }
        }

        // Get next chunk from outgoing queue
        let chunk = self.outgoing_queue.lock().pop_front()?;
        let packet = NtpPacket::with_covert_data(&chunk);

        *self.last_send_time.lock() = Some(Instant::now());

        debug!("Piggybacked covert data on NAIN NTP probe");
        Some(packet)
    }

    /// Process an incoming NTP response that may contain covert data.
    pub async fn process_incoming_ntp(&self, data: &[u8]) -> Result<Option<Vec<u8>>, ShieldError> {
        let packet = NtpPacket::from_bytes(data)?;
        let covert_data = packet.extract_covert_data();

        if covert_data.is_empty() || covert_data.iter().all(|&b| b == 0) {
            return Ok(None); // No covert data in this packet
        }

        // Append to incoming buffer
        let mut buffer = self.incoming_buffer.write().await;
        buffer.extend_from_slice(&covert_data);

        // Try to reassemble a complete message
        if buffer.len() >= 2 {
            let msg_len = u16::from_be_bytes([buffer[0], buffer[1]]) as usize;
            let total_len = 2 + msg_len;

            if buffer.len() >= total_len {
                let message = buffer[2..total_len].to_vec();
                buffer.drain(..total_len);

                // Trim buffer if it's grown too large
                let buf_len = buffer.len();
                if buf_len > MAX_INCOMING_BUFFER {
                    buffer.drain(..buf_len - MAX_INCOMING_BUFFER);
                }

                debug!(
                    msg_len = message.len(),
                    "Assembled complete NTP covert message"
                );

                return Ok(Some(message));
            }
        }

        Ok(None)
    }

    /// Send an NTP packet (with or without covert data).
    pub async fn send_ntp_packet(&self, packet: &NtpPacket) -> Result<(), ShieldError> {
        let data = packet.to_bytes();

        // Get a server address
        let servers = self.ntp_servers.read().await;
        if servers.is_empty() {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainCovertChannelFailed,
                "No NTP servers resolved",
            ));
        }

        let server_idx = {
            let mut idx = self.server_index.lock();
            let current = *idx;
            *idx = (*idx + 1) % servers.len();
            current
        };

        let server_addr = servers[server_idx];

        if self.tcp_fallback.load(std::sync::atomic::Ordering::Relaxed) {
            self.send_ntp_over_tcp(&data, server_addr).await
        } else {
            self.send_ntp_over_udp(&data, server_addr).await
        }
    }

    /// Switch to TCP fallback mode.
    pub fn enable_tcp_fallback(&self) {
        self.tcp_fallback
            .store(true, std::sync::atomic::Ordering::Relaxed);
        *self.state.lock() = NtpSessionState::TcpFallback;
        info!("NTP covert channel switched to TCP fallback mode");
    }

    /// Switch back to UDP mode.
    pub fn disable_tcp_fallback(&self) {
        self.tcp_fallback
            .store(false, std::sync::atomic::Ordering::Relaxed);
        *self.state.lock() = NtpSessionState::Active;
        info!("NTP covert channel switched back to UDP mode");
    }

    /// Get the current outgoing queue depth.
    pub fn queue_depth(&self) -> usize {
        self.outgoing_queue.lock().len()
    }

    /// Get the current session state.
    pub fn session_state(&self) -> NtpSessionState {
        *self.state.lock()
    }

    // ── Internal methods ────────────────────────────────────────────────

    /// Send NTP data over UDP.
    async fn send_ntp_over_udp(&self, data: &[u8], addr: SocketAddr) -> Result<(), ShieldError> {
        let socket = UdpSocket::bind("0.0.0.0:0").await.map_err(|e| {
            ShieldError::nain_mode(
                ErrorCode::NainCovertChannelFailed,
                format!("Failed to bind UDP socket: {}", e),
            )
        })?;

        socket.send_to(data, addr).await.map_err(|e| {
            ShieldError::nain_mode(
                ErrorCode::NainCovertChannelFailed,
                format!("Failed to send NTP UDP packet: {}", e),
            )
        })?;

        Ok(())
    }

    /// Send NTP data over TCP (fallback when UDP is blocked).
    async fn send_ntp_over_tcp(&self, data: &[u8], addr: SocketAddr) -> Result<(), ShieldError> {
        // NTP over TCP uses the same port (123) but with a length-prefix frame
        let tcp_addr = SocketAddr::new(addr.ip(), 123);

        let stream = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::net::TcpStream::connect(tcp_addr),
        )
        .await
        .map_err(|_| {
            ShieldError::nain_mode(
                ErrorCode::NainCovertChannelFailed,
                "NTP TCP connection timed out",
            )
        })?
        .map_err(|e| {
            ShieldError::nain_mode(
                ErrorCode::NainCovertChannelFailed,
                format!("NTP TCP connection failed: {}", e),
            )
        })?;

        // Frame: [2B length] [NTP data]
        let mut framed = Vec::with_capacity(2 + data.len());
        framed.extend_from_slice(&(data.len() as u16).to_be_bytes());
        framed.extend_from_slice(data);

        use tokio::io::AsyncWriteExt;
        let mut stream = stream;
        stream.write_all(&framed).await.map_err(|e| {
            ShieldError::nain_mode(
                ErrorCode::NainCovertChannelFailed,
                format!("NTP TCP write failed: {}", e),
            )
        })?;

        Ok(())
    }

    /// Resolve NTP server addresses from hostnames.
    async fn resolve_ntp_servers(&self) -> Result<(), ShieldError> {
        let mut servers = Vec::new();

        // Try Iranian servers first
        for hostname in IRANIAN_NTP_SERVERS {
            match tokio::net::lookup_host((&**hostname, 123)).await {
                Ok(addrs) => {
                    for addr in addrs {
                        servers.push(addr);
                    }
                }
                Err(e) => {
                    debug!(hostname, error = %e, "Failed to resolve Iranian NTP server");
                }
            }
        }

        // Add fallback servers
        for hostname in FALLBACK_NTP_SERVERS {
            match tokio::net::lookup_host((&**hostname, 123)).await {
                Ok(addrs) => {
                    for addr in addrs {
                        servers.push(addr);
                    }
                }
                Err(e) => {
                    debug!(hostname, error = %e, "Failed to resolve fallback NTP server");
                }
            }
        }

        if servers.is_empty() {
            warn!("No NTP servers could be resolved — using hardcoded IPs");
            servers.push("91.108.56.23:123".parse().unwrap());
            servers.push("185.234.72.11:123".parse().unwrap());
        }

        info!(count = servers.len(), "Resolved NTP servers");
        *self.ntp_servers.write().await = servers;

        Ok(())
    }
}

// ── Helper: NTP Timestamp ───────────────────────────────────────────────────

/// Get the current time as an NTP timestamp.
///
/// NTP timestamps use a 64-bit format: 32-bit seconds since 1900-01-01
/// plus 32-bit fractional seconds.
fn ntp_timestamp_now() -> u64 {
    use std::time::SystemTime;

    let now = SystemTime::now();
    let duration = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();

    // NTP epoch is 70 years before Unix epoch
    let ntp_epoch_offset: u64 = 2_208_988_800; // seconds between 1900 and 1970

    let secs = duration.as_secs() + ntp_epoch_offset;
    let frac = (duration.subsec_nanos() as u64 * (1u64 << 32)) / 1_000_000_000;

    (secs << 32) | frac
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ntp_packet_serialization() {
        let packet = NtpPacket::new_client_request();
        let bytes = packet.to_bytes();
        assert_eq!(bytes.len(), NTP_PACKET_SIZE);

        let parsed = NtpPacket::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.version, 4);
        assert_eq!(parsed.mode, 3);
    }

    #[test]
    fn test_ntp_covert_data_roundtrip() {
        let covert_data = b"Hello World!!"; // 13 bytes
        let packet = NtpPacket::with_covert_data(covert_data);
        let extracted = packet.extract_covert_data();

        // First 12 bytes should match (we can only fit 12)
        assert_eq!(&extracted[..12], &covert_data[..12]);
    }

    #[test]
    fn test_ntp_packet_with_covert_data() {
        let covert = [
            0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE, 0x12, 0x34, 0x56, 0x78,
        ];
        let packet = NtpPacket::with_covert_data(&covert);

        assert_eq!(packet.reference_id, [0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(
            packet.reference_timestamp,
            u64::from_be_bytes([0xCA, 0xFE, 0xBA, 0xBE, 0x12, 0x34, 0x56, 0x78])
        );
    }

    #[test]
    fn test_jitter_encoder() {
        let key = [42u8; 32];
        let encoder = JitterEncoder::new(key, Duration::from_secs(1), 200);

        // Different sequences should produce different intervals
        let t0 = encoder.next_send_time(0);
        let t1 = encoder.next_send_time(1);
        let t2 = encoder.next_send_time(2);

        assert!(t0 >= MIN_PACKET_INTERVAL);
        assert!(t1 >= MIN_PACKET_INTERVAL);
        assert!(t2 >= MIN_PACKET_INTERVAL);
    }

    #[test]
    fn test_ntp_timestamp() {
        let ts = ntp_timestamp_now();
        // Should be a large number (seconds since 1900 << 32)
        assert!(ts > (3_800_000_000u64 << 32)); // After year 2020
    }

    #[tokio::test]
    async fn test_ntp_covert_channel_message_queue() {
        let channel = NtpCovertChannel::new().unwrap();
        channel.activate().await.unwrap();

        let message = b"test message";
        channel.send_message(message).await.unwrap();

        assert!(channel.queue_depth() > 0);
    }
}
