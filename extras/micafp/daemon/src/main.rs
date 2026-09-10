// ─────────────────────────────────────────────────────────────────────────────
// MICAFP-UnifiedShield-vip-ultra-Quantum-ultra v8.0 — Daemon Entry Point
// Complete merge of all 13 source projects. Zero features removed.
// ─────────────────────────────────────────────────────────────────────────────

use anyhow::Result;
use shield_daemon::config::schema::ShieldConfig;
use shield_daemon::orchestrator::UnifiedOrchestrator;
use shield_daemon::watchdog::SystemWatchdog;
use std::sync::Arc;
use tokio::signal;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // ── Telemetry / tracing ──────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_thread_ids(true)
        .json()
        .init();

    tracing::info!(
        version = "8.0.0",
        project = "MICAFP-UnifiedShield-vip-ultra-Quantum-ultra",
        "Daemon starting — complete merge of all 13 source projects"
    );

    // ── Configuration ────────────────────────────────────────────────────────
    let config = ShieldConfig::load_or_default()?;
    let config = Arc::new(config);

    // ── Watchdog ─────────────────────────────────────────────────────────────
    let watchdog = Arc::new(SystemWatchdog::new(30));
    let watchdog_handle = {
        let w = Arc::clone(&watchdog);
        tokio::spawn(async move { w.run().await })
    };

    // ── Orchestrator ─────────────────────────────────────────────────────────
    let orchestrator = UnifiedOrchestrator::new(Arc::clone(&config)).await?;
    let orchestrator = Arc::new(orchestrator);

    // ── Run until signal ─────────────────────────────────────────────────────
    tokio::select! {
        result = orchestrator.run() => {
            if let Err(e) = result {
                tracing::error!(error = %e, "Orchestrator exited with error");
            }
        }
        _ = signal::ctrl_c() => {
            tracing::info!("SIGINT received — shutting down gracefully");
        }
    }

    watchdog_handle.abort();
    tracing::info!("Daemon stopped cleanly");
    Ok(())
}
