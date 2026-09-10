//! Adaptive Scan Budget and Scheduler
//!
//! Dynamically scales scanning breadth, probe concurrency, timeouts, and intervals
//! based on battery level, thermal throttling, network quality, yield rates,
//! and network severance states.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info};

use crate::scanner::network_assessor::NetworkState;

/// Scanning intensity and exploration level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ScanLevel {
    /// Extremely low power probe for background health checks when connectivity is solid
    MicroProbe,
    /// Targeted probing of high-reputation candidates
    Focused,
    /// Balanced dynamic exploration and exploitation (default)
    #[default]
    Adaptive,
    /// Broad sweep across candidate subnets when candidate pool drops below threshold
    Broad,
    /// Exhaustive multi-protocol exploration when facing aggressive filtering
    Deep,
    /// Emergency burst recovery sweep when current active connection is severed
    RecoverySweep,
    /// Domestic-only localized sweep during national intranet isolation (NIN)
    BlackoutRecovery,
}

/// Host system resource budget
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ResourceBudget {
    pub battery_percent: u8,
    pub is_charging: bool,
    pub is_thermal_throttled: bool,
    pub is_metered: bool,
    pub cpu_load_factor: f32, // 0.0 to 1.0
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            battery_percent: 85,
            is_charging: true,
            is_thermal_throttled: false,
            is_metered: false,
            cpu_load_factor: 0.2,
        }
    }
}

/// Runtime parameters calculated for the scan batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicScanPlan {
    pub level: ScanLevel,
    pub max_concurrency: usize,
    pub probe_timeout: Duration,
    pub inter_probe_jitter_min_ms: u64,
    pub inter_probe_jitter_max_ms: u64,
    pub max_candidates_per_batch: usize,
    pub probe_quic: bool,
    pub probe_tls_fragmentation: bool,
    pub target_domestic_only: bool,
}

/// Adaptive Scan Scheduler & Budget Manager
pub struct AdaptiveScanScheduler {
    current_level: ScanLevel,
    resource_budget: ResourceBudget,
}

impl AdaptiveScanScheduler {
    pub fn new() -> Self {
        Self {
            current_level: ScanLevel::Adaptive,
            resource_budget: ResourceBudget::default(),
        }
    }

    pub fn update_resource_budget(&mut self, budget: ResourceBudget) {
        self.resource_budget = budget;
    }

    /// Automatically decide optimal ScanLevel based on network state, yield, and failures
    pub fn calculate_optimal_level(
        &mut self,
        network_state: NetworkState,
        verified_count: usize,
        recent_failure_burst: bool,
        is_active_connected: bool,
    ) -> ScanLevel {
        let level = match network_state {
            NetworkState::FullSeverance => ScanLevel::RecoverySweep,
            NetworkState::DomesticOnly => ScanLevel::BlackoutRecovery,
            NetworkState::Degraded => {
                if verified_count < 3 || recent_failure_burst {
                    ScanLevel::Deep
                } else {
                    ScanLevel::Broad
                }
            }
            NetworkState::Normal => {
                if !is_active_connected {
                    if verified_count < 2 {
                        ScanLevel::Broad
                    } else {
                        ScanLevel::Focused
                    }
                } else if verified_count >= 5 {
                    // Energy conservation mode
                    if !self.resource_budget.is_charging
                        && self.resource_budget.battery_percent < 30
                    {
                        ScanLevel::MicroProbe
                    } else {
                        ScanLevel::Focused
                    }
                } else {
                    ScanLevel::Adaptive
                }
            }
        };

        // Downgrade if thermal throttled or critically low battery (unless in emergency recovery)
        let constrained_level = if (self.resource_budget.is_thermal_throttled
            || (!self.resource_budget.is_charging && self.resource_budget.battery_percent < 15))
            && level != ScanLevel::RecoverySweep
            && level != ScanLevel::BlackoutRecovery
        {
            ScanLevel::MicroProbe
        } else {
            level
        };

        if self.current_level != constrained_level {
            info!(
                "Adaptive scanner transitioning scan level: {:?} -> {:?}",
                self.current_level, constrained_level
            );
            self.current_level = constrained_level;
        }

        self.current_level
    }

    /// Compute concrete execution parameters for the active level and resource profile
    pub fn generate_plan(&self) -> DynamicScanPlan {
        let (concurrency, timeout, batch_size, j_min, j_max, quic, frag, domestic_only) =
            match self.current_level {
                ScanLevel::MicroProbe => (
                    2,
                    Duration::from_millis(1500),
                    4,
                    100,
                    300,
                    false,
                    false,
                    false,
                ),
                ScanLevel::Focused => (
                    4,
                    Duration::from_millis(1800),
                    8,
                    50,
                    150,
                    true,
                    false,
                    false,
                ),
                ScanLevel::Adaptive => (
                    8,
                    Duration::from_millis(2200),
                    16,
                    30,
                    100,
                    true,
                    true,
                    false,
                ),
                ScanLevel::Broad => (
                    12,
                    Duration::from_millis(2500),
                    28,
                    20,
                    80,
                    true,
                    true,
                    false,
                ),
                ScanLevel::Deep => (
                    16,
                    Duration::from_millis(3000),
                    40,
                    15,
                    60,
                    true,
                    true,
                    false,
                ),
                ScanLevel::RecoverySweep => (
                    20,
                    Duration::from_millis(2000),
                    50,
                    10,
                    40,
                    true,
                    true,
                    false,
                ),
                ScanLevel::BlackoutRecovery => (
                    6,
                    Duration::from_millis(2000),
                    12,
                    40,
                    120,
                    false,
                    false,
                    true,
                ),
            };

        // Scale concurrency down if CPU or battery pressure is high
        let adjusted_concurrency =
            if !self.resource_budget.is_charging && self.resource_budget.battery_percent < 25 {
                (concurrency / 2).max(1)
            } else {
                concurrency
            };

        DynamicScanPlan {
            level: self.current_level,
            max_concurrency: adjusted_concurrency,
            probe_timeout: timeout,
            inter_probe_jitter_min_ms: j_min,
            inter_probe_jitter_max_ms: j_max,
            max_candidates_per_batch: batch_size,
            probe_quic: quic,
            probe_tls_fragmentation: frag,
            target_domestic_only: domestic_only,
        }
    }
}
