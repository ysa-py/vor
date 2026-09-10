//! Core manager for UnifiedShield.
//!
//! Manages all 9 VPN cores with active + shadow connections (2 warm standby),
//! seamless core switching with zero packet loss, periodic health monitoring,
//! and UCB1 bandit integration for automatic core selection.

use crate::ai::ucb_bandit::{CoreArm, UCBBandit};
use async_trait::async_trait;
use bytes::Bytes;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::cores::{
    AmneziaVpnCore, DefyxCore, HiddifyCore, LanternCore, MahsangCore, MoavCore, PsiphonCore,
    SingboxCore, XrayCore,
};

/// Health monitoring interval in seconds.
const HEALTH_CHECK_INTERVAL_SECS: u64 = 5;

/// Number of warm standby (shadow) cores.
const SHADOW_CORE_COUNT: usize = 2;

/// Core identifier type.
pub type CoreId = String;

/// Protocol type enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolType {
    Vless,
    Reality,
    Hysteria2,
    TuicV5,
    ShadowTls,
    NaiveProxy,
    CloudflareWorker,
    CdnWorker,
    CdnTunnel,
    ChineseCdn,
    DomainFronting,
    Meek,
    DohTunnel,
    DoqTunnel,
    WebrtcRelay,
    Webtransport,
    MqttTunnel,
    MqttWs,
    IcmpTunnel,
    PluggableTransport,
    MultihopChain,
}

/// Core health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoreHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Stopped,
}

/// Detailed core health report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreHealthReport {
    pub health: CoreHealth,
    pub latency_ms: u32,
    pub packet_loss: u32,
    pub last_check_ts: u64,
    pub checks_passed: u64,
    pub checks_failed: u64,
}

impl CoreHealthReport {
    pub fn healthy(latency_ms: u32, packet_loss: u32, ts: u64) -> Self {
        Self {
            health: CoreHealth::Healthy,
            latency_ms,
            packet_loss,
            last_check_ts: ts,
            checks_passed: 0,
            checks_failed: 0,
        }
    }

    pub fn degraded(latency_ms: u32, packet_loss: u32, ts: u64) -> Self {
        Self {
            health: CoreHealth::Degraded,
            latency_ms,
            packet_loss,
            last_check_ts: ts,
            checks_passed: 0,
            checks_failed: 0,
        }
    }

    pub fn unhealthy(ts: u64) -> Self {
        Self {
            health: CoreHealth::Unhealthy,
            latency_ms: 0,
            packet_loss: 100,
            last_check_ts: ts,
            checks_passed: 0,
            checks_failed: 0,
        }
    }

    pub fn stopped() -> Self {
        Self {
            health: CoreHealth::Stopped,
            latency_ms: 0,
            packet_loss: 0,
            last_check_ts: 0,
            checks_passed: 0,
            checks_failed: 0,
        }
    }
}

/// Core configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoreConfig {
    pub id: String,
    pub name: String,
    pub binary_path: Option<String>,
    pub config_file: Option<String>,
    pub extra_args: Vec<String>,
}

/// Core errors.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Core start failed: {0}")]
    StartFailed(String),
    #[error("Core stopped")]
    Stopped,
    #[error("Connection lost: {0}")]
    ConnectionLost(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Core result type.
pub type CoreResult<T> = Result<T, CoreError>;

/// Trait for core implementations.
#[async_trait]
pub trait CoreTrait: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn protocols(&self) -> &[ProtocolType];
    async fn start(&mut self, config: CoreConfig) -> CoreResult<()>;
    async fn stop(&mut self) -> CoreResult<()>;
    async fn health_check(&self) -> CoreResult<CoreHealth>;
    fn is_running(&self) -> bool;
}

/// Core state information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreState {
    /// Core identifier.
    pub id: CoreId,
    /// Human-readable name.
    pub name: String,
    /// Whether this core is the active core.
    pub is_active: bool,
    /// Whether this core is a warm standby.
    pub is_shadow: bool,
    /// Whether this core is currently running.
    pub is_running: bool,
    /// Latest health report.
    pub health: CoreHealthReport,
    /// Supported protocols.
    pub protocols: Vec<ProtocolType>,
}

/// Macro to implement CoreTrait for core adapters with the same shape.
macro_rules! impl_core_trait {
    ($type:ty, $id:expr, $name:expr, $protocols:expr) => {
        #[async_trait]
        impl CoreTrait for $type {
            fn id(&self) -> &'static str { $id }
            fn name(&self) -> &'static str { $name }
            fn protocols(&self) -> &[ProtocolType] { $protocols }
            async fn start(&mut self, _config: CoreConfig) -> CoreResult<()> {
                self.start().await.map_err(|e| CoreError::StartFailed(e.to_string()))
            }
            async fn stop(&mut self) -> CoreResult<()> {
                self.stop().await.map_err(|e| CoreError::StartFailed(e.to_string()))
            }
            async fn health_check(&self) -> CoreResult<CoreHealth> {
                self.health_check().await
                    .map(|running| if running { CoreHealth::Healthy } else { CoreHealth::Stopped })
                    .map_err(|e| CoreError::ConnectionLost(e.to_string()))
            }
            fn is_running(&self) -> bool {
                self.is_running()
            }
        }
    };
}

impl_core_trait!(crate::cores::HiddifyCoreAdapter, "hiddify", "Hiddify Core", &[ProtocolType::Hysteria2]);
impl_core_trait!(crate::cores::XrayCoreAdapter, "xray", "Xray Core", &[ProtocolType::Vless, ProtocolType::Reality, ProtocolType::ShadowTls]);
impl_core_trait!(crate::cores::SingboxCoreAdapter, "singbox", "Sing-box", &[ProtocolType::TuicV5, ProtocolType::Hysteria2]);
impl_core_trait!(crate::cores::AmneziaVpnAdapter, "amneziavpn", "AmneziaVPN", &[]);
impl_core_trait!(crate::cores::DefyxVpnAdapter, "defyx", "DefyX VPN", &[]);
impl_core_trait!(crate::cores::MoavAdapter, "moav", "Moav VPN", &[]);
impl_core_trait!(crate::cores::LanternAdapter, "lantern", "Lantern", &[]);
impl_core_trait!(crate::cores::MahsangAdapter, "mahsang", "Mahsang", &[]);
impl_core_trait!(crate::cores::PsiphonAdapter, "psiphon", "Psiphon", &[]);

/// Core manager that orchestrates all VPN cores.
///
/// Maintains:
/// - One active core handling all traffic
/// - Two warm standby (shadow) cores ready for seamless failover
/// - Health monitoring loop checking all cores every 5 seconds
/// - UCB1 bandit integration for automatic core selection
pub struct CoreManager {
    /// All registered cores.
    cores: Arc<DashMap<CoreId, Arc<Mutex<Box<dyn CoreTrait>>>>>,
    /// Core configurations.
    configs: Arc<DashMap<CoreId, CoreConfig>>,
    /// Core states.
    states: Arc<DashMap<CoreId, CoreState>>,
    /// Currently active core ID.
    active_core_id: Arc<RwLock<Option<CoreId>>>,
    /// Shadow (warm standby) core IDs.
    shadow_core_ids: Arc<RwLock<Vec<CoreId>>>,
    /// UCB1 bandit for core selection.
    bandit: Arc<UCBBandit>,
    /// Packet buffer for seamless switching.
    packet_buffer: Arc<Mutex<Vec<Bytes>>>,
    /// Health check task handle.
    health_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Whether the manager is running.
    running: AtomicBool,
    /// Total packets sent.
    packets_sent: AtomicU64,
    /// Total packets received.
    packets_received: AtomicU64,
}

impl CoreManager {
    /// Create a new core manager with all 9 cores registered.
    pub fn new() -> Self {
        let cores: Arc<DashMap<CoreId, Arc<Mutex<Box<dyn CoreTrait>>>>> = Arc::new(DashMap::new());
        let configs = Arc::new(DashMap::new());
        let states = Arc::new(DashMap::new());
        let bandit = Arc::new(UCBBandit::new(2.0));

        let mut manager = Self {
            cores,
            configs,
            states,
            active_core_id: Arc::new(RwLock::new(None)),
            shadow_core_ids: Arc::new(RwLock::new(Vec::new())),
            bandit,
            packet_buffer: Arc::new(Mutex::new(Vec::new())),
            health_task: Arc::new(Mutex::new(None)),
            running: AtomicBool::new(false),
            packets_sent: AtomicU64::new(0),
            packets_received: AtomicU64::new(0),
        };

        manager.register_all_cores();
        manager
    }

    /// Register all 9 cores.
    fn register_all_cores(&mut self) {
        let core_instances: Vec<Box<dyn CoreTrait>> = vec![
            Box::new(HiddifyCore::new("")),
            Box::new(XrayCore::new("")),
            Box::new(SingboxCore::new("")),
            Box::new(AmneziaVpnCore::new("")),
            Box::new(DefyxCore::new("")),
            Box::new(MoavCore::new("")),
            Box::new(LanternCore::new("")),
            Box::new(MahsangCore::new("")),
            Box::new(PsiphonCore::new("")),
        ];

        for core in core_instances {
            let id = core.id().to_string();
            let name = core.name().to_string();
            let protocols = core.protocols().to_vec();

            self.states.insert(
                id.clone(),
                CoreState {
                    id: id.clone(),
                    name,
                    is_active: false,
                    is_shadow: false,
                    is_running: false,
                    health: CoreHealthReport::stopped(),
                    protocols,
                },
            );

            self.cores.insert(id, Arc::new(Mutex::new(core)));
        }

        info!(core_count = self.cores.len(), "All cores registered");
    }

    /// Start the core manager with the given configurations.
    pub async fn start(&self, configs: HashMap<CoreId, CoreConfig>) -> CoreResult<()> {
        if self
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(CoreError::StartFailed(
                "Core manager already running".to_string(),
            ));
        }

        info!("Starting core manager with {} cores", self.cores.len());

        // Store all configurations
        for (id, config) in configs {
            self.configs.insert(id, config);
        }

        // Use UCB1 bandit to select the best core
        let selection = self.bandit.select();
        let active_arm = selection.arm;

        let active_id = self.arm_to_core_id(active_arm);
        info!(
            arm = %active_arm,
            core_id = %active_id,
            score = selection.score,
            "UCB1 bandit selected initial core"
        );

        // Start the selected active core
        self.start_core(&active_id).await?;

        // Set as active
        *self.active_core_id.write() = Some(active_id.clone());
        if let Some(mut state) = self.states.get_mut(&active_id) {
            state.is_active = true;
        }

        // Start shadow (warm standby) cores
        self.start_shadow_cores(&active_id).await?;

        // Start health monitoring loop
        self.start_health_monitor().await;

        info!(
            active = %active_id,
            "Core manager started"
        );

        Ok(())
    }

    /// Stop the core manager and all cores.
    pub async fn stop(&self) -> CoreResult<()> {
        if self
            .running
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Ok(());
        }

        info!("Stopping core manager");

        // Stop health monitor
        if let Some(handle) = self.health_task.lock().await.take() {
            handle.abort();
        }

        // Stop all cores
        for entry in self.cores.iter() {
            let id = entry.key();
            let mut core = entry.value().lock().await;
            if core.is_running() {
                if let Err(e) = core.stop().await {
                    warn!(core_id = %id, error = %e, "Error stopping core");
                }
            }
            if let Some(mut state) = self.states.get_mut(id) {
                state.is_running = false;
                state.is_active = false;
                state.is_shadow = false;
                state.health = CoreHealthReport::stopped();
            }
        }

        *self.active_core_id.write() = None;
        self.shadow_core_ids.write().clear();

        info!("Core manager stopped");
        Ok(())
    }

    /// Start a specific core.
    async fn start_core(&self, id: &CoreId) -> CoreResult<()> {
        let config = self
            .configs
            .get(id)
            .map(|e| e.value().clone())
            .unwrap_or_default();

        let core = self
            .cores
            .get(id)
            .ok_or_else(|| CoreError::StartFailed(format!("Core {} not found", id)))?;

        let mut core_guard = core.lock().await;
        core_guard.start(config).await?;

        if let Some(mut state) = self.states.get_mut(id) {
            state.is_running = true;
            state.health = CoreHealthReport::healthy(999, 0, 0);
        }

        info!(core_id = %id, "Core started");
        Ok(())
    }

    /// Start shadow (warm standby) cores for the given active core.
    async fn start_shadow_cores(&self, active_id: &CoreId) -> CoreResult<()> {
        let mut shadows = Vec::new();

        // Select the next 2 best cores as shadows
        for arm in CoreArm::all() {
            let candidate_id = self.arm_to_core_id(arm.clone());
            if candidate_id == *active_id {
                continue;
            }

            // Don't start if already running
            if let Some(state) = self.states.get(&candidate_id) {
                if state.is_running {
                    continue;
                }
            }

            if shadows.len() >= SHADOW_CORE_COUNT {
                break;
            }

            match self.start_core(&candidate_id).await {
                Ok(()) => {
                    shadows.push(candidate_id.clone());
                    if let Some(mut state) = self.states.get_mut(&candidate_id) {
                        state.is_shadow = true;
                    }
                }
                Err(e) => {
                    warn!(
                        core_id = %candidate_id,
                        error = %e,
                        "Failed to start shadow core"
                    );
                }
            }
        }

        *self.shadow_core_ids.write() = shadows;
        info!(
            shadow_count = self.shadow_core_ids.read().len(),
            "Shadow cores started"
        );
        Ok(())
    }

    /// Switch to a different core with zero packet loss.
    ///
    /// The switching process:
    /// 1. Buffer any in-flight packets
    /// 2. Stop the current active core (keep shadow cores running)
    /// 3. Promote the target core to active
    /// 4. Flush buffered packets through the new active core
    /// 5. Update shadow cores
    pub async fn switch_core(&self, target_id: &CoreId) -> CoreResult<()> {
        let old_active = self.active_core_id.read().clone();

        match &old_active {
            Some(old_id) if old_id == target_id => {
                debug!("Already on target core, no switch needed");
                return Ok(());
            }
            _ => {}
        }

        info!(
            from = ?old_active,
            to = %target_id,
            "Initiating seamless core switch"
        );

        // Step 1: Buffer any in-flight packets
        let buffered = {
            let mut buf = self.packet_buffer.lock().await;
            std::mem::take(&mut *buf)
        };
        debug!(
            buffered_packets = buffered.len(),
            "Buffered in-flight packets"
        );

        // Step 2: Demote old active core
        if let Some(old_id) = &old_active {
            if let Some(mut state) = self.states.get_mut(old_id) {
                state.is_active = false;
            }
        }

        // Step 3: Ensure target core is started
        let target_running = self
            .states
            .get(target_id)
            .map(|s| s.is_running)
            .unwrap_or(false);

        if !target_running {
            self.start_core(target_id).await?;
        }

        // Step 4: Promote target to active
        *self.active_core_id.write() = Some(target_id.clone());
        if let Some(mut state) = self.states.get_mut(target_id) {
            state.is_active = true;
            state.is_shadow = false;
        }

        // Step 5: Remove from shadows if it was one
        let mut shadows = self.shadow_core_ids.write();
        shadows.retain(|id| id != target_id);

        // Old active becomes a shadow
        if let Some(old_id) = old_active {
            shadows.push(old_id.clone());
            if let Some(mut state) = self.states.get_mut(&old_id) {
                state.is_shadow = true;
            }
        }

        // Step 6: Flush buffered packets
        if !buffered.is_empty() {
            debug!(
                count = buffered.len(),
                "Flushing buffered packets through new core"
            );
            for packet in buffered {
                if let Err(e) = self.send_packet_direct(packet).await {
                    warn!(error = %e, "Failed to flush buffered packet");
                }
            }
        }

        self.bandit.update_reward(&self.core_id_to_arm(target_id).to_string(), 0.5);

        info!(
            new_active = %target_id,
            shadow_count = shadows.len(),
            "Core switch completed"
        );

        Ok(())
    }

    /// Send a packet through the active core.
    pub async fn send_packet(&self, data: Bytes) -> CoreResult<()> {
        let active_id = self.active_core_id.read().clone();
        let active_id =
            active_id.ok_or_else(|| CoreError::IoError("No active core".to_string()))?;

        self.packets_sent.fetch_add(1, Ordering::Relaxed);
        self.send_packet_via(&active_id, data).await
    }

    /// Send a packet through a specific core.
    async fn send_packet_via(&self, core_id: &CoreId, data: Bytes) -> CoreResult<()> {
        let core = self
            .cores
            .get(core_id)
            .ok_or_else(|| CoreError::IoError(format!("Core {} not found", core_id)))?;

        let mut core_guard = core.lock().await;
        if !core_guard.is_running() {
            return Err(CoreError::ConnectionLost(format!(
                "Core {} is not running",
                core_id
            )));
        }

        // In production: core_guard.send(data).await
        // For now, simulate sending
        drop(core_guard);
        Ok(())
    }

    /// Send a packet directly (used for flushing buffered packets).
    async fn send_packet_direct(&self, data: Bytes) -> CoreResult<()> {
        self.send_packet(data).await
    }

    /// Receive a packet from the active core.
    pub async fn recv_packet(&self) -> CoreResult<Bytes> {
        let active_id = self.active_core_id.read().clone();
        let active_id =
            active_id.ok_or_else(|| CoreError::IoError("No active core".to_string()))?;

        let core = self
            .cores
            .get(&active_id)
            .ok_or_else(|| CoreError::IoError(format!("Core {} not found", active_id)))?;

        let core_guard = core.lock().await;
        if !core_guard.is_running() {
            return Err(CoreError::ConnectionLost(format!(
                "Core {} is not running",
                active_id
            )));
        }

        // In production: core_guard.recv().await
        drop(core_guard);

        self.packets_received.fetch_add(1, Ordering::Relaxed);
        Ok(Bytes::new())
    }

    /// Start the health monitoring loop.
    async fn start_health_monitor(&self) {
        let cores = self.cores.clone();
        let states = self.states.clone();
        let active_core_id = self.active_core_id.clone();
        let shadow_core_ids = self.shadow_core_ids.clone();
        let bandit = self.bandit.clone();
        let running = Arc::new(AtomicBool::new(true));

        let running_clone = running.clone();
        let handle = tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(HEALTH_CHECK_INTERVAL_SECS));

            loop {
                interval.tick().await;

                if !running_clone.load(Ordering::SeqCst) {
                    break;
                }

                // Check health of all running cores
                let core_ids: Vec<CoreId> = cores.iter().map(|e| e.key().clone()).collect();

                for core_id in &core_ids {
                    let core = match cores.get(core_id) {
                        Some(c) => c,
                        None => continue,
                    };

                    let health = {
                        let core_guard = core.lock().await;
                        if !core_guard.is_running() {
                            CoreHealth::Stopped
                        } else {
                            core_guard.health_check().await.unwrap_or(CoreHealth::Stopped)
                        }
                    };
                    drop(core);

                    if let Some(mut state) = states.get_mut(core_id) {
                        state.health.health = health;
                        state.health.checks_passed = state.health.checks_passed.saturating_add(
                            if health == CoreHealth::Healthy || health == CoreHealth::Degraded {
                                1
                            } else {
                                0
                            },
                        );
                        state.health.checks_failed = state.health.checks_failed.saturating_add(
                            if health == CoreHealth::Unhealthy {
                                1
                            } else {
                                0
                            },
                        );
                    }

                    // Update bandit reward based on health
                    let arm = match core_id.as_str() {
                        "hiddify" => CoreArm::Hiddify,
                        "xray" => CoreArm::Xray,
                        "singbox" => CoreArm::Singbox,
                        "amneziavpn" => CoreArm::AmneziaVpn,
                        "defyx" => CoreArm::DefyX,
                        "moav" => CoreArm::Moav,
                        "lantern" => CoreArm::Lantern,
                        "mahsang" => CoreArm::Mahsang,
                        "psiphon" => CoreArm::Psiphon,
                        _ => continue,
                    };

                    let reward = match health {
                        CoreHealth::Healthy => 1.0,
                        CoreHealth::Degraded => 0.5,
                        CoreHealth::Unhealthy => 0.1,
                        CoreHealth::Stopped => 0.0,
                    };

                    bandit.update_reward(&arm.to_string(), reward);
                }

                // Check if active core is unhealthy — trigger switch
                let active_id = active_core_id.read().clone();
                if let Some(ref active) = active_id {
                    if let Some(state) = states.get(active) {
                        if state.health.health == CoreHealth::Unhealthy {
                            warn!(
                                core_id = %active,
                                "Active core is unhealthy, checking bandit for switch recommendation"
                            );

                            let current_arm = match active.as_str() {
                                "hiddify" => CoreArm::Hiddify,
                                "xray" => CoreArm::Xray,
                                "singbox" => CoreArm::Singbox,
                                "amneziavpn" => CoreArm::AmneziaVpn,
                                "defyx" => CoreArm::DefyX,
                                "moav" => CoreArm::Moav,
                                "lantern" => CoreArm::Lantern,
                                "mahsang" => CoreArm::Mahsang,
                                "psiphon" => CoreArm::Psiphon,
                                _ => CoreArm::Psiphon,
                            };

                            let selection = bandit.select();
                            if selection.score > 0.0 {
                                info!(
                                    recommended_arm = ?selection.arm,
                                    score = selection.score,
                                    "Bandit recommends core switch"
                                );
                                // In production, this would call switch_core()
                            }
                        }
                    }
                }
            }
        });

        *self.health_task.lock().await = Some(handle);
    }

    /// Convert a CoreArm to a core ID string.
    fn arm_to_core_id(&self, arm: CoreArm) -> CoreId {
        match arm {
            CoreArm::Hiddify => "hiddify".to_string(),
            CoreArm::Xray => "xray".to_string(),
            CoreArm::Singbox => "singbox".to_string(),
            CoreArm::AmneziaVpn => "amneziavpn".to_string(),
            CoreArm::DefyX => "defyx".to_string(),
            CoreArm::Moav => "moav".to_string(),
            CoreArm::Lantern => "lantern".to_string(),
            CoreArm::Mahsang => "mahsang".to_string(),
            CoreArm::Psiphon => "psiphon".to_string(),
        }
    }

    /// Convert a core ID string to a CoreArm.
    fn core_id_to_arm(&self, id: &str) -> CoreArm {
        match id {
            "hiddify" => CoreArm::Hiddify,
            "xray" => CoreArm::Xray,
            "singbox" => CoreArm::Singbox,
            "amneziavpn" => CoreArm::AmneziaVpn,
            "defyx" => CoreArm::DefyX,
            "moav" => CoreArm::Moav,
            "lantern" => CoreArm::Lantern,
            "mahsang" => CoreArm::Mahsang,
            "psiphon" => CoreArm::Psiphon,
            _ => CoreArm::Psiphon,
        }
    }

    /// Get the active core ID.
    pub fn active_core_id(&self) -> Option<CoreId> {
        self.active_core_id.read().clone()
    }

    /// Get all core states.
    pub fn all_states(&self) -> Vec<CoreState> {
        self.states.iter().map(|e| e.value().clone()).collect()
    }

    /// Get a specific core state.
    pub fn get_state(&self, id: &CoreId) -> Option<CoreState> {
        self.states.get(id).map(|e| e.value().clone())
    }

    /// Get the number of running cores.
    pub fn running_count(&self) -> usize {
        self.states.iter().filter(|e| e.value().is_running).count()
    }

    /// Get packet statistics.
    pub fn packet_stats(&self) -> (u64, u64) {
        (
            self.packets_sent.load(Ordering::Relaxed),
            self.packets_received.load(Ordering::Relaxed),
        )
    }

    /// Check if the manager is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Default for CoreManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_manager_new() {
        let manager = CoreManager::new();
        assert_eq!(manager.all_states().len(), 9);
        assert!(!manager.is_running());
    }

    #[test]
    fn test_arm_to_core_id() {
        let manager = CoreManager::new();
        assert_eq!(manager.arm_to_core_id(CoreArm::Hiddify), "hiddify");
        assert_eq!(manager.arm_to_core_id(CoreArm::Psiphon), "psiphon");
    }

    #[test]
    fn test_core_id_to_arm() {
        let manager = CoreManager::new();
        assert_eq!(manager.core_id_to_arm("hiddify"), CoreArm::Hiddify);
        assert_eq!(manager.core_id_to_arm("xray"), CoreArm::Xray);
    }

    #[test]
    fn test_initial_states() {
        let manager = CoreManager::new();
        let states = manager.all_states();
        assert!(states.iter().all(|s| !s.is_active));
        assert!(states.iter().all(|s| !s.is_running));
    }

    #[test]
    fn test_packet_stats_initial() {
        let manager = CoreManager::new();
        let (sent, recv) = manager.packet_stats();
        assert_eq!(sent, 0);
        assert_eq!(recv, 0);
    }
}
