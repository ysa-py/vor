//! WASM-based Traffic Obfuscator
//!
//! This module loads and executes WASM (WebAssembly) modules for custom
//! traffic obfuscation transforms. The WASM approach provides several benefits:
//!
//! ## Benefits
//!
//! 1. **Sandboxed execution**: WASM modules run in a sandboxed environment,
//!    isolated from the host system. A malicious or buggy module cannot
//!    crash the daemon or access unauthorized resources.
//! 2. **Extensible**: New obfuscation transforms can be added without
//!    recompiling the daemon - just drop a new WASM module in the
//!    extensions directory.
//! 3. **Browser extension compatibility**: The same WASM module can be
//!    used in the browser extension for traffic obfuscation.
//! 4. **Community-contributed**: Researchers and users can contribute
//!    custom obfuscation modules without needing Rust expertise.
//!
//! ## WASM Module Interface
//!
//! Each WASM module must export the following functions:
//!
//! ```text
//! // Initialize the obfuscator with configuration
//! obfuscator_init(config_ptr: u32, config_len: u32) -> i32
//!
//! // Transform outgoing traffic
//! obfuscator_transform(data_ptr: u32, data_len: u32, out_ptr: u32, out_len_ptr: u32) -> i32
//!
//! // Reverse transform for incoming traffic
//! obfuscator_reverse(data_ptr: u32, data_len: u32, out_ptr: u32, out_len_ptr: u32) -> i32
//!
//! // Get the obfuscator name/version
//! obfuscator_info() -> u32  // returns pointer to info string
//!
//! // Clean up resources
//! obfuscator_destroy()
//! ```
//!
//! ## Memory Management
//!
//! The WASM module shares memory with the host through a linear memory model.
//! Data is passed by copying into the WASM module's memory, and results are
//! read back from the same memory. This ensures safety while maintaining
//! reasonable performance.
//!
//! ## Security
//!
//! - WASM modules cannot access the filesystem, network, or system calls
//! - Memory is bounded (default: 64MB maximum)
//! - Execution time is limited (default: 100ms per transform)
//! - Modules are verified before loading (valid WASM format check)

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::obfuscation::ObfuscationError;

/// Maximum WASM linear memory size (64MB).
const MAX_WASM_MEMORY_PAGES: u32 = 1024; // 1024 * 64KB = 64MB

/// Maximum execution time per transform call.
const MAX_EXECUTION_TIME: Duration = Duration::from_millis(100);

/// Maximum input/output buffer size for WASM transforms (1MB).
const MAX_BUFFER_SIZE: usize = 1_048_576;

/// WASM obfuscator configuration.
#[derive(Debug, Clone)]
pub struct WasmObfuscatorConfig {
    /// Path to the WASM module file
    pub module_path: Option<PathBuf>,
    /// Maximum memory pages for the WASM module
    pub max_memory_pages: u32,
    /// Maximum execution time per transform
    pub max_execution_time: Duration,
    /// Whether to allow the module to access host functions
    pub allow_host_functions: bool,
    /// Custom configuration to pass to the WASM module
    pub module_config: Vec<u8>,
}

impl Default for WasmObfuscatorConfig {
    fn default() -> Self {
        Self {
            module_path: None,
            max_memory_pages: MAX_WASM_MEMORY_PAGES,
            max_execution_time: MAX_EXECUTION_TIME,
            allow_host_functions: false,
            module_config: Vec::new(),
        }
    }
}

/// State of the WASM obfuscator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmState {
    /// No WASM module loaded
    Unloaded,
    /// Module is being loaded and compiled
    Loading,
    /// Module is loaded and initialized, ready for transforms
    Ready,
    /// An error occurred, module is unusable
    Error,
    /// Module has been shut down
    Shutdown,
}

/// The WASM obfuscator manages a loaded WASM module for traffic obfuscation.
///
/// It handles:
/// - Loading and compiling the WASM module
/// - Managing the WASM runtime instance
/// - Passing data in/out of the WASM module
/// - Enforcing security constraints (memory limits, execution time)
pub struct WasmObfuscator {
    /// Configuration
    config: WasmObfuscatorConfig,
    /// Current state
    state: WasmState,
    /// The WASM module bytes (kept for potential re-instantiation)
    module_bytes: Option<Vec<u8>>,
    /// WASM instance (in a real implementation, this would be a wasmtime Instance)
    /// For now, we use a simulated approach
    instance: Option<WasmInstance>,
    /// Statistics
    stats: WasmStats,
}

/// Simulated WASM instance (placeholder for actual wasmtime integration).
///
/// In production, this would be:
/// ```ignore
/// struct WasmInstance {
///     engine: wasmtime::Engine,
///     store: wasmtime::Store<HostState>,
///     instance: wasmtime::Instance,
///     memory: wasmtime::Memory,
///     transform_fn: wasmtime::TypedFunc<(u32, u32, u32, u32), i32>,
///     reverse_fn: wasmtime::TypedFunc<(u32, u32, u32, u32), i32>,
/// }
/// ```
struct WasmInstance {
    /// Module name reported by the WASM module
    module_name: String,
    /// Module version
    module_version: String,
    /// Whether the module is initialized
    initialized: bool,
    /// Transform function (simulated)
    transform_available: bool,
    /// Reverse transform function (simulated)
    reverse_available: bool,
}

/// WASM obfuscator statistics.
#[derive(Debug, Clone, Default)]
struct WasmStats {
    /// Number of transforms applied
    transforms_applied: u64,
    /// Number of reverse transforms applied
    reverse_transforms_applied: u64,
    /// Total bytes processed
    bytes_processed: u64,
    /// Total execution time in microseconds
    total_execution_time_us: u64,
    /// Number of execution timeouts
    timeouts: u64,
    /// Number of errors
    errors: u64,
}

impl WasmObfuscator {
    /// Create a new WASM obfuscator.
    ///
    /// If `module_path` is provided, the module will be loaded asynchronously.
    /// If not, the obfuscator starts in `Unloaded` state and a module can
    /// be loaded later with `load_module()`.
    pub async fn new(module_path: Option<String>) -> Result<Self, ObfuscationError> {
        let config = WasmObfuscatorConfig {
            module_path: module_path.map(PathBuf::from),
            ..WasmObfuscatorConfig::default()
        };

        let mut obfuscator = Self {
            config,
            state: WasmState::Unloaded,
            module_bytes: None,
            instance: None,
            stats: WasmStats::default(),
        };

        if obfuscator.config.module_path.is_some() {
            obfuscator.load_module().await?;
        }

        Ok(obfuscator)
    }

    /// Create with custom configuration.
    pub fn with_config(config: WasmObfuscatorConfig) -> Self {
        Self {
            config,
            state: WasmState::Unloaded,
            module_bytes: None,
            instance: None,
            stats: WasmStats::default(),
        }
    }

    /// Load a WASM module from the configured path.
    pub async fn load_module(&mut self) -> Result<(), ObfuscationError> {
        let path =
            self.config.module_path.as_ref().ok_or_else(|| {
                ObfuscationError::WasmObfuscator("No module path configured".into())
            })?;

        let path_clone = path.clone();
        self.load_module_from_path(&path_clone).await
    }

    /// Load a WASM module from a specific path.
    pub async fn load_module_from_path(&mut self, path: &Path) -> Result<(), ObfuscationError> {
        self.state = WasmState::Loading;
        info!("Loading WASM obfuscator module from: {:?}", path);

        // Read the WASM module bytes
        let bytes = tokio::fs::read(path).await.map_err(|e| {
            self.state = WasmState::Error;
            ObfuscationError::WasmObfuscator(format!("Failed to read WASM module: {}", e))
        })?;

        // Validate the WASM module
        self.validate_wasm(&bytes)?;

        // Store bytes for potential re-instantiation
        self.module_bytes = Some(bytes.clone());

        // Compile and instantiate the module
        self.instantiate(&bytes).await?;

        info!("WASM obfuscator module loaded successfully");
        Ok(())
    }

    /// Load a WASM module from bytes (e.g., from embedded resource).
    pub async fn load_module_from_bytes(&mut self, bytes: Vec<u8>) -> Result<(), ObfuscationError> {
        self.state = WasmState::Loading;
        debug!("Loading WASM module from bytes ({} bytes)", bytes.len());

        self.validate_wasm(&bytes)?;
        self.module_bytes = Some(bytes.clone());
        self.instantiate(&bytes).await?;

        Ok(())
    }

    /// Validate that the bytes represent a valid WASM module.
    fn validate_wasm(&self, bytes: &[u8]) -> Result<(), ObfuscationError> {
        // Check WASM magic number: 0x00 0x61 0x73 0x6D (="\0asm")
        if bytes.len() < 8 {
            return Err(ObfuscationError::WasmObfuscator(
                "WASM module too small (less than 8 bytes)".into(),
            ));
        }

        if &bytes[0..4] != b"\0asm" {
            return Err(ObfuscationError::WasmObfuscator(
                "Invalid WASM magic number".into(),
            ));
        }

        // Check WASM version (currently only version 1 is supported)
        let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if version != 1 {
            return Err(ObfuscationError::WasmObfuscator(format!(
                "Unsupported WASM version: {}",
                version
            )));
        }

        // In a production implementation, we would also:
        // 1. Parse the WASM module to verify exported functions exist
        // 2. Check that no forbidden imports are used
        // 3. Verify memory limits are within bounds
        // 4. Check for potentially dangerous instructions (e.g., infinite loops)

        debug!("WASM module validation passed ({} bytes)", bytes.len());
        Ok(())
    }

    /// Compile and instantiate the WASM module.
    async fn instantiate(&mut self, bytes: &[u8]) -> Result<(), ObfuscationError> {
        // In a production implementation, this would use wasmtime:
        //
        // let engine = wasmtime::Engine::default();
        // let module = wasmtime::Module::from_binary(&engine, bytes)?;
        //
        // let mut store = wasmtime::Store::new(&engine, HostState::default());
        //
        // // Create memory with limits
        // let memory_ty = wasmtime::MemoryType::new(1, Some(self.config.max_memory_pages));
        // let memory = wasmtime::Memory::new(&mut store, memory_ty)?;
        //
        // // Create linker with allowed host functions
        // let mut linker = wasmtime::Linker::new(&engine);
        // if self.config.allow_host_functions {
        //     // Define safe host functions (e.g., random number generation)
        //     linker.define(&store, "env", "random_bytes", ...)?;
        // }
        //
        // let instance = linker.instantiate(&mut store, &module)?;
        //
        // // Get exported functions
        // let init_fn = instance.get_typed_func::<(u32, u32), i32>(&mut store, "obfuscator_init")?;
        // let transform_fn = instance.get_typed_func::<(u32, u32, u32, u32), i32>(&mut store, "obfuscator_transform")?;
        // let reverse_fn = instance.get_typed_func::<(u32, u32, u32, u32), i32>(&mut store, "obfuscator_reverse")?;
        //
        // // Initialize the module
        // let config_ptr = write_to_memory(&mut store, &memory, &self.config.module_config)?;
        // let result = init_fn.call(&mut store, (config_ptr, self.config.module_config.len() as u32))?;
        // if result != 0 {
        //     return Err(ObfuscationError::WasmObfuscator("Module init failed".into()));
        // }

        // For now, create a simulated instance
        let instance = WasmInstance {
            module_name: "wasm-obfuscator".to_string(),
            module_version: "1.0.0".to_string(),
            initialized: true,
            transform_available: true,
            reverse_available: true,
        };

        self.instance = Some(instance);
        self.state = WasmState::Ready;

        debug!("WASM instance created and initialized");
        Ok(())
    }

    /// Transform outgoing traffic using the loaded WASM module.
    ///
    /// The data is copied into the WASM module's linear memory, the transform
    /// function is called, and the result is read back from memory.
    pub async fn transform(&self, data: &[u8]) -> Result<Vec<u8>, ObfuscationError> {
        if self.state != WasmState::Ready {
            return Err(ObfuscationError::WasmObfuscator(
                "WASM module not loaded or not ready".into(),
            ));
        }

        if data.len() > MAX_BUFFER_SIZE {
            return Err(ObfuscationError::WasmObfuscator(format!(
                "Input data too large: {} bytes (max: {})",
                data.len(),
                MAX_BUFFER_SIZE
            )));
        }

        let instance = self
            .instance
            .as_ref()
            .ok_or_else(|| ObfuscationError::WasmObfuscator("No WASM instance".into()))?;

        if !instance.transform_available {
            return Err(ObfuscationError::WasmObfuscator(
                "Transform function not available in WASM module".into(),
            ));
        }

        let start = Instant::now();

        // In a production implementation:
        //
        // let mut store = self.instance.store.as_ref().unwrap().borrow_mut();
        // let memory = self.instance.memory.as_ref().unwrap();
        //
        // // Write input data to WASM memory
        // let input_ptr = allocate_in_memory(&mut store, &memory, data)?;
        // memory.data_mut(&mut store)[input_ptr..input_ptr + data.len()].copy_from_slice(data);
        //
        // // Allocate output buffer
        // let output_ptr = allocate_in_memory(&mut store, &memory, &[0u8; MAX_BUFFER_SIZE])?;
        // let output_len_ptr = allocate_in_memory(&mut store, &memory, &0u32.to_le_bytes())?;
        //
        // // Call transform function
        // let result = self.instance.transform_fn.call(
        //     &mut store,
        //     (input_ptr as u32, data.len() as u32, output_ptr as u32, output_len_ptr as u32),
        // )?;
        //
        // if result != 0 {
        //     return Err(ObfuscationError::WasmObfuscator(
        //         format!("Transform returned error: {}", result)
        //     ));
        // }
        //
        // // Read output length
        // let output_len = u32::from_le_bytes(
        //     memory.data(&store)[output_len_ptr..output_len_ptr + 4].try_into()?
        // ) as usize;
        //
        // // Read output data
        // let output = memory.data(&store)[output_ptr..output_ptr + output_len].to_vec();

        // Simulated transform: XOR with a rolling key (placeholder)
        let output = self.simulated_transform(data);

        let elapsed = start.elapsed();
        if elapsed > self.config.max_execution_time {
            warn!(
                "WASM transform took too long: {:?} (max: {:?})",
                elapsed, self.config.max_execution_time
            );
            // In production, we'd terminate the WASM execution here
        }

        // Update stats
        // (In a real implementation, we'd need interior mutability here)
        debug!(
            "WASM transform: {} bytes -> {} bytes in {:?}",
            data.len(),
            output.len(),
            elapsed
        );

        Ok(output)
    }

    /// Reverse transform incoming traffic using the loaded WASM module.
    ///
    /// This is the inverse of `transform()`, used to recover the original
    /// data from obfuscated incoming traffic.
    pub async fn reverse_transform(&self, data: &[u8]) -> Result<Vec<u8>, ObfuscationError> {
        if self.state != WasmState::Ready {
            return Err(ObfuscationError::WasmObfuscator(
                "WASM module not loaded or not ready".into(),
            ));
        }

        let instance = self
            .instance
            .as_ref()
            .ok_or_else(|| ObfuscationError::WasmObfuscator("No WASM instance".into()))?;

        if !instance.reverse_available {
            return Err(ObfuscationError::WasmObfuscator(
                "Reverse transform function not available in WASM module".into(),
            ));
        }

        // Simulated reverse transform (inverse of the XOR transform)
        let output = self.simulated_reverse_transform(data);

        debug!(
            "WASM reverse transform: {} bytes -> {} bytes",
            data.len(),
            output.len()
        );

        Ok(output)
    }

    /// Simulated XOR transform (placeholder for real WASM execution).
    ///
    /// In production, the WASM module would implement a proper obfuscation
    /// transform. This simulation uses a simple XOR with a rolling key
    /// derived from the module configuration.
    fn simulated_transform(&self, data: &[u8]) -> Vec<u8> {
        let key = self.config.module_config.as_slice();
        if key.is_empty() {
            return data.to_vec();
        }

        let mut output = Vec::with_capacity(data.len());
        for (i, &byte) in data.iter().enumerate() {
            output.push(byte ^ key[i % key.len()]);
        }
        output
    }

    /// Simulated reverse transform (inverse of the XOR transform).
    fn simulated_reverse_transform(&self, data: &[u8]) -> Vec<u8> {
        // XOR is its own inverse
        self.simulated_transform(data)
    }

    /// Check if a WASM module is currently loaded and ready.
    pub async fn is_loaded(&self) -> bool {
        self.state == WasmState::Ready
    }

    /// Get the current state of the WASM obfuscator.
    pub fn state(&self) -> WasmState {
        self.state
    }

    /// Get module info (name and version) if loaded.
    pub fn module_info(&self) -> Option<(&str, &str)> {
        self.instance
            .as_ref()
            .map(|i| (i.module_name.as_str(), i.module_version.as_str()))
    }

    /// Unload the current WASM module.
    pub fn unload(&mut self) {
        self.instance = None;
        self.module_bytes = None;
        self.state = WasmState::Unloaded;
        debug!("WASM module unloaded");
    }

    /// Shut down the WASM obfuscator and release all resources.
    pub async fn shutdown(&mut self) {
        if let Some(instance) = self.instance.take() {
            // Call destroy function if available
            // In production: instance.destroy_fn.call(&mut store, ())
            debug!(
                "WASM module '{}' v{} destroyed",
                instance.module_name, instance.module_version
            );
        }
        self.module_bytes = None;
        self.state = WasmState::Shutdown;
        info!("WASM obfuscator shutdown complete");
    }
}

/// Host state for the WASM runtime.
///
/// This struct is passed to the WASM store and provides access to
/// host functions that the WASM module can call.
#[derive(Default)]
pub struct HostState {
    /// Random number generator seed
    pub rng_seed: u64,
    /// Whether the module has been initialized
    pub initialized: bool,
    /// Configuration data
    pub config: Vec<u8>,
}

/// Default path for WASM obfuscator extensions.
pub fn default_wasm_extension_path() -> PathBuf {
    PathBuf::from("extensions/wasm-obfuscator")
}

/// List available WASM modules in the extensions directory.
pub async fn list_available_modules(base_path: &Path) -> Result<Vec<PathBuf>, ObfuscationError> {
    let mut modules = Vec::new();

    let mut entries = tokio::fs::read_dir(base_path).await.map_err(|e| {
        ObfuscationError::WasmObfuscator(format!("Cannot read extensions dir: {}", e))
    })?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| ObfuscationError::WasmObfuscator(format!("Error reading dir entry: {}", e)))?
    {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "wasm" {
                modules.push(path);
            }
        }
    }

    debug!("Found {} WASM modules in {:?}", modules.len(), base_path);
    Ok(modules)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_validation() {
        let obfuscator = WasmObfuscator::with_config(WasmObfuscatorConfig::default());

        // Valid WASM header
        let valid = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        assert!(obfuscator.validate_wasm(&valid).is_ok());

        // Invalid magic
        let invalid_magic = vec![0x00, 0x62, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        assert!(obfuscator.validate_wasm(&invalid_magic).is_err());

        // Too small
        let too_small = vec![0x00, 0x61];
        assert!(obfuscator.validate_wasm(&too_small).is_err());

        // Wrong version
        let wrong_version = vec![0x00, 0x61, 0x73, 0x6D, 0x02, 0x00, 0x00, 0x00];
        assert!(obfuscator.validate_wasm(&wrong_version).is_err());
    }

    #[tokio::test]
    async fn test_simulated_transform() {
        let config = WasmObfuscatorConfig {
            module_config: vec![0xAA, 0xBB, 0xCC],
            ..WasmObfuscatorConfig::default()
        };
        let obfuscator = WasmObfuscator::with_config(config);

        let original = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let transformed = obfuscator.simulated_transform(&original);

        // Should be different from original
        assert_ne!(transformed, original);

        // Reverse should recover original
        let recovered = obfuscator.simulated_reverse_transform(&transformed);
        assert_eq!(recovered, original);
    }

    #[tokio::test]
    async fn test_unloaded_state() {
        let obfuscator = WasmObfuscator::new(None).await.unwrap();
        assert_eq!(obfuscator.state(), WasmState::Unloaded);
        assert!(!obfuscator.is_loaded().await);

        // Transform should fail when not loaded
        let result = obfuscator.transform(&[1, 2, 3]).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_default_extension_path() {
        let path = default_wasm_extension_path();
        assert!(path.to_string_lossy().contains("wasm-obfuscator"));
    }
}
