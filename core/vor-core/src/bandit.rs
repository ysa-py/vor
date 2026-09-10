//! UCB1 bandit over engines/strategies with decayed rewards.
//!
//! Ported from MICAFP `daemon/src/ai/ucb_bandit.rs` (a real, working
//! implementation upstream). The Vor version generalizes the arm set from
//! the 9 fixed "cores" to arbitrary engine/strategy arm ids, keeps the same
//! UCB1 formula and reward decay, and adds deterministic tie-breaking
//! (lexicographically smallest arm on equal scores) so that the shared
//! conformance vectors hold on every platform.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Exploration multiplier (MICAFP used 1.414 = sqrt(2)).
pub const DEFAULT_ALPHA: f64 = 1.414;
/// Reward decay applied to the accumulated reward on every update.
pub const DEFAULT_DECAY: f64 = 0.95;
/// Number of recent rewards retained per arm.
pub const RECENT_WINDOW: usize = 100;

/// Per-arm statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArmStats {
    /// Total number of pulls.
    pub pulls: u64,
    /// Decayed reward sum.
    pub reward_sum: f64,
    /// Most recent rewards (bounded by [`RECENT_WINDOW`]).
    pub recent_rewards: Vec<f64>,
}

/// Serializable bandit state (shared across platforms as JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BanditState {
    /// Arm id -> stats. `BTreeMap` gives deterministic iteration order.
    pub arms: BTreeMap<String, ArmStats>,
    /// Total pulls across all arms.
    pub total_pulls: u64,
    /// Exploration coefficient.
    pub alpha: f64,
    /// Reward decay factor.
    pub decay: f64,
}

impl Default for BanditState {
    fn default() -> Self {
        Self {
            arms: BTreeMap::new(),
            total_pulls: 0,
            alpha: DEFAULT_ALPHA,
            decay: DEFAULT_DECAY,
        }
    }
}

impl BanditState {
    /// Create a bandit over the given arm ids.
    pub fn new(arms: &[&str]) -> Self {
        let mut state = Self::default();
        for arm in arms {
            state.arms.insert((*arm).to_string(), ArmStats::default());
        }
        state
    }

    /// UCB1 score for one arm (0.0 for unpulled arms in the score listing;
    /// selection treats unpulled arms as +inf so they are tried first).
    pub fn arm_score(&self, id: &str) -> f64 {
        match self.arms.get(id) {
            None => 0.0,
            Some(stats) if stats.pulls == 0 => 0.0,
            Some(stats) => {
                let avg = stats.reward_sum / stats.pulls as f64;
                let exploration = self.alpha
                    * ((2.0 * (self.total_pulls.max(1) as f64).ln()) / stats.pulls as f64).sqrt();
                avg + exploration
            }
        }
    }

    /// Select the best arm by UCB1. Unpulled arms win (infinite score),
    /// ties break lexicographically for determinism. Returns `None` only
    /// when the bandit has no arms at all.
    pub fn select(&self) -> Option<String> {
        let mut best: Option<(&str, f64)> = None;
        for (id, stats) in &self.arms {
            let score = if stats.pulls == 0 {
                f64::INFINITY
            } else {
                self.arm_score(id)
            };
            let better = match best {
                None => true,
                Some((_, best_score)) => {
                    score > best_score || (score == best_score && id.as_str() < best.unwrap().0)
                }
            };
            if better {
                best = Some((id.as_str(), score));
            }
        }
        best.map(|(id, _)| id.to_string())
    }

    /// Record an outcome for an arm. Reward is expected in `0.0..=1.0`
    /// (success/latency score computed by the caller). Unknown arms are
    /// ignored so callers can prune the arm set independently.
    pub fn update(&mut self, arm_id: &str, reward: f64) {
        if let Some(stats) = self.arms.get_mut(arm_id) {
            stats.reward_sum = stats.reward_sum * self.decay + reward;
            stats.pulls += 1;
            stats.recent_rewards.push(reward);
            if stats.recent_rewards.len() > RECENT_WINDOW {
                stats.recent_rewards.remove(0);
            }
        }
        self.total_pulls += 1;
    }

    /// Ranked list of `(arm, score)` best-first (deterministic order).
    pub fn ranking(&self) -> Vec<(String, f64)> {
        let mut rows: Vec<(String, f64)> = self
            .arms
            .keys()
            .map(|id| (id.clone(), self.arm_score(id)))
            .collect();
        rows.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpulled_arms_selected_first_deterministically() {
        let state = BanditState::new(&["xray", "singbox", "tor"]);
        assert_eq!(state.select().unwrap(), "singbox"); // lexicographic among +inf
    }

    #[test]
    fn ucb_prefers_successful_arm() {
        let mut state = BanditState::new(&["a", "b"]);
        for _ in 0..10 {
            state.update("a", 1.0);
            state.update("b", 0.0);
        }
        assert_eq!(state.select().unwrap(), "a");
    }

    #[test]
    fn decay_pulls_recent_rewards() {
        let mut state = BanditState::new(&["a"]);
        for _ in 0..50 {
            state.update("a", 1.0);
        }
        for _ in 0..50 {
            state.update("a", 0.0);
        }
        // decayed: old successes mostly forgotten
        let stats = &state.arms["a"];
        assert!(stats.reward_sum < 20.0, "reward_sum={}", stats.reward_sum);
        assert_eq!(stats.pulls, 100);
    }

    #[test]
    fn ranking_is_best_first() {
        let mut state = BanditState::new(&["a", "b"]);
        for _ in 0..5 {
            state.update("b", 1.0);
        }
        let ranking = state.ranking();
        assert_eq!(ranking[0].0, "b");
    }
}
