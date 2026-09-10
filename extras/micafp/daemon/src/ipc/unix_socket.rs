// Unix domain socket IPC implementation
// MICAFP-UnifiedShield-vip-ultra-Quantum-ultra v8.0

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

/// Unix domain socket IPC server.
pub struct UnixSocketIpc;

impl UnixSocketIpc {
    pub async fn listen_and_serve(socket_path: &str) -> Result<()> {
        if std::path::Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path).ok();
        }
        let listener = UnixListener::bind(socket_path).context("Failed to bind Unix socket")?;
        loop {
            let (stream, _) = listener.accept().await?;
            tokio::spawn(async move {
                let (reader, mut writer) = stream.into_split();
                let mut reader = BufReader::new(reader);
                let mut line = String::new();
                while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                    let resp = handle_ipc(line.trim());
                    let _ = writer.write_all(format!("{}\n", resp).as_bytes()).await;
                    line.clear();
                }
            });
        }
    }
}

fn handle_ipc(msg: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(msg) {
        Ok(json) => {
            let cmd = json
                .get("command")
                .and_then(|c| c.as_str())
                .unwrap_or("unknown");
            match cmd {
                "status" => r#"{"status":"running","active_core":"mahsang"}"#.into(),
                "switch_core" => format!(
                    r#"{{"result":"ok","switched_to":"{}"}}"#,
                    json.get("core_id")
                        .and_then(|c| c.as_str())
                        .unwrap_or("hiddify")
                ),
                _ => format!(r#"{{"error":"unknown: {}"}}"#, cmd),
            }
        }
        Err(_) => r#"{"error":"invalid_json"}"#.into(),
    }
}
