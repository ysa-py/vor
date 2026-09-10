//! Traffic-shaping decisions derived from the adversarial traffic profiles.
//!
//! Ported from MICAFP `traffic_shaper.rs` policy: shape traffic so its
//! statistical profile matches a cover class (video / download / browsing).
//! This module decides *how much to pad* and *how long to delay*; the
//! platforms apply the decisions to their packet paths.

use crate::features::Features;
use serde::{Deserialize, Serialize};

/// Padding policy steps (MICAFP adaptive escalation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ShaperLevel {
    /// No shaping; pass through.
    Passthrough = 0,
    /// Light: pad small packets only.
    Light = 1,
    /// Full: pad to the target distribution + IAT jitter.
    Full = 2,
    /// Maximum: pad + jitter + burst shaping.
    Maximum = 3,
}

/// One cover-class target loaded from `data/traffic-profiles.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficProfile {
    /// Profile name (youtube_hd, netflix_4k, …).
    pub name: String,
    /// Human label.
    #[serde(default)]
    pub label: String,
    /// Target packet size stats.
    pub packet_size: PacketSizeStats,
    /// Target inter-arrival stats.
    pub inter_arrival_us: InterArrivalStats,
}

/// Packet-size target statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketSizeStats {
    /// Mean packet size.
    pub mean: f64,
    /// Stddev.
    #[serde(default)]
    pub stddev: f64,
    /// Minimum.
    #[serde(default)]
    pub min: f64,
    /// Maximum.
    #[serde(default)]
    pub max: f64,
}

/// Inter-arrival target statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterArrivalStats {
    /// Mean inter-arrival in microseconds.
    pub mean: f64,
    /// Stddev.
    #[serde(default)]
    pub stddev: f64,
}

/// Parse the embedded traffic profiles.
pub fn traffic_profiles() -> Vec<TrafficProfile> {
    match serde_json::from_str::<serde_json::Value>(crate::TRAFFIC_PROFILES_JSON) {
        Ok(serde_json::Value::Object(root)) => {
            match root.get("profiles").and_then(|p| p.as_array()) {
                Some(profiles) => profiles
                    .iter()
                    .filter_map(|p| serde_json::from_value(p.clone()).ok())
                    .collect(),
                None => vec![],
            }
        }
        _ => vec![],
    }
}

/// Shaping decision for the next packet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapingDecision {
    /// Bytes of padding to append to the next outgoing packet.
    pub pad_bytes: usize,
    /// Additional delay before sending (ms).
    pub extra_delay_ms: u32,
}

/// Decide padding + delay for the next packet given the live features, the
/// target profile, and the shaper level.
///
/// Policy (from MICAFP):
/// * `Passthrough` — never pad.
/// * `Light` — pad only when the packet is far below the target mean.
/// * `Full` — pad toward the mean and add distribution-aware jitter.
/// * `Maximum` — additionally regularize inter-arrival toward the target.
pub fn decide(
    features: &Features,
    profile: &TrafficProfile,
    level: ShaperLevel,
) -> ShapingDecision {
    if level == ShaperLevel::Passthrough {
        return ShapingDecision {
            pad_bytes: 0,
            extra_delay_ms: 0,
        };
    }

    let current = features.size_mean;
    let target = profile.packet_size.mean;
    let gap = target - current;

    let pad_bytes = if gap > 8.0 {
        let raw = match level {
            ShaperLevel::Light => (gap / 2.0).min(256.0),
            ShaperLevel::Full => gap.min(512.0),
            ShaperLevel::Maximum => gap.min(1024.0),
            ShaperLevel::Passthrough => 0.0,
        };
        raw as usize
    } else {
        0
    };

    // IAT regularization (Maximum only): push mean IAT toward the target.
    let extra_delay_ms = if level == ShaperLevel::Maximum {
        let target_ms = profile.inter_arrival_us.mean / 1000.0;
        let diff = target_ms - features.iat_mean_ms;
        if diff > 1.0 {
            (diff / 4.0).min(25.0) as u32
        } else {
            0
        }
    } else {
        0
    };

    ShapingDecision {
        pad_bytes,
        extra_delay_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::TrafficWindow;

    fn profile() -> TrafficProfile {
        traffic_profiles()
            .into_iter()
            .find(|p| p.name == "web_browsing")
            .expect("web_browsing profile")
    }

    #[test]
    fn embedded_profiles_parse() {
        assert!(traffic_profiles().len() >= 4);
    }

    #[test]
    fn passthrough_never_shapes() {
        let features = Features::default();
        let decision = decide(&features, &profile(), ShaperLevel::Passthrough);
        assert_eq!(decision.pad_bytes, 0);
        assert_eq!(decision.extra_delay_ms, 0);
    }

    #[test]
    fn small_packets_get_padded_toward_target() {
        let mut window = TrafficWindow::default();
        for i in 0..20 {
            window.push(i as f64 * 5.0, 80, 0x17);
        }
        let features = window.features();
        assert!(features.size_mean < 200.0);
        let light = decide(&features, &profile(), ShaperLevel::Light);
        let full = decide(&features, &profile(), ShaperLevel::Full);
        let maximum = decide(&features, &profile(), ShaperLevel::Maximum);
        assert!(light.pad_bytes > 0);
        assert!(full.pad_bytes >= light.pad_bytes);
        assert!(maximum.pad_bytes >= full.pad_bytes);
    }

    #[test]
    fn matching_traffic_is_not_padded() {
        let mut window = TrafficWindow::default();
        for i in 0..20 {
            window.push(i as f64 * 40.0, 640, 0x17);
        }
        let features = window.features();
        let decision = decide(&features, &profile(), ShaperLevel::Full);
        assert_eq!(decision.pad_bytes, 0);
    }
}
