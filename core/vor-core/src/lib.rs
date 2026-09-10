//! # vor-core — Vor shared adaptive DPI-evasion / engine-selection core
//!
//! Single source of truth for the decision logic used by **every** Vor
//! platform (Android, Windows/Linux desktop, OpenWrt, iOS). The task
//! requirement is explicit: *"Port MICAFP's AI-assisted DPI-evasion logic as
//! a shared library usable from the Android, Windows, Linux, and OpenWrt
//! builds — do not fork it into platform-specific copies that will drift."*
//!
//! This crate is that library. It contains no I/O and no platform APIs —
//! every function is a pure `JSON -> JSON` transformation (or a pure
//! byte-level planner), which is what makes one shared implementation viable
//! across five platforms and four languages:
//!
//! * **C ABI** (`ffi.rs`) — used by OpenWrt (C netifd proto) and available
//!   to any native host. All functions are stateless: state travels in and
//!   out as JSON strings owned by the caller.
//! * **JNI** (`jni.rs`, feature `jni`) — Android bindings.
//! * **Pure-Kotlin and pure-Go interpreters** of the same decision model
//!   exist for unit-testability without native code (`android` JVM tests and
//!   `desktop` Go tests); they are kept honest by the shared conformance
//!   vectors in `tests/vectors/` which are generated from this crate.
//!
//! ## What is inside (and where it came from)
//!
//! | Module | Origin | Notes |
//! |---|---|---|
//! | `bandit` | MICAFP `daemon/src/ai/ucb_bandit.rs` | UCB1 with decayed rewards — real upstream code |
//! | `selector` | MICAFP `rl_transport_selector` (Q-learning) + UAC adaptive planner | engine/strategy selection state machine |
//! | `fragment` | MICAFP `tls_fragment.rs` + UAC `MciFragmenter` strategy taxonomy | byte-level ClientHello split planner |
//! | `threat` | MICAFP `ObfuscationCoordinator` threat ladder | None→Passive→ActiveV1→ActiveV2→Blackout |
//! | `isp` | MICAFP `configs/isp-profiles.json` + UAC carrier presets | ASN/carrier fingerprinting |
//! | `features` | MICAFP `feature_extractor.rs` | rolling-window traffic features |
//! | `shaper` | MICAFP `traffic_shaper.rs` + adversarial profiles | padding/IAT shaping decisions |
//! | `nain` | MICAFP `national_intranet` | intranet/blackout detection heuristic |
//! | `license` | Vor license spec (`license/SPEC.md`) | Ed25519 offline token verification |
//!
//! Everything here runs **on-device, offline** — no network calls, no cloud
//! AI. The "AI" is adaptive bandit/RL selection over live probe outcomes,
//! which is what actually works when the network itself is the adversary.

#![deny(missing_docs)]

pub mod b64;
pub mod bandit;
pub mod engine;
pub mod features;
pub mod fragment;
pub mod isp;
pub mod license;
pub mod nain;
pub mod selector;
pub mod shaper;
pub mod threat;

#[cfg(feature = "jni")]
pub mod jni;

pub mod ffi;

/// Crate version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Embedded shared data files (single source of ISP/DPI knowledge).
/// ISP profiles for Iranian internet service providers.
pub const ISP_PROFILES_JSON: &str = include_str!("../data/isp-profiles.json");
/// Iranian DPI signatures (FAVA RST timings, poison IPs).
pub const DPI_SIGNATURES_JSON: &str = include_str!("../data/dpi-signatures.json");
/// UAC carrier tuning presets (edges, fake SNIs, fragment matrices).
pub const CARRIER_PRESETS_JSON: &str = include_str!("../data/carrier-presets.json");
/// Adversarial traffic profiles for the traffic shaper.
pub const TRAFFIC_PROFILES_JSON: &str = include_str!("../data/traffic-profiles.json");
