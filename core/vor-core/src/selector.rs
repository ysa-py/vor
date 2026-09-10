//! The adaptive engine/strategy **selector** — the decision heart of Vor.
//!
//! This is the "Auto (AI)" mode of the transport engine selector. It fuses:
//!
//! * the **ISP/carrier profile** (which strategies/edges are plausible),
//! * the **threat ladder** (how aggressive to be),
//! * the **UCB1 bandit** over (engine × strategy) arms, updated from live
//!   probe outcomes,
//! * a small **Q-learning-style state cache** (ported from MICAFP
//!   `rl_transport_selector`) that remembers what worked per network
//!   fingerprint (carrier + ASN + network type), like the UAC adaptive
//!   planner (champion → learned winner → last-good → fresh candidates).
//!
//! Everything is a pure JSON transformation: `decide(state, context) ->
//! decision` and `observe(state, outcome) -> state`. The same state JSON is
//! interpreted by the Kotlin and Go ports, which are pinned to this crate's
//! behavior by the shared conformance vectors (`tests/vectors/`).

use crate::bandit::BanditState;
use crate::isp;
use crate::threat::{ThreatLevel, ThreatObservation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Candidate engine ids known to the selector (a subset can be supplied by
/// the platform — engines unavailable at runtime are filtered out by the
/// caller before applying a decision).
pub const KNOWN_ENGINES: &[&str] = &[
    "xray",       // in-process Xray core (VLESS/VMess/Trojan/SS/REALITY)
    "singbox",    // sing-box process core (Hysteria2/TUIC/Naive/…)
    "sni-tunnel", // EasySNI desync engine (fragment + fake hello)
    "pattern",    // UAC wrong-sequence injection engine (alternate)
    "dns-tunnel-dnstt",
    "dns-tunnel-masterdns",
    "tor",
    "psiphon",
    "mitm-fronting",
];

/// The persistent adaptive state (JSON on disk per platform).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorState {
    /// UCB1 bandit over "<engine>|<strategy>" arm ids.
    pub bandit: BanditState,
    /// Learned best arm per network fingerprint (Q-table with 1 entry per
    /// fingerprint; the MICAFP Q-learning selector reduced to what matters).
    pub q: std::collections::BTreeMap<String, LearnedEntry>,
    /// Minimum reward an arm must hold to remain "champion" for a
    /// fingerprint (hysteresis; avoids flip-flopping).
    pub champion_threshold: f64,
}

/// One learned entry (per network fingerprint).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedEntry {
    /// Best arm id observed for this fingerprint.
    pub champion: String,
    /// Champion's rolling reward estimate.
    pub champion_reward: f64,
    /// Last arm that produced a *successful* connection.
    pub last_good: String,
    /// When the entry was last updated (epoch seconds).
    pub updated_at: i64,
}

impl Default for SelectorState {
    fn default() -> Self {
        Self {
            bandit: BanditState::default(),
            q: std::collections::BTreeMap::new(),
            champion_threshold: 0.6,
        }
    }
}

/// The runtime context for one decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionContext {
    /// Carrier key ("mci", "irancell", …) or ASN ("AS41689") — used for
    /// profile lookup. Empty string = unknown.
    #[serde(default)]
    pub carrier: String,
    /// Network fingerprint (UAC-style: "wifi:AS41689:mobile"), used as the
    /// Q-table key. Empty = "default".
    #[serde(default)]
    pub fingerprint: String,
    /// Latest threat observation window.
    pub threat: ThreatObservation,
    /// Engines actually available at runtime on this platform.
    pub available_engines: Vec<String>,
    /// Engines the user pinned manually (empty in Auto mode).
    #[serde(default)]
    pub pinned_engines: Vec<String>,
    /// Current threat level override (None = derive from observation).
    #[serde(default)]
    pub threat_override: Option<String>,
}

/// The decision output consumed by every platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    /// Chosen engine id (already filtered to `available_engines`).
    pub engine: String,
    /// Ordered list of fragment strategy names to try, best-first.
    pub strategies: Vec<String>,
    /// Fragment inter-write delay (ms).
    pub fragment_delay_ms: u32,
    /// Fake SNI to use for disposable probes / wrong-seq injection.
    pub fake_sni: String,
    /// Cloudflare edges to substitute under SNI pressure (ip, port,
    /// max_split), best-first.
    pub edges: Vec<Value>,
    /// Whether traffic shaping is required right now.
    pub traffic_shaping: bool,
    /// Shaper level ("passthrough" | "light" | "full" | "maximum").
    pub shaper_level: String,
    /// Current threat level.
    pub threat_level: String,
    /// Whether the chosen arm is the learned champion (vs exploration).
    pub is_champion: bool,
    /// Ranked engine table for UI display.
    pub ranking: Vec<Value>,
}

/// One probe outcome fed back into the learner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    /// Network fingerprint the outcome belongs to.
    #[serde(default)]
    pub fingerprint: String,
    /// Engine that was tried.
    pub engine: String,
    /// Fragment strategy that was tried ("" when not applicable).
    #[serde(default)]
    pub strategy: String,
    /// Whether the connection succeeded end-to-end.
    pub success: bool,
    /// Connect latency (ms) — lower is better.
    #[serde(default)]
    pub rtt_ms: u32,
    /// Whether the connection was killed by DPI mid-session.
    #[serde(default)]
    pub dpi_kill: bool,
    /// Download throughput measured (bytes/s, 0 = unknown).
    #[serde(default)]
    pub throughput_bps: u64,
}

/// Compute the scalar reward for an outcome (shared formula — the Kotlin and
/// Go ports implement the identical function; conformance vectors pin it).
///
/// `reward = success ? (0.5 + 0.3*latency_score + 0.2*throughput_score) : 0.0`,
/// latency_score = 1 / (1 + rtt/500), throughput_score = bps/(bps+1e6);
/// DPI kills halve the reward instead of zeroing it (a connection that
/// *worked at all* under pressure still carries information).
pub fn reward_for(outcome: &Outcome) -> f64 {
    if !outcome.success {
        return 0.0;
    }
    let latency_score = 1.0 / (1.0 + outcome.rtt_ms as f64 / 500.0);
    let throughput_score =
        outcome.throughput_bps as f64 / (outcome.throughput_bps as f64 + 1_000_000.0);
    let mut reward = 0.5 + 0.3 * latency_score + 0.2 * throughput_score;
    if outcome.dpi_kill {
        reward *= 0.5;
    }
    reward
}

/// Decide the next (engine, strategies, shaping) plan.
pub fn decide(state: &SelectorState, context: &DecisionContext) -> Decision {
    // 1. Carrier / ISP profile
    let carrier = if context.carrier.is_empty() {
        None
    } else {
        isp::carrier_by_key_or_asn(&context.carrier)
    };
    let isp_profile = if context.carrier.is_empty() {
        isp::unknown_profile()
    } else {
        isp::isp_by_asn(&context.carrier)
    };

    // 2. Threat level
    let threat_level = match &context.threat_override {
        Some(override_value) => ThreatLevel::from_str_value(override_value),
        None => context.threat.level(),
    };

    // 3. Candidate arms = available engines × recommended strategies
    let available: Vec<String> = if context.available_engines.is_empty() {
        KNOWN_ENGINES.iter().map(|e| e.to_string()).collect()
    } else {
        context.available_engines.clone()
    };

    let strategies: Vec<String> = match &carrier {
        Some(preset) => isp::recommended_strategies(preset)
            .into_iter()
            .map(|s| s.name().to_string())
            .collect(),
        None => {
            let default_strategy = isp::default_strategy_for_dpi(&isp_profile.dpi_intensity);
            let mut list = vec![default_strategy.name().to_string()];
            for extra in ["record_split", "full10", "random_split", "raw"] {
                if !list.iter().any(|s| s == extra) {
                    list.push(extra.to_string());
                }
            }
            list
        }
    };

    // 4. Champion from the Q-table for this fingerprint
    let fingerprint: &str = if context.fingerprint.is_empty() {
        "default"
    } else {
        context.fingerprint.as_str()
    };
    let learned = state.q.get(fingerprint);

    // 5. Build the arm list (engine|strategy) and pick by bandit/champion
    let mut arms: Vec<String> = Vec::new();
    for engine in &available {
        for strategy in &strategies {
            if fragment_applicable(engine) {
                arms.push(format!("{engine}|{strategy}"));
            } else {
                arms.push(format!("{engine}|raw"));
                break;
            }
        }
    }

    let mut bandit = state.bandit.clone();
    for arm in &arms {
        if !bandit.arms.contains_key(arm) {
            bandit.arms.insert(arm.clone(), Default::default());
        }
    }
    // prune arms that are no longer candidates (keeps state small)
    let arm_set: std::collections::BTreeSet<String> = arms.iter().cloned().collect();
    bandit.arms.retain(|arm, _| arm_set.contains(arm));

    // Champion first if it is strong enough and still available; otherwise
    // the bandit decides (untried arms first — that is the exploration
    // policy, identical to UAC's candidate planner ordering).
    let mut chosen_arm: Option<String> = None;
    let mut is_champion = false;
    if let Some(entry) = learned {
        if entry.champion_reward >= state.champion_threshold {
            if let Some(champion_arm) = arms.iter().find(|a| **a == entry.champion) {
                chosen_arm = Some(champion_arm.clone());
                is_champion = true;
            }
        }
    }
    if chosen_arm.is_none() {
        chosen_arm = if state.bandit.total_pulls == 0 {
            // Fresh state: try the carrier's highest-priority strategy first
            // (rank 0 in the recommended list — the ISP default), tie-broken
            // lexicographically across engines offering it. Deterministic,
            // so the shared conformance vectors pin this exactly.
            let rank_of = |strategy: &str| {
                strategies
                    .iter()
                    .position(|s| s == strategy)
                    .unwrap_or(usize::MAX)
            };
            arms.iter()
                .min_by_key(|arm| {
                    let (_, strategy) = split_arm(arm);
                    (rank_of(strategy), (*arm).clone())
                })
                .cloned()
        } else {
            bandit.select()
        };
    }
    let chosen_arm = chosen_arm.unwrap_or_else(|| "xray|raw".to_string());
    let (engine, strategy) = split_arm(&chosen_arm);

    // 6. Carrier extras for the decision
    let fake_sni = carrier
        .as_ref()
        .map(|preset| preset.fake_sni.clone())
        .unwrap_or_else(|| "www.speedtest.net".to_string());
    let edges: Vec<Value> = carrier
        .as_ref()
        .map(|preset| {
            preset
                .edges
                .iter()
                .map(|edge| json!({"ip": edge.ip, "port": edge.port, "max_split": edge.max_split, "label": edge.label}))
                .collect()
        })
        .unwrap_or_default();
    let fragment_delay_ms = if threat_level.requires_aggressive() {
        0
    } else {
        5
    };

    // 7. Shaper level from the threat ladder
    let isp_needs_shaping = carrier
        .as_ref()
        .map(|preset| preset.needs_traffic_shaping)
        .unwrap_or(isp_profile.needs_traffic_shaping);
    let shaper_level = if !threat_level.layer2_shaper(isp_needs_shaping) {
        "passthrough"
    } else if threat_level.requires_aggressive() {
        "maximum"
    } else if threat_level.requires_obfuscation() {
        "full"
    } else {
        "light"
    };

    let ranking: Vec<Value> = bandit
        .ranking()
        .into_iter()
        .map(|(arm, score)| json!({"arm": arm, "score": round3(score)}))
        .collect();

    let mut strategies_ordered = strategies.clone();
    if strategy != "raw" {
        strategies_ordered.retain(|s| s.as_str() != strategy);
        strategies_ordered.insert(0, strategy.to_string());
    }

    Decision {
        engine: engine.to_string(),
        strategies: strategies_ordered,
        fragment_delay_ms,
        fake_sni,
        edges,
        traffic_shaping: shaper_level != "passthrough",
        shaper_level: shaper_level.to_string(),
        threat_level: threat_level.as_str_value().to_string(),
        is_champion,
        ranking,
    }
}

/// Fold an outcome into the state (bandit update + Q-table update).
pub fn observe(state: &SelectorState, outcome: &Outcome) -> SelectorState {
    let fingerprint: &str = if outcome.fingerprint.is_empty() {
        "default"
    } else {
        outcome.fingerprint.as_str()
    };
    let strategy = if outcome.strategy.is_empty() {
        "raw"
    } else {
        outcome.strategy.as_str()
    };
    let arm = format!("{}|{}", outcome.engine, strategy);
    let reward = reward_for(outcome);

    let mut next = state.clone();
    if !next.bandit.arms.contains_key(&arm) {
        next.bandit.arms.insert(arm.clone(), Default::default());
    }
    next.bandit.update(&arm, reward);

    let entry = next
        .q
        .entry(fingerprint.to_string())
        .or_insert(LearnedEntry {
            champion: arm.clone(),
            champion_reward: 0.0,
            last_good: "xray|raw".to_string(),
            updated_at: 0,
        });
    if outcome.success {
        entry.last_good = arm.clone();
    }
    // Exponential moving average of the champion reward
    let arm_stats = next.bandit.arms.get(&arm);
    let arm_average = arm_stats
        .map(|stats| {
            if stats.pulls == 0 {
                0.0
            } else {
                stats.reward_sum / stats.pulls as f64
            }
        })
        .unwrap_or(0.0);
    let is_current_champion = entry.champion == arm;
    if !is_current_champion && arm_average > entry.champion_reward {
        entry.champion = arm.clone();
        entry.champion_reward = round3(arm_average);
    } else if is_current_champion {
        entry.champion_reward = round3(arm_average);
    }
    entry.updated_at = crate::license::now_epoch_secs();
    next
}

/// Engines whose connection path is a plain TCP/TLS dial that fragment
/// strategies can front-run (the SNI-tunnel/pattern engines and any
/// process core driven through the Edge Bridge).
fn fragment_applicable(engine: &str) -> bool {
    matches!(engine, "sni-tunnel" | "pattern" | "xray" | "mitm-fronting")
}

fn split_arm(arm: &str) -> (&str, &str) {
    match arm.split_once('|') {
        Some((engine, strategy)) => (engine, strategy),
        None => (arm, "raw"),
    }
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(engines: &[&str]) -> DecisionContext {
        DecisionContext {
            carrier: "mci".to_string(),
            fingerprint: "test-fp".to_string(),
            threat: ThreatObservation::default(),
            available_engines: engines.iter().map(|e| e.to_string()).collect(),
            pinned_engines: vec![],
            threat_override: None,
        }
    }

    #[test]
    fn reward_formula() {
        let mut outcome = Outcome {
            fingerprint: "fp".into(),
            engine: "xray".into(),
            strategy: "sni_split".into(),
            success: true,
            rtt_ms: 0,
            dpi_kill: false,
            throughput_bps: 0,
        };
        // rtt=0, throughput=0 -> 0.5 + 0.3*1 + 0.2*0 = 0.8
        assert!((reward_for(&outcome) - 0.8).abs() < 1e-9);
        outcome.dpi_kill = true;
        assert!((reward_for(&outcome) - 0.4).abs() < 1e-9);
        outcome.success = false;
        assert_eq!(reward_for(&outcome), 0.0);
    }

    #[test]
    fn fresh_state_explores_untried_arms() {
        let state = SelectorState::default();
        let decision = decide(&state, &context(&["xray", "sni-tunnel"]));
        assert!(!decision.is_champion);
        // Untried arms: lexicographically smallest is picked
        assert!(decision.engine == "sni-tunnel" || decision.engine == "xray");
        assert!(!decision.strategies.is_empty());
        assert_eq!(decision.fake_sni, "www.speedtest.net");
        assert!(!decision.edges.is_empty());
    }

    #[test]
    fn mci_carrier_maps_to_sni_split_family() {
        let state = SelectorState::default();
        let decision = decide(&state, &context(&["sni-tunnel"]));
        assert!(decision.strategies.iter().any(|s| s == "sni_boundary"));
        assert!(decision.edges.iter().any(|e| e["ip"] == "104.18.1.1"));
    }

    #[test]
    fn champion_is_reused_when_strong() {
        let mut state = SelectorState::default();
        for _ in 0..10 {
            state = observe(
                &state,
                &Outcome {
                    fingerprint: "test-fp".into(),
                    engine: "pattern".into(),
                    strategy: "sni_split".into(),
                    success: true,
                    rtt_ms: 50,
                    dpi_kill: false,
                    throughput_bps: 0,
                },
            );
        }
        let decision = decide(&state, &context(&["xray", "pattern"]));
        assert!(decision.is_champion);
        assert_eq!(decision.engine, "pattern");
        assert_eq!(decision.strategies[0], "sni_split");
    }

    #[test]
    fn failed_outcomes_lower_arm_score() {
        let mut state = SelectorState::default();
        for _ in 0..20 {
            state = observe(
                &state,
                &Outcome {
                    fingerprint: "fp".into(),
                    engine: "xray".into(),
                    strategy: "raw".into(),
                    success: true,
                    rtt_ms: 10,
                    dpi_kill: false,
                    throughput_bps: 5_000_000,
                },
            );
        }
        for _ in 0..20 {
            state = observe(
                &state,
                &Outcome {
                    fingerprint: "fp".into(),
                    engine: "tor".into(),
                    strategy: "raw".into(),
                    success: false,
                    rtt_ms: 0,
                    dpi_kill: true,
                    throughput_bps: 0,
                },
            );
        }
        let xray_score = state.bandit.arm_score("xray|raw");
        let tor_score = state.bandit.arm_score("tor|raw");
        assert!(xray_score > tor_score);
    }

    #[test]
    fn threat_escalates_shaper() {
        let mut ctx = context(&["xray"]);
        ctx.threat_override = Some("none".into());
        let calm = decide(&SelectorState::default(), &ctx);
        assert_eq!(calm.shaper_level, "passthrough");
        ctx.threat_override = Some("active_v2".into());
        let hot = decide(&SelectorState::default(), &ctx);
        assert_eq!(hot.shaper_level, "maximum");
        assert!(hot.traffic_shaping);
    }

    #[test]
    fn state_json_roundtrip() {
        let mut state = SelectorState::default();
        state = observe(
            &state,
            &Outcome {
                fingerprint: "fp".into(),
                engine: "xray".into(),
                strategy: "raw".into(),
                success: true,
                rtt_ms: 20,
                dpi_kill: false,
                throughput_bps: 0,
            },
        );
        let text = serde_json::to_string(&state).unwrap();
        let back: SelectorState = serde_json::from_str(&text).unwrap();
        assert!(back.q.contains_key("fp"));
        assert_eq!(back.q["fp"].champion, "xray|raw");
    }
}
