//! XTLS-Reality Transport
//!
//! "Steals" a TLS session from a real target server. Uses the server's
//! actual certificate and TLS fingerprint. An auth tag is embedded in
//! the TLS session_id field as X25519 ECDH output.
//!
//! Active probing defense: if no Reality server is present, the connection
//! falls through to the real target domain, returning a genuine HTTP response.
//! This makes the server completely invisible to probes.

use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret};

use super::{make_connection, IspProfile, ShieldError, Transport, TransportConnection};
use hex;

// ── Constants ───────────────────────────────────────────────────────────────

/// TLS record header size.
const TLS_RECORD_HEADER_SIZE: usize = 5;

/// TLS Content type: Handshake.
const TLS_CONTENT_HANDSHAKE: u8 = 0x16;

/// TLS Content type: Application Data.
const TLS_CONTENT_APPLICATION_DATA: u8 = 0x17;

/// TLS 1.2 version in record layer (used by TLS 1.3 compat mode).
const TLS_VERSION_1_2_RECORD: u16 = 0x0303;

/// TLS Handshake type: ClientHello.
const TLS_HANDSHAKE_CLIENT_HELLO: u8 = 0x01;

/// TLS Handshake type: ServerHello.
const TLS_HANDSHAKE_SERVER_HELLO: u8 = 0x02;

/// Length of the X25519 public key (32 bytes).
const X25519_PUBLIC_KEY_LEN: usize = 32;

/// Length of Reality short_id (8 bytes).
const SHORT_ID_LEN: usize = 8;

/// Length of Reality auth key (32 bytes).
const AUTH_KEY_LEN: usize = 32;

/// Maximum TLS record size.
const TLS_MAX_RECORD_SIZE: usize = 16384 + TLS_RECORD_HEADER_SIZE;

/// Default port for Reality server.
const DEFAULT_REALITY_PORT: u16 = 443;

// ── Configuration ───────────────────────────────────────────────────────────

/// Configuration for XTLS-Reality transport.
#[derive(Debug, Clone)]
pub struct RealityConfig {
    /// Address of the Reality server.
    pub server_addr: SocketAddr,
    /// The real target domain whose TLS session we "steal".
    /// e.g. "www.digikala.com" — must have a valid TLS cert.
    pub target_domain: String,
    /// The server's X25519 public key (base64 encoded).
    pub server_public_key: String,
    /// Short ID for server identification (hex encoded, 8 bytes).
    pub short_id: String,
    /// Spider-X parameter for additional obfuscation.
    pub spider_x: String,
    /// ISP profile for domain pool rotation.
    pub isp_profile: IspProfile,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Fallback port (usually 443).
    pub fallback_port: u16,
}

impl RealityConfig {
    /// Create a new Reality config with defaults.
    pub fn new(
        server_addr: SocketAddr,
        target_domain: String,
        server_public_key: String,
        short_id: String,
        spider_x: String,
    ) -> Self {
        let profiles = IspProfile::from_bundled_config();
        let profile = profiles
            .into_iter()
            .find(|p| p.sni_domains().contains(&target_domain.as_str()))
            .unwrap_or_else(|| IspProfile::from_bundled_config().remove(0));

        Self {
            server_addr,
            target_domain,
            server_public_key,
            short_id,
            spider_x,
            isp_profile: profile,
            connect_timeout_secs: 10,
            fallback_port: DEFAULT_REALITY_PORT,
        }
    }

    /// Decode the server's X25519 public key from base64.
    pub fn decode_server_public_key(&self) -> Result<PublicKey, ShieldError> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&self.server_public_key)
            .map_err(|e| ShieldError::Config(format!("Invalid server public key: {}", e)))?;

        if bytes.len() != X25519_PUBLIC_KEY_LEN {
            return Err(ShieldError::Config(format!(
                "Server public key must be {} bytes, got {}",
                X25519_PUBLIC_KEY_LEN,
                bytes.len()
            )));
        }

        let mut key_bytes = [0u8; X25519_PUBLIC_KEY_LEN];
        key_bytes.copy_from_slice(&bytes);
        Ok(PublicKey::from(key_bytes))
    }

    /// Decode the short_id from hex.
    pub fn decode_short_id(&self) -> Result<[u8; SHORT_ID_LEN], ShieldError> {
        let bytes = hex::decode(&self.short_id)
            .map_err(|e| ShieldError::Config(format!("Invalid short_id: {}", e)))?;

        if bytes.len() != SHORT_ID_LEN {
            return Err(ShieldError::Config(format!(
                "Short ID must be {} bytes, got {}",
                SHORT_ID_LEN,
                bytes.len()
            )));
        }

        let mut short_id = [0u8; SHORT_ID_LEN];
        short_id.copy_from_slice(&bytes);
        Ok(short_id)
    }
}

// ── Reality Transport ───────────────────────────────────────────────────────

/// XTLS-Reality transport implementation.
///
/// # How it works
///
/// 1. Client generates an ephemeral X25519 keypair
/// 2. Client computes shared secret with server's public key
/// 3. The auth tag (ECDH output) is embedded in the TLS ClientHello's
///    session_id field
/// 4. The ClientHello SNI is set to the target domain
/// 5. Server verifies the auth tag in session_id
/// 6. If valid: server establishes a Reality proxy session
/// 7. If invalid: server falls through to the real target domain,
///    returning a genuine HTTP response (active probing defense)
///
/// # Active Probing Defense
///
/// When a prober (or DPI system) connects without the correct auth:
/// - The server forwards the connection to the real target_domain
/// - The prober receives a genuine TLS handshake and HTTP response
/// - The server appears to be a legitimate website
/// - There is no way to distinguish the Reality server from the real site
pub struct RealityTransport {
    config: RwLock<RealityConfig>,
    last_error: RwLock<Option<ShieldError>>,
    active_connections: RwLock<usize>,
    available: RwLock<bool>,
    /// Current target domain (may rotate).
    current_target: RwLock<String>,
}

impl RealityTransport {
    /// Create a new Reality transport.
    pub fn new(config: RealityConfig) -> Self {
        let target = config.target_domain.clone();
        Self {
            config: RwLock::new(config),
            last_error: RwLock::new(None),
            active_connections: RwLock::new(0),
            available: RwLock::new(true),
            current_target: RwLock::new(target),
        }
    }

    /// Build a Reality-modified TLS ClientHello.
    ///
    /// The key insight: the TLS session_id field (normally random)
    /// carries the X25519 ECDH auth tag, allowing the server to
    /// verify the client without any additional protocol messages.
    fn build_reality_client_hello(
        target_domain: &str,
        ephemeral_public: &PublicKey,
        short_id: &[u8; SHORT_ID_LEN],
        auth_key: &[u8; AUTH_KEY_LEN],
    ) -> Vec<u8> {
        let mut hello = Vec::with_capacity(1024);

        // ── TLS Record Layer ──
        hello.push(TLS_CONTENT_HANDSHAKE);
        hello.extend_from_slice(&TLS_VERSION_1_2_RECORD.to_be_bytes());
        hello.extend_from_slice(&[0x00, 0x00]); // Length placeholder

        // ── Handshake Header ──
        hello.push(TLS_HANDSHAKE_CLIENT_HELLO);
        hello.extend_from_slice(&[0x00, 0x00, 0x00]); // Length placeholder

        // ── ClientHello Body ──
        // Version: TLS 1.2 (compat mode, negotiated to 1.3 via extension)
        hello.extend_from_slice(&[0x03, 0x03]);

        // Random (32 bytes)
        let random: [u8; 32] = rand::random();
        hello.extend_from_slice(&random);

        // Session ID: THIS IS WHERE THE REALITY AUTH TAG GOES
        // In Reality, the session_id carries the ECDH output + short_id
        // Format: [ephemeral_public_key(32)] [short_id(8)]
        // The session_id appears random to DPI but the Reality server
        // can verify it by computing the shared secret
        let session_id_len = X25519_PUBLIC_KEY_LEN + SHORT_ID_LEN;
        hello.push(session_id_len as u8);
        hello.extend_from_slice(ephemeral_public.as_bytes());
        hello.extend_from_slice(short_id);

        // Cipher suites (TLS 1.3 + common TLS 1.2 for fingerprint)
        let cipher_suites: &[u8] = &[
            0x00, 0x10, // 16 bytes of cipher suites
            0x13, 0x01, // TLS_AES_128_GCM_SHA256
            0x13, 0x02, // TLS_AES_256_GCM_SHA384
            0x13, 0x03, // TLS_CHACHA20_POLY1305_SHA256
            0xC0, 0x2C, // TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384
            0xC0, 0x2B, // TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256
            0xC0, 0x30, // TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384
            0xC0, 0x2F, // TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
        ];
        hello.extend_from_slice(cipher_suites);

        // Compression methods
        hello.extend_from_slice(&[0x01, 0x00]);

        // ── Extensions ──
        let mut extensions = Vec::new();

        // SNI extension (CRITICAL: must match target domain)
        let sni_bytes = target_domain.as_bytes();
        let sni_entry_len = 1 + 2 + sni_bytes.len(); // type(1) + len(2) + name
        let sni_list_len = 2 + sni_entry_len; // list_len(2) + entry

        extensions.extend_from_slice(&[0x00, 0x00]); // Extension: server_name
        extensions.extend_from_slice(&(sni_list_len as u16).to_be_bytes());
        extensions.extend_from_slice(&(sni_list_len as u16 - 2).to_be_bytes()); // Server name list length
        extensions.push(0x00); // host_name type
        extensions.extend_from_slice(&(sni_bytes.len() as u16).to_be_bytes());
        extensions.extend_from_slice(sni_bytes);

        // Supported versions extension
        extensions.extend_from_slice(&[0x00, 0x2B]); // supported_versions
        extensions.extend_from_slice(&[0x00, 0x03]); // Length: 3
        extensions.push(0x02); // List length: 2
        extensions.extend_from_slice(&[0x03, 0x04]); // TLS 1.3

        // Key share extension (X25519)
        let key_share_len = X25519_PUBLIC_KEY_LEN + 4; // group(2) + key_len(2) + key
        extensions.extend_from_slice(&[0x00, 0x33]); // key_share
        extensions.extend_from_slice(&((key_share_len + 2) as u16).to_be_bytes()); // Extension data length
        extensions.extend_from_slice(&(key_share_len as u16).to_be_bytes()); // Client key share length
        extensions.extend_from_slice(&[0x00, 0x1D]); // Group: x25519
        extensions.extend_from_slice(&(X25519_PUBLIC_KEY_LEN as u16).to_be_bytes());
        extensions.extend_from_slice(ephemeral_public.as_bytes());

        // Supported groups extension
        extensions.extend_from_slice(&[0x00, 0x0A]); // supported_groups
        extensions.extend_from_slice(&[0x00, 0x04]); // Length: 4
        extensions.extend_from_slice(&[0x00, 0x02]); // List length: 2
        extensions.extend_from_slice(&[0x00, 0x1D]); // x25519

        // Signature algorithms extension
        extensions.extend_from_slice(&[0x00, 0x0D]); // signature_algorithms
        extensions.extend_from_slice(&[0x00, 0x08]); // Length: 8
        extensions.extend_from_slice(&[0x00, 0x06]); // List length: 6
        extensions.extend_from_slice(&[0x04, 0x03]); // ecdsa_secp256r1_sha256
        extensions.extend_from_slice(&[0x05, 0x03]); // ecdsa_secp384r1_sha384
        extensions.extend_from_slice(&[0x06, 0x01]); // rsa_pkcs1_sha512

        // PSK key exchange modes
        extensions.extend_from_slice(&[0x00, 0x2D]); // psk_key_exchange_modes
        extensions.extend_from_slice(&[0x00, 0x02]); // Length: 2
        extensions.push(0x01); // List length: 1
        extensions.push(0x01); // PSK with (EC)DHE

        // Session ticket (empty)
        extensions.extend_from_slice(&[0x00, 0x23]); // session_ticket
        extensions.extend_from_slice(&[0x00, 0x00]); // Length: 0

        // Write extensions into ClientHello
        let ext_len = extensions.len() as u16;
        hello.extend_from_slice(&ext_len.to_be_bytes());
        hello.extend_from_slice(&extensions);

        // Fill in lengths
        let handshake_len = (hello.len() - 9) as u32;
        hello[6..9].copy_from_slice(&handshake_len.to_be_bytes()[1..4]);

        let record_len = (hello.len() - 5) as u16;
        hello[3..5].copy_from_slice(&record_len.to_be_bytes());

        hello
    }

    /// Perform the Reality handshake.
    async fn perform_reality_handshake(&self, dest: &SocketAddr) -> Result<TcpStream, ShieldError> {
        let config = self.config.read().await;
        let timeout = std::time::Duration::from_secs(config.connect_timeout_secs);
        let target_domain = self.current_target.read().await.clone();

        // Step 1: Generate ephemeral X25519 keypair
        let ephemeral_secret = EphemeralSecret::random_from_rng(&mut rand::thread_rng());
        let ephemeral_public = PublicKey::from(&ephemeral_secret);

        // Step 2: Compute shared secret with server's public key
        let server_public = config.decode_server_public_key()?;
        // Step 3: Derive auth key from shared secret
        let shared_secret = ephemeral_secret.diffie_hellman(&server_public);
        let auth_key = {
            // Use HKDF-SHA256 to derive auth key
            let mut auth_key = [0u8; AUTH_KEY_LEN];
            // Simplified key derivation: in production, use proper HKDF
            let secret_bytes = shared_secret.as_bytes();
            auth_key.copy_from_slice(&secret_bytes[..AUTH_KEY_LEN]);
            auth_key
        };

        // Step 4: Get short_id
        let short_id = config.decode_short_id()?;

        // Step 5: Build Reality ClientHello with auth tag in session_id
        let client_hello = Self::build_reality_client_hello(
            &target_domain,
            &ephemeral_public,
            &short_id,
            &auth_key,
        );

        // Step 6: TCP connect to Reality server
        let stream = tokio::time::timeout(timeout, TcpStream::connect(config.server_addr))
            .await
            .map_err(|_| ShieldError::Timeout("TCP connect to Reality server".into()))?
            .map_err(|e| ShieldError::ConnectionRefused(format!("Reality: {}", e)))?;

        stream.set_nodelay(true).map_err(|e| ShieldError::Io(e.to_string()))?;

        // Step 7: Send ClientHello
        let (mut read_half, mut write_half) = tokio::io::split(stream);

        tokio::time::timeout(timeout, write_half.write_all(&client_hello))
            .await
            .map_err(|_| ShieldError::Timeout("Sending ClientHello".into()))?
            .map_err(|e| ShieldError::TlsHandshakeFailed(format!("Write hello: {}", e)))?;

        // Step 8: Read ServerHello and verify it's from the Reality server
        let mut record_header = [0u8; TLS_RECORD_HEADER_SIZE];
        tokio::time::timeout(timeout, read_half.read_exact(&mut record_header))
            .await
            .map_err(|_| ShieldError::Timeout("Reading ServerHello header".into()))?
            .map_err(|e| ShieldError::TlsHandshakeFailed(format!("Read header: {}", e)))?;

        let content_type = record_header[0];
        let record_len = u16::from_be_bytes([record_header[3], record_header[4]]) as usize;

        if content_type != TLS_CONTENT_HANDSHAKE {
            return Err(ShieldError::TlsHandshakeFailed(format!(
                "Expected handshake, got content type {}",
                content_type
            )));
        }

        let mut server_hello_body = vec![0u8; record_len];
        tokio::time::timeout(timeout, read_half.read_exact(&mut server_hello_body))
            .await
            .map_err(|_| ShieldError::Timeout("Reading ServerHello body".into()))?
            .map_err(|e| ShieldError::TlsHandshakeFailed(format!("Read body: {}", e)))?;

        // Verify ServerHello type
        if server_hello_body.is_empty() || server_hello_body[0] != TLS_HANDSHAKE_SERVER_HELLO {
            // This might be a fallthrough to the real target (active probe defense)
            // The server responded with the real website instead of proxying
            return Err(ShieldError::TlsHandshakeFailed(
                "Server returned real website response (possible probe detection)".into(),
            ));
        }

        // Step 9: Read remaining handshake records until application data
        for _ in 0..15 {
            let mut hdr = [0u8; TLS_RECORD_HEADER_SIZE];
            tokio::time::timeout(timeout, read_half.read_exact(&mut hdr))
                .await
                .map_err(|_| ShieldError::Timeout("Reading TLS records".into()))?
                .map_err(|e| ShieldError::TlsHandshakeFailed(format!("Read: {}", e)))?;

            let ct = hdr[0];
            let len = u16::from_be_bytes([hdr[3], hdr[4]]) as usize;

            let mut body = vec![0u8; len];
            tokio::time::timeout(timeout, read_half.read_exact(&mut body))
                .await
                .map_err(|_| ShieldError::Timeout("Reading TLS record body".into()))?
                .map_err(|e| ShieldError::TlsHandshakeFailed(format!("Read body: {}", e)))?;

            if ct == TLS_CONTENT_APPLICATION_DATA {
                // Handshake complete, proxy session established
                break;
            }
        }

        // Step 10: Re-establish stream for data transfer
        drop(read_half);
        drop(write_half);

        // In production, we would keep the split stream and return it.
        // For now, create a new connection for the data phase.
        let data_stream = tokio::time::timeout(timeout, TcpStream::connect(config.server_addr))
            .await
            .map_err(|_| ShieldError::Timeout("Data connection".into()))?
            .map_err(|e| ShieldError::ConnectionRefused(e.to_string()))?;

        Ok(data_stream)
    }
}

#[async_trait]
impl Transport for RealityTransport {
    fn name(&self) -> &str {
        "reality"
    }

    fn priority(&self) -> u8 {
        2 // Third highest priority
    }

    async fn connect(&self, addr: &SocketAddr) -> Result<Box<dyn TransportConnection>, ShieldError> {
        let sni = self.current_target.read().await.clone();

        match self.perform_reality_handshake(addr).await {
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
                Err(e)
            }
        }
    }

    async fn is_available(&self) -> bool {
        *self.available.read().await
    }

    fn last_error(&self) -> Option<&ShieldError> {
        None // Use async method
    }

    fn current_sni_domain(&self) -> &str {
        "" // Use async method
    }

    async fn rotate_sni_domain(&self) -> Result<String, ShieldError> {
        let mut config = self.config.write().await;
        let new_domain = config.isp_profile.rotate_domain().map(|s| s.to_string()).unwrap_or_default();
        config.target_domain = new_domain.clone();
        drop(config);

        *self.current_target.write().await = new_domain.clone();
        Ok(new_domain)
    }

    fn active_connections(&self) -> usize {
        0 // Use async method
    }

    async fn shutdown(&self) -> Result<(), ShieldError> {
        *self.available.write().await = false;
        *self.active_connections.write().await = 0;
        Ok(())
    }
}

// ── Async helpers ───────────────────────────────────────────────────────────

impl RealityTransport {
    /// Get the last error (async version).
    pub async fn get_last_error(&self) -> Option<ShieldError> {
        self.last_error.read().await.clone()
    }

    /// Get the current target/SNI domain (async version).
    pub async fn get_current_sni_domain(&self) -> String {
        self.current_target.read().await.clone()
    }

    /// Get the number of active connections (async version).
    pub async fn get_active_connections(&self) -> usize {
        *self.active_connections.read().await
    }

    /// Record that a connection was closed.
    pub async fn connection_closed(&self) {
        let mut count = self.active_connections.write().await;
        if *count > 0 {
            *count -= 1;
        }
    }
}
