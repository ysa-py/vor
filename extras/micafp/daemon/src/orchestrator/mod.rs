// ─────────────────────────────────────────────────────────────────────────────
// Orchestrator — Central control plane
// MICAFP-UnifiedShield-vip-ultra-Quantum-ultra v8.0
// ─────────────────────────────────────────────────────────────────────────────

use std::time::Duration;

pub mod control_plane;
pub mod failover;
pub mod health_monitor;

pub use control_plane::UnifiedOrchestrator;
pub use failover::FailoverEngine;
pub use health_monitor::HealthMonitor;

/// Alias for the orchestrator health monitor.
pub type OrchestratorHealthMonitor = HealthMonitor;
/// Alias for the failover engine.
pub type FailoverController = FailoverEngine;

/// Configuration for the UnifiedOrchestrator.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Interval between health-check cycles.
    pub health_check_interval: Duration,
    /// Interval between telemetry flushes.
    pub telemetry_interval: Duration,
    /// Maximum number of failover attempts before giving up.
    pub max_failover_attempts: u32,
    /// Whether the quantum subsystem is enabled.
    pub quantum_enabled: bool,
    /// Whether the AI/ML subsystem is enabled.
    pub ai_enabled: bool,
    /// Whether P2P mesh networking is enabled.
    pub mesh_enabled: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            health_check_interval: Duration::from_secs(30),
            telemetry_interval: Duration::from_secs(300),
            max_failover_attempts: 5,
            quantum_enabled: true,
            ai_enabled: true,
            mesh_enabled: true,
        }
    }
}

/// A point-in-time snapshot of the orchestrator's system state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemStateSnapshot {
    pub active_transport: String,
    pub active_core: String,
    pub threat_level: String,
    pub health_score: f64,
    pub uptime_secs: u64,
    pub bytes_transferred: u64,
    pub failover_count: u32,
    pub battery_pct: Option<u8>,
    pub nain_active: bool,
}
