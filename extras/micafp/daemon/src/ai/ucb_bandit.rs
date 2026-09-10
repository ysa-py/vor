use rand::Rng;
use std::collections::HashMap;
use parking_lot::Mutex;

/// Core selector arm for UCB1 bandit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoreArm {
    Hiddify,
    Xray,
    Singbox,
    AmneziaVpn,
    DefyX,
    Moav,
    Lantern,
    Mahsang,
    Psiphon,
}

impl std::fmt::Display for CoreArm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CoreArm::Hiddify => "hiddify",
            CoreArm::Xray => "xray",
            CoreArm::Singbox => "singbox",
            CoreArm::AmneziaVpn => "amneziavpn",
            CoreArm::DefyX => "defyx",
            CoreArm::Moav => "moav",
            CoreArm::Lantern => "lantern",
            CoreArm::Mahsang => "mahsang",
            CoreArm::Psiphon => "psiphon",
        };
        write!(f, "{}", s)
    }
}

impl CoreArm {
    pub fn all() -> &'static [CoreArm] {
        &[
            CoreArm::Hiddify,
            CoreArm::Xray,
            CoreArm::Singbox,
            CoreArm::AmneziaVpn,
            CoreArm::DefyX,
            CoreArm::Moav,
            CoreArm::Lantern,
            CoreArm::Mahsang,
            CoreArm::Psiphon,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct ArmStats {
    pub pulls: u64,
    pub reward_sum: f64,
    pub recent_rewards: Vec<f64>,
    pub decay_factor: f64,
}

#[derive(Debug)]
struct UCBBanditInner {
    arms: HashMap<String, ArmStats>,
    total_pulls: u64,
    alpha: f64,
}

pub struct UCBBandit {
    inner: Mutex<UCBBanditInner>,
}

impl UCBBandit {
    pub fn new(alpha: f64) -> Self {
        Self {
            inner: Mutex::new(UCBBanditInner {
                arms: HashMap::new(),
                total_pulls: 0,
                alpha,
            }),
        }
    }

    pub fn add_arm(&self, id: &str) {
        let mut inner = self.inner.lock();
        inner.arms.insert(
            id.to_string(),
            ArmStats {
                pulls: 0,
                reward_sum: 0.0,
                recent_rewards: Vec::new(),
                decay_factor: 0.95,
            },
        );
    }

    pub fn select(&self) -> SelectionResult {
        let inner = self.inner.lock();
        let mut best: Option<String> = None;
        let mut best_score = f64::NEG_INFINITY;
        for (id, stats) in &inner.arms {
            let score = if stats.pulls == 0 {
                f64::INFINITY
            } else {
                let avg = stats.reward_sum / stats.pulls as f64;
                let exploration = inner.alpha
                    * ((2.0 * (inner.total_pulls as f64).ln() / stats.pulls as f64).sqrt());
                avg + exploration
            };
            if score > best_score {
                best_score = score;
                best = Some(id.clone());
            }
        }
        SelectionResult {
            arm: parse_arm(&best),
            score: best_score,
        }
    }

    pub fn select_arm(&self) -> Option<String> {
        let inner = self.inner.lock();
        let mut best: Option<String> = None;
        let mut best_score = f64::NEG_INFINITY;
        for (id, stats) in &inner.arms {
            let score = if stats.pulls == 0 {
                f64::INFINITY
            } else {
                let avg = stats.reward_sum / stats.pulls as f64;
                let exploration = inner.alpha
                    * ((2.0 * (inner.total_pulls as f64).ln() / stats.pulls as f64).sqrt());
                avg + exploration
            };
            if score > best_score {
                best_score = score;
                best = Some(id.clone());
            }
        }
        best
    }

    pub fn update_reward(&self, arm_id: &str, reward: f64) {
        let mut inner = self.inner.lock();
        if let Some(stats) = inner.arms.get_mut(arm_id) {
            stats.reward_sum = stats.reward_sum * stats.decay_factor + reward;
            stats.pulls += 1;
            stats.recent_rewards.push(reward);
            if stats.recent_rewards.len() > 100 {
                stats.recent_rewards.remove(0);
            }
        }
        inner.total_pulls += 1;
    }

    pub fn get_scores(&self) -> Vec<(String, f64)> {
        let inner = self.inner.lock();
        inner
            .arms
            .iter()
            .map(|(id, stats)| {
                let score = if stats.pulls == 0 {
                    0.0
                } else {
                    stats.reward_sum / stats.pulls as f64
                        + inner.alpha
                            * ((2.0 * (inner.total_pulls as f64).ln() / stats.pulls as f64).sqrt())
                };
                (id.clone(), score)
            })
            .collect()
    }
}

pub struct SelectionResult {
    pub arm: CoreArm,
    pub score: f64,
}

fn parse_arm(arm_str: &Option<String>) -> CoreArm {
    match arm_str.as_deref() {
        Some("hiddify") => CoreArm::Hiddify,
        Some("xray") => CoreArm::Xray,
        Some("singbox") => CoreArm::Singbox,
        Some("amneziavpn") => CoreArm::AmneziaVpn,
        Some("defyx") => CoreArm::DefyX,
        Some("moav") => CoreArm::Moav,
        Some("lantern") => CoreArm::Lantern,
        Some("mahsang") => CoreArm::Mahsang,
        Some("psiphon") => CoreArm::Psiphon,
        _ => CoreArm::Xray, // Default fallback
    }
}