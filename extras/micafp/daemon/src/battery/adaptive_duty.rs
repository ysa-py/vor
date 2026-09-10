//! Adaptive duty cycling system for battery optimization
//!
//! This module controls the duty cycle of all background tasks based on
//! the current power mode. Each task has defined intervals for each mode,
//! ensuring that battery consumption is minimized while maintaining
//! connectivity.
//!
//! ## Power Modes
//!
//! | Mode | When | Key Behavior |
//! |------|------|-------------|
//! | Performance | Charging | All tasks at full speed |
//! | Normal | Battery > 50% | Reduced scan frequencies |
//! | Save | Battery 20-50% | WiFi Aware OFF, reduced BLE |
//! | Critical | Battery < 20% | Minimum activity, tunnel only |
//!
//! ## Task Intervals by Mode
//!
//! | Task | Performance | Normal | Save | Critical |
//! |------|------------|--------|------|----------|
//! | NAIN Probe | 30s | 60s | 120s | 300s |
//! | WiFi Aware Scan | 30s | 60s | OFF | OFF |
//! | BLE Scan | continuous | 30s/30s | 10s/50s | OFF |
//! | Acoustic Passive | ON | ON | low-power | OFF |
//! | NTP Covert Data | 32s | 64s | 128s | 320s |
//!
//! ## Screen State Effect
//!
//! When the screen is OFF, ALL intervals are doubled (2x multiplier).
//! This is handled by the BatteryCoordinator, not this module directly.
//!
//! ## Override: CompleteBlackout
//!
//! When NAIN detects a CompleteBlackout, the BatteryCoordinator temporarily
//! boosts to Performance mode regardless of battery level. This ensures
//! maximum connectivity recovery capability.

use std::time::Duration;

use tracing::{debug, info, warn};

/// Power mode determines the duty cycle of all background tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PowerMode {
    /// Maximum performance - device is charging.
    /// All tasks run at full speed.
    Performance,

    /// Normal mode - battery above 50%.
    /// Most tasks at moderate speed, some reduction.
    Normal,

    /// Power save mode - battery between 20% and 50%.
    /// WiFi Aware OFF, BLE scan reduced, acoustic low-power.
    Save,

    /// Critical mode - battery below 20%.
    /// Only essential tasks running. WiFi Aware OFF, BLE OFF, Acoustic OFF.
    Critical,
}

impl std::fmt::Display for PowerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowerMode::Performance => write!(f, "PERFORMANCE"),
            PowerMode::Normal => write!(f, "NORMAL"),
            PowerMode::Save => write!(f, "SAVE"),
            PowerMode::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Identifiers for all background tasks that can be duty-cycled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskId {
    /// NAIN (Network Analysis and Identification Node) probe
    /// Checks for DPI and censorship patterns
    NainProbe,

    /// WiFi Aware scanning for peer discovery
    /// Used for mesh networking capabilities
    WifiAwareScan,

    /// BLE (Bluetooth Low Energy) scanning
    /// Scan duration / interval pattern
    BleScan,

    /// Acoustic passive monitoring
    /// Listens for acoustic beacon signals from nearby devices
    AcousticPassive,

    /// NTP covert data channel
    /// Uses NTP packets as a covert channel for small data
    NtpCovertData,

    /// Connection health check
    /// Periodic check that the WireGuard tunnel is still functional
    ConnectionHealthCheck,

    /// Telemetry upload
    /// Uploads anonymized usage statistics
    TelemetryUpload,

    /// Config update check
    /// Checks for updated server lists and configuration
    ConfigUpdateCheck,

    /// Key rotation check
    /// Checks if WireGuard keys need rotation
    KeyRotationCheck,
}

/// Configuration for a single task's duty cycle across power modes.
#[derive(Debug, Clone)]
pub struct TaskDutyConfig {
    /// Task identifier
    pub task_id: TaskId,
    /// Interval in Performance mode
    pub performance_interval: Duration,
    /// Interval in Normal mode
    pub normal_interval: Duration,
    /// Interval in Save mode (None = disabled)
    pub save_interval: Option<Duration>,
    /// Interval in Critical mode (None = disabled)
    pub critical_interval: Option<Duration>,
    /// Whether this task is connectivity-critical
    /// (cannot be deferred, always runs when enabled)
    pub connectivity_critical: bool,
}

impl TaskDutyConfig {
    /// Get the interval for a given power mode.
    pub fn interval_for_mode(&self, mode: PowerMode) -> Option<Duration> {
        match mode {
            PowerMode::Performance => Some(self.performance_interval),
            PowerMode::Normal => Some(self.normal_interval),
            PowerMode::Save => self.save_interval,
            PowerMode::Critical => self.critical_interval,
        }
    }
}

/// All task duty cycle configurations.
pub struct TaskDutyTable {
    tasks: Vec<TaskDutyConfig>,
}

impl TaskDutyTable {
    /// Create the default duty table with standard intervals.
    pub fn new() -> Self {
        let tasks = vec![
            // NAIN Probe: connectivity-critical
            TaskDutyConfig {
                task_id: TaskId::NainProbe,
                performance_interval: Duration::from_secs(30),
                normal_interval: Duration::from_secs(60),
                save_interval: Some(Duration::from_secs(120)),
                critical_interval: Some(Duration::from_secs(300)),
                connectivity_critical: true,
            },
            // WiFi Aware Scan: optional, can be disabled
            TaskDutyConfig {
                task_id: TaskId::WifiAwareScan,
                performance_interval: Duration::from_secs(30),
                normal_interval: Duration::from_secs(60),
                save_interval: None,     // OFF in Save mode
                critical_interval: None, // OFF in Critical mode
                connectivity_critical: false,
            },
            // BLE Scan: used for peer discovery
            TaskDutyConfig {
                task_id: TaskId::BleScan,
                performance_interval: Duration::from_secs(30),
                normal_interval: Duration::from_secs(30), // 30s scan / 30s pause
                save_interval: Some(Duration::from_secs(10)), // 10s scan / 50s pause
                critical_interval: None,                  // OFF in Critical mode
                connectivity_critical: false,
            },
            // Acoustic Passive: always-on in Performance/Normal
            TaskDutyConfig {
                task_id: TaskId::AcousticPassive,
                performance_interval: Duration::from_secs(1), // Continuous
                normal_interval: Duration::from_secs(1),      // Continuous
                save_interval: Some(Duration::from_secs(5)),  // Low-power: check every 5s
                critical_interval: None,                      // OFF in Critical mode
                connectivity_critical: false,
            },
            // NTP Covert Data: pairs with NAIN probe for coalescing
            TaskDutyConfig {
                task_id: TaskId::NtpCovertData,
                performance_interval: Duration::from_secs(32),
                normal_interval: Duration::from_secs(64),
                save_interval: Some(Duration::from_secs(128)),
                critical_interval: Some(Duration::from_secs(320)),
                connectivity_critical: true,
            },
            // Connection Health Check: essential
            TaskDutyConfig {
                task_id: TaskId::ConnectionHealthCheck,
                performance_interval: Duration::from_secs(15),
                normal_interval: Duration::from_secs(30),
                save_interval: Some(Duration::from_secs(60)),
                critical_interval: Some(Duration::from_secs(120)),
                connectivity_critical: true,
            },
            // Telemetry Upload: can be deferred
            TaskDutyConfig {
                task_id: TaskId::TelemetryUpload,
                performance_interval: Duration::from_secs(300),
                normal_interval: Duration::from_secs(600),
                save_interval: Some(Duration::from_secs(1800)),
                critical_interval: None, // OFF in Critical mode
                connectivity_critical: false,
            },
            // Config Update Check: important but not critical
            TaskDutyConfig {
                task_id: TaskId::ConfigUpdateCheck,
                performance_interval: Duration::from_secs(1800), // 30 minutes
                normal_interval: Duration::from_secs(3600),      // 1 hour
                save_interval: Some(Duration::from_secs(7200)),  // 2 hours
                critical_interval: None,                         // OFF in Critical mode
                connectivity_critical: false,
            },
            // Key Rotation Check: security-critical
            TaskDutyConfig {
                task_id: TaskId::KeyRotationCheck,
                performance_interval: Duration::from_secs(300),
                normal_interval: Duration::from_secs(600),
                save_interval: Some(Duration::from_secs(1200)),
                critical_interval: Some(Duration::from_secs(3600)), // 1 hour
                connectivity_critical: true,
            },
        ];

        Self { tasks }
    }

    /// Get the duty config for a specific task.
    pub fn get(&self, task_id: TaskId) -> Option<&TaskDutyConfig> {
        self.tasks.iter().find(|t| t.task_id == task_id)
    }

    /// Get all tasks.
    pub fn all_tasks(&self) -> &[TaskDutyConfig] {
        &self.tasks
    }

    /// Get tasks that are enabled in a given power mode.
    pub fn enabled_tasks(&self, mode: PowerMode) -> Vec<&TaskDutyConfig> {
        self.tasks
            .iter()
            .filter(|t| t.interval_for_mode(mode).is_some())
            .collect()
    }

    /// Get tasks that are disabled in a given power mode.
    pub fn disabled_tasks(&self, mode: PowerMode) -> Vec<&TaskDutyConfig> {
        self.tasks
            .iter()
            .filter(|t| t.interval_for_mode(mode).is_none())
            .collect()
    }
}

impl Default for TaskDutyTable {
    fn default() -> Self {
        Self::new()
    }
}

/// The AdaptiveDutyCycler manages task execution intervals based on power mode.
///
/// It tracks:
/// - Current power mode
/// - Last execution time for each task
/// - Whether each task is due to run
/// - Screen state multiplier
pub struct AdaptiveDutyCycler {
    /// Current power mode
    mode: PowerMode,
    /// Task duty table
    duty_table: TaskDutyTable,
    /// Last execution time for each task
    last_execution: std::collections::HashMap<TaskId, std::time::Instant>,
    /// Screen state multiplier (1.0 = screen on, 2.0 = screen off)
    screen_multiplier: f32,
}

impl AdaptiveDutyCycler {
    /// Create a new adaptive duty cycler with the given initial mode.
    pub fn new(initial_mode: PowerMode) -> Self {
        Self {
            mode: initial_mode,
            duty_table: TaskDutyTable::new(),
            last_execution: std::collections::HashMap::new(),
            screen_multiplier: 1.0,
        }
    }

    /// Update the power mode.
    ///
    /// This may enable or disable certain tasks.
    pub fn update_mode(&mut self, mode: PowerMode) {
        let prev = self.mode;
        self.mode = mode;

        if prev != mode {
            let enabled = self.duty_table.enabled_tasks(mode);
            let disabled = self.duty_table.disabled_tasks(mode);

            info!(
                "AdaptiveDutyCycler mode changed: {} -> {} ({} tasks enabled, {} disabled)",
                prev,
                mode,
                enabled.len(),
                disabled.len()
            );

            for task in &disabled {
                debug!("Task {:?} DISABLED in {} mode", task.task_id, mode);
            }
        }
    }

    /// Update the screen state multiplier.
    pub fn set_screen_off(&mut self, screen_off: bool) {
        self.screen_multiplier = if screen_off { 2.0 } else { 1.0 };
        debug!(
            "Screen state changed: multiplier={:.1}x",
            self.screen_multiplier
        );
    }

    /// Check if a task is due to run.
    ///
    /// Returns true if the task should be executed now based on:
    /// 1. Whether the task is enabled in the current power mode
    /// 2. Whether enough time has elapsed since the last execution
    /// 3. The screen state multiplier
    pub fn is_task_due(&self, task_id: TaskId) -> bool {
        let config = match self.duty_table.get(task_id) {
            Some(c) => c,
            None => return false,
        };

        let interval = match config.interval_for_mode(self.mode) {
            Some(i) => i,
            None => return false, // Task is disabled in this mode
        };

        // Apply screen state multiplier
        let effective_interval = interval.mul_f64(self.screen_multiplier as f64);

        match self.last_execution.get(&task_id) {
            Some(last) => {
                let elapsed = last.elapsed();
                elapsed >= effective_interval
            }
            None => true, // Never executed, should run now
        }
    }

    /// Mark a task as executed.
    ///
    /// Call this after a task has been successfully executed.
    pub fn mark_executed(&mut self, task_id: TaskId) {
        self.last_execution
            .insert(task_id, std::time::Instant::now());
    }

    /// Get the effective interval for a task in the current mode.
    ///
    /// Returns None if the task is disabled in the current mode.
    pub fn effective_interval(&self, task_id: TaskId) -> Option<Duration> {
        let config = self.duty_table.get(task_id)?;
        let interval = config.interval_for_mode(self.mode)?;
        Some(interval.mul_f64(self.screen_multiplier as f64))
    }

    /// Get the time remaining until a task is next due.
    ///
    /// Returns None if the task is disabled in the current mode.
    /// Returns Zero if the task is due now.
    pub fn time_until_due(&self, task_id: TaskId) -> Option<Duration> {
        let config = self.duty_table.get(task_id)?;
        let interval = config.interval_for_mode(self.mode)?;
        let effective_interval = interval.mul_f64(self.screen_multiplier as f64);

        match self.last_execution.get(&task_id) {
            Some(last) => {
                let elapsed = last.elapsed();
                if elapsed >= effective_interval {
                    Some(Duration::ZERO)
                } else {
                    Some(effective_interval - elapsed)
                }
            }
            None => Some(Duration::ZERO),
        }
    }

    /// Get all tasks that are currently due.
    pub fn due_tasks(&self) -> Vec<TaskId> {
        use TaskId::*;
        let all = [
            NainProbe,
            WifiAwareScan,
            BleScan,
            AcousticPassive,
            NtpCovertData,
            ConnectionHealthCheck,
            TelemetryUpload,
            ConfigUpdateCheck,
            KeyRotationCheck,
        ];

        all.iter()
            .filter(|&&task| self.is_task_due(task))
            .copied()
            .collect()
    }

    /// Get the current power mode.
    pub fn mode(&self) -> PowerMode {
        self.mode
    }

    /// Get the screen state multiplier.
    pub fn screen_multiplier(&self) -> f32 {
        self.screen_multiplier
    }

    /// Get a summary of the current duty cycle state.
    pub fn summary(&self) -> DutyCycleSummary {
        let enabled: Vec<TaskId> = self
            .duty_table
            .enabled_tasks(self.mode)
            .iter()
            .map(|t| t.task_id)
            .collect();

        let disabled: Vec<TaskId> = self
            .duty_table
            .disabled_tasks(self.mode)
            .iter()
            .map(|t| t.task_id)
            .collect();

        let due = self.due_tasks();

        DutyCycleSummary {
            mode: self.mode,
            screen_multiplier: self.screen_multiplier,
            enabled_tasks: enabled,
            disabled_tasks: disabled,
            due_tasks: due,
        }
    }
}

/// Summary of the current duty cycle state.
#[derive(Debug, Clone)]
pub struct DutyCycleSummary {
    /// Current power mode
    pub mode: PowerMode,
    /// Current screen state multiplier
    pub screen_multiplier: f32,
    /// Tasks enabled in current mode
    pub enabled_tasks: Vec<TaskId>,
    /// Tasks disabled in current mode
    pub disabled_tasks: Vec<TaskId>,
    /// Tasks that are currently due to run
    pub due_tasks: Vec<TaskId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_mode_display() {
        assert_eq!(PowerMode::Performance.to_string(), "PERFORMANCE");
        assert_eq!(PowerMode::Normal.to_string(), "NORMAL");
        assert_eq!(PowerMode::Save.to_string(), "SAVE");
        assert_eq!(PowerMode::Critical.to_string(), "CRITICAL");
    }

    #[test]
    fn test_duty_table_defaults() {
        let table = TaskDutyTable::new();

        // NAIN probe should be enabled in all modes
        let nain = table.get(TaskId::NainProbe).unwrap();
        assert!(nain.interval_for_mode(PowerMode::Critical).is_some());

        // WiFi Aware should be disabled in Save and Critical modes
        let wifi = table.get(TaskId::WifiAwareScan).unwrap();
        assert!(wifi.interval_for_mode(PowerMode::Performance).is_some());
        assert!(wifi.interval_for_mode(PowerMode::Save).is_none());
        assert!(wifi.interval_for_mode(PowerMode::Critical).is_none());

        // BLE should be disabled in Critical mode
        let ble = table.get(TaskId::BleScan).unwrap();
        assert!(ble.interval_for_mode(PowerMode::Critical).is_none());
    }

    #[test]
    fn test_enabled_disabled_tasks() {
        let table = TaskDutyTable::new();

        let perf_enabled = table.enabled_tasks(PowerMode::Performance);
        let crit_enabled = table.enabled_tasks(PowerMode::Critical);

        // Performance should have more enabled tasks than Critical
        assert!(perf_enabled.len() > crit_enabled.len());

        // NAIN probe should be enabled in Critical
        assert!(crit_enabled.iter().any(|t| t.task_id == TaskId::NainProbe));
    }

    #[test]
    fn test_duty_cycler_task_due() {
        let mut cycler = AdaptiveDutyCycler::new(PowerMode::Performance);

        // Tasks should be due initially
        assert!(cycler.is_task_due(TaskId::NainProbe));

        // Mark as executed
        cycler.mark_executed(TaskId::NainProbe);

        // Should not be due immediately after execution
        assert!(!cycler.is_task_due(TaskId::NainProbe));
    }

    #[test]
    fn test_duty_cycler_mode_change() {
        let mut cycler = AdaptiveDutyCycler::new(PowerMode::Performance);

        // WiFi Aware should be enabled in Performance
        assert!(cycler.is_task_due(TaskId::WifiAwareScan));

        // Switch to Save mode
        cycler.update_mode(PowerMode::Save);

        // WiFi Aware should be disabled in Save mode
        assert!(!cycler.is_task_due(TaskId::WifiAwareScan));
    }

    #[test]
    fn test_screen_state_multiplier() {
        let mut cycler = AdaptiveDutyCycler::new(PowerMode::Performance);

        // Get effective interval with screen on
        let interval_on = cycler.effective_interval(TaskId::NainProbe).unwrap();

        // Turn screen off
        cycler.set_screen_off(true);
        let interval_off = cycler.effective_interval(TaskId::NainProbe).unwrap();

        // Screen off should double the interval
        assert!(interval_off > interval_on);
        assert!((interval_off.as_secs_f64() / interval_on.as_secs_f64() - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_due_tasks() {
        let cycler = AdaptiveDutyCycler::new(PowerMode::Performance);
        let due = cycler.due_tasks();

        // All tasks should be due initially
        assert!(!due.is_empty());
    }

    #[test]
    fn test_connectivity_critical_tasks() {
        let table = TaskDutyTable::new();

        // These tasks should be connectivity-critical
        assert!(table.get(TaskId::NainProbe).unwrap().connectivity_critical);
        assert!(
            table
                .get(TaskId::ConnectionHealthCheck)
                .unwrap()
                .connectivity_critical
        );
        assert!(
            table
                .get(TaskId::KeyRotationCheck)
                .unwrap()
                .connectivity_critical
        );

        // These should NOT be connectivity-critical
        assert!(
            !table
                .get(TaskId::WifiAwareScan)
                .unwrap()
                .connectivity_critical
        );
        assert!(!table.get(TaskId::BleScan).unwrap().connectivity_critical);
    }

    #[test]
    fn test_summary() {
        let cycler = AdaptiveDutyCycler::new(PowerMode::Normal);
        let summary = cycler.summary();

        assert_eq!(summary.mode, PowerMode::Normal);
        assert!(!summary.enabled_tasks.is_empty());
    }
}
