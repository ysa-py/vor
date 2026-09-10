// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — Anti-Forensics Wipe System
//
// Trigger conditions:
//   TRIGGER_A — 5 rapid taps on the UI panic button
//   TRIGGER_B — SMS containing a pre-shared HMAC token
//   TRIGGER_C — 3 wrong PIN attempts on the UI lock screen
//   TRIGGER_D — Remote wipe command received via MQTT
//
// Wipe procedure MUST complete in <3 seconds:
//   1. Overwrite all SQLite files with crypto-random bytes, then unlink (O_SYNC)
//   2. Overwrite configs/ directory with random bytes, unlink
//   3. Overwrite ai-models/cache/ with random bytes, unlink
//   4. On Android: trigger package removal intent
//   5. On iOS: remove all app containers
//   6. Kill daemon process
//
// After wipe: show steganographic blank calculator UI.
// ─────────────────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rand::RngCore;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;
use tracing::{error, info, warn};

use crate::error::{ErrorCode, ShieldError};
use hex;

// ── Wipe trigger types ───────────────────────────────────────────────────────

/// Enumerates the conditions that can trigger an emergency wipe.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum WipeTrigger {
    /// TRIGGER_A: 5 rapid taps on the UI panic button.
    RapidTap,
    /// TRIGGER_B: SMS containing a pre-shared HMAC token.
    SmsHmacToken,
    /// TRIGGER_C: 3 wrong PIN attempts on the UI lock screen.
    WrongPinAttempts,
    /// TRIGGER_D: Remote wipe command received via MQTT.
    RemoteMqttWipe,
}

/// Result of a wipe operation — which steps succeeded and which failed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WipeResult {
    /// Files that were successfully overwritten and unlinked.
    pub wiped_files: Vec<String>,
    /// Files/directories that could not be wiped (partial failure).
    pub failed_wipes: Vec<(String, String)>,
    /// Whether the platform-specific cleanup (Android/iOS) succeeded.
    pub platform_cleanup: bool,
    /// Total time taken for the wipe operation in milliseconds.
    pub elapsed_ms: u64,
    /// Whether the steganographic UI was activated.
    pub steganographic_ui: bool,
}

// ── WipeController ───────────────────────────────────────────────────────────

/// Controller that monitors the wipe signal and executes the emergency wipe
/// procedure when triggered.
pub struct WipeController {
    /// Receiver for the wipe signal (true = wipe requested).
    wipe_rx: watch::Receiver<bool>,
    /// Platform identifier.
    platform: RuntimePlatform,
    /// Base data directory for the application.
    data_dir: PathBuf,
    /// Paths to wipe (resolved at construction time).
    wipe_paths: Vec<PathBuf>,
    /// Whether steganographic UI mode is enabled after wipe.
    steganographic_mode: bool,
}

/// Runtime platform identifier used by the wipe controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePlatform {
    Linux,
    Android,
    Windows,
    MacOS,
    Ios,
    Unknown,
}

impl WipeController {
    /// Create a new wipe controller.
    pub fn new(platform: RuntimePlatform, wipe_rx: watch::Receiver<bool>) -> Self {
        let data_dir = resolve_data_dir(platform);
        let wipe_paths = build_wipe_paths(&data_dir);

        let steganographic_mode = std::env::var("SHIELD_STEGANO_MODE").unwrap_or_default() == "1";

        Self {
            wipe_rx,
            platform,
            data_dir,
            wipe_paths,
            steganographic_mode,
        }
    }

    /// Monitor the wipe signal and execute the wipe when triggered.
    pub async fn monitor(&mut self, mut shutdown_rx: tokio::sync::broadcast::Receiver<()>) {
        loop {
            tokio::select! {
                // Check for wipe signal
                changed = self.wipe_rx.changed() => {
                    match changed {
                        Ok(()) => {
                            if *self.wipe_rx.borrow() {
                                warn!("EMERGENCY WIPE signal received — executing wipe procedure");
                                let trigger = WipeTrigger::RemoteMqttWipe; // default for programmatic triggers
                                let result = self.execute_wipe(trigger).await;
                                match result {
                                    Ok(wipe_result) => {
                                        if wipe_result.failed_wipes.is_empty() {
                                            info!("Emergency wipe completed successfully in {}ms", wipe_result.elapsed_ms);
                                        } else {
                                            warn!(
                                                failed = wipe_result.failed_wipes.len(),
                                                elapsed_ms = wipe_result.elapsed_ms,
                                                "Emergency wipe completed with partial failures"
                                            );
                                        }
                                        // Kill the daemon process after wipe
                                        self.kill_self();
                                    }
                                    Err(e) => {
                                        error!(error = %e, "EMERGENCY WIPE FAILED — critical security failure");
                                        // Still try to kill the process
                                        self.kill_self();
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            // Channel closed — daemon is shutting down
                            info!("Wipe signal channel closed — stopping monitor");
                            return;
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Wipe controller received shutdown signal");
                    return;
                }
            }
        }
    }

    /// Execute the emergency wipe procedure.
    ///
    /// This MUST complete in under 3 seconds.  Each step is timed and
    /// failures are recorded but do not block subsequent steps.
    pub async fn execute_wipe(&self, _trigger: WipeTrigger) -> Result<WipeResult, ShieldError> {
        let start = std::time::Instant::now();

        let mut wiped_files = Vec::new();
        let mut failed_wipes = Vec::new();

        info!(
            paths_count = self.wipe_paths.len(),
            "Starting emergency wipe procedure"
        );

        // ── Step 1-3: Overwrite files in all wipe paths ────────────────
        for path in &self.wipe_paths {
            match wipe_path(path).await {
                Ok(()) => {
                    wiped_files.push(path.display().to_string());
                }
                Err(e) => {
                    failed_wipes.push((path.display().to_string(), e.to_string()));
                }
            }
        }

        // ── Step 4: Platform-specific cleanup ──────────────────────────
        let platform_cleanup = self.platform_cleanup().await;

        // ── Step 5: Activate steganographic UI ─────────────────────────
        let steganographic_ui = if self.steganographic_mode {
            self.activate_steganographic_ui()
        } else {
            false
        };

        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;

        // Warn if we exceeded the 3-second budget
        if elapsed > std::time::Duration::from_secs(3) {
            warn!(
                elapsed_ms,
                "Wipe procedure exceeded 3-second budget — this is a security concern"
            );
        }

        Ok(WipeResult {
            wiped_files,
            failed_wipes,
            platform_cleanup,
            elapsed_ms,
            steganographic_ui,
        })
    }

    /// Platform-specific cleanup after file wipe.
    async fn platform_cleanup(&self) -> bool {
        match self.platform {
            RuntimePlatform::Android => {
                info!("Android: attempting package removal intent");
                self.android_package_removal().await
            }
            RuntimePlatform::Ios => {
                info!("iOS: removing app containers");
                self.ios_container_removal().await
            }
            RuntimePlatform::Linux | RuntimePlatform::MacOS | RuntimePlatform::Windows => {
                // On desktop platforms, just clear the keychain entries
                info!("Desktop: clearing keychain entries");
                self.desktop_keychain_clear().await
            }
            RuntimePlatform::Unknown => {
                warn!("Unknown platform — skipping platform-specific cleanup");
                false
            }
        }
    }

    /// Android: trigger package removal intent via `pm uninstall`.
    async fn android_package_removal(&self) -> bool {
        // This would typically be called from Java/Kotlin via JNI.
        // From the daemon we can attempt a shell command if we have
        // the necessary permissions (which we usually don't without root).
        let result = tokio::process::Command::new("pm")
            .arg("uninstall")
            .arg("net.micafp.unifiedshield")
            .output()
            .await;

        match result {
            Ok(output) => {
                if output.status.success() {
                    info!("Android package removal initiated");
                    true
                } else {
                    warn!(
                        stderr = %String::from_utf8_lossy(&output.stderr),
                        "Android package removal failed"
                    );
                    false
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to execute pm uninstall");
                false
            }
        }
    }

    /// iOS: remove all app containers.
    async fn ios_container_removal(&self) -> bool {
        // On iOS, apps cannot uninstall themselves.  However, we can
        // remove all data in the app's container directory.
        let container_dir = self.data_dir.join("Containers");
        if container_dir.exists() {
            match wipe_path(&container_dir).await {
                Ok(()) => {
                    info!("iOS container data wiped");
                    true
                }
                Err(e) => {
                    warn!(error = %e, "iOS container wipe failed");
                    false
                }
            }
        } else {
            // Container directory doesn't exist — nothing to clean
            true
        }
    }

    /// Desktop: clear keychain entries.
    async fn desktop_keychain_clear(&self) -> bool {
        // On desktop platforms we clear our own config directory
        // and rely on the OS keychain to be cleared by the device
        // secret manager's Drop implementation.
        let config_dir = self.data_dir.join("configs");
        if config_dir.exists() {
            match wipe_path(&config_dir).await {
                Ok(()) => true,
                Err(e) => {
                    warn!(error = %e, "Desktop keychain/config clear failed");
                    false
                }
            }
        } else {
            true
        }
    }

    /// Activate steganographic mode: after wipe, the app should display
    /// a blank calculator UI to disguise its presence.
    fn activate_steganographic_ui(&self) -> bool {
        info!("Activating steganographic calculator UI");

        // Write a flag file that the Flutter UI checks on startup.
        // The Flutter side is responsible for actually rendering the
        // calculator UI.
        let flag_path = self.data_dir.join(".steg_mode");
        match std::fs::write(&flag_path, b"1") {
            Ok(()) => {
                info!("Steganographic mode flag written");
                true
            }
            Err(e) => {
                warn!(error = %e, "Failed to write steganographic mode flag");
                false
            }
        }
    }

    /// Kill the daemon process.
    fn kill_self(&self) -> ! {
        info!("Killing daemon process as final step of emergency wipe");
        #[cfg(unix)]
        unsafe {
            let _ = libc::kill(libc::getpid(), libc::SIGKILL);
            loop { std::thread::sleep(std::time::Duration::from_secs(1)); }
        }
        #[cfg(windows)]
        {
            // On Windows, terminate the current process
            std::process::exit(1);
        }
        #[cfg(not(any(unix, windows)))]
        {
            std::process::exit(1);
        }
    }
}

// ── File wipe operations ─────────────────────────────────────────────────────

/// Overwrite a file or directory with crypto-random bytes, then unlink it.
///
/// For files: overwrite content with random bytes, sync (O_SYNC), then delete.
/// For directories: recurse into children, wipe each, then remove the directory.
async fn wipe_path(path: &Path) -> Result<(), ShieldError> {
    if !path.exists() {
        // Path doesn't exist — nothing to wipe. Not an error.
        return Ok(());
    }

    if path.is_dir() {
        Box::pin(wipe_directory(path)).await
    } else {
        Box::pin(wipe_file(path)).await
    }
}

/// Overwrite a single file with crypto-random bytes, sync, and unlink.
async fn wipe_file(path: &Path) -> Result<(), ShieldError> {
    let file_size = match tokio::fs::metadata(path).await {
        Ok(meta) => meta.len() as usize,
        Err(e) => {
            return Err(ShieldError::anti_forensics(
                ErrorCode::AntiForensicsWipeFailed,
                format!("Cannot stat file {:?}: {}", path, e),
            ));
        }
    };

    if file_size == 0 {
        // Empty file — just unlink
        tokio::fs::remove_file(path)
            .await
            .map_err(|e| wipe_error(path, e))?;
        return Ok(());
    }

    // Generate crypto-random bytes to overwrite the file content
    let mut random_bytes = vec![0u8; file_size];
    rand::thread_rng().fill_bytes(&mut random_bytes);

    // Open the file for writing (overwrite mode)
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .truncate(false) // Don't truncate — overwrite in place
        .open(path)
        .await
        .map_err(|e| wipe_error(path, e))?;

    // Write random bytes
    file.write_all(&random_bytes)
        .await
        .map_err(|e| wipe_error(path, e))?;

    // Sync to ensure data is written to disk (O_SYNC semantics)
    file.sync_all().await.map_err(|e| wipe_error(path, e))?;

    // Close the file
    drop(file);

    // Write a second pass with different random data for extra security
    // (some SSDs and filesystems may use copy-on-write)
    let mut random_bytes2 = vec![0u8; file_size];
    rand::thread_rng().fill_bytes(&mut random_bytes2);

    let mut file2 = tokio::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .await
        .map_err(|e| wipe_error(path, e))?;

    file2
        .write_all(&random_bytes2)
        .await
        .map_err(|e| wipe_error(path, e))?;

    file2.sync_all().await.map_err(|e| wipe_error(path, e))?;

    drop(file2);

    // Finally, unlink (delete) the file
    tokio::fs::remove_file(path)
        .await
        .map_err(|e| wipe_error(path, e))?;

    Ok(())
}

/// Recursively wipe a directory: wipe all children, then remove the directory.
async fn wipe_directory(path: &Path) -> Result<(), ShieldError> {
    let mut entries = tokio::fs::read_dir(path)
        .await
        .map_err(|e| wipe_error(path, e))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| wipe_error(path, e))?
    {
        let entry_path = entry.path();
        // Wipe each child — individual failures are logged but don't stop the process
        if let Err(e) = Box::pin(wipe_path(&entry_path)).await {
            warn!(path = %entry_path.display(), error = %e, "Failed to wipe child path");
        }
    }

    // Remove the now-empty directory
    tokio::fs::remove_dir(path)
        .await
        .map_err(|e| wipe_error(path, e))?;

    Ok(())
}

/// Build the list of paths to wipe based on the data directory.
fn build_wipe_paths(data_dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Step 1: SQLite database files
    paths.push(data_dir.join("db"));
    paths.push(data_dir.join("db.sqlite"));
    paths.push(data_dir.join("db.sqlite-wal"));
    paths.push(data_dir.join("db.sqlite-shm"));

    // Step 2: Configuration directory
    paths.push(data_dir.join("configs"));

    // Step 3: AI models and cache
    paths.push(data_dir.join("ai-models"));
    paths.push(data_dir.join("cache"));

    // Additional sensitive paths
    paths.push(data_dir.join("logs"));
    paths.push(data_dir.join("peers.dat"));
    paths.push(data_dir.join("device-secret.enc"));

    paths
}

/// Resolve the application data directory for the given platform.
fn resolve_data_dir(platform: RuntimePlatform) -> PathBuf {
    match platform {
        RuntimePlatform::Android => {
            // Android uses the app's internal storage
            PathBuf::from("/data/data/net.micafp.unifiedshield/files")
        }
        RuntimePlatform::Ios => {
            // iOS uses the app's Documents directory
            PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join("Documents")
                .join("shield")
        }
        RuntimePlatform::Linux | RuntimePlatform::MacOS => {
            // Use XDG data dir on Linux, Application Support on macOS
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("unified-shield")
        }
        RuntimePlatform::Windows => {
            // Use %LOCALAPPDATA% on Windows
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("C:\\Temp"))
                .join("UnifiedShield")
        }
        RuntimePlatform::Unknown => PathBuf::from("/tmp/unified-shield"),
    }
}

/// Helper to create a ShieldError for wipe failures.
fn wipe_error(path: &Path, source: std::io::Error) -> ShieldError {
    ShieldError::anti_forensics(
        ErrorCode::AntiForensicsWipeFailed,
        format!("Wipe failed for {:?}: {}", path, source),
    )
}

// ── HMAC token validation for SMS trigger ────────────────────────────────────

/// Validate an SMS HMAC token against the device secret.
///
/// The token is computed as:
/// ```text
/// token = HMAC-SHA256(key=device_secret, msg=sms_body)
/// ```
///
/// This prevents adversaries from triggering a wipe by sending arbitrary SMS.
pub fn validate_sms_hmac_token(
    device_secret: &[u8; 32],
    sms_body: &str,
    expected_token_hex: &str,
) -> bool {
    use ring::hmac;

    let key = hmac::Key::new(hmac::HMAC_SHA256, device_secret);
    let tag = hmac::sign(&key, sms_body.as_bytes());
    let computed_hex = hex::encode(tag);

    // Constant-time comparison to prevent timing attacks
    ring::constant_time::verify_slices_are_equal(
        computed_hex.as_bytes(),
        expected_token_hex.as_bytes(),
    )
    .is_ok()
}

// ── Rapid tap detector ───────────────────────────────────────────────────────

/// Tracks rapid taps to detect the TRIGGER_A panic pattern.
pub struct RapidTapDetector {
    /// Timestamps of recent taps.
    tap_times: Vec<std::time::Instant>,
    /// Number of taps required to trigger.
    threshold: usize,
    /// Maximum time window for the taps in milliseconds.
    window_ms: u64,
}

impl RapidTapDetector {
    /// Create a new detector with the given threshold and window.
    pub fn new(threshold: usize, window_ms: u64) -> Self {
        Self {
            tap_times: Vec::with_capacity(threshold),
            threshold,
            window_ms,
        }
    }

    /// Record a tap and return true if the threshold is met within the window.
    pub fn tap(&mut self) -> bool {
        let now = std::time::Instant::now();
        self.tap_times.push(now);

        // Remove taps outside the window
        let cutoff = now - std::time::Duration::from_millis(self.window_ms);
        self.tap_times.retain(|&t| t >= cutoff);

        self.tap_times.len() >= self.threshold
    }

    /// Reset the detector.
    pub fn reset(&mut self) {
        self.tap_times.clear();
    }
}

// ── Wrong PIN tracker ────────────────────────────────────────────────────────

/// Tracks wrong PIN attempts to detect TRIGGER_C.
pub struct PinAttemptTracker {
    /// Number of consecutive wrong attempts.
    wrong_count: u32,
    /// Maximum wrong attempts before triggering wipe.
    max_attempts: u32,
}

impl PinAttemptTracker {
    /// Create a new tracker with the given max attempts.
    pub fn new(max_attempts: u32) -> Self {
        Self {
            wrong_count: 0,
            max_attempts,
        }
    }

    /// Record a wrong PIN attempt. Returns true if the threshold is met.
    pub fn wrong_attempt(&mut self) -> bool {
        self.wrong_count += 1;
        self.wrong_count >= self.max_attempts
    }

    /// Record a correct PIN — reset the counter.
    pub fn correct_attempt(&mut self) {
        self.wrong_count = 0;
    }

    /// Current wrong attempt count.
    pub fn wrong_count(&self) -> u32 {
        self.wrong_count
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rapid_tap_detector_triggers() {
        let mut detector = RapidTapDetector::new(5, 3000);
        // 4 taps should not trigger
        for _ in 0..4 {
            assert!(!detector.tap());
        }
        // 5th tap should trigger
        assert!(detector.tap());
    }

    #[test]
    fn test_rapid_tap_detector_resets() {
        let mut detector = RapidTapDetector::new(3, 100);
        assert!(!detector.tap());
        assert!(!detector.tap());
        detector.reset();
        // After reset, we need 3 more taps
        assert!(!detector.tap());
        assert!(!detector.tap());
        assert!(detector.tap());
    }

    #[test]
    fn test_pin_attempt_tracker() {
        let mut tracker = PinAttemptTracker::new(3);
        assert!(!tracker.wrong_attempt()); // 1
        assert!(!tracker.wrong_attempt()); // 2
        assert!(tracker.wrong_attempt()); // 3 — trigger!
        tracker.correct_attempt();
        assert_eq!(tracker.wrong_count(), 0);
    }

    #[test]
    fn test_validate_sms_hmac() {
        let secret = [42u8; 32];
        let sms_body = "WIPE:confirm";

        // Compute the expected token
        use ring::hmac;
        let key = hmac::Key::new(hmac::HMAC_SHA256, &secret);
        let tag = hmac::sign(&key, sms_body.as_bytes());
        let token_hex = hex::encode(tag);

        // Valid token
        assert!(validate_sms_hmac_token(&secret, sms_body, &token_hex));

        // Wrong token
        assert!(!validate_sms_hmac_token(&secret, sms_body, "deadbeef"));

        // Wrong body
        assert!(!validate_sms_hmac_token(&secret, "wrong body", &token_hex));
    }
}
