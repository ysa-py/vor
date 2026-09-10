// ─────────────────────────────────────────────────────────────────────────────
// Battery / power management subsystem
// MICAFP-UnifiedShield-vip-ultra-Quantum-ultra v8.0
// ─────────────────────────────────────────────────────────────────────────────

pub mod adaptive_duty;
pub mod coalesced_timer;
pub mod optimizer;
pub mod power_state;

pub use adaptive_duty::{AdaptiveDutyCycler, PowerMode, TaskDutyTable, TaskId};
pub use coalesced_timer::CoalescedTimer;
pub use power_state::PowerState;

use std::fmt;

/// Battery-specific error type.
#[derive(Debug, Clone)]
pub struct BatteryError(pub String);

impl fmt::Display for BatteryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Battery error: {}", self.0)
    }
}

impl std::error::Error for BatteryError {}
