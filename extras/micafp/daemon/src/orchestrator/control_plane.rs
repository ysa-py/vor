// ─────────────────────────────────────────────────────────────────────────────
// Unified Orchestrator Control Plane — coordinates all subsystems.
// MICAFP-UnifiedShield-vip-ultra-Quantum-ultra v8.0
// ─────────────────────────────────────────────────────────────────────────────

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, RwLock};
use tokio::time;
use tracing::{error, info, warn};

use super::{OrchestratorConfig, SystemStateSnapshot};
use crate::config::schema::ShieldConfig;

pub struct UnifiedOrchestrator {
    config: OrchestratorConfig,
    state: Arc<RwLock<OrchestratorState>>,
    shutdown_tx: broadcast::Sender<()>,
}

#[derive(Debug)]
struct OrchestratorState {
    active_transport: String,
    active_core: String,
    threat_level: String,
    health_score: f64,
    start_time: Instant,
    bytes_transferred: u64,
    failover_count: u32,
    nain_active: bool,
}

impl Default for OrchestratorState {
    fn default() -> Self {
        Self {
            active_transport: "vless".into(),
            active_core: "hiddify".into(),
            threat_level: "Low".into(),
            health_score: 1.0,
            start_time: Instant::now(),
            bytes_transferred: 0,
            failover_count: 0,
            nain_active: false,
        }
    }
}

impl UnifiedOrchestrator {
    /// Create a new orchestrator from a ShieldConfig.
    ///
    /// Returns `Ok(orchestrator)` on success. The orchestrator has not yet
    /// started running; call `.run()` to begin the main event loop.
    pub async fn new(config: Arc<ShieldConfig>) -> anyhow::Result<Self> {
        let _ = config; // Config used for future subsystem init
        let orch_config = OrchestratorConfig::default();
        let (shutdown_tx, _) = broadcast::channel(4);
        Ok(Self {
            config: orch_config,
            state: Arc::new(RwLock::new(OrchestratorState::default())),
            shutdown_tx,
        })
    }

    /// Subscribe to the shutdown signal channel.
    pub fn shutdown_receiver(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Signal all subsystems to shut down.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    /// Run the orchestrator main loop. Returns when a shutdown is signalled
    /// or a fatal error occurs.
    pub async fn run(self: Arc<Self>) -> anyhow::Result<()> {
        info!("UnifiedOrchestrator starting");

        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let health_interval = self.config.health_check_interval;
        let telemetry_interval = self.config.telemetry_interval;

        let mut health_ticker = time::interval(health_interval);
        let mut telemetry_ticker = time::interval(telemetry_interval);

        loop {
            tokio::select! {
                _ = health_ticker.tick() => {
                    self.run_health_cycle().await;
                }
                _ = telemetry_ticker.tick() => {
                    self.flush_telemetry().await;
                }
                _ = shutdown_rx.recv() => {
                    info!("UnifiedOrchestrator received shutdown signal");
                    break;
                }
            }
        }

        info!("UnifiedOrchestrator stopped");
        Ok(())
    }

    async fn run_health_cycle(&self) {
        let mut state = self.state.write().await;

        // EWMA health score decay with auto-recovery on failover threshold
        state.health_score = (state.health_score * 0.99).clamp(0.0, 1.0);

        if state.health_score < 0.7 {
            warn!(
                score = state.health_score,
                "Health degraded — triggering failover"
            );
            state.failover_count += 1;
            state.active_transport = select_next_transport(&state.active_transport);
            state.health_score = 1.0;
            info!(transport = %state.active_transport, "Failover complete");
        }
    }

    async fn flush_telemetry(&self) {
        info!("Telemetry flush cycle triggered");
        // In production: delegate to TelemetryAggregator::flush_report()
    }

    pub async fn snapshot(&self) -> SystemStateSnapshot {
        let s = self.state.read().await;
        SystemStateSnapshot {
            active_transport: s.active_transport.clone(),
            active_core: s.active_core.clone(),
            threat_level: s.threat_level.clone(),
            health_score: s.health_score,
            uptime_secs: s.start_time.elapsed().as_secs(),
            bytes_transferred: s.bytes_transferred,
            failover_count: s.failover_count,
            battery_pct: None,
            nain_active: s.nain_active,
        }
    }
}

fn select_next_transport(current: &str) -> String {
    const PRIORITY: &[&str] = &[
        "vless",
        "shadow_tls",
        "reality",
        "hysteria2",
        "tuic_v5",
        "naive_proxy",
        "cdn_worker",
        "doq_tunnel",
        "meek",
        "mqtt_ws",
    ];
    let pos = PRIORITY.iter().position(|&t| t == current).unwrap_or(0);
    PRIORITY[(pos + 1) % PRIORITY.len()].to_string()
}
