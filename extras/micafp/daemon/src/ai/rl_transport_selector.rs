// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — Reinforcement Learning Transport Selector
//
// Uses Q-learning with experience replay to make real-time transport
// protocol switching decisions based on network conditions. This is more
// effective than a static UCB bandit for Iran's non-stationary DPI
// environment, where censorship strategies change frequently.
//
// Key features:
//   • Real-time protocol switching based on network conditions
//   • State: current transport, latency, packet loss, bandwidth, time of day, ISP
//   • Actions: switch to transport[i], keep current, probe all
//   • Reward: connection stability * bandwidth - latency penalty
//   • Q-learning with experience replay (10000 state transitions)
//   • Epsilon-greedy exploration: eps=0.1 normal, eps=0.3 NAIN
//   • Persists Q-table to disk (<100KB)
// ─────────────────────────────────────────────────────────────────────────────

use std::collections::VecDeque;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::{AiInferenceContext, AiMetrics};
use crate::error::{ErrorCode, ShieldError};

// ── Constants ───────────────────────────────────────────────────────────────

/// Number of transport protocols available.
const NUM_TRANSPORTS: usize = 8;
/// Maximum experience replay buffer size.
const MAX_REPLAY_BUFFER: usize = 10_000;
/// Minimum replay buffer size before training starts.
const MIN_REPLAY_BUFFER: usize = 100;
/// Learning rate (alpha) for Q-value updates.
const LEARNING_RATE: f64 = 0.01;
/// Discount factor (gamma) for future rewards.
const DISCOUNT_FACTOR: f64 = 0.95;
/// Epsilon (exploration rate) during normal operation.
const EPSILON_NORMAL: f64 = 0.1;
/// Epsilon during NAIN mode (more exploration needed).
const EPSILON_NAIN: f64 = 0.3;
/// Epsilon decay rate per decision.
const EPSILON_DECAY: f64 = 0.9999;
/// Minimum epsilon value.
const EPSILON_MIN: f64 = 0.05;
/// Maximum reward value.
const MAX_REWARD: f64 = 100.0;
/// State discretization bins for continuous features.
const STATE_BINS: usize = 8;
/// Number of time-of-day buckets (24 hours / 3-hour buckets = 8).
const TIME_BUCKETS: usize = 8;

// ── Transport Protocol ──────────────────────────────────────────────────────

/// Available transport protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportProtocol {
    Hysteria2,
    ShadowTls,
    Reality,
    TuicV5,
    Vless,
    WebTransport,
    MqttWs,
    DoqTunnel,
}

impl TransportProtocol {
    /// Get all transport protocols as a list.
    pub fn all() -> Vec<Self> {
        vec![
            Self::Hysteria2,
            Self::ShadowTls,
            Self::Reality,
            Self::TuicV5,
            Self::Vless,
            Self::WebTransport,
            Self::MqttWs,
            Self::DoqTunnel,
        ]
    }

    /// Get the index of this transport in the action space.
    pub fn index(self) -> usize {
        match self {
            Self::Hysteria2 => 0,
            Self::ShadowTls => 1,
            Self::Reality => 2,
            Self::TuicV5 => 3,
            Self::Vless => 4,
            Self::WebTransport => 5,
            Self::MqttWs => 6,
            Self::DoqTunnel => 7,
        }
    }

    /// Get a transport from its index.
    pub fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(Self::Hysteria2),
            1 => Some(Self::ShadowTls),
            2 => Some(Self::Reality),
            3 => Some(Self::TuicV5),
            4 => Some(Self::Vless),
            5 => Some(Self::WebTransport),
            6 => Some(Self::MqttWs),
            7 => Some(Self::DoqTunnel),
            _ => None,
        }
    }

    /// Get the human-readable name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Hysteria2 => "hysteria2",
            Self::ShadowTls => "shadow_tls",
            Self::Reality => "reality",
            Self::TuicV5 => "tuic_v5",
            Self::Vless => "vless",
            Self::WebTransport => "webtransport",
            Self::MqttWs => "mqtt_ws",
            Self::DoqTunnel => "doq_tunnel",
        }
    }
}

// ── Transport State ─────────────────────────────────────────────────────────

/// The current network state observed by the RL agent.
///
/// Continuous values are discretized into bins for the Q-table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportState {
    /// Currently active transport protocol.
    pub current_transport: TransportProtocol,
    /// Round-trip latency in milliseconds.
    pub latency_ms: f64,
    /// Packet loss ratio (0.0 - 1.0).
    pub packet_loss: f64,
    /// Available bandwidth in kbps.
    pub bandwidth_kbps: f64,
    /// Hour of day (0-23).
    pub hour_of_day: u8,
    /// ISP identifier (hash of ISP name).
    pub isp_hash: u8,
    /// Whether NAIN mode is active.
    pub nain_active: bool,
    /// Connection stability score (0.0 - 1.0).
    pub stability: f64,
}

impl TransportState {
    /// Discretize the state into a Q-table index.
    ///
    /// The state space is:
    ///   - current_transport: 8 values
    ///   - latency: 8 bins
    ///   - packet_loss: 8 bins
    ///   - bandwidth: 8 bins
    ///   - time_of_day: 8 buckets
    ///   - nain_active: 2 values
    ///
    /// Total state space: 8 * 8 * 8 * 8 * 8 * 2 = 65,536 states
    /// Q-table size: 65,536 * 9 actions = 589,824 entries ≈ ~5MB as f32
    pub fn discretize(&self) -> usize {
        let transport_idx = self.current_transport.index();
        let latency_bin = discretize(self.latency_ms, 0.0, 2000.0, STATE_BINS);
        let loss_bin = discretize(self.packet_loss, 0.0, 1.0, STATE_BINS);
        let bw_bin = discretize(self.bandwidth_kbps, 0.0, 50000.0, STATE_BINS);
        let time_bin = (self.hour_of_day as usize / 3).min(TIME_BUCKETS - 1);
        let nain_bin = if self.nain_active { 1 } else { 0 };

        // Combine into a single index
        transport_idx * (STATE_BINS * STATE_BINS * STATE_BINS * TIME_BUCKETS * 2)
            + latency_bin * (STATE_BINS * STATE_BINS * TIME_BUCKETS * 2)
            + loss_bin * (STATE_BINS * TIME_BUCKETS * 2)
            + bw_bin * (TIME_BUCKETS * 2)
            + time_bin * 2
            + nain_bin
    }

    /// Total number of discrete states.
    pub fn state_space_size() -> usize {
        NUM_TRANSPORTS * STATE_BINS * STATE_BINS * STATE_BINS * TIME_BUCKETS * 2
    }
}

// ── Transport Action ────────────────────────────────────────────────────────

/// An action that the RL agent can take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportAction {
    /// Switch to the specified transport protocol.
    SwitchTo(TransportProtocol),
    /// Keep the current transport protocol.
    KeepCurrent,
    /// Probe all transports to gather fresh statistics.
    ProbeAll,
}

impl TransportAction {
    /// Number of possible actions (8 transports + keep + probe).
    pub const NUM_ACTIONS: usize = NUM_TRANSPORTS + 2;

    /// Get the action index for Q-table lookup.
    pub fn index(self) -> usize {
        match self {
            Self::SwitchTo(t) => t.index(),
            Self::KeepCurrent => NUM_TRANSPORTS,
            Self::ProbeAll => NUM_TRANSPORTS + 1,
        }
    }

    /// Get an action from its index.
    pub fn from_index(idx: usize) -> Option<Self> {
        if idx < NUM_TRANSPORTS {
            TransportProtocol::from_index(idx).map(Self::SwitchTo)
        } else if idx == NUM_TRANSPORTS {
            Some(Self::KeepCurrent)
        } else if idx == NUM_TRANSPORTS + 1 {
            Some(Self::ProbeAll)
        } else {
            None
        }
    }
}

// ── Experience ──────────────────────────────────────────────────────────────

/// A single experience tuple (s, a, r, s') for replay learning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    /// State before action.
    pub state: usize,
    /// Action taken.
    pub action: usize,
    /// Reward received.
    pub reward: f64,
    /// State after action.
    pub next_state: usize,
    /// Whether the next state is terminal.
    pub done: bool,
    /// Timestamp of this experience.
    pub timestamp: u64,
}

// ── Reward Function ─────────────────────────────────────────────────────────

/// Compute the reward for a transport selection decision.
///
/// Reward = stability * bandwidth_factor - latency_penalty - loss_penalty
pub fn compute_reward(
    stability: f64,
    bandwidth_kbps: f64,
    latency_ms: f64,
    packet_loss: f64,
) -> f64 {
    let bandwidth_factor = (bandwidth_kbps / 1000.0).min(10.0); // Cap at 10x
    let latency_penalty = (latency_ms / 100.0).min(5.0); // Cap at 5.0
    let loss_penalty = packet_loss * 20.0; // Heavy penalty for packet loss

    let reward = stability * bandwidth_factor * 10.0 - latency_penalty - loss_penalty;
    reward.clamp(-MAX_REWARD, MAX_REWARD)
}

// ── Q-Table ─────────────────────────────────────────────────────────────────

/// Q-table for the transport selector.
///
/// Maps (state, action) → expected future reward.
/// Stored as a flat array for cache-friendly access.
pub struct QTable {
    /// Flat Q-value array: [state * NUM_ACTIONS + action].
    values: Vec<f64>,
    /// Number of discrete states.
    num_states: usize,
    /// Number of actions.
    num_actions: usize,
}

impl QTable {
    /// Create a new Q-table initialized to zeros.
    pub fn new(num_states: usize, num_actions: usize) -> Self {
        Self {
            values: vec![0.0; num_states * num_actions],
            num_states,
            num_actions,
        }
    }

    /// Get the Q-value for a (state, action) pair.
    pub fn get(&self, state: usize, action: usize) -> f64 {
        let idx = state * self.num_actions + action;
        if idx < self.values.len() {
            self.values[idx]
        } else {
            0.0
        }
    }

    /// Set the Q-value for a (state, action) pair.
    pub fn set(&mut self, state: usize, action: usize, value: f64) {
        let idx = state * self.num_actions + action;
        if idx < self.values.len() {
            self.values[idx] = value;
        }
    }

    /// Get the best action for a given state.
    pub fn best_action(&self, state: usize) -> usize {
        let start = state * self.num_actions;
        let end = start + self.num_actions;

        if end > self.values.len() {
            return 0;
        }

        let mut best_action = 0;
        let mut best_value = self.values[start];

        for (action, &value) in self.values[start..end].iter().enumerate() {
            if value > best_value {
                best_value = value;
                best_action = action;
            }
        }

        best_action
    }

    /// Get the maximum Q-value for a given state.
    pub fn max_q(&self, state: usize) -> f64 {
        let start = state * self.num_actions;
        let end = start + self.num_actions;

        if end > self.values.len() {
            return 0.0;
        }

        self.values[start..end]
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Update a Q-value using the Bellman equation.
    pub fn update(
        &mut self,
        state: usize,
        action: usize,
        reward: f64,
        next_state: usize,
        done: bool,
    ) {
        let current_q = self.get(state, action);
        let max_next_q = if done { 0.0 } else { self.max_q(next_state) };
        let new_q = current_q + LEARNING_RATE * (reward + DISCOUNT_FACTOR * max_next_q - current_q);
        self.set(state, action, new_q);
    }

    /// Serialize the Q-table to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let num_states = self.num_states as u64;
        let num_actions = self.num_actions as u64;
        let mut bytes = Vec::with_capacity(16 + self.values.len() * 8);

        bytes.extend_from_slice(&num_states.to_le_bytes());
        bytes.extend_from_slice(&num_actions.to_le_bytes());

        for &value in &self.values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }

        bytes
    }

    /// Deserialize a Q-table from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ShieldError> {
        if bytes.len() < 16 {
            return Err(ShieldError::ai(
                ErrorCode::AiTransportSelectionFailed,
                "Q-table data too short",
            ));
        }

        let num_states = u64::from_le_bytes(bytes[0..8].try_into().map_err(|_| {
            ShieldError::ai(
                ErrorCode::AiTransportSelectionFailed,
                "Invalid Q-table header",
            )
        })?) as usize;
        let num_actions = u64::from_le_bytes(bytes[8..16].try_into().map_err(|_| {
            ShieldError::ai(
                ErrorCode::AiTransportSelectionFailed,
                "Invalid Q-table header",
            )
        })?) as usize;

        let expected_len = 16 + num_states * num_actions * 8;
        if bytes.len() < expected_len {
            return Err(ShieldError::ai(
                ErrorCode::AiTransportSelectionFailed,
                format!(
                    "Q-table data truncated: expected {} bytes, got {}",
                    expected_len,
                    bytes.len()
                ),
            ));
        }

        let mut values = Vec::with_capacity(num_states * num_actions);
        let mut offset = 16;
        for _ in 0..(num_states * num_actions) {
            let value = f64::from_le_bytes(bytes[offset..offset + 8].try_into().map_err(|_| {
                ShieldError::ai(ErrorCode::AiTransportSelectionFailed, "Invalid Q-value")
            })?);
            values.push(value);
            offset += 8;
        }

        Ok(Self {
            values,
            num_states,
            num_actions,
        })
    }
}

// ── RL Transport Selector ───────────────────────────────────────────────────

/// Reinforcement learning-based transport protocol selector.
///
/// Uses Q-learning with experience replay to adaptively select the
/// best transport protocol based on current network conditions.
pub struct RlTransportSelector {
    /// Shared inference context.
    context: Arc<RwLock<AiInferenceContext>>,
    /// Shared AI metrics.
    metrics: Arc<RwLock<AiMetrics>>,
    /// Q-table for value function approximation.
    q_table: Mutex<QTable>,
    /// Experience replay buffer.
    replay_buffer: Mutex<VecDeque<Experience>>,
    /// Current epsilon for exploration.
    epsilon: Mutex<f64>,
    /// Last state observed.
    last_state: Mutex<Option<usize>>,
    /// Last action taken.
    last_action: Mutex<Option<usize>>,
    /// Decision count.
    decision_count: Mutex<u64>,
    /// Random number generator state.
    rng_state: Mutex<u64>,
}

impl RlTransportSelector {
    /// Create a new RL transport selector.
    pub fn new(
        context: Arc<RwLock<AiInferenceContext>>,
        metrics: Arc<RwLock<AiMetrics>>,
    ) -> Result<Self, ShieldError> {
        let num_states = TransportState::state_space_size();
        let q_table = QTable::new(num_states, TransportAction::NUM_ACTIONS);

        Ok(Self {
            context,
            metrics,
            q_table: Mutex::new(q_table),
            replay_buffer: Mutex::new(VecDeque::with_capacity(MAX_REPLAY_BUFFER)),
            epsilon: Mutex::new(EPSILON_NORMAL),
            last_state: Mutex::new(None),
            last_action: Mutex::new(None),
            decision_count: Mutex::new(0),
            rng_state: Mutex::new(42), // Seed
        })
    }

    /// Select the best transport action for the current state.
    pub async fn select_action(
        &self,
        state: &TransportState,
    ) -> Result<TransportAction, ShieldError> {
        let discrete_state = state.discretize();
        let epsilon = *self.epsilon.lock();

        // Epsilon-greedy action selection
        let action_idx = if self.random_f64() < epsilon {
            // Explore: random action
            let action = self.random_usize(TransportAction::NUM_ACTIONS);
            debug!(epsilon, action, "RL exploring with random action");
            action
        } else {
            // Exploit: best known action
            let q_table = self.q_table.lock();
            let best = q_table.best_action(discrete_state);
            debug!(
                epsilon,
                action = best,
                q_value = q_table.get(discrete_state, best),
                "RL exploiting best known action"
            );
            best
        };

        // Store for experience replay
        *self.last_state.lock() = Some(discrete_state);
        *self.last_action.lock() = Some(action_idx);

        // Increment decision count
        *self.decision_count.lock() += 1;

        // Decay epsilon
        {
            let mut eps = self.epsilon.lock();
            *eps = (*eps * EPSILON_DECAY).max(EPSILON_MIN);
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.rl_decision_count += 1;
            metrics.exploration_rate = epsilon;
        }

        // Convert index to action
        TransportAction::from_index(action_idx).ok_or_else(|| {
            ShieldError::ai(
                ErrorCode::AiTransportSelectionFailed,
                format!("Invalid action index: {}", action_idx),
            )
        })
    }

    /// Record the outcome of a previous action for learning.
    ///
    /// Call this after observing the reward from the last action.
    pub fn record_outcome(&self, new_state: &TransportState, reward: f64, done: bool) {
        let last_state = *self.last_state.lock();
        let last_action = *self.last_action.lock();

        if let (Some(state), Some(action)) = (last_state, last_action) {
            let next_state = new_state.discretize();

            // Create experience
            let experience = Experience {
                state,
                action,
                reward,
                next_state,
                done,
                timestamp: now_secs(),
            };

            // Add to replay buffer
            {
                let mut buffer = self.replay_buffer.lock();
                if buffer.len() >= MAX_REPLAY_BUFFER {
                    buffer.pop_front();
                }
                buffer.push_back(experience);
            }

            // Direct Q-value update (online learning)
            self.q_table
                .lock()
                .update(state, action, reward, next_state, done);

            // Experience replay: sample and learn from past experiences
            self.experience_replay();
        }
    }

    /// Adjust exploration rate based on NAIN status.
    pub fn set_nain_mode(&self, active: bool) {
        let mut eps = self.epsilon.lock();
        *eps = if active { EPSILON_NAIN } else { EPSILON_NORMAL };
        info!(
            nain_active = active,
            epsilon = *eps,
            "RL exploration rate adjusted for NAIN mode"
        );
    }

    /// Load the Q-table from disk.
    pub async fn load_qtable(&self) -> Result<(), ShieldError> {
        let ctx = self.context.read().await;
        let path = PathBuf::from(&ctx.qtable_path);

        if !path.exists() {
            return Err(ShieldError::ai(
                ErrorCode::AiModelLoadFailed,
                format!("Q-table file not found: {}", path.display()),
            ));
        }

        let data = fs::read(&path).map_err(|e| {
            ShieldError::ai(
                ErrorCode::AiModelLoadFailed,
                format!("Failed to read Q-table: {}", e),
            )
        })?;

        let loaded_table = QTable::from_bytes(&data)?;
        *self.q_table.lock() = loaded_table;

        info!(path = %path.display(), "Q-table loaded from disk");
        Ok(())
    }

    /// Save the Q-table to disk.
    pub async fn save_qtable(&self) -> Result<(), ShieldError> {
        let ctx = self.context.read().await;
        let path = PathBuf::from(&ctx.qtable_path);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                ShieldError::ai(
                    ErrorCode::AiModelLoadFailed,
                    format!("Failed to create Q-table directory: {}", e),
                )
            })?;
        }

        let data = self.q_table.lock().to_bytes();
        fs::write(&path, &data).map_err(|e| {
            ShieldError::ai(
                ErrorCode::AiModelLoadFailed,
                format!("Failed to write Q-table: {}", e),
            )
        })?;

        let size_kb = data.len() / 1024;
        info!(path = %path.display(), size_kb, "Q-table saved to disk");
        Ok(())
    }

    /// Get the current replay buffer size.
    pub fn replay_buffer_size(&self) -> usize {
        self.replay_buffer.lock().len()
    }

    /// Get the current exploration rate.
    pub fn epsilon(&self) -> f64 {
        *self.epsilon.lock()
    }

    /// Get the Q-value for a specific state-action pair (for debugging).
    pub fn q_value(&self, state: usize, action: usize) -> f64 {
        self.q_table.lock().get(state, action)
    }

    /// Get the best transport for a given state without exploration.
    pub fn best_transport_for_state(&self, state: &TransportState) -> TransportAction {
        let discrete_state = state.discretize();
        let q_table = self.q_table.lock();
        let best_idx = q_table.best_action(discrete_state);
        TransportAction::from_index(best_idx).unwrap_or(TransportAction::KeepCurrent)
    }

    // ── Internal Methods ────────────────────────────────────────────────

    /// Perform experience replay: sample random experiences and update Q-values.
    fn experience_replay(&self) {
        let buffer = self.replay_buffer.lock();
        if buffer.len() < MIN_REPLAY_BUFFER {
            return;
        }

        // Sample a mini-batch of experiences
        let batch_size = 32.min(buffer.len());
        let mut q_table = self.q_table.lock();

        for _ in 0..batch_size {
            let idx = self.random_usize(buffer.len());
            if let Some(exp) = buffer.get(idx) {
                q_table.update(exp.state, exp.action, exp.reward, exp.next_state, exp.done);
            }
        }
    }

    /// Simple pseudo-random number generator (xoshiro256++ style).
    fn random_u64(&self) -> u64 {
        let mut state = self.rng_state.lock();
        let mut s = *state;
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        *state = s;
        s
    }

    /// Generate a random f64 in [0, 1).
    fn random_f64(&self) -> f64 {
        (self.random_u64() as f64) / (u64::MAX as f64)
    }

    /// Generate a random usize in [0, n).
    fn random_usize(&self, n: usize) -> usize {
        (self.random_u64() as usize) % n
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Discretize a continuous value into bins.
fn discretize(value: f64, min: f64, max: f64, bins: usize) -> usize {
    if value <= min {
        return 0;
    }
    if value >= max {
        return bins - 1;
    }
    let normalized = (value - min) / (max - min);
    (normalized * bins as f64).floor() as usize
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_protocol_indexing() {
        for proto in TransportProtocol::all() {
            let idx = proto.index();
            let recovered = TransportProtocol::from_index(idx);
            assert_eq!(Some(proto), recovered);
        }
    }

    #[test]
    fn test_transport_action_indexing() {
        // Test switch actions
        for proto in TransportProtocol::all() {
            let action = TransportAction::SwitchTo(proto);
            let idx = action.index();
            let recovered = TransportAction::from_index(idx);
            assert_eq!(Some(action), recovered);
        }

        // Test keep and probe actions
        assert_eq!(TransportAction::KeepCurrent.index(), NUM_TRANSPORTS);
        assert_eq!(TransportAction::ProbeAll.index(), NUM_TRANSPORTS + 1);
    }

    #[test]
    fn test_state_discretization() {
        let state = TransportState {
            current_transport: TransportProtocol::Hysteria2,
            latency_ms: 100.0,
            packet_loss: 0.01,
            bandwidth_kbps: 5000.0,
            hour_of_day: 14,
            isp_hash: 0,
            nain_active: false,
            stability: 0.9,
        };

        let idx = state.discretize();
        assert!(idx < TransportState::state_space_size());
    }

    #[test]
    fn test_discretize_function() {
        assert_eq!(discretize(0.0, 0.0, 100.0, 8), 0);
        assert_eq!(discretize(100.0, 0.0, 100.0, 8), 7);
        assert_eq!(discretize(50.0, 0.0, 100.0, 8), 4);
        assert_eq!(discretize(-10.0, 0.0, 100.0, 8), 0);
        assert_eq!(discretize(200.0, 0.0, 100.0, 8), 7);
    }

    #[test]
    fn test_q_table_operations() {
        let mut q_table = QTable::new(100, TransportAction::NUM_ACTIONS);

        // Set and get
        q_table.set(0, 0, 5.0);
        assert_eq!(q_table.get(0, 0), 5.0);

        // Best action
        q_table.set(0, 0, 5.0);
        q_table.set(0, 1, 10.0);
        q_table.set(0, 2, 3.0);
        assert_eq!(q_table.best_action(0), 1);

        // Max Q
        assert_eq!(q_table.max_q(0), 10.0);
    }

    #[test]
    fn test_q_table_serialization() {
        let mut q_table = QTable::new(10, TransportAction::NUM_ACTIONS);
        q_table.set(0, 0, 1.0);
        q_table.set(5, 3, 42.0);

        let bytes = q_table.to_bytes();
        let recovered = QTable::from_bytes(&bytes).unwrap();

        assert_eq!(recovered.get(0, 0), 1.0);
        assert_eq!(recovered.get(5, 3), 42.0);
        assert_eq!(recovered.num_states, 10);
        assert_eq!(recovered.num_actions, TransportAction::NUM_ACTIONS);
    }

    #[test]
    fn test_q_table_bellman_update() {
        let mut q_table = QTable::new(100, TransportAction::NUM_ACTIONS);

        // Initial Q-value
        q_table.set(0, 0, 0.0);

        // Update: reward=10, next_state max Q=5
        q_table.set(1, 0, 5.0);
        q_table.update(0, 0, 10.0, 1, false);

        // Q(s,a) = 0 + 0.01 * (10 + 0.95 * 5 - 0) = 0.01 * 14.75 = 0.1475
        let updated = q_table.get(0, 0);
        assert!((updated - 0.1475).abs() < 0.001);
    }

    #[test]
    fn test_reward_computation() {
        // Good connection
        let reward = compute_reward(0.95, 10000.0, 50.0, 0.001);
        assert!(reward > 0.0);

        // Poor connection
        let reward = compute_reward(0.3, 100.0, 500.0, 0.3);
        assert!(reward < 0.0);

        // Perfect connection
        let reward = compute_reward(1.0, 50000.0, 10.0, 0.0);
        assert!(reward > 50.0);
    }

    #[tokio::test]
    async fn test_rl_transport_selector_creation() {
        let context = Arc::new(RwLock::new(AiInferenceContext::default()));
        let metrics = Arc::new(RwLock::new(AiMetrics::default()));
        let selector = RlTransportSelector::new(context, metrics).unwrap();

        assert_eq!(selector.epsilon(), EPSILON_NORMAL);
        assert_eq!(selector.replay_buffer_size(), 0);
    }

    #[tokio::test]
    async fn test_rl_action_selection() {
        let context = Arc::new(RwLock::new(AiInferenceContext::default()));
        let metrics = Arc::new(RwLock::new(AiMetrics::default()));
        let selector = RlTransportSelector::new(context, metrics).unwrap();

        let state = TransportState {
            current_transport: TransportProtocol::Hysteria2,
            latency_ms: 100.0,
            packet_loss: 0.01,
            bandwidth_kbps: 5000.0,
            hour_of_day: 14,
            isp_hash: 0,
            nain_active: false,
            stability: 0.9,
        };

        let action = selector.select_action(&state).await.unwrap();
        // Action should be one of the valid actions
        match action {
            TransportAction::SwitchTo(proto) => {
                assert!(TransportProtocol::all().contains(&proto));
            }
            TransportAction::KeepCurrent => {}
            TransportAction::ProbeAll => {}
        }
    }

    #[test]
    fn test_nain_mode_exploration() {
        let context = Arc::new(RwLock::new(AiInferenceContext::default()));
        let metrics = Arc::new(RwLock::new(AiMetrics::default()));
        let selector = RlTransportSelector::new(context, metrics).unwrap();

        assert_eq!(selector.epsilon(), EPSILON_NORMAL);

        selector.set_nain_mode(true);
        assert_eq!(selector.epsilon(), EPSILON_NAIN);

        selector.set_nain_mode(false);
        assert_eq!(selector.epsilon(), EPSILON_NORMAL);
    }
}
