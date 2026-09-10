//! National-Intranet (NAIN) detection heuristic.
//!
//! Ported from MICAFP `daemon/src/national_intranet/` detection concepts:
//! infer when the device is inside the national intranet / total blackout
//! from probe outcomes, and adjust probe cadence to the screen state
//! (30 s active / 120 s background / 600 s idle) exactly like upstream.

use serde::{Deserialize, Serialize};

/// Probe cadence in seconds by device state (upstream NAIN constants).
pub const PROBE_INTERVAL_ACTIVE_S: u32 = 30;
/// Background probe interval.
pub const PROBE_INTERVAL_BACKGROUND_S: u32 = 120;
/// Idle probe interval.
pub const PROBE_INTERVAL_IDLE_S: u32 = 600;

/// Latest global connectivity probes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectivityProbe {
    /// Whether an international site was reachable.
    pub international_reachable: bool,
    /// Whether domestic (national intranet) sites were reachable.
    pub domestic_reachable: bool,
    /// Whether any DNS answer came from known poison IPs.
    pub dns_poisoned: bool,
}

/// NAIN state classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NainState {
    /// Normal international connectivity.
    Normal,
    /// Partial: some international reachability, interference detected.
    Degraded,
    /// National intranet only (domestic works, international fails).
    Intranet,
    /// Total blackout.
    Blackout,
}

impl NainState {
    /// Wire name.
    pub fn as_str(&self) -> &'static str {
        match self {
            NainState::Normal => "normal",
            NainState::Degraded => "degraded",
            NainState::Intranet => "intranet",
            NainState::Blackout => "blackout",
        }
    }
}

/// Classify the current NAIN state from the latest probes.
pub fn classify(probe: &ConnectivityProbe, consecutive_intl_failures: u32) -> NainState {
    if probe.domestic_reachable && probe.international_reachable && !probe.dns_poisoned {
        return NainState::Normal;
    }
    if !probe.domestic_reachable {
        return NainState::Blackout;
    }
    if !probe.international_reachable {
        if consecutive_intl_failures >= 3 {
            NainState::Intranet
        } else {
            NainState::Degraded
        }
    } else if probe.dns_poisoned {
        NainState::Degraded
    } else {
        NainState::Normal
    }
}

/// Recommended probe interval for the device state.
pub fn probe_interval_s(screen_on: bool, app_in_foreground: bool) -> u32 {
    match (screen_on, app_in_foreground) {
        (true, true) => PROBE_INTERVAL_ACTIVE_S,
        (true, false) => PROBE_INTERVAL_BACKGROUND_S,
        (false, _) => PROBE_INTERVAL_IDLE_S,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_when_everything_works() {
        let probe = ConnectivityProbe {
            international_reachable: true,
            domestic_reachable: true,
            dns_poisoned: false,
        };
        assert_eq!(classify(&probe, 0), NainState::Normal);
    }

    #[test]
    fn intranet_when_only_domestic_works() {
        let probe = ConnectivityProbe {
            international_reachable: false,
            domestic_reachable: true,
            dns_poisoned: true,
        };
        assert_eq!(classify(&probe, 5), NainState::Intranet);
    }

    #[test]
    fn degraded_before_three_failures() {
        let probe = ConnectivityProbe {
            international_reachable: false,
            domestic_reachable: true,
            dns_poisoned: false,
        };
        assert_eq!(classify(&probe, 1), NainState::Degraded);
        assert_eq!(classify(&probe, 3), NainState::Intranet);
    }

    #[test]
    fn blackout_when_nothing_works() {
        let probe = ConnectivityProbe::default();
        assert_eq!(classify(&probe, 10), NainState::Blackout);
    }

    #[test]
    fn cadence_matches_upstream() {
        assert_eq!(probe_interval_s(true, true), 30);
        assert_eq!(probe_interval_s(true, false), 120);
        assert_eq!(probe_interval_s(false, true), 600);
        assert_eq!(probe_interval_s(false, false), 600);
    }
}
