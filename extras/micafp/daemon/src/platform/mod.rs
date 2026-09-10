#[cfg(target_os = "android")]
pub mod android;
#[cfg(target_os = "ios")]
pub mod ios;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "windows")]
pub mod windows;

use std::fmt;

/// Platform-specific error type.
#[derive(Debug, Clone)]
pub struct PlatformError(pub String);

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Platform error: {}", self.0)
    }
}

impl std::error::Error for PlatformError {}

/// Platform context for platform-specific operations.
pub struct PlatformContext;

impl PlatformContext {
    pub fn new() -> Self {
        PlatformContext
    }
}
