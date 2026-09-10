pub mod adversarial_traffic;
pub mod dpi_classifier;
pub mod feature_extractor;
pub mod onnx_runtime;
pub mod rl_transport_selector;
pub mod traffic_predictor;
pub mod ucb_bandit;

pub use adversarial_traffic::TrafficShape;
pub use dpi_classifier::DpiClassifier;
pub use feature_extractor::FeatureExtractor;
pub use onnx_runtime::OnnxRuntime;
pub use rl_transport_selector::RlTransportSelector;
pub use traffic_predictor::TrafficPredictor;
pub use ucb_bandit::{CoreArm, UCBBandit};

pub type UcbBandit = UCBBandit;
pub type AdversarialTrafficGenerator = TrafficShape;

// ── Shared AI context types ───────────────────────────────────────────────────

/// Shared context passed to AI inference engines.
#[derive(Debug, Clone)]
pub struct AiInferenceContext {
    /// Current detected DPI probability (0.0 — 1.0).
    pub dpi_probability: f64,
    /// Number of recent connection failures.
    pub recent_failures: u32,
    /// Average round-trip time in milliseconds.
    pub avg_rtt_ms: f64,
    /// Whether NAIN mode is active.
    pub nain_active: bool,
    /// Path to the GAN model for adversarial traffic.
    pub gan_model_path: String,
    /// Path to the Q-table for RL.
    pub qtable_path: String,
}

impl Default for AiInferenceContext {
    fn default() -> Self {
        Self {
            dpi_probability: 0.0,
            recent_failures: 0,
            avg_rtt_ms: 150.0,
            nain_active: false,
            gan_model_path: String::new(),
            qtable_path: String::new(),
        }
    }
}

/// Aggregated AI performance metrics.
#[derive(Debug, Clone, Default)]
pub struct AiMetrics {
    pub inference_count: u64,
    pub total_inference_ms: u64,
    pub model_version: String,
    pub last_update_ts: u64,
    pub rl_decision_count: u64,
    pub exploration_rate: f64,
}
