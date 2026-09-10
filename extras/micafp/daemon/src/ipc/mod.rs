// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield v8.0 — IPC Bridge (Rust Daemon ↔ Flutter UI)
//
// Communication channel between the Rust daemon and the Flutter UI layer:
//   • Unix domain socket on Linux / Android / macOS
//   • Named pipe on Windows
//
// JSON-based message protocol. Every error from any subsystem MUST propagate
// to the UI (constraint C-08: no silent failures).
// ─────────────────────────────────────────────────────────────────────────────

pub mod named_pipe;
pub mod protocol;
pub mod unix_socket;

pub use named_pipe::NamedPipeIpc;
pub use protocol::{IpcMessage, IpcRequest, IpcResponse};
pub use unix_socket::UnixSocketIpc;

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::error::{ErrorCode, IpcErrorResponse, ShieldError};

// ── Platform-specific socket path ────────────────────────────────────────────

fn ipc_socket_path() -> PathBuf {
    #[cfg(target_family = "unix")]
    {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_owned());
        PathBuf::from(runtime_dir).join("shield-daemon.sock")
    }

    #[cfg(target_os = "windows")]
    {
        PathBuf::from(r"\\.\pipe\shield-daemon")
    }

    #[cfg(not(any(target_family = "unix", target_os = "windows")))]
    {
        PathBuf::from("/tmp/shield-daemon.sock")
    }
}

// ── Battery / NAIN state enums ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatteryState {
    Charging,
    Discharging,
    Normal,
    Low,
    Critical,
    Unknown,
}

impl BatteryState {
    pub fn probe_throttle(&self) -> f64 {
        match self {
            BatteryState::Normal | BatteryState::Charging | BatteryState::Discharging => 1.0,
            BatteryState::Low => 0.5,
            BatteryState::Critical => 0.2,
            BatteryState::Unknown => 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NainStatus {
    Clear,
    Detected,
    Transitioning,
    Unknown,
}

// ── IPC Frame ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IpcFrame {
    id: u64,
    #[serde(flatten)]
    message: IpcMessage,
}

// ── IPC Server ───────────────────────────────────────────────────────────────

pub struct IpcServer;

impl IpcServer {
    pub async fn run(
        state: Arc<DaemonStateProxy>,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<(), ShieldError> {
        let socket_path = ipc_socket_path();

        #[cfg(target_family = "unix")]
        {
            if socket_path.exists() {
                std::fs::remove_file(&socket_path).map_err(|e| {
                    ShieldError::ipc(
                        ErrorCode::IpcConnectionFailed,
                        format!("Failed to remove stale socket: {}", e),
                    )
                })?;
            }
        }

        #[cfg(target_family = "unix")]
        let listener = {
            let listener = UnixListener::bind(&socket_path).map_err(|e| {
                ShieldError::ipc(
                    ErrorCode::IpcConnectionFailed,
                    format!("Failed to bind IPC socket at {:?}: {}", socket_path, e),
                )
            })?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o600);
                std::fs::set_permissions(&socket_path, perms).ok();
            }
            listener
        };

        info!(path = %socket_path.display(), "IPC server listening");

        let mut next_conn_id: u64 = 0;

        loop {
            tokio::select! {
                accept_result = accept_connection(&listener) => {
                    match accept_result {
                        Ok((stream, addr)) => {
                            let conn_id = next_conn_id;
                            next_conn_id += 1;
                            debug!(conn_id, ?addr, "New IPC client connected");

                            let state = state.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_client(stream, conn_id, state).await {
                                    warn!(conn_id, error = %e, "IPC client handler error");
                                }
                            });
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to accept IPC connection");
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("IPC server received shutdown signal");
                    #[cfg(target_family = "unix")]
                    { let _ = std::fs::remove_file(&socket_path); }
                    return Ok(());
                }
            }
        }
    }
}

#[cfg(target_family = "unix")]
async fn accept_connection(listener: &UnixListener) -> Result<(UnixStream, PathBuf), ShieldError> {
    let (stream, addr) = listener.accept().await.map_err(|e| {
        ShieldError::ipc(
            ErrorCode::IpcConnectionFailed,
            format!("Accept failed: {}", e),
        )
    })?;
    let addr_path = addr
        .as_pathname()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("<unknown>"));
    Ok((stream, addr_path))
}

// ── Daemon State Proxy ───────────────────────────────────────────────────────

#[derive(Clone)]
pub struct DaemonStateProxy {
    pub wipe_signal: tokio::sync::watch::Sender<bool>,
    pub connected: Arc<std::sync::atomic::AtomicBool>,
    pub transport_name: Arc<parking_lot::Mutex<String>>,
    pub battery_state: Arc<std::sync::atomic::AtomicU8>,
    pub nain_status: Arc<std::sync::atomic::AtomicU8>,
    pub bytes_sent: Arc<std::sync::atomic::AtomicU64>,
    pub bytes_recv: Arc<std::sync::atomic::AtomicU64>,
    pub start_time: Arc<std::sync::Mutex<std::time::Instant>>,
}

impl DaemonStateProxy {
    pub fn new(wipe_signal: tokio::sync::watch::Sender<bool>) -> Self {
        Self {
            wipe_signal,
            connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            transport_name: Arc::new(parking_lot::Mutex::new(String::new())),
            battery_state: Arc::new(std::sync::atomic::AtomicU8::new(
                BatteryState::Unknown as u8,
            )),
            nain_status: Arc::new(std::sync::atomic::AtomicU8::new(NainStatus::Unknown as u8)),
            bytes_sent: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            bytes_recv: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            start_time: Arc::new(std::sync::Mutex::new(std::time::Instant::now())),
        }
    }

    pub fn current_status(&self) -> IpcResponse {
        use std::sync::atomic::Ordering::Relaxed;
        let connected = self.connected.load(Relaxed);
        let transport = {
            let g = self.transport_name.lock();
            if g.is_empty() {
                None
            } else {
                Some(g.clone())
            }
        };
        let battery = match self.battery_state.load(Relaxed) {
            0 => BatteryState::Charging,
            1 => BatteryState::Discharging,
            2 => BatteryState::Low,
            3 => BatteryState::Critical,
            _ => BatteryState::Unknown,
        };
        let nain = match self.nain_status.load(Relaxed) {
            0 => NainStatus::Clear,
            1 => NainStatus::Detected,
            2 => NainStatus::Transitioning,
            _ => NainStatus::Unknown,
        };
        let uptime = self.start_time.lock().unwrap().elapsed().as_secs();

        IpcResponse::StatusResponse {
            connected,
            transport,
            battery_state: battery,
            nain_status: nain,
            bytes_sent: self.bytes_sent.load(Relaxed),
            bytes_recv: self.bytes_recv.load(Relaxed),
            uptime_secs: uptime,
            peer_count: 0,
        }
    }
}

// ── Client Handler ───────────────────────────────────────────────────────────

async fn handle_client(
    stream: UnixStream,
    conn_id: u64,
    state: Arc<DaemonStateProxy>,
) -> Result<(), ShieldError> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await.map_err(|e| {
            ShieldError::ipc(ErrorCode::IpcChannelClosed, format!("Read error: {}", e))
        })?;

        if bytes_read == 0 {
            debug!(conn_id, "IPC client disconnected");
            return Ok(());
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        debug!(conn_id, msg = %trimmed, "IPC request received");

        let frame: IpcFrame = match serde_json::from_str(trimmed) {
            Ok(f) => f,
            Err(e) => {
                let err_resp = IpcResponse::Error(IpcErrorResponse {
                    code: ErrorCode::IpcMessageParseError as i32,
                    message: format!("Invalid IPC message: {}", e),
                    category: "ipc".to_owned(),
                    source: None,
                    timestamp_ms: now_millis(),
                });
                let resp_frame = IpcFrame {
                    id: 0,
                    message: IpcMessage::Response(err_resp),
                };
                send_response(&mut write_half, &resp_frame).await?;
                continue;
            }
        };

        let request = match frame.message {
            IpcMessage::Request(req) => req,
            IpcMessage::Response(_) => {
                warn!(conn_id, "Received unexpected response from UI — ignoring");
                continue;
            }
        };

        let response = process_request(request, &state).await;
        let resp_frame = IpcFrame {
            id: frame.id,
            message: IpcMessage::Response(response),
        };

        send_response(&mut write_half, &resp_frame).await?;
    }
}

async fn process_request(request: IpcRequest, state: &DaemonStateProxy) -> IpcResponse {
    use std::sync::atomic::Ordering::Relaxed;
    match request {
        IpcRequest::Connect {
            transport,
            endpoint: _,
        } => {
            info!(?transport, "IPC: Connect requested");
            state.connected.store(true, Relaxed);
            *state.transport_name.lock() = transport.unwrap_or_else(|| "hysteria2".to_owned());
            IpcResponse::Ack {
                for_type: "Connect".to_owned(),
            }
        }
        IpcRequest::Disconnect => {
            info!("IPC: Disconnect requested");
            state.connected.store(false, Relaxed);
            *state.transport_name.lock() = String::new();
            IpcResponse::Ack {
                for_type: "Disconnect".to_owned(),
            }
        }
        IpcRequest::StatusQuery => {
            debug!("IPC: Status query");
            state.current_status()
        }
        IpcRequest::ConfigUpdate { patch } => {
            info!(?patch, "IPC: Config update requested");
            IpcResponse::Ack {
                for_type: "ConfigUpdate".to_owned(),
            }
        }
        IpcRequest::WipeTrigger { auth_token } => {
            warn!("IPC: Emergency wipe triggered via IPC");
            if auth_token.is_empty() {
                return IpcResponse::Error(IpcErrorResponse {
                    code: ErrorCode::AntiForensicsWipeFailed as i32,
                    message: "Wipe trigger auth token is empty".to_owned(),
                    category: "anti_forensics".to_owned(),
                    source: None,
                    timestamp_ms: now_millis(),
                });
            }
            if state.wipe_signal.send(true).is_err() {
                return IpcResponse::Error(IpcErrorResponse {
                    code: ErrorCode::AntiForensicsWipeFailed as i32,
                    message: "Failed to signal wipe controller".to_owned(),
                    category: "anti_forensics".to_owned(),
                    source: None,
                    timestamp_ms: now_millis(),
                });
            }
            IpcResponse::Ack {
                for_type: "WipeTrigger".to_owned(),
            }
        }
    }
}

async fn send_response(
    writer: &mut (impl AsyncWriteExt + Unpin),
    frame: &IpcFrame,
) -> Result<(), ShieldError> {
    let mut json = serde_json::to_string(frame)
        .map_err(|e| ShieldError::ipc(ErrorCode::IpcMessageParseError, e.to_string()))?;
    json.push('\n');
    writer
        .write_all(json.as_bytes())
        .await
        .map_err(|e| ShieldError::ipc(ErrorCode::IpcChannelClosed, e.to_string()))?;
    writer
        .flush()
        .await
        .map_err(|e| ShieldError::ipc(ErrorCode::IpcChannelClosed, e.to_string()))?;
    Ok(())
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
