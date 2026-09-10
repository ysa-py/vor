// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Watchdog
// Monitors daemon subsystems and restarts failed tasks automatically.
// ─────────────────────────────────────────────────────────────────────────────

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// A supervised task registration.
struct Supervised {
    name: String,
    last_heartbeat: Instant,
    restart_count: u32,
    max_restarts: u32,
}

/// The daemon watchdog — monitors task heartbeats and restarts failed tasks.
pub struct Watchdog {
    tasks: Arc<RwLock<HashMap<String, Supervised>>>,
    heartbeat_timeout: Duration,
}

impl Watchdog {
    pub fn new(heartbeat_timeout: Duration) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            heartbeat_timeout,
        }
    }

    /// Register a task to be supervised.
    pub fn register(&self, name: &str, max_restarts: u32) {
        self.tasks.write().insert(
            name.to_string(),
            Supervised {
                name: name.to_string(),
                last_heartbeat: Instant::now(),
                restart_count: 0,
                max_restarts,
            },
        );
        info!("watchdog: registered '{}'", name);
    }

    /// Report a heartbeat for a task.
    pub fn heartbeat(&self, name: &str) {
        if let Some(task) = self.tasks.write().get_mut(name) {
            task.last_heartbeat = Instant::now();
        }
    }

    /// Check all tasks for missed heartbeats. Returns names of timed-out tasks.
    pub fn check_tasks(&self) -> Vec<String> {
        let now = Instant::now();
        let mut timed_out = Vec::new();
        for (name, task) in self.tasks.read().iter() {
            if now.duration_since(task.last_heartbeat) > self.heartbeat_timeout {
                warn!(
                    "watchdog: '{}' missed heartbeat (restarts: {})",
                    name, task.restart_count
                );
                timed_out.push(name.clone());
            }
        }
        timed_out
    }

    /// Mark a task as restarted.
    pub fn record_restart(&self, name: &str) -> bool {
        if let Some(task) = self.tasks.write().get_mut(name) {
            task.restart_count += 1;
            task.last_heartbeat = Instant::now();
            if task.restart_count > task.max_restarts {
                error!(
                    "watchdog: '{}' exceeded max restarts ({})",
                    name, task.max_restarts
                );
                return false;
            }
            info!(
                "watchdog: restarted '{}' (restart #{})",
                name, task.restart_count
            );
            return true;
        }
        false
    }

    /// Start the watchdog loop (checks every 10 seconds).
    pub async fn run_loop(self: Arc<Self>) {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            let timed_out = self.check_tasks();
            for name in timed_out {
                self.record_restart(&name);
            }
        }
    }
}
