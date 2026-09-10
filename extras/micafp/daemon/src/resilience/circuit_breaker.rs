// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Circuit Breaker
//
// Standard circuit breaker pattern (Closed → Open → Half-Open).
// Prevents cascade failures when a transport is experiencing repeated failures.
//
// States:
//   Closed     — normal operation, all requests pass through
//   Open       — transport is down, requests fail fast (no connection attempts)
//   Half-Open  — probe mode: one request allowed to test recovery
// ─────────────────────────────────────────────────────────────────────────────

use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum BreakerState {
    Closed,
    Open,
    HalfOpen,
}

struct BreakerInner {
    state: BreakerState,
    failure_count: u32,
    last_failure: Option<Instant>,
    last_success: Option<Instant>,
}

/// Per-transport circuit breaker.
pub struct CircuitBreaker {
    name: String,
    failure_threshold: u32,
    recovery_timeout: Duration,
    inner: Mutex<BreakerInner>,
}

impl CircuitBreaker {
    pub fn new(
        name: impl Into<String>,
        failure_threshold: u32,
        recovery_timeout: Duration,
    ) -> Self {
        Self {
            name: name.into(),
            failure_threshold,
            recovery_timeout,
            inner: Mutex::new(BreakerInner {
                state: BreakerState::Closed,
                failure_count: 0,
                last_failure: None,
                last_success: None,
            }),
        }
    }

    /// Returns true if a request should be allowed through.
    pub fn allow_request(&self) -> bool {
        let mut inner = self.inner.lock();
        match inner.state {
            BreakerState::Closed => true,
            BreakerState::Open => {
                // Check if recovery timeout has elapsed
                if let Some(lf) = inner.last_failure {
                    if lf.elapsed() >= self.recovery_timeout {
                        inner.state = BreakerState::HalfOpen;
                        info!("circuit_breaker [{}]: Open → HalfOpen", self.name);
                        true
                    } else {
                        false // still open
                    }
                } else {
                    false
                }
            }
            BreakerState::HalfOpen => true,
        }
    }

    /// Record a successful operation.
    pub fn record_success(&self) {
        let mut inner = self.inner.lock();
        if inner.state == BreakerState::HalfOpen {
            info!(
                "circuit_breaker [{}]: HalfOpen → Closed (recovered)",
                self.name
            );
        }
        inner.state = BreakerState::Closed;
        inner.failure_count = 0;
        inner.last_success = Some(Instant::now());
    }

    /// Record a failed operation.
    pub fn record_failure(&self) {
        let mut inner = self.inner.lock();
        inner.failure_count += 1;
        inner.last_failure = Some(Instant::now());

        if inner.failure_count >= self.failure_threshold && inner.state == BreakerState::Closed {
            inner.state = BreakerState::Open;
            warn!(
                "circuit_breaker [{}]: Closed → Open (failures: {})",
                self.name, inner.failure_count
            );
        } else if inner.state == BreakerState::HalfOpen {
            inner.state = BreakerState::Open;
            warn!(
                "circuit_breaker [{}]: HalfOpen → Open (probe failed)",
                self.name
            );
        }
    }

    pub fn state(&self) -> BreakerState {
        self.inner.lock().state
    }

    pub fn failure_count(&self) -> u32 {
        self.inner.lock().failure_count
    }
}
