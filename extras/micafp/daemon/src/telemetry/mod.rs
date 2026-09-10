// ─────────────────────────────────────────────────────────────────────────────
// Telemetry — differential-privacy telemetry pipeline
// MICAFP-UnifiedShield-vip-ultra-Quantum-ultra v8.0
// ─────────────────────────────────────────────────────────────────────────────

pub mod aggregator;
pub mod dp_noise;
pub mod reporter;

pub use aggregator::TelemetryAggregator;
pub use reporter::TelemetryReporter;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A censorship event recorded for telemetry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CensorshipEvent {
    pub event_type: CensorshipEventType,
    pub transport: String,
    pub rtt_ms: u32,
    pub bypass_succeeded: bool,
    pub timestamp_secs: u64,
}

/// Type of censorship/blocking event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CensorshipEventType {
    DpiBlock,
    Timeout,
    TcpReset,
    DnsHijack,
    NainTransition,
    SuccessfulBypass,
}

/// An aggregated, differentially-private telemetry report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryReport {
    pub report_id: String,
    pub period_start: u64,
    pub period_end: u64,
    pub event_counts: HashMap<String, f64>,
    pub bypass_success_rate: f64,
    pub avg_rtt_ms: f64,
    pub epsilon: f64,
}

/// Wrapper for differential privacy noise utilities (no-op type alias).
pub struct DifferentialPrivacyNoise;
