// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Retry Policy
// Exponential backoff with full jitter for connection retries.
// ─────────────────────────────────────────────────────────────────────────────

use rand::Rng;
use std::time::Duration;

/// Retry policy configuration.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 500,
            max_delay_ms: 30_000,
            multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Compute the delay before attempt `n` (0-indexed).
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base = (self.initial_delay_ms as f64 * self.multiplier.powi(attempt as i32)) as u64;
        let capped = base.min(self.max_delay_ms);

        let final_ms = if self.jitter {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..=capped)
        } else {
            capped
        };
        Duration::from_millis(final_ms)
    }

    /// Execute an async closure with retry logic.
    pub async fn execute<F, Fut, T, E>(&self, mut f: F) -> Result<T, E>
    where
        F: FnMut(u32) -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        for attempt in 0..self.max_attempts {
            match f(attempt).await {
                Ok(val) => return Ok(val),
                Err(e) => {
                    if attempt + 1 >= self.max_attempts {
                        return Err(e);
                    }
                    let delay = self.delay_for_attempt(attempt);
                    tokio::time::sleep(delay).await;
                }
            }
        }
        unreachable!()
    }
}
