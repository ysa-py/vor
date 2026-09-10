// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Dual-Mode Transport Core (Directive v70)
//
// Root module for Dual-Mode Transport Architecture:
//   • Mode A (Fast / Default): Parallel Multipath Transport via QUIC over UDP
//   • Mode B (Layered / Opt-in): 5-Hop Nested Onion Encapsulation
//   • Pluggable PathScheduler trait and implementations
//   • Zero-copy TUN pipeline and mobile FFI bridge
//   • Comparative benchmark harness
// ─────────────────────────────────────────────────────────────────────────────

pub mod benchmarks;
pub mod ffi_bridge;
pub mod mode_a_fast;
pub mod mode_b_layered;
pub mod scheduler;
pub mod tests;
pub mod tun_pipeline;

pub use benchmarks::{BenchmarkResultRecord, DualModeBenchmarkHarness};
pub use ffi_bridge::{
    dual_mode_ffi_init, dual_mode_ffi_process_outbound_packet, dual_mode_ffi_set_mode,
    dual_mode_ffi_shutdown,
};
pub use mode_a_fast::{
    CipherSuite, ModeAFastConfig, ModeAFastEngine, MultipathChannel, PmtudController,
};
pub use mode_b_layered::{
    GeoRegion, ModeBLayeredConfig, ModeBLayeredEngine, OnionHopNode, MODE_B_HOP_COUNT,
};
pub use scheduler::{
    AdaptiveMultipathScheduler, CongestionAlgorithm, CongestionState, FixedHopChainScheduler,
    PathMetrics, PathScheduler, PathScore,
};
pub use tun_pipeline::{
    DualModeCoreConfig, DualModeCoreEngine, DualModeStatsSnapshot, DualTransportMode,
};
