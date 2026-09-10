// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Fallback Chain
//
// Ordered chain of fallback strategies for extreme censorship scenarios.
// Each strategy is tried in priority order. If all fail, the mesh network
// and ICMP tunnel are used as last resort.
//
// Default fallback order:
//   1. Primary transport (VLESS/ShadowTLS/Reality)
//   2. CDN Workers (Chinese CDN)
//   3. P2P libp2p relay
//   4. DoH tunnel (DNS-over-HTTPS)
//   5. ICMP tunnel
//   6. Mesh network (BLE + WiFi Aware)
//   7. Tor Bridges (Snowflake/Meek)
// ─────────────────────────────────────────────────────────────────────────────

use std::sync::Arc;
use tracing::{info, warn};

/// A single fallback strategy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub enum FallbackStrategy {
    PrimaryTransport,
    ChineseCdnWorker,
    P2pLibp2pRelay,
    DohTunnel,
    IcmpTunnel,
    MeshNetwork,
    TorBridgeSnowflake,
    TorBridgeMeek,
}

/// Manages the ordered fallback chain with per-strategy health tracking.
pub struct FallbackChain {
    strategies: Vec<FallbackStrategy>,
    current_index: parking_lot::Mutex<usize>,
    failures: parking_lot::Mutex<std::collections::HashMap<FallbackStrategy, u32>>,
}

impl FallbackChain {
    /// Create the default fallback chain for Iran censorship scenarios.
    pub fn default_for_iran() -> Self {
        Self {
            strategies: vec![
                FallbackStrategy::PrimaryTransport,
                FallbackStrategy::ChineseCdnWorker,
                FallbackStrategy::P2pLibp2pRelay,
                FallbackStrategy::DohTunnel,
                FallbackStrategy::IcmpTunnel,
                FallbackStrategy::MeshNetwork,
                FallbackStrategy::TorBridgeSnowflake,
                FallbackStrategy::TorBridgeMeek,
            ],
            current_index: parking_lot::Mutex::new(0),
            failures: parking_lot::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Get the current active strategy.
    pub fn current(&self) -> &FallbackStrategy {
        let idx = *self.current_index.lock();
        &self.strategies[idx.min(self.strategies.len() - 1)]
    }

    /// Record failure of current strategy and advance to next.
    pub fn advance(&self) -> Option<&FallbackStrategy> {
        let mut idx = self.current_index.lock();
        let failed = &self.strategies[*idx];
        *self.failures.lock().entry(failed.clone()).or_insert(0) += 1;
        warn!("fallback_chain: {:?} failed, advancing", failed);

        if *idx + 1 < self.strategies.len() {
            *idx += 1;
            let next = &self.strategies[*idx];
            info!("fallback_chain: activating {:?}", next);
            Some(next)
        } else {
            warn!("fallback_chain: all strategies exhausted, staying on last");
            None
        }
    }

    /// Reset to primary transport (called after successful reconnection).
    pub fn reset(&self) {
        *self.current_index.lock() = 0;
        info!("fallback_chain: reset to PrimaryTransport");
    }

    pub fn all_strategies(&self) -> &[FallbackStrategy] {
        &self.strategies
    }
}
