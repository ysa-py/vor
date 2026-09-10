// ─────────────────────────────────────────────────────────────────────────────
// Load Balancer — Smooth Weighted Round Robin + Session Affinity
// MICAFP-UnifiedShield-vip-ultra-Quantum-ultra v8.0
// ─────────────────────────────────────────────────────────────────────────────

pub mod session_affinity;
pub mod swrr;

pub use session_affinity::SessionAffinityTable;
pub use swrr::SmoothedWeightedRoundRobin;

/// Canonical exported name used throughout the codebase.
pub type SmoothWeightedRoundRobin = SmoothedWeightedRoundRobin;
/// Canonical exported name for session affinity table.
pub type SessionAffinity = SessionAffinityTable;

// ── Shared load balancer types ────────────────────────────────────────────────

/// A named item with a static weight and running current_weight for SWRR.
pub use crate::transport::TransportWeight;
