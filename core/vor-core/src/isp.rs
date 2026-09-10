//! Iranian ISP / carrier fingerprinting and profile lookup.
//!
//! Data is the shared JSON embedded from `data/isp-profiles.json` (MICAFP,
//! 11 ISP profiles with ASN, blocked/working protocols, DPI intensity) plus
//! `data/carrier-presets.json` (UAC carrier tuning: edges, fake SNI,
//! finalmask parameters, fragment strategy matrix).

use crate::fragment::Strategy;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parsed ISP profile (subset of the MICAFP schema that Vor uses).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IspProfile {
    /// English display name.
    pub name_en: String,
    /// Persian display name.
    #[serde(default)]
    pub name_fa: String,
    /// Primary ASN ("AS41689").
    pub asn: String,
    /// DPI intensity: low | medium | high | very-high.
    #[serde(default = "default_intensity")]
    pub dpi_intensity: String,
    /// Protocols known blocked on this ISP.
    #[serde(default)]
    pub blocked_protocols: Vec<String>,
    /// Protocols known working on this ISP.
    #[serde(default)]
    pub working_protocols: Vec<String>,
    /// Recommended strategy per MICAFP FAVA mapping.
    #[serde(default)]
    pub default_strategy: String,
    /// Traffic shaping recommended?
    #[serde(default)]
    pub needs_traffic_shaping: bool,
}

fn default_intensity() -> String {
    "medium".to_string()
}

/// Parsed carrier preset (UAC tuning model).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarrierPreset {
    /// Carrier key: mci | irancell | rightel | auto.
    pub key: String,
    /// ASNs that map to this carrier.
    #[serde(default)]
    pub match_asns: Vec<String>,
    /// English display name.
    pub name_en: String,
    /// Persian display name.
    #[serde(default)]
    pub name_fa: String,
    /// DPI flavor: fava-v1 | fava-v2 | light | unknown.
    #[serde(default)]
    pub dpi: String,
    /// Default fragment strategy name.
    #[serde(default)]
    pub default_strategy: String,
    /// Fake SNI used for disposable probes.
    #[serde(default)]
    pub fake_sni: String,
    /// Candidate fragment strategy names, best-first.
    #[serde(default)]
    pub fragment_strategies: Vec<String>,
    /// Cloudflare edge IPs to substitute under SNI pressure.
    #[serde(default)]
    pub edges: Vec<CarrierEdge>,
    /// Recommended TUN MTU.
    #[serde(default = "default_mtu")]
    pub tun_mtu: u32,
    /// Traffic shaping recommended for this carrier.
    #[serde(default)]
    pub needs_traffic_shaping: bool,
}

fn default_mtu() -> u32 {
    1280
}

/// One carrier edge endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarrierEdge {
    /// Edge IPv4.
    pub ip: String,
    /// Edge port (443).
    pub port: u16,
    /// maxSplit for the finalmask config.
    #[serde(default = "default_max_split")]
    pub max_split: u32,
    /// Human label (primary / fallback / …).
    #[serde(default)]
    pub label: String,
}

fn default_max_split() -> u32 {
    2
}

/// Fallback profile used when no ISP data matches.
pub fn unknown_profile() -> IspProfile {
    IspProfile {
        name_en: "Unknown ISP".to_string(),
        name_fa: "ای‌اس‌پی ناشناس".to_string(),
        asn: "unknown".to_string(),
        dpi_intensity: "medium".to_string(),
        blocked_protocols: vec![],
        working_protocols: vec![],
        default_strategy: "sni_split".to_string(),
        needs_traffic_shaping: false,
    }
}

/// Parse all ISP profiles from the embedded JSON.
pub fn isp_profiles() -> Vec<IspProfile> {
    let parsed: Result<Value, _> = serde_json::from_str(crate::ISP_PROFILES_JSON);
    match parsed {
        Ok(Value::Object(root)) => match root.get("profiles").and_then(|p| p.as_array()) {
            Some(profiles) => profiles
                .iter()
                .filter_map(|p| serde_json::from_value(p.clone()).ok())
                .collect(),
            None => vec![],
        },
        _ => vec![],
    }
}

/// Lookup an ISP profile by ASN ("AS41689" or "41689" or "as41689").
pub fn isp_by_asn(asn: &str) -> IspProfile {
    let normalized = asn.trim().to_uppercase().replace("AS", "");
    for profile in isp_profiles() {
        if profile.asn.trim().to_uppercase().replace("AS", "") == normalized {
            return profile;
        }
    }
    unknown_profile()
}

/// Parse all carrier presets from the embedded JSON.
pub fn carrier_presets() -> Vec<CarrierPreset> {
    let parsed: Result<Value, _> = serde_json::from_str(crate::CARRIER_PRESETS_JSON);
    match parsed {
        Ok(Value::Object(root)) => match root.get("carriers").and_then(|c| c.as_array()) {
            Some(carriers) => carriers
                .iter()
                .filter_map(|c| serde_json::from_value(c.clone()).ok())
                .collect(),
            None => vec![],
        },
        _ => vec![],
    }
}

/// Lookup a carrier preset by carrier key or ASN.
pub fn carrier_by_key_or_asn(key_or_asn: &str) -> Option<CarrierPreset> {
    let normalized = key_or_asn.trim().to_lowercase();
    let asn_normalized = key_or_asn.trim().to_uppercase().replace("AS", "");
    for carrier in carrier_presets() {
        if carrier.key == normalized {
            return Some(carrier);
        }
        for asn in &carrier.match_asns {
            if asn.trim().to_uppercase().replace("AS", "") == asn_normalized {
                return Some(carrier);
            }
        }
    }
    None
}

/// Strategies recommended for an ISP/carrier, best-first, always ending with
/// `raw` (the control candidate so the bandit can always measure the
/// baseline) and `random_split` (the entropy fallback).
pub fn recommended_strategies(carrier: &CarrierPreset) -> Vec<Strategy> {
    let mut strategies: Vec<Strategy> = carrier
        .fragment_strategies
        .iter()
        .map(|name| Strategy::from_name(name))
        .filter(|s| *s != Strategy::Raw)
        .collect();
    let default = Strategy::from_name(&carrier.default_strategy);
    if default != Strategy::Raw && !strategies.contains(&default) {
        strategies.insert(0, default);
    }
    if !strategies.contains(&Strategy::RandomSplit) {
        strategies.push(Strategy::RandomSplit);
    }
    strategies.push(Strategy::Raw);
    strategies
}

/// The default recommended strategy for a DPI flavor (MICAFP mapping):
/// FAVA v2 -> SniSplit, FAVA v1 -> RecordSplit, light -> RecordSplit.
pub fn default_strategy_for_dpi(dpi: &str) -> Strategy {
    match dpi {
        "fava-v2" => Strategy::SniSplit,
        "fava-v1" => Strategy::RecordSplit,
        "light" => Strategy::RecordSplit,
        _ => Strategy::SniSplit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_profiles_parse() {
        let profiles = isp_profiles();
        assert!(
            profiles.len() >= 8,
            "expected >=8 ISP profiles, got {}",
            profiles.len()
        );
        let mci = isp_by_asn("AS41689");
        assert!(mci.name_en.contains("MCI"));
        assert!(mci.dpi_intensity.contains("high"));
    }

    #[test]
    fn carrier_lookup_by_asn_and_key() {
        let by_asn = carrier_by_key_or_asn("AS41689").expect("mci by asn");
        assert_eq!(by_asn.key, "mci");
        assert_eq!(by_asn.fake_sni, "www.speedtest.net");
        let by_key = carrier_by_key_or_asn("irancell").expect("irancell by key");
        assert_eq!(by_key.fake_sni, "chatgpt.com");
        assert!(carrier_by_key_or_asn("no-such-carrier").is_none());
    }

    #[test]
    fn strategies_end_with_raw_and_include_random() {
        let carrier = carrier_by_key_or_asn("mci").unwrap();
        let strategies = recommended_strategies(&carrier);
        assert_eq!(*strategies.last().unwrap(), Strategy::Raw);
        assert!(strategies.contains(&Strategy::RandomSplit));
        assert!(strategies.contains(&Strategy::SniSplit));
    }

    #[test]
    fn dpi_flavor_mapping() {
        assert_eq!(default_strategy_for_dpi("fava-v2"), Strategy::SniSplit);
        assert_eq!(default_strategy_for_dpi("fava-v1"), Strategy::RecordSplit);
    }
}
