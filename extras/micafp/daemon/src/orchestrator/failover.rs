// Failover Engine — executes rapid transport switching on health degradation.

use std::time::{Duration, Instant};
use tracing::{info, warn};

pub struct FailoverEngine {
    pub timeout: Duration,
    pub attempts: u32,
    pub last_failover: Option<Instant>,
    pub cooldown: Duration,
}

impl FailoverEngine {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            attempts: 0,
            last_failover: None,
            cooldown: Duration::from_secs(5),
        }
    }

    /// Returns true if a failover is allowed (respects cooldown).
    pub fn can_failover(&self) -> bool {
        match self.last_failover {
            None => true,
            Some(t) => t.elapsed() > self.cooldown,
        }
    }

    /// Record a failover event and update cooldown.
    pub fn record_failover(&mut self, from: &str, to: &str) {
        self.attempts += 1;
        self.last_failover = Some(Instant::now());
        info!(
            from,
            to,
            attempt = self.attempts,
            "Transport failover executed"
        );
    }
}
