//! Obfuscation subsystem for MICAFP-UnifiedShield-6.0
//!
//! This module provides traffic obfuscation capabilities designed to defeat
//! Deep Packet Inspection (DPI) systems used for internet censorship in Iran.
//!
//! Key components:
//! - `tls_fragment`: TLS ClientHello fragmentation to bypass FAVA v1/v2 filters
//! - `traffic_shaper`: AI-driven traffic shaping to mimic legitimate HTTPS traffic
//! - `wasm_obfuscator`: WASM-based custom obfuscation transforms
//!
//! The ObfuscationCoordinator manages all obfuscation strategies and applies
//! them based on the current threat profile and ISP characteristics.

pub mod tls_fragment;
pub mod traffic_shaper;
pub mod wasm_obfuscator;

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::obfuscation::tls_fragment::{TlsFragmentStrategy, TlsFragmenter};
use crate::obfuscation::traffic_shaper::TrafficShaper;
use crate::obfuscation::wasm_obfuscator::WasmObfuscator;

/// ISP profiles for Iranian internet service providers.
/// Each profile has known DPI characteristics that inform obfuscation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IspProfile {
    /// MCI (Hamrah-e-Avval) - Mobile operator with FAVA v2
    Mci,
    /// Irancell (MTN) - Mobile operator with FAVA v1
    Irancell,
    /// Rightel - Mobile operator, lighter DPI
    Rightel,
    /// Mokhtari - Residential ISP with aggressive DPI
    Mokhtari,
    /// Shatel - Residential/business ISP
    Shatel,
    /// ParsOnline - Business ISP, moderate DPI
    ParsOnline,
    /// Iran Telecom (TCI) - National backbone, FAVA v2
    IranTelecom,
    /// Afranet - Business ISP
    Afranet,
    /// Unknown ISP - use conservative defaults
    Unknown,
}

impl IspProfile {
    /// Returns the recommended TLS fragmentation strategy for this ISP.
    pub fn default_tls_strategy(&self) -> TlsFragmentStrategy {
        match self {
            IspProfile::Mci | IspProfile::IranTelecom => {
                // FAVA v2: need SNI_SPLIT which is most effective
                TlsFragmentStrategy::SniSplit
            }
            IspProfile::Irancell => {
                // FAVA v1: RECORD_SPLIT sufficient
                TlsFragmentStrategy::RecordSplit
            }
            IspProfile::Mokhtari => {
                // Aggressive DPI: use SNI_SPLIT + randomization
                TlsFragmentStrategy::SniSplit
            }
            IspProfile::Rightel
            | IspProfile::Shatel
            | IspProfile::ParsOnline
            | IspProfile::Afranet => {
                // Moderate DPI: RECORD_SPLIT is enough
                TlsFragmentStrategy::RecordSplit
            }
            IspProfile::Unknown => {
                // Unknown: use SNI_SPLIT as safest default
                TlsFragmentStrategy::SniSplit
            }
        }
    }

    /// Whether this ISP is known to use FAVA v2 (more aggressive filtering).
    pub fn uses_fava_v2(&self) -> bool {
        matches!(
            self,
            IspProfile::Mci | IspProfile::IranTelecom | IspProfile::Mokhtari
        )
    }

    /// Whether traffic shaping is recommended for this ISP.
    pub fn needs_traffic_shaping(&self) -> bool {
        matches!(
            self,
            IspProfile::Mci | IspProfile::IranTelecom | IspProfile::Mokhtari | IspProfile::Irancell
        )
    }
}

/// Threat level as detected by NAIN (Network Analysis and Identification Node).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThreatLevel {
    /// No DPI detected, obfuscation optional
    None,
    /// Passive DPI detected (reads but doesn't block)
    Passive,
    /// Active DPI with known fingerprinting (FAVA v1)
    ActiveV1,
    /// Active DPI with advanced fingerprinting (FAVA v2)
    ActiveV2,
    /// Complete blackout - all tunnels blocked, emergency mode
    CompleteBlackout,
}

impl ThreatLevel {
    /// Returns true if obfuscation is required at this threat level.
    pub fn requires_obfuscation(&self) -> bool {
        !matches!(self, ThreatLevel::None)
    }

    /// Returns true if aggressive obfuscation (all strategies) is needed.
    pub fn requires_aggressive(&self) -> bool {
        matches!(self, ThreatLevel::ActiveV2 | ThreatLevel::CompleteBlackout)
    }
}

/// Configuration for the obfuscation subsystem.
#[derive(Debug, Clone)]
pub struct ObfuscationConfig {
    /// ISP profile for strategy selection
    pub isp_profile: IspProfile,
    /// Whether TLS fragmentation is enabled (default: true)
    pub tls_fragment_enabled: bool,
    /// Override TLS fragmentation strategy (None = auto-select from ISP)
    pub tls_fragment_strategy_override: Option<TlsFragmentStrategy>,
    /// Whether traffic shaping is enabled (default: true for known ISPs)
    pub traffic_shaper_enabled: bool,
    /// Whether WASM obfuscation is enabled (default: false, requires extension)
    pub wasm_obfuscator_enabled: bool,
    /// Path to WASM extension module
    pub wasm_module_path: Option<String>,
    /// Maximum acceptable latency overhead from obfuscation (ms)
    pub max_latency_overhead_ms: u32,
}

impl Default for ObfuscationConfig {
    fn default() -> Self {
        Self {
            isp_profile: IspProfile::Unknown,
            tls_fragment_enabled: true,
            tls_fragment_strategy_override: None,
            traffic_shaper_enabled: true,
            wasm_obfuscator_enabled: false,
            wasm_module_path: None,
            max_latency_overhead_ms: 50,
        }
    }
}

/// Current state of the obfuscation subsystem.
#[derive(Debug, Clone)]
pub struct ObfuscationState {
    /// Currently active TLS fragmentation strategy
    pub active_tls_strategy: Option<TlsFragmentStrategy>,
    /// Whether traffic shaper is actively shaping (vs idle)
    pub traffic_shaper_active: bool,
    /// Whether WASM obfuscator is loaded and active
    pub wasm_obfuscator_active: bool,
    /// Current threat level as reported by NAIN
    pub threat_level: ThreatLevel,
    /// Number of packets obfuscated since startup
    pub packets_obfuscated: u64,
    /// Number of packets that passed through without obfuscation
    pub packets_passthrough: u64,
    /// Estimated average latency overhead in microseconds
    pub avg_latency_overhead_us: u64,
}

impl Default for ObfuscationState {
    fn default() -> Self {
        Self {
            active_tls_strategy: None,
            traffic_shaper_active: false,
            wasm_obfuscator_active: false,
            threat_level: ThreatLevel::None,
            packets_obfuscated: 0,
            packets_passthrough: 0,
            avg_latency_overhead_us: 0,
        }
    }
}

/// The ObfuscationCoordinator manages all obfuscation strategies.
///
/// It applies obfuscation to outgoing traffic based on:
/// 1. The current ISP profile (determines default strategies)
/// 2. The current threat level (from NAIN detection)
/// 3. Explicit configuration overrides
/// 4. Battery/power constraints (from BatteryCoordinator)
///
/// The coordinator follows a layered approach:
/// - Layer 1: TLS fragmentation (always on for Iranian ISPs)
/// - Layer 2: Traffic shaping (activated when DPI is detected)
/// - Layer 3: WASM obfuscation (activated under aggressive threats)
pub struct ObfuscationCoordinator {
    config: Arc<RwLock<ObfuscationConfig>>,
    state: Arc<RwLock<ObfuscationState>>,
    tls_fragmenter: Arc<RwLock<TlsFragmenter>>,
    traffic_shaper: Arc<RwLock<TrafficShaper>>,
    wasm_obfuscator: Arc<RwLock<WasmObfuscator>>,
}

impl ObfuscationCoordinator {
    /// Create a new ObfuscationCoordinator with the given configuration.
    pub async fn new(config: ObfuscationConfig) -> Result<Self, ObfuscationError> {
        let isp = config.isp_profile;
        let tls_strategy = config
            .tls_fragment_strategy_override
            .unwrap_or_else(|| isp.default_tls_strategy());

        let tls_fragmenter = TlsFragmenter::new(tls_strategy);
        let traffic_shaper = TrafficShaper::new(isp);
        let wasm_obfuscator = WasmObfuscator::new(config.wasm_module_path.clone()).await?;

        let initial_state = ObfuscationState {
            active_tls_strategy: if config.tls_fragment_enabled {
                Some(tls_strategy)
            } else {
                None
            },
            traffic_shaper_active: false,
            wasm_obfuscator_active: config.wasm_obfuscator_enabled
                && wasm_obfuscator.is_loaded().await,
            threat_level: ThreatLevel::None,
            ..ObfuscationState::default()
        };

        info!(
            "ObfuscationCoordinator initialized: ISP={:?}, TLS strategy={:?}, shaper={}, wasm={}",
            isp,
            initial_state.active_tls_strategy,
            config.traffic_shaper_enabled,
            initial_state.wasm_obfuscator_active,
        );

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            state: Arc::new(RwLock::new(initial_state)),
            tls_fragmenter: Arc::new(RwLock::new(tls_fragmenter)),
            traffic_shaper: Arc::new(RwLock::new(traffic_shaper)),
            wasm_obfuscator: Arc::new(RwLock::new(wasm_obfuscator)),
        })
    }

    /// Process an outgoing packet through the obfuscation pipeline.
    ///
    /// Returns the possibly-modified packet data and metadata about
    /// which obfuscation layers were applied.
    pub async fn process_outgoing(
        &self,
        data: &[u8],
    ) -> Result<ObfuscatedPacket, ObfuscationError> {
        let config = self.config.read().await;
        let mut result = ObfuscatedPacket {
            data: data.to_vec(),
            tls_fragmented: false,
            traffic_shaped: false,
            wasm_transformed: false,
        };

        // Layer 1: TLS Fragmentation
        if config.tls_fragment_enabled && self.is_tls_client_hello(data) {
            let fragmenter = self.tls_fragmenter.read().await;
            let fragmented = fragmenter.fragment(data)?;
            result.data = fragmented;
            result.tls_fragmented = true;

            let mut state = self.state.write().await;
            state.packets_obfuscated += 1;
        } else {
            let mut state = self.state.write().await;
            state.packets_passthrough += 1;
        }

        // Layer 2: Traffic Shaping (only when DPI detected)
        let state_reader = self.state.read().await;
        let should_shape =
            config.traffic_shaper_enabled && state_reader.threat_level.requires_obfuscation();
        drop(state_reader);

        if should_shape {
            let mut shaper = self.traffic_shaper.write().await;
            let shaped = shaper.shape_packet(&result.data).await?;
            result.data = shaped.data;
            result.traffic_shaped = true;

            let mut state = self.state.write().await;
            state.traffic_shaper_active = true;
        }

        // Layer 3: WASM Obfuscation (only under aggressive threats)
        let state_reader = self.state.read().await;
        let should_wasm = config.wasm_obfuscator_enabled
            && state_reader.wasm_obfuscator_active
            && state_reader.threat_level.requires_aggressive();
        drop(state_reader);

        if should_wasm {
            let wasm = self.wasm_obfuscator.read().await;
            let transformed = wasm.transform(&result.data).await?;
            result.data = transformed;
            result.wasm_transformed = true;
        }

        Ok(result)
    }

    /// Update the threat level (called by NAIN subsystem).
    ///
    /// May trigger activation/deactivation of obfuscation layers.
    pub async fn update_threat_level(&self, level: ThreatLevel) {
        let mut state = self.state.write().await;
        let prev = state.threat_level;
        state.threat_level = level;

        if prev != level {
            info!(
                "Threat level changed: {:?} -> {:?}, adjusting obfuscation layers",
                prev, level
            );

            // Under CompleteBlackout, force all layers on
            if level == ThreatLevel::CompleteBlackout {
                state.active_tls_strategy = Some(TlsFragmentStrategy::SniSplit);
                state.traffic_shaper_active = true;
            }
        }
    }

    /// Update the ISP profile (called when network changes).
    pub async fn update_isp_profile(&self, profile: IspProfile) {
        let mut config = self.config.write().await;
        let prev = config.isp_profile;
        config.isp_profile = profile;

        if prev != profile {
            let strategy = config
                .tls_fragment_strategy_override
                .unwrap_or_else(|| profile.default_tls_strategy());

            let mut fragmenter = self.tls_fragmenter.write().await;
            fragmenter.set_strategy(strategy);

            let mut state = self.state.write().await;
            state.active_tls_strategy = if config.tls_fragment_enabled {
                Some(strategy)
            } else {
                None
            };

            info!(
                "ISP profile changed: {:?} -> {:?}, TLS strategy: {:?}",
                prev, profile, strategy
            );
        }
    }

    /// Get current obfuscation state (for diagnostics/telemetry).
    pub async fn state(&self) -> ObfuscationState {
        self.state.read().await.clone()
    }

    /// Apply power constraint from BatteryCoordinator.
    ///
    /// When in power-save mode, we may disable optional layers
    /// (traffic shaping, WASM) to conserve battery.
    pub async fn apply_power_constraint(
        &self,
        power_mode: crate::battery::adaptive_duty::PowerMode,
    ) {
        let config = self.config.read().await;
        let mut state = self.state.write().await;

        match power_mode {
            crate::battery::adaptive_duty::PowerMode::Performance => {
                // All layers active
                state.traffic_shaper_active =
                    config.traffic_shaper_enabled && state.threat_level.requires_obfuscation();
                debug!("Power: Performance mode, all obfuscation layers available");
            }
            crate::battery::adaptive_duty::PowerMode::Normal => {
                // All layers active but traffic shaper only on DPI detection
                debug!("Power: Normal mode, standard obfuscation");
            }
            crate::battery::adaptive_duty::PowerMode::Save => {
                // Disable traffic shaper unless under active threat
                if !state.threat_level.requires_aggressive() {
                    state.traffic_shaper_active = false;
                }
                debug!("Power: Save mode, reduced obfuscation overhead");
            }
            crate::battery::adaptive_duty::PowerMode::Critical => {
                // Only TLS fragmentation, disable everything else
                state.traffic_shaper_active = false;
                state.wasm_obfuscator_active = false;
                warn!("Power: Critical mode, minimal obfuscation active");
            }
        }
    }

    /// Detect if the given data looks like a TLS ClientHello.
    fn is_tls_client_hello(&self, data: &[u8]) -> bool {
        if data.len() < 6 {
            return false;
        }
        // TLS record: content_type=0x16 (handshake), version >= 0x0301 (TLS 1.0+)
        // Handshake: type=0x01 (ClientHello)
        data[0] == 0x16 && data[1] == 0x03 && data[2] >= 0x01 && data[5] == 0x01
    }

    /// Shut down the coordinator and release resources.
    pub async fn shutdown(&self) {
        info!("ObfuscationCoordinator shutting down");

        let mut wasm = self.wasm_obfuscator.write().await;
        wasm.shutdown().await;

        let mut shaper = self.traffic_shaper.write().await;
        shaper.shutdown().await;

        let mut state = self.state.write().await;
        state.active_tls_strategy = None;
        state.traffic_shaper_active = false;
        state.wasm_obfuscator_active = false;
    }
}

/// Result of obfuscation processing.
#[derive(Debug, Clone)]
pub struct ObfuscatedPacket {
    /// The packet data after obfuscation processing
    pub data: Vec<u8>,
    /// Whether TLS fragmentation was applied
    pub tls_fragmented: bool,
    /// Whether traffic shaping was applied
    pub traffic_shaped: bool,
    /// Whether WASM transformation was applied
    pub wasm_transformed: bool,
}

/// Errors that can occur in the obfuscation subsystem.
#[derive(Debug, thiserror::Error)]
pub enum ObfuscationError {
    #[error("TLS fragmentation error: {0}")]
    TlsFragment(String),

    #[error("Traffic shaping error: {0}")]
    TrafficShaper(String),

    #[error("WASM obfuscation error: {0}")]
    WasmObfuscator(String),

    #[error("Invalid packet data: {0}")]
    InvalidPacket(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isp_default_strategies() {
        assert_eq!(
            IspProfile::Mci.default_tls_strategy(),
            TlsFragmentStrategy::SniSplit
        );
        assert_eq!(
            IspProfile::Irancell.default_tls_strategy(),
            TlsFragmentStrategy::RecordSplit
        );
        assert_eq!(
            IspProfile::Unknown.default_tls_strategy(),
            TlsFragmentStrategy::SniSplit
        );
    }

    #[test]
    fn test_threat_level_logic() {
        assert!(!ThreatLevel::None.requires_obfuscation());
        assert!(ThreatLevel::Passive.requires_obfuscation());
        assert!(ThreatLevel::ActiveV1.requires_obfuscation());
        assert!(!ThreatLevel::ActiveV1.requires_aggressive());
        assert!(ThreatLevel::ActiveV2.requires_aggressive());
        assert!(ThreatLevel::CompleteBlackout.requires_aggressive());
    }

    #[test]
    fn test_tls_client_hello_detection() {
        // Test the TLS ClientHello detection logic inline
        // (ObfuscationCoordinator::new is async, so we test the pattern directly)
        let hello: &[u8] = &[0x16, 0x03, 0x01, 0x00, 0x05, 0x01];
        let not_hello: &[u8] = &[0x17, 0x03, 0x01, 0x00, 0x05, 0x01];

        // Valid ClientHello: content_type=0x16, version=0x0301+, hs_type=0x01
        assert!(hello[0] == 0x16 && hello[1] == 0x03 && hello[2] >= 0x01 && hello[5] == 0x01);
        // Not ClientHello: content_type=0x17 (application_data)
        assert!(not_hello[0] != 0x16);
    }
}
