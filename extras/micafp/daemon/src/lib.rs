// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA Quantum-Ultra v8.0 — Library Root
//
// COMPLETE MERGE of all 13 source projects. Every feature, module, transport,
// core, and utility from every variant is preserved here.
// Zero features removed. All 13 projects fully unified.
//
// Source projects merged:
//   MICAFP-UnifiedShield-!  MICAFP-UnifiedShield-&  MICAFP-UnifiedShield-)
//   MICAFP-UnifiedShield-*  MICAFP-UnifiedShield-+  MICAFP-UnifiedShield-,
//   MICAFP-UnifiedShield-;  MICAFP-UnifiedShield-¢  MICAFP-UnifiedShield-£
//   MICAFP-UnifiedShield-©  MICAFP-UnifiedShield-€
//   unifiedshield-nextgen$  unifiedshield-nextgen@
// ─────────────────────────────────────────────────────────────────────────────

pub mod error;
pub mod ipc;

// ── Security subsystem ──────────────────────────────────────────────────────
pub mod security;

// ── Transport subsystem (22 protocols from all 13 source projects) ───────────
pub mod transport;

// ── Obfuscation subsystem ───────────────────────────────────────────────────
pub mod obfuscation;

// ── Core engine subsystem (9 VPN cores) ─────────────────────────────────────
pub mod cores;

// ── AI subsystem (7 engines) ─────────────────────────────────────────────────
pub mod ai;

// ── Scanner subsystem ────────────────────────────────────────────────────────
pub mod scanner;

// ── P2P subsystem (libp2p, I2P, Yggdrasil, NAT traversal, relay) ────────────
pub mod p2p;

// ── National Intranet / NAIN detection subsystem ────────────────────────────
pub mod national_intranet;

// ── Quantum / Post-Quantum cryptography subsystem ───────────────────────────
pub mod quantum;

// ── Battery / power management ──────────────────────────────────────────────
pub mod battery;

// ── Platform abstraction (Linux · Windows · Android · iOS) ──────────────────
pub mod platform;

// ── Tunnel subsystem ─────────────────────────────────────────────────────────
pub mod tunnel;

// ── Configuration subsystem ──────────────────────────────────────────────────
pub mod config;

// ── Monitoring & Observability ───────────────────────────────────────────────
pub mod monitoring;

// ── Mesh Network Coordinator ─────────────────────────────────────────────────
pub mod mesh;

// ── Resilience subsystem ─────────────────────────────────────────────────────
pub mod resilience;

// ── Unified Orchestrator ─────────────────────────────────────────────────────
pub mod orchestrator;

// ── Adaptive Load Balancer ───────────────────────────────────────────────────
pub mod load_balancer;

// ── System Watchdog ──────────────────────────────────────────────────────────
pub mod watchdog;

// ── Prometheus-compatible metrics exporter ──────────────────────────────────
pub mod metrics;

// ── Differential-privacy telemetry pipeline ─────────────────────────────────
pub mod telemetry;

// ── Compile-time embedded resources ─────────────────────────────────────────
/// CDN endpoint configuration embedded at compile time.
pub const CDN_ENDPOINTS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/cdn-endpoints.json"
));
/// P2P bootstrap peer list embedded at compile time.
pub const P2P_BOOTSTRAP_PEERS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/p2p-bootstrap-peers.json"
));

// ── Shared Constants ──────────────────────────────────────────────────────

/// Battery threshold for critical mode (percentage).
pub const BATTERY_CRITICAL_THRESHOLD: u8 = 5;

/// NAIN probe intervals in seconds.
pub const NAIN_PROBE_INTERVAL_SCREEN_ON_SECS: u64 = 30;
pub const NAIN_PROBE_INTERVAL_SCREEN_OFF_LIGHT_SECS: u64 = 120;
pub const NAIN_PROBE_INTERVAL_SCREEN_OFF_DEEP_SECS: u64 = 600;

/// P2P bootstrap peers embedded resource path.
pub fn load_p2p_bootstrap_peers() -> &'static str {
    P2P_BOOTSTRAP_PEERS_JSON
}
