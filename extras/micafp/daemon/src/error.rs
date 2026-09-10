// ─────────────────────────────────────────────────────────────────────────────
// MICAFP-UnifiedShield-vip-ultra-Quantum-ultra v8.0 — Error Types
// Complete unified error system for all 13 source projects.
// ─────────────────────────────────────────────────────────────────────────────

use thiserror::Error;

/// Structured error codes for every failure mode in UnifiedShield.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ErrorCode {
    // ── IPC Errors (1xxx) ────────────────────────────────────────────────────
    IpcConnectionFailed = 1001,
    IpcChannelClosed = 1002,
    IpcMessageParseError = 1003,
    IpcTimeout = 1004,

    // ── Transport Errors (2xxx) ──────────────────────────────────────────────
    TransportConnectionFailed = 2001,
    TransportTimeout = 2002,
    AllTransportsExhausted = 2003,
    DpiBlockDetected = 2004,

    // ── Config Errors (3xxx) ─────────────────────────────────────────────────
    ConfigParseFailed = 3001,
    ConfigNotFound = 3002,
    ConfigUpdateFailed = 3003,
    ConfigError = 3004,

    // ── Crypto Errors (4xxx) ─────────────────────────────────────────────────
    CryptoKeyExchangeFailed = 4001,
    CryptoSignatureInvalid = 4002,
    CryptoDecryptionFailed = 4003,
    CryptoEncryptionFailed = 4004,
    CryptoKeyGenerationFailed = 4005,
    CryptoHkdfFailed = 4006,
    CryptoError = 4007,
    CryptoPostQuantumFailed = 4101,

    // ── Anti-Forensics Errors (5xxx) ─────────────────────────────────────────
    AntiForensicsWipeFailed = 5001,
    AntiForensicsDeviceSecretCorrupted = 5002,

    // ── AI / Inference Errors (6xxx) ─────────────────────────────────────────
    AiInferenceFailed = 6001,
    AiModelNotFound = 6002,
    AiModelLoadFailed = 6003,
    AiTransportSelectionFailed = 6004,

    // ── P2P Errors (7xxx) ────────────────────────────────────────────────────
    P2pI2pError = 7001,
    P2pYggdrasilError = 7002,
    P2pPeerExchangeFailed = 7003,

    // ── NAIN / National-Intranet Errors (8xxx) ───────────────────────────────
    NainCovertChannelFailed = 8001,
    NainAcousticChannelFailed = 8002,
    NainWifiAwareUnavailable = 8003,
    NainBleMeshFailed = 8004,

    // ── Unknown / Generic ────────────────────────────────────────────────────
    Unknown = 9999,
}

impl ErrorCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

/// Structured IPC error payload sent to the UI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IpcErrorResponse {
    pub code: i32,
    pub message: String,
    pub category: String,
    pub source: Option<String>,
    pub timestamp_ms: u64,
}

/// The unified error type for the ShieldDaemon.
///
/// Variants provide human-readable messages for the common failure modes,
/// while the [`ShieldError::Coded`] variant carries a structured
/// [`ErrorCode`] for the many category-specific helpers used across the
/// merged projects.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ShieldError {
    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Crypto (post-quantum) error: {0}")]
    CryptoPostQuantum(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("IPC error [{code:?}]: {message}")]
    Ipc { code: ErrorCode, message: String },

    #[error("IO error: {0}")]
    Io(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Connection refused: {0}")]
    ConnectionRefused(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("All transports exhausted: {0}")]
    AllTransportsExhausted(String),

    #[error("All endpoints exhausted")]
    AllEndpointsExhausted,

    #[error("NAIN detected — switching to covert channel: {0}")]
    NainDetected(String),

    #[error("DPI block detected — triggering failover: {0}")]
    DpiBlock(String),

    #[error("QUIC error: {0}")]
    QuicError(String),

    #[error("TLS handshake failed: {0}")]
    TlsHandshakeFailed(String),

    #[error("DNS resolution failed: {0}")]
    DnsResolutionFailed(String),

    #[error("ICMP error: {0}")]
    IcmpError(String),

    #[error("P2P/I2P error: {0}")]
    P2pI2pError(String),

    #[error("AI transport selection failed: {0}")]
    AiTransportSelectionFailed(String),

    #[error("Anti-forensics device secret corrupted")]
    AntiForensicsDeviceSecretCorrupted,

    #[error("NAIN covert channel failed: {0}")]
    NainCovertChannelFailed(String),

    #[error("NAIN acoustic channel failed: {0}")]
    NainAcousticChannelFailed(String),

    #[error("CDN worker error: {0}")]
    CdnWorkerError(String),

    #[error("Endpoint unreachable: {0}")]
    EndpointUnreachable(String),

    #[error("Transport unavailable: {0}")]
    TransportUnavailable(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Quantum key exchange failed: {0}")]
    QuantumKex(String),

    #[error("Peer exchange failed: {0}")]
    PeerExchange(String),

    #[error("Unknown error: {0}")]
    Unknown(String),

    /// Generic coded error produced by the category helpers.
    #[error("{message}")]
    Coded {
        code: ErrorCode,
        message: String,
        details: Option<String>,
    },
}

impl ShieldError {
    /// Build a coded error from a category code and message.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        ShieldError::Coded {
            code,
            message: message.into(),
            details: None,
        }
    }

    /// Create an AI-category error.
    pub fn ai(code: ErrorCode, message: impl Into<String>) -> Self {
        ShieldError::new(code, message)
    }

    /// Create a P2P-category error.
    pub fn p2p(code: ErrorCode, message: impl Into<String>) -> Self {
        ShieldError::new(code, message)
    }

    /// Create a NAIN-mode error.
    pub fn nain_mode(code: ErrorCode, message: impl Into<String>) -> Self {
        ShieldError::new(code, message)
    }

    /// Create an anti-forensics error.
    pub fn anti_forensics(code: ErrorCode, message: impl Into<String>) -> Self {
        ShieldError::new(code, message)
    }

    /// Create an IPC-category error.
    pub fn ipc(code: ErrorCode, message: impl Into<String>) -> Self {
        ShieldError::Ipc {
            code,
            message: message.into(),
        }
    }

    /// Create a config-category error.
    pub fn config(message: impl Into<String>) -> Self {
        ShieldError::Config(message.into())
    }

    /// Create a transport-category error.
    pub fn transport(message: impl Into<String>) -> Self {
        ShieldError::Transport(message.into())
    }

    /// Create a crypto-category error.
    pub fn crypto(code: ErrorCode, message: impl Into<String>) -> Self {
        ShieldError::new(code, message)
    }

    /// Create a crypto-category error with source info.
    pub fn crypto_with_source(
        code: ErrorCode,
        message: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        ShieldError::Coded {
            code,
            message: message.into(),
            details: Some(source.into()),
        }
    }
}

impl From<anyhow::Error> for ShieldError {
    fn from(e: anyhow::Error) -> Self {
        ShieldError::Unknown(e.to_string())
    }
}

impl From<std::io::Error> for ShieldError {
    fn from(e: std::io::Error) -> Self {
        ShieldError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for ShieldError {
    fn from(e: serde_json::Error) -> Self {
        ShieldError::Serialization(e.to_string())
    }
}
