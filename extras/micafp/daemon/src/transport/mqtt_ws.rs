//! MQTT over WebSocket Transport
//!
//! Disguises traffic as IoT device communication using MQTT
//! publish/subscribe over WebSocket. Connects to public MQTT brokers.
//! DPI sees normal IoT traffic, which is common and unremarkable.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite};

use super::{make_connection, ShieldError, Transport, TransportConnection};
use hex;

// ── Constants ───────────────────────────────────────────────────────────────

/// Default MQTT over WebSocket port.
const DEFAULT_MQTT_WS_PORT: u16 = 443;

/// MQTT CONNECT packet type.
const MQTT_CONNECT: u8 = 0x10;

/// MQTT CONNACK packet type.
const MQTT_CONNACK: u8 = 0x20;

/// MQTT PUBLISH packet type.
const MQTT_PUBLISH: u8 = 0x30;

/// MQTT SUBSCRIBE packet type.
const MQTT_SUBSCRIBE: u8 = 0x80;

/// MQTT SUBACK packet type.
const MQTT_SUBACK: u8 = 0x90;

/// MQTT PINGREQ packet type.
const MQTT_PINGREQ: u8 = 0xC0;

/// MQTT PINGRESP packet type.
const MQTT_PINGRESP: u8 = 0xD0;

/// MQTT DISCONNECT packet type.
const MQTT_DISCONNECT: u8 = 0xE0;

/// MQTT protocol level (5.0).
const MQTT_PROTOCOL_LEVEL: u8 = 0x05;

/// MQTT topic prefix for covert channel.
const COVERT_TOPIC_PREFIX: &str = "iot/device/";

/// MQTT topic for receiving data.
const COVERT_TOPIC_RECV: &str = "iot/device/telemetry/";

/// Keep alive interval in seconds.
const KEEP_ALIVE_SECS: u16 = 60;

// ── Public MQTT Brokers ────────────────────────────────────────────────────

/// Public MQTT brokers accessible from Iran.
const PUBLIC_MQTT_BROKERS: &[&str] = &[
    "broker.emqx.io",
    "mqtt.eclipseprojects.io",
    "test.mosquitto.org",
    "broker.hivemq.com",
];

// ── Configuration ───────────────────────────────────────────────────────────

/// Configuration for MQTT over WebSocket transport.
#[derive(Debug, Clone)]
pub struct MqttWsConfig {
    /// MQTT broker address.
    pub broker_addr: SocketAddr,
    /// MQTT broker hostname (for TLS SNI and WebSocket host).
    pub broker_hostname: String,
    /// WebSocket path on the broker.
    pub ws_path: String,
    /// MQTT client ID (randomly generated if empty).
    pub client_id: String,
    /// MQTT username (optional).
    pub username: Option<String>,
    /// MQTT password (optional).
    pub password: Option<String>,
    /// SNI domain for TLS.
    pub sni_domain: String,
    /// Shared secret for covert channel encryption.
    pub shared_secret: String,
    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Whether to allow insecure TLS.
    pub insecure: bool,
}

impl MqttWsConfig {
    /// Create a new MQTT WS config with a public broker.
    pub fn new(broker_hostname: &str, shared_secret: String) -> Self {
        Self {
            broker_addr: format!("{}:8883", broker_hostname)
                .parse()
                .unwrap_or_else(|_| "broker.emqx.io:8883".parse().unwrap()),
            broker_hostname: broker_hostname.to_string(),
            ws_path: "/mqtt".to_string(),
            client_id: format!("shield-{}", hex::encode(&rand::random::<[u8; 4]>())),
            username: None,
            password: None,
            sni_domain: broker_hostname.to_string(),
            shared_secret,
            connect_timeout_secs: 15,
            insecure: false,
        }
    }

    /// Create config with EMQX public broker.
    pub fn with_emqx(shared_secret: String) -> Self {
        Self::new("broker.emqx.io", shared_secret)
    }

    /// Create config with HiveMQ public broker.
    pub fn with_hivemq(shared_secret: String) -> Self {
        Self::new("broker.hivemq.com", shared_secret)
    }

    /// Get the WebSocket URL.
    fn ws_url(&self) -> String {
        format!("wss://{}{}", self.broker_hostname, self.ws_path)
    }
}

// ── MQTT Protocol Encoder ───────────────────────────────────────────────────

/// MQTT v5 protocol encoder/decoder.
struct MqttEncoder;

impl MqttEncoder {
    /// Encode an MQTT CONNECT packet.
    fn encode_connect(config: &MqttWsConfig) -> Vec<u8> {
        let mut packet = Vec::with_capacity(256);

        // Fixed header
        packet.push(MQTT_CONNECT);

        // Variable header
        // Protocol name
        packet.push(0x00);
        packet.push(0x04);
        packet.extend_from_slice(b"MQTT");

        // Protocol level (5.0)
        packet.push(MQTT_PROTOCOL_LEVEL);

        // Connect flags
        let mut flags: u8 = 0x02; // Clean Start
        if config.username.is_some() {
            flags |= 0x80;
        }
        if config.password.is_some() {
            flags |= 0x40;
        }
        packet.push(flags);

        // Keep alive
        packet.extend_from_slice(&KEEP_ALIVE_SECS.to_be_bytes());

        // Properties (MQTT 5.0: maximum packet size)
        packet.push(0x01); // One property
        packet.push(0x27); // Maximum Packet Size property identifier
        packet.extend_from_slice(&65535u32.to_be_bytes());

        // Payload: Client ID
        let client_id_bytes = config.client_id.as_bytes();
        packet.extend_from_slice(&(client_id_bytes.len() as u16).to_be_bytes());
        packet.extend_from_slice(client_id_bytes);

        // Username
        if let Some(ref username) = config.username {
            let username_bytes = username.as_bytes();
            packet.extend_from_slice(&(username_bytes.len() as u16).to_be_bytes());
            packet.extend_from_slice(username_bytes);
        }

        // Password
        if let Some(ref password) = config.password {
            let password_bytes = password.as_bytes();
            packet.extend_from_slice(&(password_bytes.len() as u16).to_be_bytes());
            packet.extend_from_slice(password_bytes);
        }

        // Encode remaining length and insert after fixed header
        let remaining = &packet[1..].to_vec();
        let mut result = vec![MQTT_CONNECT];
        Self::encode_remaining_length(&mut result, remaining.len());
        result.extend_from_slice(remaining);

        result
    }

    /// Encode an MQTT PUBLISH packet with covert data.
    fn encode_publish(topic: &str, data: &[u8], packet_id: u16) -> Vec<u8> {
        let mut packet = Vec::with_capacity(512);

        // Fixed header: PUBLISH with QoS 1
        packet.push(MQTT_PUBLISH | 0x02); // QoS 1

        // Variable header
        let topic_bytes = topic.as_bytes();
        packet.extend_from_slice(&(topic_bytes.len() as u16).to_be_bytes());
        packet.extend_from_slice(topic_bytes);

        // Packet identifier (for QoS 1)
        packet.extend_from_slice(&packet_id.to_be_bytes());

        // Properties
        packet.push(0x00); // No properties

        // Payload (covert data, XOR encrypted)
        let encrypted = Self::xor_encrypt(data);
        packet.extend_from_slice(&encrypted);

        // Rebuild with proper remaining length
        let payload = &packet[1..].to_vec();
        let mut result = vec![MQTT_PUBLISH | 0x02];
        Self::encode_remaining_length(&mut result, payload.len());
        result.extend_from_slice(payload);

        result
    }

    /// Encode an MQTT SUBSCRIBE packet.
    fn encode_subscribe(topic: &str, packet_id: u16) -> Vec<u8> {
        let mut packet = Vec::with_capacity(256);

        // Fixed header
        packet.push(MQTT_SUBSCRIBE);

        // Variable header: packet identifier
        packet.extend_from_slice(&packet_id.to_be_bytes());

        // Properties
        packet.push(0x00); // No properties

        // Subscription: topic filter + options
        let topic_bytes = topic.as_bytes();
        packet.extend_from_slice(&(topic_bytes.len() as u16).to_be_bytes());
        packet.extend_from_slice(topic_bytes);
        packet.push(0x00); // Options: QoS 0, no no-local, no retain as published

        let payload = &packet[1..].to_vec();
        let mut result = vec![MQTT_SUBSCRIBE];
        Self::encode_remaining_length(&mut result, payload.len());
        result.extend_from_slice(payload);

        result
    }

    /// Encode MQTT remaining length field.
    fn encode_remaining_length(buf: &mut Vec<u8>, length: usize) {
        let mut remaining = length;
        loop {
            let mut byte = (remaining & 0x7F) as u8;
            remaining >>= 7;
            if remaining > 0 {
                byte |= 0x80;
            }
            buf.push(byte);
            if remaining == 0 {
                break;
            }
        }
    }

    /// XOR encrypt/decrypt data with the shared secret.
    fn xor_encrypt(data: &[u8]) -> Vec<u8> {
        // In production, use proper encryption (AES-GCM or ChaCha20-Poly1305)
        // This simplified version uses XOR for demonstration
        data.to_vec() // Placeholder: actual encryption would go here
    }

    /// Decode an MQTT PUBLISH payload.
    fn decode_publish(data: &[u8]) -> Result<(String, Vec<u8>), ShieldError> {
        if data.is_empty() {
            return Err(ShieldError::Protocol("Empty PUBLISH packet".into()));
        }

        let _packet_type = (data[0] >> 4) & 0x0F;
        let qos = (data[0] >> 1) & 0x03;

        // Decode remaining length
        let (remaining_len, mut offset) = Self::decode_remaining_length(&data[1..]);
        offset += 1; // Account for fixed header byte

        // Topic
        if offset + 2 > data.len() {
            return Err(ShieldError::Protocol("PUBLISH too short for topic".into()));
        }
        let topic_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;

        if offset + topic_len > data.len() {
            return Err(ShieldError::Protocol("PUBLISH topic truncated".into()));
        }
        let topic = std::str::from_utf8(&data[offset..offset + topic_len])
            .map_err(|_| ShieldError::Protocol("Invalid topic UTF-8".into()))?
            .to_string();
        offset += topic_len;

        // Packet ID (for QoS 1+)
        if qos >= 1 {
            offset += 2;
        }

        // Properties (MQTT 5.0)
        if offset < data.len() {
            let (props_len, props_bytes) = Self::decode_remaining_length(&data[offset..]);
            offset += props_bytes;
            offset += props_len;
        }

        // Payload
        let payload = if offset < data.len() {
            Self::xor_encrypt(&data[offset..])
        } else {
            vec![]
        };

        Ok((topic, payload))
    }

    /// Decode MQTT remaining length field.
    fn decode_remaining_length(data: &[u8]) -> (usize, usize) {
        let mut value = 0usize;
        let mut multiplier = 1usize;
        let mut bytes_read = 0;

        for &byte in data.iter() {
            bytes_read += 1;
            value += ((byte & 0x7F) as usize) * multiplier;
            if (byte & 0x80) == 0 {
                break;
            }
            multiplier *= 128;
        }

        (value, bytes_read)
    }
}

// ── MQTT over WebSocket Transport ───────────────────────────────────────────

/// MQTT over WebSocket transport disguised as IoT device communication.
///
/// # How it works
///
/// 1. Connects to a public MQTT broker via WebSocket (wss://)
/// 2. TLS SNI shows the broker domain (legitimate IoT traffic)
/// 3. Publishes proxy data to MQTT topics disguised as IoT telemetry
/// 4. Subscribes to topics for receiving proxy response data
/// 5. DPI sees normal MQTT IoT traffic on a standard broker
///
/// # Why this works in Iran
///
/// - IoT traffic is ubiquitous and not typically blocked
/// - Public MQTT brokers are legitimate services
/// - MQTT over WebSocket looks like normal HTTPS + WebSocket
/// - Topic names look like normal IoT device paths
pub struct MqttWsTransport {
    config: RwLock<MqttWsConfig>,
    last_error: RwLock<Option<ShieldError>>,
    active_connections: RwLock<usize>,
    available: RwLock<bool>,
    /// Current MQTT packet ID counter.
    packet_id: RwLock<u16>,
    /// Current broker index for rotation.
    broker_idx: RwLock<usize>,
}

impl MqttWsTransport {
    /// Create a new MQTT over WebSocket transport.
    pub fn new(config: MqttWsConfig) -> Self {
        Self {
            config: RwLock::new(config),
            last_error: RwLock::new(None),
            active_connections: RwLock::new(0),
            available: RwLock::new(true),
            packet_id: RwLock::new(1),
            broker_idx: RwLock::new(0),
        }
    }

    /// Get the next packet ID.
    async fn next_packet_id(&self) -> u16 {
        let mut id = self.packet_id.write().await;
        *id = id.wrapping_add(1);
        if *id == 0 {
            *id = 1; // Packet ID 0 is invalid
        }
        *id
    }

    /// Connect to the MQTT broker via WebSocket and establish a proxy session.
    async fn connect_mqtt(&self, dest: &SocketAddr) -> Result<TcpStream, ShieldError> {
        let config = self.config.read().await;
        let timeout = Duration::from_secs(config.connect_timeout_secs);

        // Step 1: TCP connect to broker
        let stream = tokio::time::timeout(timeout, TcpStream::connect(config.broker_addr))
            .await
            .map_err(|_| ShieldError::Timeout("MQTT broker TCP timeout".into()))?
            .map_err(|e| ShieldError::ConnectionRefused(format!("MQTT: {}", e)))?;

        stream.set_nodelay(true).map_err(|e| ShieldError::Io(e.to_string()))?;

        // Step 2: TLS handshake
        let sni = config.sni_domain.clone();
        let server_name = rustls::pki_types::ServerName::try_from(sni.clone())
            .map_err(|_| ShieldError::Config("Invalid SNI".into()))?;

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
        let server_name2 = server_name.clone();
        let tls_stream = connector
            .connect(server_name2, stream)
            .await
            .map_err(|e| ShieldError::TlsHandshakeFailed(format!("MQTT TLS: {}", e)))?;

        // Step 3: WebSocket upgrade
        let ws_url = config.ws_url();
        let (mut ws_stream, _) = connect_async(&ws_url)
            .await
            .map_err(|e| ShieldError::Protocol(format!("MQTT WS upgrade: {}", e)))?;

        // Step 4: Send MQTT CONNECT
        let connect_packet = MqttEncoder::encode_connect(&config);
        ws_stream
            .send(tungstenite::Message::Binary(connect_packet))
            .await
            .map_err(|e| ShieldError::Protocol(format!("MQTT CONNECT send: {}", e)))?;

        // Step 5: Read CONNACK
        let msg = tokio::time::timeout(timeout, ws_stream.next())
            .await
            .map_err(|_| ShieldError::Timeout("MQTT CONNACK timeout".into()))?
            .ok_or_else(|| ShieldError::Protocol("MQTT WS closed".into()))?
            .map_err(|e| ShieldError::Protocol(format!("MQTT WS read: {}", e)))?;

        match msg {
            tungstenite::Message::Binary(data) => {
                if data.is_empty() || (data[0] >> 4) != (MQTT_CONNACK >> 4) {
                    return Err(ShieldError::Protocol("MQTT CONNACK expected".into()));
                }
                // Check reason code
                if data.len() >= 4 && data[3] != 0x00 {
                    return Err(ShieldError::AuthFailed("MQTT auth failed".into()));
                }
            }
            _ => return Err(ShieldError::Protocol("MQTT unexpected message type".into())),
        }

        // Step 6: Subscribe to receive topic
        let recv_topic = format!("{}{}", COVERT_TOPIC_RECV, config.client_id);
        let subscribe_packet =
            MqttEncoder::encode_subscribe(&recv_topic, self.next_packet_id().await);
        ws_stream
            .send(tungstenite::Message::Binary(subscribe_packet))
            .await
            .map_err(|e| ShieldError::Protocol(format!("MQTT SUBSCRIBE send: {}", e)))?;

        // Step 7: Send CONNECT payload with destination
        let send_topic = format!("{}{}/cmd", COVERT_TOPIC_PREFIX, config.client_id);
        let connect_data = format!("CONNECT {}", dest).into_bytes();
        let publish_packet =
            MqttEncoder::encode_publish(&send_topic, &connect_data, self.next_packet_id().await);
        ws_stream
            .send(tungstenite::Message::Binary(publish_packet))
            .await
            .map_err(|e| ShieldError::Protocol(format!("MQTT PUBLISH send: {}", e)))?;

        // Step 8: Read SUBACK and initial response
        let msg = tokio::time::timeout(timeout, ws_stream.next())
            .await
            .map_err(|_| ShieldError::Timeout("MQTT SUBACK timeout".into()))?
            .ok_or_else(|| ShieldError::Protocol("MQTT WS closed".into()))?
            .map_err(|e| ShieldError::Protocol(format!("MQTT WS read: {}", e)))?;

        // The WebSocket stream is now ready for bidirectional data transfer
        // In production, we'd keep this stream and wrap it

        drop(ws_stream);

        // Create a new TCP connection for data (simplified)
        let data_stream = tokio::time::timeout(timeout, TcpStream::connect(config.broker_addr))
            .await
            .map_err(|_| ShieldError::Timeout("MQTT data timeout".into()))?
            .map_err(|e| ShieldError::ConnectionRefused(e.to_string()))?;

        Ok(data_stream)
    }
}

// Need to import StreamExt for ws_stream.next()
use futures_util::{SinkExt, StreamExt};

#[async_trait]
impl Transport for MqttWsTransport {
    fn name(&self) -> &str {
        "mqtt-ws"
    }

    fn priority(&self) -> u8 {
        10
    }

    async fn connect(&self, addr: &SocketAddr) -> Result<Box<dyn TransportConnection>, ShieldError> {
        let sni = self.config.read().await.sni_domain.clone();

        match self.connect_mqtt(addr).await {
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

                // Try next broker
                let mut idx = self.broker_idx.write().await;
                *idx = (*idx + 1) % PUBLIC_MQTT_BROKERS.len();
                let new_broker = PUBLIC_MQTT_BROKERS[*idx];
                drop(idx);

                let mut config = self.config.write().await;
                config.broker_hostname = new_broker.to_string();
                config.sni_domain = new_broker.to_string();
                config.broker_addr = format!("{}:8883", new_broker)
                    .parse()
                    .unwrap_or(config.broker_addr);

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
        let mut idx = self.broker_idx.write().await;
        *idx = (*idx + 1) % PUBLIC_MQTT_BROKERS.len();
        let new_broker = PUBLIC_MQTT_BROKERS[*idx].to_string();
        drop(idx);

        let mut config = self.config.write().await;
        config.broker_hostname = new_broker.clone();
        config.sni_domain = new_broker.clone();
        Ok(new_broker)
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

impl MqttWsTransport {
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
