// ─────────────────────────────────────────────────────────────────────────────
// Resilience subsystem — circuit breaker, retry, fallback chain, watchdog
// MICAFP-UnifiedShield-vip-ultra-Quantum-ultra v8.0
// ─────────────────────────────────────────────────────────────────────────────

pub mod circuit_breaker;
pub mod fallback_chain;
pub mod retry_policy;
pub mod watchdog;

pub use circuit_breaker::CircuitBreaker;
pub use fallback_chain::FallbackChain;
pub use retry_policy::RetryPolicy;
pub use watchdog::Watchdog;

/// Alias: SubsystemWatchdog is the same as the general Watchdog struct.
pub type SubsystemWatchdog = watchdog::Watchdog;
