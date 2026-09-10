//! Battery Optimizer for MICAFP-UnifiedShield
//!
//! This module implements an advanced adaptive battery optimization system that
//! dynamically adjusts the daemon's behavior based on the device's power state,
//! battery level, and user preferences. The goal is to maximize the VPN daemon's
//! uptime while minimizing battery drain.
//!
//! # Adaptive Strategy
//!
//! The optimizer uses a state machine that transitions between power states:
//!
//! ```text
//! ┌──────────┐    screen off    ┌─────────────────┐    5 min idle    ┌─────────────────┐
//! │ ScreenOn │ ───────────────▶ │ ScreenOffLight  │ ───────────────▶ │ ScreenOffDeep   │
//! └──────────┘                  └─────────────────┘                  └─────────────────┘
//!       ▲                              ▲                                     │
//!       │         screen on            │          screen on                  │
//!       └──────────────────────────────┴─────────────────────────────────────┘
//!                                       ▲                                     │
//!                                       │         charger connected           │
//!                                ┌──────────┐                                │
//!                                │ Charging │ ◀────────────────────────────────┘
//!                                └──────────┘
//! ```
//!
//! # Ultra-Low-Power Mode
//!
//! When battery drops below 15%, the optimizer enters ultra-low-power mode:
//! - Only the VPN tunnel is kept alive
//! - All scanning and probing is disabled
//! - NAIN detection is paused
//! - P2P relay is disabled

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::platform::PlatformContext;
use crate::{
    BATTERY_CRITICAL_THRESHOLD, NAIN_PROBE_INTERVAL_SCREEN_OFF_DEEP_SECS,
    NAIN_PROBE_INTERVAL_SCREEN_OFF_LIGHT_SECS, NAIN_PROBE_INTERVAL_SCREEN_ON_SECS,
};

// ─── Power State ────────────────────────────────────────────────────────────

/// Device power state that determines the daemon's activity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerState {
    /// Screen is on — all services active.
    ScreenOn,
    /// Screen is off but device is lightly sleeping — reduced activity.
    ScreenOffLight,
    /// Screen is off and device is in deep sleep — minimal activity.
    ScreenOffDeep,
    /// Device is charging — maximum capacity allowed.
    Charging,
}

impl PowerState {
    /// Get the NAIN probe interval for this power state.
    pub fn nain_probe_interval(&self) -> Duration {
        match self {
            PowerState::ScreenOn => Duration::from_secs(NAIN_PROBE_INTERVAL_SCREEN_ON_SECS),
            PowerState::ScreenOffLight => {
                Duration::from_secs(NAIN_PROBE_INTERVAL_SCREEN_OFF_LIGHT_SECS)
            }
            PowerState::ScreenOffDeep => {
                Duration::from_secs(NAIN_PROBE_INTERVAL_SCREEN_OFF_DEEP_SECS)
            }
            PowerState::Charging => Duration::from_secs(NAIN_PROBE_INTERVAL_SCREEN_ON_SECS),
        }
    }

    /// Whether acoustic listening (ultrasonic/multicast) is active.
    pub fn acoustic_listening_active(&self) -> bool {
        matches!(self, PowerState::ScreenOn | PowerState::Charging)
    }

    /// Whether WiFi Aware scanning is active.
    pub fn wifi_aware_active(&self) -> bool {
        matches!(self, PowerState::ScreenOn | PowerState::Charging)
    }

    /// Whether P2P relay functionality is active.
    pub fn p2p_relay_active(&self) -> bool {
        matches!(
            self,
            PowerState::ScreenOn | PowerState::ScreenOffLight | PowerState::Charging
        )
    }

    /// Whether AI inference is active.
    pub fn ai_inference_active(&self) -> bool {
        matches!(self, PowerState::ScreenOn | PowerState::Charging)
    }

    /// Get the WiFi Aware scan interval for this power state.
    pub fn wifi_aware_scan_interval(&self) -> Duration {
        match self {
            PowerState::ScreenOn => Duration::from_secs(30),
            PowerState::ScreenOffLight => Duration::from_secs(300), // 5 min
            PowerState::ScreenOffDeep => Duration::from_secs(0),    // Disabled
            PowerState::Charging => Duration::from_secs(15),
        }
    }

    /// Whether the VPN tunnel should be kept alive (always true).
    pub fn vpn_tunnel_active(&self) -> bool {
        true // VPN tunnel is always active regardless of power state
    }

    /// Get a human-readable description of this power state.
    pub fn description(&self) -> &str {
        match self {
            PowerState::ScreenOn => "Screen On — All services active",
            PowerState::ScreenOffLight => "Screen Off (Light) — Reduced scanning",
            PowerState::ScreenOffDeep => "Screen Off (Deep) — VPN tunnel only",
            PowerState::Charging => "Charging — Maximum capacity",
        }
    }
}

impl std::fmt::Display for PowerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowerState::ScreenOn => write!(f, "screen_on"),
            PowerState::ScreenOffLight => write!(f, "screen_off_light"),
            PowerState::ScreenOffDeep => write!(f, "screen_off_deep"),
            PowerState::Charging => write!(f, "charging"),
        }
    }
}

// ─── Battery Strategy ───────────────────────────────────────────────────────

/// User-configurable battery optimization strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatteryStrategy {
    /// Automatically adapt based on battery level and power state.
    Auto,
    /// Maximize performance regardless of battery impact.
    Performance,
    /// Conserve battery whenever possible.
    PowerSave,
    /// Ultra-low-power: VPN tunnel only, no scanning.
    UltraLow,
}

impl std::fmt::Display for BatteryStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatteryStrategy::Auto => write!(f, "auto"),
            BatteryStrategy::Performance => write!(f, "performance"),
            BatteryStrategy::PowerSave => write!(f, "powersave"),
            BatteryStrategy::UltraLow => write!(f, "ultra_low"),
        }
    }
}

// ─── Adaptive Strategy ──────────────────────────────────────────────────────

/// Adaptive strategy configuration that determines behavior for each power state.
///
/// This struct encapsulates all the tunable parameters for each power state,
/// allowing fine-grained control over the daemon's behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveStrategy {
    /// The NAIN probe interval in seconds for each power state.
    pub nain_probe_intervals: [u64; 4], // [ScreenOn, ScreenOffLight, ScreenOffDeep, Charging]
    /// Whether acoustic listening is enabled for each power state.
    pub acoustic_enabled: [bool; 4],
    /// Whether WiFi Aware scanning is enabled for each power state.
    pub wifi_aware_enabled: [bool; 4],
    /// WiFi Aware scan interval in seconds for each power state.
    pub wifi_aware_intervals: [u64; 4],
    /// Whether P2P relay is enabled for each power state.
    pub p2p_relay_enabled: [bool; 4],
    /// Whether AI inference is enabled for each power state.
    pub ai_enabled: [bool; 4],
    /// Battery percentage threshold for ultra-low-power mode.
    pub critical_battery_threshold: u8,
    /// Maximum consecutive failed probes before exponential backoff kicks in.
    pub backoff_trigger_count: u8,
    /// Maximum backoff multiplier (e.g., 8 means max 8x the base interval).
    pub max_backoff_multiplier: u32,
}

impl Default for AdaptiveStrategy {
    fn default() -> Self {
        Self {
            nain_probe_intervals: [
                NAIN_PROBE_INTERVAL_SCREEN_ON_SECS,
                NAIN_PROBE_INTERVAL_SCREEN_OFF_LIGHT_SECS,
                NAIN_PROBE_INTERVAL_SCREEN_OFF_DEEP_SECS,
                NAIN_PROBE_INTERVAL_SCREEN_ON_SECS, // Charging = ScreenOn
            ],
            acoustic_enabled: [true, false, false, true],
            wifi_aware_enabled: [true, true, false, true],
            wifi_aware_intervals: [30, 300, 0, 15],
            p2p_relay_enabled: [true, true, false, true],
            ai_enabled: [true, false, false, true],
            critical_battery_threshold: BATTERY_CRITICAL_THRESHOLD,
            backoff_trigger_count: 3,
            max_backoff_multiplier: 8,
        }
    }
}

impl AdaptiveStrategy {
    /// Create a performance-oriented strategy (all features always on).
    pub fn performance() -> Self {
        Self {
            nain_probe_intervals: [30, 30, 30, 15],
            acoustic_enabled: [true, true, true, true],
            wifi_aware_enabled: [true, true, true, true],
            wifi_aware_intervals: [30, 30, 30, 15],
            p2p_relay_enabled: [true, true, true, true],
            ai_enabled: [true, true, true, true],
            critical_battery_threshold: 5,
            backoff_trigger_count: 10,
            max_backoff_multiplier: 2,
        }
    }

    /// Create a power-saving strategy (aggressive battery conservation).
    pub fn power_save() -> Self {
        Self {
            nain_probe_intervals: [60, 180, 600, 30],
            acoustic_enabled: [true, false, false, true],
            wifi_aware_enabled: [true, false, false, true],
            wifi_aware_intervals: [60, 0, 0, 30],
            p2p_relay_enabled: [true, false, false, true],
            ai_enabled: [false, false, false, true],
            critical_battery_threshold: 25,
            backoff_trigger_count: 2,
            max_backoff_multiplier: 16,
        }
    }

    /// Create an ultra-low-power strategy (VPN only, no scanning).
    pub fn ultra_low() -> Self {
        Self {
            nain_probe_intervals: [300, 600, 900, 120],
            acoustic_enabled: [false, false, false, false],
            wifi_aware_enabled: [false, false, false, false],
            wifi_aware_intervals: [0, 0, 0, 0],
            p2p_relay_enabled: [false, false, false, false],
            ai_enabled: [false, false, false, false],
            critical_battery_threshold: 50,
            backoff_trigger_count: 1,
            max_backoff_multiplier: 32,
        }
    }

    /// Get the power state index for array lookups.
    fn state_index(state: PowerState) -> usize {
        match state {
            PowerState::ScreenOn => 0,
            PowerState::ScreenOffLight => 1,
            PowerState::ScreenOffDeep => 2,
            PowerState::Charging => 3,
        }
    }

    /// Get the NAIN probe interval for a given power state.
    pub fn nain_probe_interval(&self, state: PowerState) -> Duration {
        let idx = Self::state_index(state);
        Duration::from_secs(self.nain_probe_intervals[idx])
    }

    /// Whether acoustic listening is enabled for a given power state.
    pub fn acoustic_enabled(&self, state: PowerState) -> bool {
        self.acoustic_enabled[Self::state_index(state)]
    }

    /// Whether WiFi Aware scanning is enabled for a given power state.
    pub fn wifi_aware_enabled(&self, state: PowerState) -> bool {
        self.wifi_aware_enabled[Self::state_index(state)]
    }

    /// Whether P2P relay is enabled for a given power state.
    pub fn p2p_relay_enabled(&self, state: PowerState) -> bool {
        self.p2p_relay_enabled[Self::state_index(state)]
    }

    /// Whether AI inference is enabled for a given power state.
    pub fn ai_enabled(&self, state: PowerState) -> bool {
        self.ai_enabled[Self::state_index(state)]
    }
}

// ─── Battery Optimizer ──────────────────────────────────────────────────────

/// The main battery optimizer that manages power state transitions and
/// provides adaptive strategy configuration to all daemon subsystems.
///
/// # Architecture
///
/// ```text
/// ┌──────────────────────────────────────────────┐
/// │              BatteryOptimizer                │
/// │                                              │
/// │  ┌──────────┐  ┌────────────┐  ┌─────────┐ │
/// │  │ Platform │  │  Adaptive  │  │ Expon.  │ │
/// │  │  Bridge  │  │  Strategy  │  │ Backoff │ │
/// │  │ (JNI/    │  │            │  │ Engine  │ │
/// │  │  Swift)  │  │            │  │         │ │
/// │  └──────────┘  └────────────┘  └─────────┘ │
/// │         │              │             │       │
/// │         ▼              ▼             ▼       │
/// │  ┌──────────────────────────────────────────┐│
/// │  │         Power State Machine              ││
/// │  │  [ScreenOn] → [ScreenOffLight] → [Deep] ││
/// │  │       ↑              ↑          ↑       ││
/// │  │       └──── [Charging] ─────────┘       ││
/// │  └──────────────────────────────────────────┘│
/// └──────────────────────────────────────────────┘
/// ```
pub struct BatteryOptimizer {
    /// The user-configured battery strategy.
    strategy: RwLock<BatteryStrategy>,
    /// The adaptive strategy configuration.
    adaptive: RwLock<AdaptiveStrategy>,
    /// Current power state.
    current_state: RwLock<PowerState>,
    /// Current battery percentage (0-100).
    battery_percent: AtomicU8,
    /// Whether the device is charging.
    is_charging: AtomicBool,
    /// Whether the screen is on.
    screen_on: AtomicBool,
    /// Whether we're in ultra-low-power mode.
    ultra_low_power: AtomicBool,
    /// Consecutive failed probe counter for exponential backoff.
    failed_probe_count: AtomicU8,
    /// Time of the last wake lock acquisition.
    last_wake_lock: RwLock<Option<Instant>>,
    /// Platform context for platform-specific operations.
    platform_ctx: Arc<PlatformContext>,
    /// Timestamp of last power state transition.
    last_state_change: RwLock<Instant>,
}

impl BatteryOptimizer {
    /// Create a new battery optimizer with the given strategy and platform context.
    pub fn new(strategy: BatteryStrategy, platform_ctx: Arc<PlatformContext>) -> Self {
        let adaptive = match strategy {
            BatteryStrategy::Auto => AdaptiveStrategy::default(),
            BatteryStrategy::Performance => AdaptiveStrategy::performance(),
            BatteryStrategy::PowerSave => AdaptiveStrategy::power_save(),
            BatteryStrategy::UltraLow => AdaptiveStrategy::ultra_low(),
        };

        Self {
            strategy: RwLock::new(strategy),
            adaptive: RwLock::new(adaptive),
            current_state: RwLock::new(PowerState::ScreenOn),
            battery_percent: AtomicU8::new(100),
            is_charging: AtomicBool::new(false),
            screen_on: AtomicBool::new(true),
            ultra_low_power: AtomicBool::new(false),
            failed_probe_count: AtomicU8::new(0),
            last_wake_lock: RwLock::new(None),
            platform_ctx,
            last_state_change: RwLock::new(Instant::now()),
        }
    }

    /// Start the battery monitoring loop.
    ///
    /// This runs in the background and:
    /// - Periodically queries the platform for battery level and charging state
    /// - Monitors screen on/off transitions
    /// - Updates the power state machine
    /// - Triggers ultra-low-power mode when battery is critically low
    pub async fn start_monitoring(&self) -> Result<()> {
        info!(
            "Battery optimizer monitoring started (strategy: {})",
            self.get_strategy()
        );

        let mut monitor_interval = tokio::time::interval(Duration::from_secs(30));
        let mut deep_sleep_timer = Instant::now();
        let deep_sleep_threshold = Duration::from_secs(300); // 5 min to deep sleep

        loop {
            monitor_interval.tick().await;

            // Query platform for current battery state
            let (battery_pct, charging, screen) = self.query_platform_state().await;

            // Update atomic state
            let prev_battery = self.battery_percent.swap(battery_pct, Ordering::SeqCst);
            self.is_charging.store(charging, Ordering::SeqCst);

            let screen_changed = self.screen_on.swap(screen, Ordering::SeqCst) != screen;

            // Log significant changes
            if prev_battery != battery_pct && battery_pct % 10 == 0 {
                debug!("Battery: {}%, charging: {}", battery_pct, charging);
            }

            // Check for ultra-low-power mode trigger
            let ultra_low = battery_pct <= BATTERY_CRITICAL_THRESHOLD && !charging;
            let was_ultra_low = self.ultra_low_power.swap(ultra_low, Ordering::SeqCst);

            if ultra_low && !was_ultra_low {
                warn!(
                    "Battery critically low ({}%), entering ultra-low-power mode",
                    battery_pct
                );
            } else if !ultra_low && was_ultra_low {
                info!(
                    "Battery recovered ({}%), exiting ultra-low-power mode",
                    battery_pct
                );
            }

            // Update power state machine
            if ultra_low {
                // Ultra-low-power overrides everything
                self.update_power_state(PowerState::ScreenOffDeep);
            } else if charging {
                self.update_power_state(PowerState::Charging);
            } else if screen {
                self.update_power_state(PowerState::ScreenOn);
                deep_sleep_timer = Instant::now(); // Reset deep sleep timer
            } else {
                // Screen is off — decide between light and deep sleep
                let time_since_screen_off = deep_sleep_timer.elapsed();
                if time_since_screen_off > deep_sleep_threshold {
                    self.update_power_state(PowerState::ScreenOffDeep);
                } else {
                    self.update_power_state(PowerState::ScreenOffLight);
                }
            }

            // Handle screen state changes
            if screen_changed {
                if screen {
                    info!("Screen turned on — activating all services");
                } else {
                    info!("Screen turned off — reducing activity");
                    deep_sleep_timer = Instant::now();
                }
            }
        }
    }

    /// Update the power state and log transitions.
    fn update_power_state(&self, new_state: PowerState) {
        let mut current = self.current_state.write();
        if *current != new_state {
            let old_state = *current;
            *current = new_state;
            *self.last_state_change.write() = Instant::now();
            info!("Power state transition: {} → {}", old_state, new_state);
        }
    }

    /// Query the platform for current battery and screen state.
    ///
    /// On Android, this uses JNI to call:
    /// ```java
    /// BatteryManager.getIntProperty(BatteryManager.BATTERY_PROPERTY_CAPACITY);
    /// PowerManager.isScreenOn();
    /// ```
    ///
    /// On iOS, this uses Swift bridge to call:
    /// ```swift
    /// UIDevice.current.batteryLevel
    /// UIApplication.shared.applicationState != .background
    /// ```
    async fn query_platform_state(&self) -> (u8, bool, bool) {
        // In production, this would call platform-specific bridges.
        // For now, return simulated values that can be overridden by tests.

        #[cfg(target_os = "android")]
        {
            // JNI call to Android BatteryManager and PowerManager
            // let env = self.platform_ctx.jni_env();
            // let battery = env.call_method(battery_manager, "getIntProperty", ...);
            // let charging = env.call_method(battery_manager, "isCharging", ...);
            // let screen = env.call_method(power_manager, "isScreenOn", ...);
        }

        #[cfg(target_os = "ios")]
        {
            // Swift bridge call to UIDevice and UIApplication
            // let battery = swift_bridge::get_battery_level();
            // let charging = swift_bridge::is_charging();
            // let screen = swift_bridge::is_screen_on();
        }

        // Default: return current atomic state (simulated)
        let battery = self.battery_percent.load(Ordering::SeqCst);
        let charging = self.is_charging.load(Ordering::SeqCst);
        let screen = self.screen_on.load(Ordering::SeqCst);

        (battery, charging, screen)
    }

    // ─── Public API for Subsystems ───────────────────────────────────────

    /// Get the current power state.
    pub fn current_power_state(&self) -> PowerState {
        *self.current_state.read()
    }

    /// Get the current battery percentage.
    pub fn battery_percent(&self) -> u8 {
        self.battery_percent.load(Ordering::SeqCst)
    }

    /// Whether the device is currently charging.
    pub fn is_charging(&self) -> bool {
        self.is_charging.load(Ordering::SeqCst)
    }

    /// Whether the screen is currently on.
    pub fn is_screen_on(&self) -> bool {
        self.screen_on.load(Ordering::SeqCst)
    }

    /// Whether ultra-low-power mode is active.
    pub fn is_ultra_low_power(&self) -> bool {
        self.ultra_low_power.load(Ordering::SeqCst)
    }

    /// Get the current battery strategy.
    pub fn get_strategy(&self) -> BatteryStrategy {
        *self.strategy.read()
    }

    /// Set the battery strategy (allows runtime reconfiguration).
    pub fn set_strategy(&self, strategy: BatteryStrategy) {
        let new_adaptive = match strategy {
            BatteryStrategy::Auto => AdaptiveStrategy::default(),
            BatteryStrategy::Performance => AdaptiveStrategy::performance(),
            BatteryStrategy::PowerSave => AdaptiveStrategy::power_save(),
            BatteryStrategy::UltraLow => AdaptiveStrategy::ultra_low(),
        };

        *self.strategy.write() = strategy;
        *self.adaptive.write() = new_adaptive;
        info!("Battery strategy changed to: {}", strategy);
    }

    /// Get the NAIN probe interval for the current power state,
    /// including exponential backoff adjustment.
    ///
    /// The backoff is applied when consecutive probes fail:
    /// - After `backoff_trigger_count` failures: interval × 2
    /// - After more failures: interval × 4, × 8, up to max_backoff_multiplier
    /// - Reset to base interval on first successful probe
    pub fn nain_probe_interval(&self) -> Duration {
        let state = self.current_power_state();
        let adaptive = self.adaptive.read();
        let base_interval = adaptive.nain_probe_interval(state);

        let failed = self.failed_probe_count.load(Ordering::SeqCst);
        if failed <= adaptive.backoff_trigger_count {
            return base_interval;
        }

        // Calculate backoff multiplier
        let excess_failures = failed - adaptive.backoff_trigger_count;
        let backoff_power = std::cmp::min(excess_failures as u32, 8); // Max 2^8 = 256x
        let multiplier = std::cmp::min(
            1u64 << backoff_power,
            adaptive.max_backoff_multiplier as u64,
        );

        let backed_off = base_interval.as_secs().saturating_mul(multiplier);
        Duration::from_secs(std::cmp::min(backed_off, 3600)) // Cap at 1 hour
    }

    /// Record a successful NAIN probe (resets backoff).
    pub fn record_probe_success(&self) {
        self.failed_probe_count.store(0, Ordering::SeqCst);
    }

    /// Record a failed NAIN probe (increases backoff).
    pub fn record_probe_failure(&self) {
        let prev = self.failed_probe_count.fetch_add(1, Ordering::SeqCst);
        if prev > 0 && prev % 5 == 0 {
            warn!("NAIN probe failures: {} (backoff active)", prev + 1);
        }
    }

    /// Whether acoustic listening is currently enabled.
    pub fn acoustic_listening_enabled(&self) -> bool {
        let state = self.current_power_state();
        if self.ultra_low_power.load(Ordering::SeqCst) {
            return false;
        }
        self.adaptive.read().acoustic_enabled(state)
    }

    /// Whether WiFi Aware scanning is currently enabled.
    pub fn wifi_aware_enabled(&self) -> bool {
        let state = self.current_power_state();
        if self.ultra_low_power.load(Ordering::SeqCst) {
            return false;
        }
        self.adaptive.read().wifi_aware_enabled(state)
    }

    /// Whether P2P relay is currently enabled.
    pub fn p2p_relay_enabled(&self) -> bool {
        let state = self.current_power_state();
        if self.ultra_low_power.load(Ordering::SeqCst) {
            return false;
        }
        self.adaptive.read().p2p_relay_enabled(state)
    }

    /// Whether AI inference is currently enabled.
    pub fn ai_inference_enabled(&self) -> bool {
        let state = self.current_power_state();
        if self.ultra_low_power.load(Ordering::SeqCst) {
            return false;
        }
        self.adaptive.read().ai_enabled(state)
    }

    /// Get the WiFi Aware scan interval for the current power state.
    pub fn wifi_aware_scan_interval(&self) -> Duration {
        let state = self.current_power_state();
        let adaptive = self.adaptive.read();
        let idx = AdaptiveStrategy::state_index(state);
        Duration::from_secs(adaptive.wifi_aware_intervals[idx])
    }

    /// Acquire a coalesced wake lock.
    ///
    /// Instead of each subsystem acquiring its own wake lock (which drains
    /// battery), the optimizer batches all wake operations into a single
    /// wake window. This method records the wake request and returns the
    /// time until the next wake window.
    ///
    /// # Android Implementation
    ///
    /// Uses a single `PartialWakeLock` with a timeout:
    /// ```java
    /// wakeLock = powerManager.newWakeLock(
    ///     PowerManager.PARTIAL_WAKE_LOCK, "micafp:coalesced"
    /// );
    /// wakeLock.acquire(WAKE_WINDOW_MS);
    /// ```
    ///
    /// # iOS Implementation
    ///
    /// Uses `BGProcessingTask` with earliest begin date:
    /// ```swift
    /// let request = BGProcessingTaskRequest(identifier: "com.micafp.coalesced")
    /// request.earliestBeginDate = Date(timeIntervalSinceNow: nextWindow)
    /// request.requiresNetworkConnectivity = true
    /// request.requiresExternalPower = false
    /// try BGTaskScheduler.shared.submit(request)
    /// ```
    pub async fn acquire_wake_lock(&self, reason: &str) -> Result<Duration> {
        let now = Instant::now();
        let mut last = self.last_wake_lock.write();

        // Coalesce: if a wake lock was recently acquired, reuse its window
        if let Some(last_time) = *last {
            let elapsed = now.duration_since(last_time);
            let wake_window = Duration::from_secs(10); // 10-second coalesce window

            if elapsed < wake_window {
                let remaining = wake_window - elapsed;
                debug!(
                    "Wake lock coalesced for '{}' ({}ms remaining in window)",
                    reason,
                    remaining.as_millis()
                );
                return Ok(remaining);
            }
        }

        // Acquire a new wake lock
        *last = Some(now);

        #[cfg(target_os = "android")]
        {
            // JNI call to acquire Android wake lock
            // let env = self.platform_ctx.jni_env();
            // env.call_method(wake_lock, "acquire", "(J)V", &[J::from(10_000)]);
            debug!("Android: acquired partial wake lock for '{}'", reason);
        }

        #[cfg(target_os = "ios")]
        {
            // Swift bridge to schedule BGProcessingTask
            debug!("iOS: scheduled BGProcessingTask for '{}'", reason);
        }

        debug!("Wake lock acquired for '{}' (10s window)", reason);
        Ok(Duration::from_secs(10))
    }

    /// Release the coalesced wake lock.
    pub fn release_wake_lock(&self) {
        *self.last_wake_lock.write() = None;

        #[cfg(target_os = "android")]
        {
            debug!("Android: released partial wake lock");
        }

        #[cfg(target_os = "ios")]
        {
            debug!("iOS: BGProcessingTask completed");
        }
    }

    /// Set the battery percentage (for testing or Flutter bridge override).
    pub fn set_battery_percent(&self, percent: u8) {
        self.battery_percent.store(percent, Ordering::SeqCst);
    }

    /// Set the charging state (for testing or Flutter bridge override).
    pub fn set_charging(&self, charging: bool) {
        self.is_charging.store(charging, Ordering::SeqCst);
    }

    /// Set the screen state (for testing or Flutter bridge override).
    pub fn set_screen_on(&self, screen_on: bool) {
        self.screen_on.store(screen_on, Ordering::SeqCst);
    }

    /// Get a comprehensive status report for the Flutter bridge.
    pub fn status_report(&self) -> BatteryStatusReport {
        BatteryStatusReport {
            strategy: self.get_strategy(),
            power_state: self.current_power_state(),
            battery_percent: self.battery_percent(),
            is_charging: self.is_charging(),
            is_screen_on: self.is_screen_on(),
            is_ultra_low_power: self.is_ultra_low_power(),
            nain_probe_interval_secs: self.nain_probe_interval().as_secs(),
            acoustic_enabled: self.acoustic_listening_enabled(),
            wifi_aware_enabled: self.wifi_aware_enabled(),
            p2p_relay_enabled: self.p2p_relay_enabled(),
            ai_enabled: self.ai_inference_enabled(),
            failed_probe_count: self.failed_probe_count.load(Ordering::SeqCst),
        }
    }

    /// Schedule periodic work using platform-specific APIs.
    ///
    /// - **Android**: Uses WorkManager for periodic tasks with constraints
    ///   (battery not low, device charging, etc.)
    /// - **iOS**: Uses BGTaskScheduler for periodic background tasks
    pub fn schedule_periodic_work(&self, task_id: &str, interval: Duration) -> Result<()> {
        #[cfg(target_os = "android")]
        {
            info!(
                "Android: scheduling WorkManager periodic task '{}' every {}s",
                task_id,
                interval.as_secs()
            );
            // In production, this would use JNI to schedule:
            // val request = PeriodicWorkRequestBuilder<MicafpWorker>(interval, TimeUnit.SECONDS)
            //     .setConstraints(Constraints.Builder()
            //         .setRequiresBatteryNotLow(true)
            //         .build())
            //     .build()
            // WorkManager.getInstance(context).enqueueUniquePeriodicWork(
            //     taskId, ExistingPeriodicWorkPolicy.KEEP, request)
        }

        #[cfg(target_os = "ios")]
        {
            info!(
                "iOS: scheduling BGAppRefreshTask '{}' every {}s",
                task_id,
                interval.as_secs()
            );
            // In production, this would use Swift bridge:
            // let request = BGAppRefreshTaskRequest(identifier: taskId)
            // request.earliestBeginDate = Date(timeIntervalSinceNow: interval)
            // try BGTaskScheduler.shared.submit(request)
        }

        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            debug!(
                "Desktop: periodic task '{}' scheduled (every {}s) — no platform API needed",
                task_id,
                interval.as_secs()
            );
        }

        Ok(())
    }

    /// Request background processing on iOS when the device is charging.
    ///
    /// This uses BGProcessingTask which allows more CPU time when charging.
    pub fn request_background_processing(&self) -> Result<()> {
        #[cfg(target_os = "ios")]
        {
            info!("iOS: requesting BGProcessingTask (requires external power)");
            // let request = BGProcessingTaskRequest(identifier: "com.micafp.bg-processing")
            // request.requiresExternalPower = true
            // request.requiresNetworkConnectivity = true
            // request.earliestBeginDate = Date(timeIntervalSinceNow: 0)
            // try BGTaskScheduler.shared.submit(request)
        }

        Ok(())
    }
}

// ─── Status Report ──────────────────────────────────────────────────────────

/// Comprehensive battery and power state status report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryStatusReport {
    /// Current battery strategy.
    pub strategy: BatteryStrategy,
    /// Current power state.
    pub power_state: PowerState,
    /// Current battery percentage (0-100).
    pub battery_percent: u8,
    /// Whether the device is charging.
    pub is_charging: bool,
    /// Whether the screen is on.
    pub is_screen_on: bool,
    /// Whether ultra-low-power mode is active.
    pub is_ultra_low_power: bool,
    /// Current NAIN probe interval in seconds (including backoff).
    pub nain_probe_interval_secs: u64,
    /// Whether acoustic listening is enabled.
    pub acoustic_enabled: bool,
    /// Whether WiFi Aware scanning is enabled.
    pub wifi_aware_enabled: bool,
    /// Whether P2P relay is enabled.
    pub p2p_relay_enabled: bool,
    /// Whether AI inference is enabled.
    pub ai_enabled: bool,
    /// Number of consecutive failed probes (for backoff calculation).
    pub failed_probe_count: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_state_nain_intervals() {
        assert_eq!(
            PowerState::ScreenOn.nain_probe_interval(),
            Duration::from_secs(30)
        );
        assert_eq!(
            PowerState::ScreenOffLight.nain_probe_interval(),
            Duration::from_secs(120)
        );
        assert_eq!(
            PowerState::ScreenOffDeep.nain_probe_interval(),
            Duration::from_secs(300)
        );
        assert_eq!(
            PowerState::Charging.nain_probe_interval(),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn test_adaptive_strategy_default() {
        let strategy = AdaptiveStrategy::default();
        assert!(strategy.acoustic_enabled(PowerState::ScreenOn));
        assert!(!strategy.acoustic_enabled(PowerState::ScreenOffLight));
        assert!(strategy.p2p_relay_enabled(PowerState::ScreenOn));
        assert!(!strategy.p2p_relay_enabled(PowerState::ScreenOffDeep));
    }

    #[test]
    fn test_adaptive_strategy_performance() {
        let strategy = AdaptiveStrategy::performance();
        assert!(strategy.acoustic_enabled(PowerState::ScreenOffDeep));
        assert!(strategy.wifi_aware_enabled(PowerState::ScreenOffDeep));
        assert!(strategy.ai_enabled(PowerState::ScreenOffDeep));
    }

    #[test]
    fn test_adaptive_strategy_ultra_low() {
        let strategy = AdaptiveStrategy::ultra_low();
        assert!(!strategy.acoustic_enabled(PowerState::ScreenOn));
        assert!(!strategy.wifi_aware_enabled(PowerState::Charging));
        assert!(!strategy.p2p_relay_enabled(PowerState::ScreenOn));
    }

    #[test]
    fn test_battery_strategy_display() {
        assert_eq!(BatteryStrategy::Auto.to_string(), "auto");
        assert_eq!(BatteryStrategy::Performance.to_string(), "performance");
        assert_eq!(BatteryStrategy::PowerSave.to_string(), "powersave");
        assert_eq!(BatteryStrategy::UltraLow.to_string(), "ultra_low");
    }

    #[test]
    fn test_power_state_display() {
        assert_eq!(PowerState::ScreenOn.to_string(), "screen_on");
        assert_eq!(PowerState::ScreenOffLight.to_string(), "screen_off_light");
        assert_eq!(PowerState::ScreenOffDeep.to_string(), "screen_off_deep");
        assert_eq!(PowerState::Charging.to_string(), "charging");
    }
}
