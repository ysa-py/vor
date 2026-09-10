//! Linux platform module for MICAFP-UnifiedShield-6.0
//!
//! This module provides Linux-specific DPI circumvention using
//! Zapret-style nfqueue packet mangling.

pub mod zapret;

use crate::platform::PlatformError;

/// Initialize the Linux platform subsystem.
pub async fn init() -> Result<(), PlatformError> {
    tracing::info!("Initializing Linux platform subsystem");
    Ok(())
}

/// Shut down the Linux platform subsystem.
pub async fn shutdown() -> Result<(), PlatformError> {
    tracing::info!("Shutting down Linux platform subsystem");
    Ok(())
}
