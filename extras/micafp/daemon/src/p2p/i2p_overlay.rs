// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — I2P Overlay Network
//
// I2P (Invisible Internet Project) provides garlic-routed anonymous
// communication through a distributed network. Unlike Tor, I2P is
// optimized for peer-to-peer services rather than exit nodes.
//
// This module integrates with the I2P router via the SAM (Simple Anonymous
// Messaging) bridge, providing TCP-like connections through I2P streaming.
//
// Key features:
//   • Garlic routing for anonymous communication
//   • I2P streaming protocol for TCP-like connections
//   • SAM bridge integration (127.0.0.1:7656)
//   • Tunnel creation with configurable hop count
//   • Destination generation from local keys
// ─────────────────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::{ErrorCode, ShieldError};

// ── Constants ───────────────────────────────────────────────────────────────

/// Default SAM bridge address.
const SAM_BRIDGE_ADDR: &str = "127.0.0.1:7656";
/// Default SAM protocol version.
const SAM_VERSION: &str = "3.1";
/// Default tunnel hop count (for inbound tunnels).
const DEFAULT_INBOUND_HOPS: u8 = 3;
/// Default tunnel hop count (for outbound tunnels).
const DEFAULT_OUTBOUND_HOPS: u8 = 3;
/// Connection timeout for SAM bridge.
const SAM_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Maximum tunnel lease duration.
const MAX_LEASE_DURATION: Duration = Duration::from_secs(600);
/// Maximum number of concurrent I2P tunnels.
const MAX_TUNNELS: usize = 32;

// ── I2P Destination ─────────────────────────────────────────────────────────

/// An I2P destination (public key + signing key + certificate).
///
/// I2P destinations are 516+ byte base64-encoded identifiers that
/// uniquely identify an I2P service.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct I2pDestination {
    /// Base64-encoded destination string.
    pub base64: String,
    /// Whether this is a local destination (we hold the private key).
    pub is_local: bool,
}

impl I2pDestination {
    /// Create a new I2P destination from a base64 string.
    pub fn from_base64(base64: String, is_local: bool) -> Self {
        Self { base64, is_local }
    }

    /// Get the .i2p address (SHA-256 hash of the destination, base32 encoded).
    pub fn i2p_address(&self) -> String {
        // In production, this computes:
        //   base32(sha256(destination_bytes)) + ".b32.i2p"
        // For now, return a truncated hash
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(self.base64.as_bytes());
        format!("{}.b32.i2p", base32_encode(&hash[..10]))
    }
}

// ── I2P Tunnel ──────────────────────────────────────────────────────────────

/// An active I2P tunnel.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct I2pTunnel {
    /// Unique tunnel identifier.
    pub id: u32,
    /// Local destination for this tunnel.
    pub local_destination: I2pDestination,
    /// Remote destination (if connected).
    pub remote_destination: Option<I2pDestination>,
    /// Inbound hop count.
    pub inbound_hops: u8,
    /// Outbound hop count.
    pub outbound_hops: u8,
    /// Local port that the tunnel is bound to.
    pub local_port: u16,
    /// Whether the tunnel is active.
    pub active: bool,
    /// When the tunnel was created (UNIX timestamp ms).
    #[serde(default)]
    pub created_at: u64,
    /// Bytes sent through this tunnel.
    pub bytes_sent: u64,
    /// Bytes received through this tunnel.
    pub bytes_recv: u64,
}

// ── I2P Configuration ──────────────────────────────────────────────────────

/// Configuration for the I2P overlay.
#[derive(Debug, Clone)]
pub struct I2pConfig {
    /// SAM bridge address.
    pub sam_addr: String,
    /// SAM protocol version.
    pub sam_version: String,
    /// Default inbound tunnel hop count.
    pub inbound_hops: u8,
    /// Default outbound tunnel hop count.
    pub outbound_hops: u8,
    /// Whether to start a local I2P router if one isn't running.
    pub start_local_router: bool,
    /// Maximum number of concurrent tunnels.
    pub max_tunnels: usize,
}

impl Default for I2pConfig {
    fn default() -> Self {
        Self {
            sam_addr: SAM_BRIDGE_ADDR.to_string(),
            sam_version: SAM_VERSION.to_string(),
            inbound_hops: DEFAULT_INBOUND_HOPS,
            outbound_hops: DEFAULT_OUTBOUND_HOPS,
            start_local_router: false,
            max_tunnels: MAX_TUNNELS,
        }
    }
}

// ── SAM Bridge Client ───────────────────────────────────────────────────────

/// Client for the I2P SAM (Simple Anonymous Messaging) bridge.
///
/// The SAM bridge allows applications to create I2P sessions and
/// tunnels without embedding a full I2P router in the process.
pub struct SamBridgeClient {
    /// SAM bridge address.
    sam_addr: String,
    /// SAM protocol version.
    version: String,
    /// Whether we have an active connection to the SAM bridge.
    connected: Arc<std::sync::atomic::AtomicBool>,
}

impl SamBridgeClient {
    /// Create a new SAM bridge client.
    pub fn new(sam_addr: String, version: String) -> Self {
        Self {
            sam_addr,
            version,
            connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Connect to the SAM bridge and perform the handshake.
    pub async fn connect(&self) -> Result<TcpStream, ShieldError> {
        let stream = tokio::time::timeout(SAM_CONNECT_TIMEOUT, TcpStream::connect(&self.sam_addr))
            .await
            .map_err(|_| {
                ShieldError::p2p(ErrorCode::P2pI2pError, "SAM bridge connection timed out")
            })?
            .map_err(|e| {
                ShieldError::p2p(
                    ErrorCode::P2pI2pError,
                    format!("SAM bridge connection failed: {}", e),
                )
            })?;

        Ok(stream)
    }

    /// Perform the SAM handshake.
    pub async fn handshake(&self, stream: &mut TcpStream) -> Result<(), ShieldError> {
        // Send HELLO
        let hello = format!(
            "HELLO VERSION MIN={version} MAX={version}\n",
            version = self.version
        );
        stream.write_all(hello.as_bytes()).await.map_err(|e| {
            ShieldError::p2p(
                ErrorCode::P2pI2pError,
                format!("SAM HELLO write failed: {}", e),
            )
        })?;

        // Read response
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response).await.map_err(|e| {
            ShieldError::p2p(
                ErrorCode::P2pI2pError,
                format!("SAM HELLO read failed: {}", e),
            )
        })?;

        if !response.contains("HELLO REPLY RESULT=OK") {
            return Err(ShieldError::p2p(
                ErrorCode::P2pI2pError,
                format!("SAM handshake failed: {}", response.trim()),
            ));
        }

        self.connected
            .store(true, std::sync::atomic::Ordering::Relaxed);
        debug!("SAM bridge handshake successful");

        Ok(())
    }

    /// Generate a new I2P destination via the SAM bridge.
    pub async fn generate_destination(
        &self,
        stream: &mut TcpStream,
    ) -> Result<I2pDestination, ShieldError> {
        // Send DEST GENERATE
        let cmd = "DEST GENERATE\n";
        stream.write_all(cmd.as_bytes()).await.map_err(|e| {
            ShieldError::p2p(
                ErrorCode::P2pI2pError,
                format!("SAM DEST GENERATE write failed: {}", e),
            )
        })?;

        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response).await.map_err(|e| {
            ShieldError::p2p(
                ErrorCode::P2pI2pError,
                format!("SAM DEST GENERATE read failed: {}", e),
            )
        })?;

        // Parse: DEST REPLY PUB=... PRIV=...
        if let Some(pub_start) = response.find("PUB=") {
            let rest = &response[pub_start + 4..];
            if let Some(end) = rest.find(' ') {
                let pubkey_b64 = &rest[..end];
                return Ok(I2pDestination::from_base64(pubkey_b64.to_string(), true));
            } else if let Some(end) = rest.find('\n') {
                let pubkey_b64 = &rest[..end];
                return Ok(I2pDestination::from_base64(pubkey_b64.to_string(), true));
            }
        }

        Err(ShieldError::p2p(
            ErrorCode::P2pI2pError,
            format!(
                "Failed to parse SAM DEST GENERATE response: {}",
                response.trim()
            ),
        ))
    }

    /// Create a SAM session for streaming.
    pub async fn create_session(
        &self,
        stream: &mut TcpStream,
        session_id: &str,
        destination: &I2pDestination,
        inbound_hops: u8,
        outbound_hops: u8,
    ) -> Result<(), ShieldError> {
        let cmd = format!(
            "SESSION CREATE STYLE=STREAM ID={id} DESTINATION={dest} \
             inbound.length={in_hops} outbound.length={out_hops} i2cp.leaseSetEncType=4\n",
            id = session_id,
            dest = destination.base64,
            in_hops = inbound_hops,
            out_hops = outbound_hops,
        );

        stream.write_all(cmd.as_bytes()).await.map_err(|e| {
            ShieldError::p2p(
                ErrorCode::P2pI2pError,
                format!("SAM SESSION CREATE write failed: {}", e),
            )
        })?;

        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response).await.map_err(|e| {
            ShieldError::p2p(
                ErrorCode::P2pI2pError,
                format!("SAM SESSION CREATE read failed: {}", e),
            )
        })?;

        if !response.contains("SESSION STATUS RESULT=OK") {
            return Err(ShieldError::p2p(
                ErrorCode::P2pI2pError,
                format!("SAM session creation failed: {}", response.trim()),
            ));
        }

        debug!(session_id, "SAM session created");
        Ok(())
    }

    /// Connect to a remote I2P destination.
    pub async fn connect_to_destination(
        &self,
        stream: &mut TcpStream,
        session_id: &str,
        remote_destination: &str,
    ) -> Result<(), ShieldError> {
        let cmd = format!(
            "STREAM CONNECT ID={id} DESTINATION={dest} SILENT=false\n",
            id = session_id,
            dest = remote_destination,
        );

        stream.write_all(cmd.as_bytes()).await.map_err(|e| {
            ShieldError::p2p(
                ErrorCode::P2pI2pError,
                format!("SAM STREAM CONNECT write failed: {}", e),
            )
        })?;

        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response).await.map_err(|e| {
            ShieldError::p2p(
                ErrorCode::P2pI2pError,
                format!("SAM STREAM CONNECT read failed: {}", e),
            )
        })?;

        if !response.contains("STREAM STATUS RESULT=OK") {
            return Err(ShieldError::p2p(
                ErrorCode::P2pI2pError,
                format!("SAM stream connect failed: {}", response.trim()),
            ));
        }

        debug!(
            session_id,
            remote = remote_destination,
            "SAM stream connected"
        );
        Ok(())
    }

    /// Check if we're connected to the SAM bridge.
    pub fn is_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::Relaxed)
    }
}

// ── I2P Overlay ─────────────────────────────────────────────────────────────

/// I2P overlay network manager.
///
/// Manages I2P tunnels and provides an interface for routing traffic
/// through the I2P network via the SAM bridge.
pub struct I2pOverlay {
    /// Configuration.
    config: I2pConfig,
    /// SAM bridge client.
    sam_client: SamBridgeClient,
    /// Active tunnels.
    tunnels: Arc<RwLock<HashMap<u32, I2pTunnel>>>,
    /// Whether the overlay is running.
    running: Arc<std::sync::atomic::AtomicBool>,
    /// Local I2P destinations.
    destinations: Arc<RwLock<Vec<I2pDestination>>>,
    /// Next tunnel ID.
    next_tunnel_id: Mutex<u32>,
    /// SAM bridge TCP stream (persistent connection).
    sam_stream: Arc<RwLock<Option<TcpStream>>>,
}

impl I2pOverlay {
    /// Create a new I2P overlay.
    pub fn new() -> Result<Self, ShieldError> {
        let config = I2pConfig::default();
        let sam_client = SamBridgeClient::new(config.sam_addr.clone(), config.sam_version.clone());

        Ok(Self {
            config,
            sam_client,
            tunnels: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            destinations: Arc::new(RwLock::new(Vec::new())),
            next_tunnel_id: Mutex::new(1),
            sam_stream: Arc::new(RwLock::new(None)),
        })
    }

    /// Start the I2P overlay.
    pub async fn start(&self) -> Result<(), ShieldError> {
        if self.running.load(std::sync::atomic::Ordering::Relaxed) {
            debug!("I2P overlay already running");
            return Ok(());
        }

        info!(
            sam_addr = %self.config.sam_addr,
            inbound_hops = self.config.inbound_hops,
            outbound_hops = self.config.outbound_hops,
            "Starting I2P overlay"
        );

        // Connect to SAM bridge
        match self.sam_client.connect().await {
            Ok(mut stream) => {
                // Perform handshake
                if let Err(e) = self.sam_client.handshake(&mut stream).await {
                    warn!(error = %e, "SAM bridge handshake failed — I2P overlay unavailable");
                    return Err(e);
                }

                // Generate a local destination
                match self.sam_client.generate_destination(&mut stream).await {
                    Ok(dest) => {
                        info!(
                            i2p_address = %dest.i2p_address(),
                            "Generated I2P destination"
                        );
                        self.destinations.write().await.push(dest);
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to generate I2P destination");
                    }
                }

                *self.sam_stream.write().await = Some(stream);
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Failed to connect to SAM bridge — I2P overlay unavailable. \
                     Ensure i2pd or Java I2P is running."
                );
                return Err(e);
            }
        }

        self.running
            .store(true, std::sync::atomic::Ordering::Relaxed);
        info!("I2P overlay started");
        Ok(())
    }

    /// Stop the I2P overlay.
    pub async fn stop(&self) -> Result<(), ShieldError> {
        if !self.running.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }

        info!("Stopping I2P overlay");

        // Close all tunnels
        let mut tunnels = self.tunnels.write().await;
        for (_, tunnel) in tunnels.iter_mut() {
            tunnel.active = false;
        }
        tunnels.clear();

        // Close SAM bridge connection
        *self.sam_stream.write().await = None;

        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        info!("I2P overlay stopped");
        Ok(())
    }

    /// Check if the overlay is running.
    pub async fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get the number of active tunnels.
    pub async fn tunnel_count(&self) -> u32 {
        self.tunnels
            .read()
            .await
            .values()
            .filter(|t| t.active)
            .count() as u32
    }

    /// Create a new I2P tunnel to a remote destination.
    pub async fn create_tunnel(
        &self,
        remote_destination: &str,
        local_port: u16,
    ) -> Result<u32, ShieldError> {
        if !self.running.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(ShieldError::p2p(
                ErrorCode::P2pI2pError,
                "I2P overlay is not running",
            ));
        }

        if self.tunnels.read().await.len() >= self.config.max_tunnels {
            return Err(ShieldError::p2p(
                ErrorCode::P2pI2pError,
                "Maximum tunnel count reached",
            ));
        }

        let tunnel_id = {
            let mut id = self.next_tunnel_id.lock();
            let current = *id;
            *id += 1;
            current
        };

        let session_id = format!("shield-{}", tunnel_id);

        // Create a session and connect
        let mut stream_guard = self.sam_stream.write().await;
        if let Some(ref mut stream) = *stream_guard {
            let destinations = self.destinations.read().await;
            if let Some(local_dest) = destinations.first() {
                // Create session
                self.sam_client
                    .create_session(
                        stream,
                        &session_id,
                        local_dest,
                        self.config.inbound_hops,
                        self.config.outbound_hops,
                    )
                    .await?;

                // Connect to remote
                self.sam_client
                    .connect_to_destination(stream, &session_id, remote_destination)
                    .await?;
            } else {
                return Err(ShieldError::p2p(
                    ErrorCode::P2pI2pError,
                    "No local I2P destination available",
                ));
            }
        } else {
            return Err(ShieldError::p2p(
                ErrorCode::P2pI2pError,
                "SAM bridge not connected",
            ));
        }

        // Record the tunnel
        let tunnel = I2pTunnel {
            id: tunnel_id,
            local_destination: self
                .destinations
                .read()
                .await
                .first()
                .cloned()
                .unwrap_or_else(|| I2pDestination::from_base64(String::new(), true)),
            remote_destination: Some(I2pDestination::from_base64(
                remote_destination.to_string(),
                false,
            )),
            inbound_hops: self.config.inbound_hops,
            outbound_hops: self.config.outbound_hops,
            local_port,
            active: true,
            created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
            bytes_sent: 0,
            bytes_recv: 0,
        };

        self.tunnels.write().await.insert(tunnel_id, tunnel);

        info!(
            tunnel_id,
            remote = remote_destination,
            local_port,
            "I2P tunnel created"
        );

        Ok(tunnel_id)
    }

    /// Destroy an I2P tunnel.
    pub async fn destroy_tunnel(&self, tunnel_id: u32) -> Result<(), ShieldError> {
        let mut tunnels = self.tunnels.write().await;
        if let Some(mut tunnel) = tunnels.remove(&tunnel_id) {
            tunnel.active = false;
            info!(tunnel_id, "I2P tunnel destroyed");
            Ok(())
        } else {
            Err(ShieldError::p2p(
                ErrorCode::P2pI2pError,
                format!("Tunnel {} not found", tunnel_id),
            ))
        }
    }

    /// Get all active tunnels.
    pub async fn get_tunnels(&self) -> Vec<I2pTunnel> {
        self.tunnels
            .read()
            .await
            .values()
            .filter(|t| t.active)
            .cloned()
            .collect()
    }

    /// Get local I2P destinations.
    pub async fn get_destinations(&self) -> Vec<I2pDestination> {
        self.destinations.read().await.clone()
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Simple base32 encoding (lowercase, RFC 4648).
fn base32_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";
    let mut result = String::new();
    let mut bits = 0u32;
    let mut n_bits = 0;

    for &byte in data {
        bits = (bits << 8) | byte as u32;
        n_bits += 8;
        while n_bits >= 5 {
            n_bits -= 5;
            let idx = ((bits >> n_bits) & 0x1F) as usize;
            result.push(ALPHABET[idx] as char);
        }
    }

    if n_bits > 0 {
        let idx = ((bits << (5 - n_bits)) & 0x1F) as usize;
        result.push(ALPHABET[idx] as char);
    }

    result
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base32_encoding() {
        // Test vectors for base32
        assert_eq!(base32_encode(b"hello"), "nbswy3dp");
        assert_eq!(base32_encode(b""), "");
    }

    #[test]
    fn test_i2p_destination() {
        let dest = I2pDestination::from_base64("test_destination_base64".to_string(), true);
        assert!(dest.is_local);
        assert!(!dest.base64.is_empty());
        let addr = dest.i2p_address();
        assert!(addr.ends_with(".b32.i2p"));
    }

    #[test]
    fn test_i2p_config_default() {
        let config = I2pConfig::default();
        assert_eq!(config.sam_addr, "127.0.0.1:7656");
        assert_eq!(config.inbound_hops, 3);
        assert_eq!(config.outbound_hops, 3);
    }

    #[tokio::test]
    async fn test_i2p_overlay_creation() {
        let overlay = I2pOverlay::new().unwrap();
        assert!(!overlay.is_running().await);
        assert_eq!(overlay.tunnel_count().await, 0);
    }
}
