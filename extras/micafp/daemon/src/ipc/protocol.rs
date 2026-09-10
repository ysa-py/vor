// ─────────────────────────────────────────────────────────────────────────────
// IPC Protocol — request/response message types
// MICAFP-UnifiedShield-vip-ultra-Quantum-ultra v8.0
// ─────────────────────────────────────────────────────────────────────────────

use super::{BatteryState, NainStatus};
use crate::error::IpcErrorResponse;
use serde::{Deserialize, Serialize};

/// Commands sent from the Flutter UI to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcRequest {
    /// Connect to the anti-censorship network.
    Connect {
        transport: Option<String>,
        endpoint: Option<String>,
    },
    /// Disconnect from the network.
    Disconnect,
    /// Query current daemon status.
    StatusQuery,
    /// Update runtime configuration.
    ConfigUpdate { patch: serde_json::Value },
    /// Trigger emergency wipe (anti-forensics).
    WipeTrigger { auth_token: String },
}

/// Responses sent from the daemon to the Flutter UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcResponse {
    /// Acknowledgement of a command (success).
    Ack { for_type: String },
    /// Current status response.
    StatusResponse {
        connected: bool,
        transport: Option<String>,
        battery_state: BatteryState,
        nain_status: NainStatus,
        bytes_sent: u64,
        bytes_recv: u64,
        uptime_secs: u64,
        peer_count: u32,
    },
    /// Structured error response.
    Error(IpcErrorResponse),
}

/// Union of request and response for serialisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IpcMessage {
    Request(IpcRequest),
    Response(IpcResponse),
}
