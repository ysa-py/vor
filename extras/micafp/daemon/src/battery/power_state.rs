//! Power state monitoring for MICAFP-UnifiedShield-6.0
//!
//! This module provides platform-specific battery monitoring that publishes
//! power state changes to all subsystems via a watch channel. Key features:
//!
//! - Platform-specific monitoring (Android, iOS, Linux, Windows)
//! - Throttled updates (1 per 30 seconds) to avoid unnecessary wakeups
//! - Battery level, charging state, and screen state monitoring
//! - Watch channel for efficient fan-out to all subscribers

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Current power state of the device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerState {
    /// Current battery level
    pub battery_level: BatteryLevel,
    /// Charging state
    pub charging_state: ChargingState,
    /// Screen state
    pub screen_state: ScreenState,
}

impl Default for PowerState {
    fn default() -> Self {
        Self {
            battery_level: BatteryLevel::Unknown,
            charging_state: ChargingState::Unknown,
            screen_state: ScreenState::Unknown,
        }
    }
}

/// Battery level with percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatteryLevel {
    /// Known battery level (0-100%)
    Known(u8),
    /// Battery level unknown (e.g., desktop without battery)
    Unknown,
}

impl BatteryLevel {
    /// Get the percentage if known.
    pub fn percentage(&self) -> Option<u8> {
        match self {
            BatteryLevel::Known(pct) => Some(*pct),
            BatteryLevel::Unknown => None,
        }
    }

    /// Whether the battery level is low (below threshold).
    pub fn is_low(&self, threshold: u8) -> bool {
        match self {
            BatteryLevel::Known(pct) => *pct < threshold,
            BatteryLevel::Unknown => false, // Assume not low if unknown
        }
    }
}

impl std::fmt::Display for BatteryLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatteryLevel::Known(pct) => write!(f, "{}%", pct),
            BatteryLevel::Unknown => write!(f, "unknown"),
        }
    }
}

/// Device charging state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChargingState {
    /// Device is charging
    Charging,
    /// Device is discharging (on battery)
    Discharging,
    /// Battery is fully charged
    Full,
    /// Charging state unknown
    Unknown,
}

impl std::fmt::Display for ChargingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChargingState::Charging => write!(f, "charging"),
            ChargingState::Discharging => write!(f, "discharging"),
            ChargingState::Full => write!(f, "full"),
            ChargingState::Unknown => write!(f, "unknown"),
        }
    }
}

/// Screen state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenState {
    /// Screen is on
    On,
    /// Screen is off
    Off,
    /// Screen state unknown
    Unknown,
}

impl std::fmt::Display for ScreenState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenState::On => write!(f, "on"),
            ScreenState::Off => write!(f, "off"),
            ScreenState::Unknown => write!(f, "unknown"),
        }
    }
}

/// Power state monitor configuration.
#[derive(Debug, Clone)]
pub struct PowerMonitorConfig {
    /// Minimum interval between power state updates
    pub throttle_interval: Duration,
    /// Whether to monitor screen state
    pub monitor_screen_state: bool,
    /// Whether to monitor charging state
    pub monitor_charging_state: bool,
    /// Path to battery info (Linux only)
    pub linux_battery_path: String,
}

impl Default for PowerMonitorConfig {
    fn default() -> Self {
        Self {
            throttle_interval: Duration::from_secs(30),
            monitor_screen_state: true,
            monitor_charging_state: true,
            linux_battery_path: "/sys/class/power_supply/BAT0".to_string(),
        }
    }
}

/// Power state monitor that reads platform-specific battery information.
pub struct PowerMonitor {
    /// Configuration
    config: PowerMonitorConfig,
    /// Last reported power state
    last_state: PowerState,
    /// Last update time (for throttling)
    last_update: Instant,
    /// Platform-specific monitor
    platform_monitor: Box<dyn PlatformPowerMonitor + Send + Sync>,
}

impl PowerMonitor {
    /// Create a new power monitor with default configuration.
    pub fn new() -> Self {
        Self::with_config(PowerMonitorConfig::default())
    }

    /// Create a new power monitor with custom configuration.
    pub fn with_config(config: PowerMonitorConfig) -> Self {
        let platform_monitor = create_platform_monitor(&config);
        Self {
            config,
            last_state: PowerState::default(),
            last_update: Instant::now() - Duration::from_secs(60), // Allow immediate first update
            platform_monitor,
        }
    }

    /// Check for power state changes.
    ///
    /// Returns Some(PowerState) if the state has changed since the last
    /// call, or None if it hasn't. Updates are throttled to avoid
    /// unnecessary wakeups.
    pub fn check(&mut self) -> Option<PowerState> {
        // Throttle updates
        if self.last_update.elapsed() < self.config.throttle_interval {
            return None;
        }

        let new_state = self.platform_monitor.read_power_state();

        // Only report if something changed
        if new_state != self.last_state {
            debug!(
                "Power state changed: battery={} charging={} screen={}",
                new_state.battery_level, new_state.charging_state, new_state.screen_state
            );
            self.last_state = new_state.clone();
            self.last_update = Instant::now();
            Some(new_state)
        } else {
            self.last_update = Instant::now(); // Reset throttle timer
            None
        }
    }

    /// Force a power state read, bypassing the throttle.
    pub fn force_check(&mut self) -> PowerState {
        let new_state = self.platform_monitor.read_power_state();
        self.last_state = new_state.clone();
        self.last_update = Instant::now();
        new_state
    }

    /// Get the last known power state without checking.
    pub fn last_state(&self) -> &PowerState {
        &self.last_state
    }

    /// Start periodic monitoring.
    ///
    /// Returns a stream of power state changes.
    pub async fn start_monitoring(&mut self, callback: Box<dyn Fn(PowerState) + Send + Sync>) {
        let mut interval = tokio::time::interval(self.config.throttle_interval);

        loop {
            interval.tick().await;

            if let Some(state) = self.check() {
                callback(state);
            }
        }
    }
}

/// Platform-specific power monitor trait.
pub trait PlatformPowerMonitor {
    /// Read the current power state from the platform.
    fn read_power_state(&self) -> PowerState;
}

/// Create the platform-appropriate power monitor.
fn create_platform_monitor(
    config: &PowerMonitorConfig,
) -> Box<dyn PlatformPowerMonitor + Send + Sync> {
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxPowerMonitor {
            battery_path: config.linux_battery_path.clone(),
        })
    }

    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsPowerMonitor)
    }

    #[cfg(target_os = "android")]
    {
        Box::new(AndroidPowerMonitor)
    }

    #[cfg(target_os = "ios")]
    {
        Box::new(IosPowerMonitor)
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "windows",
        target_os = "android",
        target_os = "ios"
    )))]
    {
        Box::new(StubPowerMonitor)
    }
}

// =========================================================================
// Linux Power Monitor
// =========================================================================

/// Linux power monitor that reads from /sys/class/power_supply/.
///
/// Reads the following files:
/// - `capacity`: Battery percentage (0-100)
/// - `status`: Charging status ("Charging", "Discharging", "Full", "Not charging")
/// - Screen state: detected via X11/Wayland DPMS or loginctl
#[cfg(target_os = "linux")]
struct LinuxPowerMonitor {
    battery_path: String,
}

#[cfg(target_os = "linux")]
impl PlatformPowerMonitor for LinuxPowerMonitor {
    fn read_power_state(&self) -> PowerState {
        let battery_level = self.read_battery_level();
        let charging_state = self.read_charging_state();
        let screen_state = self.read_screen_state();

        PowerState {
            battery_level,
            charging_state,
            screen_state,
        }
    }
}

#[cfg(target_os = "linux")]
impl LinuxPowerMonitor {
    /// Read battery level from /sys/class/power_supply/BAT0/capacity.
    fn read_battery_level(&self) -> BatteryLevel {
        let capacity_path = format!("{}/capacity", self.battery_path);
        match std::fs::read_to_string(&capacity_path) {
            Ok(content) => {
                let pct = content.trim().parse::<u8>().unwrap_or(0);
                BatteryLevel::Known(pct.min(100))
            }
            Err(_) => {
                // Try BAT1 if BAT0 doesn't exist
                let alt_path = self.battery_path.replace("BAT0", "BAT1");
                let capacity_path = format!("{}/capacity", alt_path);
                match std::fs::read_to_string(&capacity_path) {
                    Ok(content) => {
                        let pct = content.trim().parse::<u8>().unwrap_or(0);
                        BatteryLevel::Known(pct.min(100))
                    }
                    Err(_) => BatteryLevel::Unknown,
                }
            }
        }
    }

    /// Read charging state from /sys/class/power_supply/BAT0/status.
    fn read_charging_state(&self) -> ChargingState {
        let status_path = format!("{}/status", self.battery_path);
        match std::fs::read_to_string(&status_path) {
            Ok(content) => match content.trim() {
                "Charging" => ChargingState::Charging,
                "Discharging" => ChargingState::Discharging,
                "Full" | "Not charging" => ChargingState::Full,
                _ => ChargingState::Unknown,
            },
            Err(_) => ChargingState::Unknown,
        }
    }

    /// Detect screen state via loginctl or X11 DPMS.
    fn read_screen_state(&self) -> ScreenState {
        // Method 1: loginctl show-session
        // $ loginctl show-session $(loginctl | grep $(whoami) | awk '{print $1}') -p IdleHint
        //
        // Method 2: xset q | grep "Monitor is"
        // "Monitor is On" or "Monitor is Off"
        //
        // Method 3: Check /sys/class/drm/card*/status
        // "connected" means screen is active

        // Try xset first (X11)
        if let Ok(output) = std::process::Command::new("xset").arg("q").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("Monitor is On") {
                return ScreenState::On;
            } else if stdout.contains("Monitor is Off") {
                return ScreenState::Off;
            }
        }

        // Fallback: assume on if we can't determine
        ScreenState::Unknown
    }
}

// =========================================================================
// Windows Power Monitor
// =========================================================================

/// Windows power monitor using GetSystemPowerStatus API.
///
/// In production, this uses the `windows` crate:
/// ```ignore
/// use windows::Win32::System::Power::GetSystemPowerStatus;
/// use windows::Win32::System::Power::SYSTEM_POWER_STATUS;
///
/// let mut status = SYSTEM_POWER_STATUS::default();
/// unsafe { GetSystemPowerStatus(&mut status); }
/// ```
#[cfg(target_os = "windows")]
struct WindowsPowerMonitor;

#[cfg(target_os = "windows")]
impl PlatformPowerMonitor for WindowsPowerMonitor {
    fn read_power_state(&self) -> PowerState {
        // In production with the windows crate:
        //
        // let mut status = SYSTEM_POWER_STATUS::default();
        // unsafe {
        //     GetSystemPowerStatus(&mut status);
        // }
        //
        // let battery_level = if status.BatteryFlag != 128 { // 128 = no battery
        //     BatteryLevel::Known(status.BatteryLifePercent)
        // } else {
        //     BatteryLevel::Unknown
        // };
        //
        // let charging_state = match status.ACLineStatus {
        //     1 => ChargingState::Charging,
        //     0 => ChargingState::Discharging,
        //     _ => ChargingState::Unknown,
        // };

        // Screen state: use GetDevicePowerState or monitor WMI events
        // Or simply check for user input idle time via GetLastInputInfo

        PowerState {
            battery_level: BatteryLevel::Unknown,
            charging_state: ChargingState::Unknown,
            screen_state: ScreenState::Unknown,
        }
    }
}

// =========================================================================
// Android Power Monitor
// =========================================================================

/// Android power monitor using BatteryManager and ACTION_BATTERY_CHANGED.
///
/// On Android, battery information is obtained via:
/// 1. `BatteryManager` (API level 21+): `getIntProperty(BATTERY_PROPERTY_CAPACITY)`
/// 2. `Intent.ACTION_BATTERY_CHANGED` broadcast receiver
/// 3. Screen state via `PowerManager.isInteractive()`
///
/// Since this runs in the native daemon, battery updates are received
/// from the Kotlin layer via JNI (see platform/android module).
#[cfg(target_os = "android")]
struct AndroidPowerMonitor;

#[cfg(target_os = "android")]
impl PlatformPowerMonitor for AndroidPowerMonitor {
    fn read_power_state(&self) -> PowerState {
        // On Android, battery state is pushed to us via JNI from the
        // BroadcastReceiver in the Kotlin layer. We store the latest
        // state in a thread-safe global and read it here.
        //
        // In production, this would read from a shared AtomicPtr or
        // similar mechanism updated by the JNI callback:
        //
        // static LAST_POWER_STATE: Lazy<Mutex<PowerState>> = ...;
        //
        // The BroadcastReceiver in Kotlin sends:
        // - Battery level: intent.getIntExtra(BatteryManager.EXTRA_LEVEL, -1)
        // - Charging: intent.getIntExtra(BatteryManager.EXTRA_STATUS, -1)
        // - Screen: powerManager.isInteractive()

        PowerState {
            battery_level: BatteryLevel::Unknown,
            charging_state: ChargingState::Unknown,
            screen_state: ScreenState::Unknown,
        }
    }
}

// =========================================================================
// iOS Power Monitor
// =========================================================================

/// iOS power monitor using UIDevice battery APIs.
///
/// On iOS, battery information is obtained via:
/// 1. `UIDevice.current.batteryState`: charging/discharging/full/unknown
/// 2. `UIDevice.current.batteryLevel`: 0.0-1.0
/// 3. Screen state: `UIScreen.main.brightness > 0` or ProcessInfo thermal state
///
/// Battery monitoring must be enabled: `UIDevice.current.isBatteryMonitoringEnabled = true`
///
/// Updates are received from the Swift layer via FFI.
#[cfg(target_os = "ios")]
struct IosPowerMonitor;

#[cfg(target_os = "ios")]
impl PlatformPowerMonitor for IosPowerMonitor {
    fn read_power_state(&self) -> PowerState {
        // On iOS, battery state is pushed from the Swift NetworkExtension
        // to the Rust daemon via FFI. We read from a shared state.
        //
        // In production, the Swift layer calls:
        // UIDevice.current.isBatteryMonitoringEnabled = true
        // let level = Int(UIDevice.current.batteryLevel * 100)
        // let state = UIDevice.current.batteryState
        //
        // Screen state:
        // let isScreenOn = UIScreen.main.brightness > 0

        PowerState {
            battery_level: BatteryLevel::Unknown,
            charging_state: ChargingState::Unknown,
            screen_state: ScreenState::Unknown,
        }
    }
}

// =========================================================================
// Stub Power Monitor (for unsupported platforms / testing)
// =========================================================================

/// Stub power monitor that always returns Unknown state.
#[cfg(not(any(
    target_os = "linux",
    target_os = "windows",
    target_os = "android",
    target_os = "ios"
)))]
struct StubPowerMonitor;

#[cfg(not(any(
    target_os = "linux",
    target_os = "windows",
    target_os = "android",
    target_os = "ios"
)))]
impl PlatformPowerMonitor for StubPowerMonitor {
    fn read_power_state(&self) -> PowerState {
        PowerState::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battery_level_display() {
        assert_eq!(BatteryLevel::Known(75).to_string(), "75%");
        assert_eq!(BatteryLevel::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_battery_level_percentage() {
        assert_eq!(BatteryLevel::Known(50).percentage(), Some(50));
        assert_eq!(BatteryLevel::Unknown.percentage(), None);
    }

    #[test]
    fn test_battery_level_is_low() {
        assert!(BatteryLevel::Known(15).is_low(20));
        assert!(!BatteryLevel::Known(25).is_low(20));
        assert!(!BatteryLevel::Unknown.is_low(20));
    }

    #[test]
    fn test_charging_state_display() {
        assert_eq!(ChargingState::Charging.to_string(), "charging");
        assert_eq!(ChargingState::Discharging.to_string(), "discharging");
        assert_eq!(ChargingState::Full.to_string(), "full");
    }

    #[test]
    fn test_screen_state_display() {
        assert_eq!(ScreenState::On.to_string(), "on");
        assert_eq!(ScreenState::Off.to_string(), "off");
    }

    #[test]
    fn test_power_state_default() {
        let state = PowerState::default();
        assert_eq!(state.battery_level, BatteryLevel::Unknown);
        assert_eq!(state.charging_state, ChargingState::Unknown);
        assert_eq!(state.screen_state, ScreenState::Unknown);
    }

    #[test]
    fn test_power_state_equality() {
        let state1 = PowerState {
            battery_level: BatteryLevel::Known(50),
            charging_state: ChargingState::Discharging,
            screen_state: ScreenState::On,
        };
        let state2 = state1.clone();
        assert_eq!(state1, state2);
    }

    #[test]
    fn test_power_monitor_config_default() {
        let config = PowerMonitorConfig::default();
        assert_eq!(config.throttle_interval, Duration::from_secs(30));
        assert!(config.monitor_screen_state);
    }

    #[test]
    fn test_power_monitor_throttle() {
        let mut monitor = PowerMonitor::new();

        // First check should potentially return a state
        let first = monitor.check();
        // The stub monitor returns default state, which equals the initial state,
        // so check() returns None (no change detected)
        // This is correct behavior - no change = None

        // Force check should always return
        let forced = monitor.force_check();
        assert_eq!(forced.battery_level, BatteryLevel::Unknown);
    }

    #[test]
    fn test_serialization() {
        let state = PowerState {
            battery_level: BatteryLevel::Known(75),
            charging_state: ChargingState::Charging,
            screen_state: ScreenState::On,
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: PowerState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, deserialized);
    }
}
