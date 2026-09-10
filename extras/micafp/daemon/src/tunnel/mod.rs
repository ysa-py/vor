pub mod amneziawg;
pub mod boringtun_adapter;
pub mod split_tunnel;
pub mod tun_device;
pub mod wireguard;

pub use amneziawg::AmneziaWGTunnel;
pub use boringtun_adapter::BoringTunAdapter;
pub use split_tunnel::SplitTunnel;
pub use tun_device::TunDevice;
pub use wireguard::WireGuardTunnel;

pub type AmneziaWgTunnel = AmneziaWGTunnel;
pub type BoringtunAdapter = BoringTunAdapter;

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Tunnel configuration.
#[derive(Debug, Clone)]
pub struct TunnelConfig {
    pub local_ip: Ipv4Addr,
    pub remote_ip: Ipv4Addr,
    pub gateway: Ipv4Addr,
    pub mtu: u16,
    pub endpoint: SocketAddr,
    pub port: u16,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        TunnelConfig {
            local_ip: Ipv4Addr::new(10, 0, 0, 1),
            remote_ip: Ipv4Addr::new(10, 0, 0, 2),
            gateway: Ipv4Addr::new(10, 0, 0, 1),
            mtu: 1500,
            endpoint: SocketAddr::from(([127, 0, 0, 1], 443)),
            port: 443,
        }
    }
}

/// Tunnel error type — enum variant with common constructors.
#[derive(Debug, Clone)]
pub enum TunnelError {
    ConnectionFailed(String),
    Config(String),
    Io(String),
    Timeout(String),
    Other(String),
}

impl TunnelError {
    pub fn connection_failed(msg: impl Into<String>) -> Self {
        TunnelError::ConnectionFailed(msg.into())
    }
    pub fn config(msg: impl Into<String>) -> Self {
        TunnelError::Config(msg.into())
    }
}

impl fmt::Display for TunnelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TunnelError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            TunnelError::Config(msg) => write!(f, "Config error: {}", msg),
            TunnelError::Io(msg) => write!(f, "IO error: {}", msg),
            TunnelError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            TunnelError::Other(msg) => write!(f, "Tunnel error: {}", msg),
        }
    }
}

impl std::error::Error for TunnelError {}

/// Tunnel state enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelState {
    Idle,
    Connecting,
    Connected,
    Disconnecting,
    Disconnected,
    Error,
    Unconfigured,
    Active,
    Failed,
    Handshaking,
    ShuttingDown,
}

impl fmt::Display for TunnelState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TunnelState::Idle => write!(f, "idle"),
            TunnelState::Connecting => write!(f, "connecting"),
            TunnelState::Connected => write!(f, "connected"),
            TunnelState::Disconnecting => write!(f, "disconnecting"),
            TunnelState::Disconnected => write!(f, "disconnected"),
            TunnelState::Error => write!(f, "error"),
            TunnelState::Unconfigured => write!(f, "unconfigured"),
            TunnelState::Active => write!(f, "active"),
            TunnelState::Failed => write!(f, "failed"),
            TunnelState::Handshaking => write!(f, "handshaking"),
            TunnelState::ShuttingDown => write!(f, "shutting_down"),
        }
    }
}

/// Tunnel statistics.
#[derive(Debug, Clone, Default)]
pub struct TunnelStats {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub reconnections: u64,
    pub handshakes: u64,
    pub last_handshake: u64,
}

/// Obfuscation mode for tunnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObfuscationMode {
    None,
    CloudflareWorker,
    CdnGateway,
    IpfsRelay,
    MqttObfuscation,
}