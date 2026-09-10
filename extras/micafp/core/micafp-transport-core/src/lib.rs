//! MICAFP Dual-Mode Transport Core (Directive v70)
//! Provides two distinct runtime-selectable transport architectures:
//! - MODE A: Parallel Multipath Transport (QUIC over UDP, N paths, single-layer crypto, dynamic PMTUD, BBR/CUBIC)
//! - MODE B: 5-Hop Nested Layered Encryption (Sphinx / Noise, geographically diverse hops, fail-closed anonymity)

pub mod config;
pub mod scheduler;
pub mod mode_a_multipath;
pub mod mode_b_layered;
pub mod tun_loop;
pub mod benchmarks;
pub mod tests;
pub mod ffi_android;
pub mod ffi_ios;

pub use config::{DualModeConfig, TransportMode, CongestionControl, CipherSuite};
pub use scheduler::{PathScheduler, AdaptiveMultiPathScheduler, FixedChainScheduler, PathMetrics};
pub use mode_a_multipath::ModeAMultipathEngine;
pub use mode_b_layered::ModeBLayeredEngine;
pub use tun_loop::TunDualModeController;
pub use benchmarks::{BenchmarkResult, DualModeBenchmarkHarness};
