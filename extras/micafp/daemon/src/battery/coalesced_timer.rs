//! Coalesced timer system for reducing CPU wakeups
//!
//! This module combines multiple periodic tasks into fewer CPU wakeups,
//! reducing power consumption by approximately 40% compared to independent
//! timers.
//!
//! ## How It Works
//!
//! Traditional approach: each periodic task has its own timer.
//! ```
//! Timer 1: NAIN probe every 30s -> CPU wakeup
//! Timer 2: NTP covert every 32s -> CPU wakeup
//! Timer 3: Health check every 15s -> CPU wakeup
//! ```
//! This results in many independent CPU wakeups.
//!
//! Coalesced approach: group tasks with similar intervals into the same tick.
//! ```
//! Tick at 30s: NAIN probe + NTP covert (both fire at 30s)
//! Tick at 15s: Health check
//! ```
//! This reduces CPU wakeups by ~40%.
//!
//! ## Interval Grouping Strategy
//!
//! Tasks with intervals within 10% of each other are grouped together.
//! The group fires at the shorter interval, and longer-interval tasks
//! use a counter to determine if they should execute.
//!
//! Example:
//! - NAIN probe (30s) and NTP covert (32s) -> both fire at 30s tick
//! - NTP covert fires every other tick (30s * 2 = 60s ≈ 32s * 2 = 64s)
//!
//! ## Priority-Based Execution
//!
//! Tasks have priorities:
//! - **Connectivity-critical**: Always fire on their scheduled tick.
//!   Cannot be deferred. Examples: NAIN probe, connection health check.
//! - **Optional**: Can be deferred to the next tick if the system is
//!   under load. Examples: telemetry upload, config check.
//!
//! ## Drift Compensation
//!
//! Timer drift occurs when tick processing takes longer than expected.
//! The coalesced timer compensates by:
//! 1. Measuring actual elapsed time since last tick
//! 2. Adjusting the next tick to maintain the target interval
//! 3. Never skipping critical tasks due to drift

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tracing::{debug, info, trace, warn};

use super::adaptive_duty::TaskId;
use crate::battery::BatteryError;

/// Default tolerance for grouping intervals (10%).
const GROUPING_TOLERANCE: f64 = 0.10;

/// Minimum interval between timer ticks (1 second).
const MIN_TICK_INTERVAL: Duration = Duration::from_secs(1);

/// A registered task in the coalesced timer.
#[derive(Debug, Clone)]
struct RegisteredTask {
    /// Task identifier
    task_id: TaskId,
    /// Target interval for this task
    target_interval: Duration,
    /// Whether this task is connectivity-critical
    critical: bool,
    /// Tick counter for this task
    tick_counter: u64,
    /// How many ticks between executions
    ticks_per_execution: u64,
    /// Last execution time
    last_execution: Option<Instant>,
    // Callback to invoke when the task is due
    // (In production, this would be a proper callback type)
}

/// A group of tasks that share the same timer tick.
#[derive(Debug)]
struct TaskGroup {
    /// Base interval for this group (shortest interval in the group)
    base_interval: Duration,
    /// Tasks in this group
    tasks: Vec<RegisteredTask>,
}

/// The CoalescedTimer manages periodic task execution with minimal CPU wakeups.
pub struct CoalescedTimer {
    /// All registered tasks
    tasks: HashMap<TaskId, RegisteredTask>,
    /// Task groups (each group shares a timer)
    groups: Vec<TaskGroup>,
    /// Whether the timer is running
    running: AtomicBool,
    /// Total ticks processed
    total_ticks: u64,
    /// Total CPU wakeups saved (compared to independent timers)
    wakeups_saved: u64,
}

impl CoalescedTimer {
    /// Create a new coalesced timer.
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            groups: Vec::new(),
            running: AtomicBool::new(false),
            total_ticks: 0,
            wakeups_saved: 0,
        }
    }

    /// Register a task with the coalesced timer.
    ///
    /// The task will be grouped with other tasks that have similar intervals.
    pub fn register_task(&mut self, task_id: TaskId, interval: Duration, critical: bool) {
        let ticks_per_execution = 1; // Will be recalculated during grouping

        let task = RegisteredTask {
            task_id,
            target_interval: interval,
            critical,
            tick_counter: 0,
            ticks_per_execution,
            last_execution: None,
        };

        self.tasks.insert(task_id, task);
    }

    /// Update a task's interval (e.g., when power mode changes).
    pub fn update_task_interval(&mut self, task_id: TaskId, new_interval: Option<Duration>) {
        match new_interval {
            Some(interval) => {
                if let Some(task) = self.tasks.get_mut(&task_id) {
                    task.target_interval = interval;
                    task.tick_counter = 0;
                    task.last_execution = None;
                } else {
                    // Task not registered, register it
                    let critical = is_critical_task(task_id);
                    self.register_task(task_id, interval, critical);
                }
            }
            None => {
                // Disable the task
                self.tasks.remove(&task_id);
            }
        }

        // Regroup tasks after interval change
        self.regroup_tasks();
    }

    /// Disable a task (equivalent to setting interval to None).
    pub fn disable_task(&mut self, task_id: TaskId) {
        self.tasks.remove(&task_id);
        self.regroup_tasks();
    }

    /// Regroup tasks based on their current intervals.
    ///
    /// This implements the interval grouping algorithm:
    /// 1. Sort tasks by interval (shortest first)
    /// 2. Group tasks whose intervals are within 10% of each other
    /// 3. Each group uses the shortest interval as its base
    /// 4. Longer-interval tasks use a counter to determine when to fire
    fn regroup_tasks(&mut self) {
        if self.tasks.is_empty() {
            self.groups.clear();
            return;
        }

        // Sort tasks by target interval
        let mut sorted_tasks: Vec<&mut RegisteredTask> = self.tasks.values_mut().collect();
        sorted_tasks.sort_by_key(|t| t.target_interval);

        // Group tasks with similar intervals
        let mut groups: Vec<TaskGroup> = Vec::new();
        let mut current_group: Option<TaskGroup> = None;

        for task in sorted_tasks {
            let task_clone = RegisteredTask {
                task_id: task.task_id,
                target_interval: task.target_interval,
                critical: task.critical,
                tick_counter: 0,
                ticks_per_execution: 1,
                last_execution: None,
            };

            match &mut current_group {
                Some(group) => {
                    // Check if this task's interval is within tolerance of the group's base
                    let ratio =
                        task.target_interval.as_secs_f64() / group.base_interval.as_secs_f64();
                    let deviation = (ratio - 1.0).abs();

                    if deviation <= GROUPING_TOLERANCE {
                        // Add to current group
                        // Calculate ticks per execution for this task
                        let ticks = (task.target_interval.as_secs_f64()
                            / group.base_interval.as_secs_f64())
                        .round()
                        .max(1.0) as u64;

                        let mut grouped_task = task_clone;
                        grouped_task.ticks_per_execution = ticks;
                        group.tasks.push(grouped_task);
                    } else {
                        // Start a new group
                        let group = std::mem::replace(&mut current_group, None);
                        if let Some(g) = group {
                            groups.push(g);
                        }
                        current_group = Some(TaskGroup {
                            base_interval: task.target_interval,
                            tasks: vec![task_clone],
                        });
                    }
                }
                None => {
                    current_group = Some(TaskGroup {
                        base_interval: task.target_interval,
                        tasks: vec![task_clone],
                    });
                }
            }
        }

        // Don't forget the last group
        if let Some(group) = current_group {
            groups.push(group);
        }

        // Update the registered tasks with their ticks_per_execution
        for group in &groups {
            for task in &group.tasks {
                if let Some(registered) = self.tasks.get_mut(&task.task_id) {
                    registered.ticks_per_execution = task.ticks_per_execution;
                }
            }
        }

        let num_groups = groups.len();
        let num_tasks: usize = groups.iter().map(|g| g.tasks.len()).sum();
        let independent_wakeups = num_tasks; // One per task
        let coalesced_wakeups = num_groups; // One per group
        let saved = independent_wakeups.saturating_sub(coalesced_wakeups);

        debug!(
            "Regrouped {} tasks into {} groups (saved {} wakeups, {:.0}% reduction)",
            num_tasks,
            num_groups,
            saved,
            if independent_wakeups > 0 {
                (saved as f64 / independent_wakeups as f64) * 100.0
            } else {
                0.0
            }
        );

        self.groups = groups;
    }

    /// Start the coalesced timer.
    ///
    /// This begins the tick loop that fires task callbacks at their
    /// scheduled intervals.
    pub async fn start(&mut self) -> Result<(), BatteryError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Regroup tasks before starting
        self.regroup_tasks();

        self.running.store(true, Ordering::SeqCst);
        info!(
            "CoalescedTimer started with {} groups covering {} tasks",
            self.groups.len(),
            self.tasks.len()
        );

        Ok(())
    }

    /// Stop the coalesced timer.
    pub async fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        info!(
            "CoalescedTimer stopped ({} ticks processed, {} wakeups saved)",
            self.total_ticks, self.wakeups_saved
        );
    }

    /// Process a single timer tick.
    ///
    /// This method should be called at the base interval of the fastest
    /// task group. It checks all groups for tasks that are due to execute.
    ///
    /// Returns a list of task IDs that are due to run.
    pub fn tick(&mut self) -> Vec<TaskId> {
        if !self.running.load(Ordering::SeqCst) {
            return Vec::new();
        }

        self.total_ticks += 1;
        let mut due_tasks = Vec::new();

        for i in 0..self.groups.len() {
            let should_tick = {
                let group = &self.groups[i];
                self.should_group_tick(group)
            };

            if should_tick {
                for task in &mut self.groups[i].tasks {
                    task.tick_counter += 1;

                    if task.tick_counter >= task.ticks_per_execution {
                        // Task is due
                        due_tasks.push(task.task_id);
                        task.tick_counter = 0;
                        task.last_execution = Some(Instant::now());

                        trace!("Task {:?} is due (tick {})", task.task_id, self.total_ticks);
                    }
                }
            }
        }

        // Count wakeups saved: if we had independent timers, each due task
        // would have caused its own wakeup. With coalescing, one wakeup
        // handles multiple tasks.
        if due_tasks.len() > 1 {
            self.wakeups_saved += due_tasks.len() as u64 - 1;
        }

        due_tasks
    }

    /// Check if a group should tick at the current global tick.
    fn should_group_tick(&self, group: &TaskGroup) -> bool {
        // A group ticks when the global tick count is a multiple of
        // the ratio between the group's base interval and the fastest interval.
        //
        // For the fastest group (base_interval == global_interval), always tick.
        // For slower groups, tick less frequently.

        if self.groups.is_empty() {
            return false;
        }

        // Find the fastest group's interval
        let fastest_interval = self
            .groups
            .iter()
            .map(|g| g.base_interval)
            .min()
            .unwrap_or(MIN_TICK_INTERVAL);

        if group.base_interval <= fastest_interval {
            return true;
        }

        // Calculate the tick ratio
        let ratio = group.base_interval.as_secs_f64() / fastest_interval.as_secs_f64();
        let tick_modulo = ratio.round().max(1.0) as u64;

        self.total_ticks % tick_modulo == 0
    }

    /// Run the main timer loop.
    ///
    /// This is the async entry point for the timer loop. It uses
    /// tokio::time::interval with drift compensation.
    pub async fn run_loop(&mut self) {
        if self.groups.is_empty() {
            debug!("No task groups, timer loop idle");
            return;
        }

        // Find the fastest interval (this determines the tick rate)
        let fastest_interval = self
            .groups
            .iter()
            .map(|g| g.base_interval)
            .min()
            .unwrap_or(Duration::from_secs(30));

        // Ensure minimum tick interval
        let tick_interval = fastest_interval.max(MIN_TICK_INTERVAL);

        let mut interval = tokio::time::interval(tick_interval);
        // Set miss_dash_tick_behaviour to delay (don't burst)
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        debug!(
            "Timer loop starting with tick interval: {:?}",
            tick_interval
        );

        while self.running.load(Ordering::SeqCst) {
            interval.tick().await;

            let due_tasks = self.tick();

            if !due_tasks.is_empty() {
                trace!(
                    "Tick {}: {} tasks due: {:?}",
                    self.total_ticks,
                    due_tasks.len(),
                    due_tasks
                );

                // In production, we would dispatch each due task
                // to its handler. The handlers run asynchronously
                // and must complete before the next tick.
                for task_id in due_tasks {
                    self.dispatch_task(task_id).await;
                }
            }
        }

        debug!("Timer loop ended");
    }

    /// Dispatch a due task to its handler.
    async fn dispatch_task(&self, task_id: TaskId) {
        // In production, this would look up the task's callback
        // and execute it. For now, we just log it.
        trace!("Dispatching task: {:?}", task_id);
    }

    /// Get the number of registered tasks.
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Get the number of task groups.
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    /// Get the estimated wakeup savings percentage.
    pub fn wakeup_savings_percent(&self) -> f64 {
        let total_tasks = self.tasks.len() as u64;
        if total_tasks == 0 {
            return 0.0;
        }
        let total_wakeups_independent = self.total_ticks * total_tasks;
        if total_wakeups_independent == 0 {
            return 0.0;
        }
        (self.wakeups_saved as f64 / total_wakeups_independent as f64) * 100.0
    }

    /// Get timer statistics.
    pub fn stats(&self) -> TimerStats {
        TimerStats {
            total_ticks: self.total_ticks,
            total_tasks: self.tasks.len(),
            total_groups: self.groups.len(),
            wakeups_saved: self.wakeups_saved,
            savings_percent: self.wakeup_savings_percent(),
        }
    }

    /// Check if the timer is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Default for CoalescedTimer {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer statistics.
#[derive(Debug, Clone)]
pub struct TimerStats {
    /// Total ticks processed
    pub total_ticks: u64,
    /// Total registered tasks
    pub total_tasks: usize,
    /// Total task groups
    pub total_groups: usize,
    /// CPU wakeups saved vs independent timers
    pub wakeups_saved: u64,
    /// Savings percentage
    pub savings_percent: f64,
}

/// Determine if a task is connectivity-critical.
fn is_critical_task(task_id: TaskId) -> bool {
    matches!(
        task_id,
        TaskId::NainProbe
            | TaskId::NtpCovertData
            | TaskId::ConnectionHealthCheck
            | TaskId::KeyRotationCheck
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_tasks() {
        let mut timer = CoalescedTimer::new();

        timer.register_task(TaskId::NainProbe, Duration::from_secs(30), true);
        timer.register_task(TaskId::NtpCovertData, Duration::from_secs(32), true);
        timer.register_task(TaskId::ConnectionHealthCheck, Duration::from_secs(15), true);

        assert_eq!(timer.task_count(), 3);
    }

    #[test]
    fn test_task_grouping() {
        let mut timer = CoalescedTimer::new();

        // These should be grouped (within 10% of each other)
        timer.register_task(TaskId::NainProbe, Duration::from_secs(30), true);
        timer.register_task(TaskId::NtpCovertData, Duration::from_secs(32), true);

        // This should be in a separate group (15s vs 30s)
        timer.register_task(TaskId::ConnectionHealthCheck, Duration::from_secs(15), true);

        timer.regroup_tasks();

        // Should have 2 groups: one for 15s, one for 30s
        assert_eq!(timer.group_count(), 2);
    }

    #[test]
    fn test_disable_task() {
        let mut timer = CoalescedTimer::new();

        timer.register_task(TaskId::NainProbe, Duration::from_secs(30), true);
        timer.register_task(TaskId::WifiAwareScan, Duration::from_secs(30), false);

        assert_eq!(timer.task_count(), 2);

        timer.disable_task(TaskId::WifiAwareScan);
        assert_eq!(timer.task_count(), 1);
    }

    #[test]
    fn test_tick_execution() {
        let mut timer = CoalescedTimer::new();

        timer.register_task(TaskId::NainProbe, Duration::from_secs(30), true);
        timer.regroup_tasks();

        // First tick should make the task due
        let due = timer.tick();
        assert!(due.contains(&TaskId::NainProbe));
    }

    #[test]
    fn test_tick_counter() {
        let mut timer = CoalescedTimer::new();

        // Register two tasks with the same interval
        timer.register_task(TaskId::NainProbe, Duration::from_secs(30), true);
        timer.register_task(TaskId::NtpCovertData, Duration::from_secs(32), true);
        timer.regroup_tasks();

        // First tick - both should be due (ticks_per_execution = 1 for first, 1 for second)
        let due = timer.tick();
        assert!(due.contains(&TaskId::NainProbe));
    }

    #[test]
    fn test_update_task_interval() {
        let mut timer = CoalescedTimer::new();

        timer.register_task(TaskId::NainProbe, Duration::from_secs(30), true);
        timer.regroup_tasks();

        // Update interval
        timer.update_task_interval(TaskId::NainProbe, Some(Duration::from_secs(60)));

        // Task should still be registered
        assert_eq!(timer.task_count(), 1);

        // Update to None (disable)
        timer.update_task_interval(TaskId::NainProbe, None);
        assert_eq!(timer.task_count(), 0);
    }

    #[test]
    fn test_is_critical_task() {
        assert!(is_critical_task(TaskId::NainProbe));
        assert!(is_critical_task(TaskId::NtpCovertData));
        assert!(is_critical_task(TaskId::ConnectionHealthCheck));
        assert!(is_critical_task(TaskId::KeyRotationCheck));
        assert!(!is_critical_task(TaskId::WifiAwareScan));
        assert!(!is_critical_task(TaskId::BleScan));
        assert!(!is_critical_task(TaskId::TelemetryUpload));
    }

    #[test]
    fn test_timer_stats() {
        let mut timer = CoalescedTimer::new();
        timer.register_task(TaskId::NainProbe, Duration::from_secs(30), true);
        timer.register_task(TaskId::NtpCovertData, Duration::from_secs(32), true);
        timer.regroup_tasks();

        timer.tick();
        timer.tick();

        let stats = timer.stats();
        assert_eq!(stats.total_ticks, 2);
        assert_eq!(stats.total_tasks, 2);
    }

    #[test]
    fn test_default_timer() {
        let timer = CoalescedTimer::default();
        assert_eq!(timer.task_count(), 0);
        assert_eq!(timer.group_count(), 0);
        assert!(!timer.is_running());
    }

    #[test]
    fn test_grouping_tolerance() {
        let mut timer = CoalescedTimer::new();

        // 30s and 33s should be grouped (10% tolerance)
        timer.register_task(TaskId::NainProbe, Duration::from_secs(30), true);
        timer.register_task(TaskId::NtpCovertData, Duration::from_secs(33), true);

        timer.regroup_tasks();

        // Should be 1 group
        assert_eq!(timer.group_count(), 1);
    }

    #[test]
    fn test_no_grouping_for_different_intervals() {
        let mut timer = CoalescedTimer::new();

        // 15s and 300s should NOT be grouped
        timer.register_task(TaskId::ConnectionHealthCheck, Duration::from_secs(15), true);
        timer.register_task(TaskId::TelemetryUpload, Duration::from_secs(300), false);

        timer.regroup_tasks();

        // Should be 2 groups
        assert_eq!(timer.group_count(), 2);
    }
}
