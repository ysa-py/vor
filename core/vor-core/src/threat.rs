//! Threat-level ladder for obfuscation escalation.
//!
//! Ported from MICAFP `daemon/src/obfuscation/mod.rs` (ObfuscationCoordinator):
//! `None -> Passive -> ActiveV1 -> ActiveV2 -> CompleteBlackout`, with the
//! same three-layer policy:
//!
//! * Layer 1 — TLS fragmentation: always-on for Iranian ISPs.
//! * Layer 2 — traffic shaping: only when DPI is detected.
//! * Layer 3 — aggressive transforms: only under `ActiveV2`/blackout.

use serde::{Deserialize, Serialize};

/// Detected censorship threat level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ThreatLevel {
    /// No DPI detected; obfuscation optional.
    None = 0,
    /// Passive DPI (reads but does not block).
    Passive = 1,
    /// Active DPI with known fingerprinting (FAVA v1: RST ~95–320 ms).
    ActiveV1 = 2,
    /// Active DPI with advanced fingerprinting (FAVA v2: SNI-aware RST).
    ActiveV2 = 3,
    /// Complete blackout — all tunnels blocked; emergency mode.
    CompleteBlackout = 4,
}

impl ThreatLevel {
    /// Parse from the wire string used in JSON state.
    pub fn from_str_value(value: &str) -> ThreatLevel {
        match value {
            "passive" => ThreatLevel::Passive,
            "active_v1" | "active-v1" | "ActiveV1" => ThreatLevel::ActiveV1,
            "active_v2" | "active-v2" | "ActiveV2" => ThreatLevel::ActiveV2,
            "blackout" | "complete_blackout" | "CompleteBlackout" => ThreatLevel::CompleteBlackout,
            _ => ThreatLevel::None,
        }
    }

    /// Wire string used in JSON state.
    pub fn as_str_value(&self) -> &'static str {
        match self {
            ThreatLevel::None => "none",
            ThreatLevel::Passive => "passive",
            ThreatLevel::ActiveV1 => "active_v1",
            ThreatLevel::ActiveV2 => "active_v2",
            ThreatLevel::CompleteBlackout => "blackout",
        }
    }

    /// Obfuscation required at this threat level?
    pub fn requires_obfuscation(&self) -> bool {
        !matches!(self, ThreatLevel::None)
    }

    /// Aggressive (all-strategies) obfuscation needed?
    pub fn requires_aggressive(&self) -> bool {
        matches!(self, ThreatLevel::ActiveV2 | ThreatLevel::CompleteBlackout)
    }

    /// Layer 1 (fragmentation) enabled? Always on from Passive upward —
    /// and even at `None` for Iranian-ISP traffic per MICAFP policy.
    pub fn layer1_fragment(&self, is_iranian_traffic: bool) -> bool {
        is_iranian_traffic || self.requires_obfuscation()
    }

    /// Layer 2 (traffic shaping) enabled?
    pub fn layer2_shaper(&self, isp_needs_shaping: bool) -> bool {
        match self {
            ThreatLevel::None => false,
            ThreatLevel::Passive => isp_needs_shaping,
            _ => true,
        }
    }

    /// Layer 3 (aggressive transforms) enabled?
    pub fn layer3_aggressive(&self) -> bool {
        self.requires_aggressive()
    }

    /// Minimum hop count for multi-hop chains at this threat level
    /// (MICAFP multihop policy).
    pub fn min_hops(&self) -> usize {
        match self {
            ThreatLevel::None | ThreatLevel::Passive => 2,
            ThreatLevel::ActiveV1 => 3,
            ThreatLevel::ActiveV2 => 4,
            ThreatLevel::CompleteBlackout => 5,
        }
    }
}

/// Escalate the threat level from live DPI observations (heuristic port of
/// MICAFP's detection inputs): consecutive blocked connections, RST storms,
/// DNS poison answers and SNI-filter events raise the level; a window of
/// clean traffic lowers it one step.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ThreatObservation {
    /// Connections in the last window that were reset after ClientHello.
    pub rst_after_hello: u32,
    /// Connections that completed fine in the last window.
    pub clean_connections: u32,
    /// DNS answers observed from known poison IPs (10.10.34.34/35).
    pub dns_poison_answers: u32,
    /// HTTP 403-with-RST censorship responses observed.
    pub http_block_responses: u32,
    /// SNI filter events (connection reset immediately after SNI send).
    pub sni_filter_events: u32,
}

impl ThreatObservation {
    /// Compute the threat level implied by this observation window.
    /// Thresholds follow MICAFP's documented FAVA timings/signatures.
    pub fn level(&self) -> ThreatLevel {
        let blocking_signals = self.rst_after_hello + self.sni_filter_events;
        let total = blocking_signals + self.clean_connections.max(1);
        if self.dns_poison_answers >= 3 || self.http_block_responses >= 5 {
            return ThreatLevel::CompleteBlackout;
        }
        if blocking_signals == 0 && self.dns_poison_answers == 0 && self.http_block_responses == 0 {
            return ThreatLevel::None;
        }
        let block_ratio = blocking_signals as f64 / total as f64;
        if block_ratio >= 0.8 {
            ThreatLevel::CompleteBlackout
        } else if self.sni_filter_events > 0 || block_ratio >= 0.5 {
            ThreatLevel::ActiveV2
        } else if blocking_signals > 0 {
            ThreatLevel::ActiveV1
        } else {
            ThreatLevel::Passive
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder_ordering() {
        assert!(ThreatLevel::None < ThreatLevel::CompleteBlackout);
        assert_eq!(ThreatLevel::ActiveV2.min_hops(), 4);
    }

    #[test]
    fn clean_traffic_is_none() {
        let obs = ThreatObservation {
            clean_connections: 50,
            ..Default::default()
        };
        assert_eq!(obs.level(), ThreatLevel::None);
    }

    #[test]
    fn sni_filtering_is_active_v2() {
        let obs = ThreatObservation {
            clean_connections: 2,
            sni_filter_events: 2,
            ..Default::default()
        };
        assert_eq!(obs.level(), ThreatLevel::ActiveV2);
    }

    #[test]
    fn dns_poison_is_blackout() {
        let obs = ThreatObservation {
            dns_poison_answers: 4,
            clean_connections: 10,
            ..Default::default()
        };
        assert_eq!(obs.level(), ThreatLevel::CompleteBlackout);
    }

    #[test]
    fn layers_policy() {
        assert!(ThreatLevel::ActiveV1.layer1_fragment(false));
        assert!(!ThreatLevel::None.layer1_fragment(false));
        assert!(ThreatLevel::None.layer1_fragment(true)); // Iranian traffic: always fragment
        assert!(ThreatLevel::ActiveV2.layer3_aggressive());
        assert!(!ThreatLevel::ActiveV1.layer3_aggressive());
    }

    #[test]
    fn wire_roundtrip() {
        for level in [
            ThreatLevel::None,
            ThreatLevel::Passive,
            ThreatLevel::ActiveV1,
            ThreatLevel::ActiveV2,
            ThreatLevel::CompleteBlackout,
        ] {
            assert_eq!(ThreatLevel::from_str_value(level.as_str_value()), level);
        }
    }
}
