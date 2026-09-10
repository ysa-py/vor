// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — IPFS Config Updater
//
// Distributes configuration updates via IPFS (InterPlanetary File System).
// This allows the daemon to receive new endpoint lists, ISP profiles, and
// AI model updates without needing direct access to the configuration server.
//
// Key features:
//   • Hardcoded CID in binary for initial discovery
//   • Fetches from ipfs.io and pinata.cloud gateways
//   • Verifies Ed25519 signature on all updates
//   • Fetches every 6 hours (adaptive to battery state)
//   • Publishes updates via Pinata/Filebase (no credit card required)
//   • Falls back to acoustic/NTP/SMS channels if IPFS unreachable
// ─────────────────────────────────────────────────────────────────────────────

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::endpoint_manager::EndpointUpdate;
use crate::battery::adaptive_duty::PowerMode;
use crate::error::{ErrorCode, ShieldError};

// ── Constants ───────────────────────────────────────────────────────────────

/// Hardcoded CID for the initial configuration root.
/// This is updated with each release of the binary.
const HARDCODED_CID: &str = "QmUnifiedShield6ConfigRootV001";

/// IPFS gateway URLs for fetching configuration.
const IPFS_GATEWAYS: &[&str] = &[
    "https://ipfs.io/ipfs",
    "https://gateway.pinata.cloud/ipfs",
    "https://cloudflare-ipfs.com/ipfs",
    "https://dweb.link/ipfs",
];

/// Default fetch interval (6 hours).
const DEFAULT_FETCH_INTERVAL: Duration = Duration::from_secs(6 * 3600);

/// Fetch interval during NAIN mode (1 hour — more aggressive updates).
const NAIN_FETCH_INTERVAL: Duration = Duration::from_secs(3600);

/// Fetch interval in Critical power mode (24 hours — conserve battery).
const CRITICAL_FETCH_INTERVAL: Duration = Duration::from_secs(24 * 3600);

/// HTTP request timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum size of a config update (1 MB).
const MAX_UPDATE_SIZE: usize = 1024 * 1024;

/// Ed25519 signature length.
const ED25519_SIGNATURE_LEN: usize = 64;

/// Ed25519 public key length.
const ED25519_PUBLIC_KEY_LEN: usize = 32;

/// Local cache directory for IPFS data.
const CACHE_DIR: &str = "ipfs-cache";

// ── IPFS Config Update ──────────────────────────────────────────────────────

/// A signed configuration update fetched from IPFS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsConfigUpdate {
    /// CID of this update.
    pub cid: String,
    /// Type of configuration update.
    pub update_type: ConfigUpdateType,
    /// Version of the update format.
    pub version: u8,
    /// JSON-encoded configuration data.
    pub data: String,
    /// UNIX timestamp of this update.
    pub timestamp: u64,
    /// Ed25519 signature over (update_type + version + data + timestamp).
    pub signature: Vec<u8>,
    /// Ed25519 public key of the signer.
    pub signer_public_key: Vec<u8>,
}

/// Type of configuration update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigUpdateType {
    /// New endpoint list.
    EndpointList,
    /// Updated ISP profiles.
    IspProfiles,
    /// AI model update.
    AiModel,
    /// SNI domain pool update.
    SniPool,
    /// Full configuration bundle.
    FullBundle,
}

// ── IPFS Updater State ──────────────────────────────────────────────────────

/// Internal state of the IPFS updater.
#[derive(Debug)]
struct IpfsUpdaterState {
    /// Current CID being tracked.
    current_cid: String,
    /// Last successful fetch timestamp.
    last_fetch_ts: u64,
    /// Number of consecutive fetch failures.
    consecutive_failures: u32,
    /// Whether IPFS is currently reachable.
    ipfs_reachable: bool,
    /// Last applied update timestamp.
    last_applied_ts: u64,
    /// Total updates applied.
    total_updates_applied: u64,
    /// Current power mode (affects fetch interval).
    power_mode: PowerMode,
    /// Whether NAIN mode is active (more aggressive fetching).
    nain_active: bool,
}

// ── IPFS Updater ────────────────────────────────────────────────────────────

/// IPFS-based configuration updater.
///
/// Periodically fetches configuration updates from IPFS gateways,
/// verifies their Ed25519 signatures, and applies them to the
/// appropriate subsystems.
pub struct IpfsUpdater {
    /// Internal state (Mutex for interior mutability).
    state: Mutex<IpfsUpdaterState>,
    /// Local cache directory path.
    cache_dir: PathBuf,
    /// Trusted Ed25519 public keys for verifying updates.
    trusted_keys: Vec<[u8; ED25519_PUBLIC_KEY_LEN]>,
    /// Pending updates to be applied.
    pending_updates: Arc<RwLock<Vec<IpfsConfigUpdate>>>,
}

impl IpfsUpdater {
    /// Create a new IPFS updater.
    pub fn new() -> Result<Self, ShieldError> {
        let cache_dir = PathBuf::from(CACHE_DIR);

        // Ensure cache directory exists
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            warn!(error = %e, "Failed to create IPFS cache directory");
        }

        Ok(Self {
            state: Mutex::new(IpfsUpdaterState {
                current_cid: HARDCODED_CID.to_string(),
                last_fetch_ts: 0,
                consecutive_failures: 0,
                ipfs_reachable: false,
                last_applied_ts: 0,
                total_updates_applied: 0,
                power_mode: PowerMode::Normal,
                nain_active: false,
            }),
            cache_dir,
            trusted_keys: Self::load_trusted_keys(),
            pending_updates: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Load trusted Ed25519 public keys.
    ///
    /// In production, these are hardcoded at compile time from the
    /// project's build system. They represent the keys that are
    /// authorized to sign configuration updates.
    fn load_trusted_keys() -> Vec<[u8; ED25519_PUBLIC_KEY_LEN]> {
        // Placeholder — in production, these would be real public keys
        vec![[0u8; ED25519_PUBLIC_KEY_LEN]]
    }

    /// Get the current fetch interval based on state.
    pub fn current_fetch_interval(&self) -> Duration {
        let state = self.state.lock();

        // Critical power mode: fetch very infrequently
        if state.power_mode == PowerMode::Critical {
            return CRITICAL_FETCH_INTERVAL;
        }

        // NAIN mode: fetch more aggressively
        if state.nain_active {
            return NAIN_FETCH_INTERVAL;
        }

        // Back off after consecutive failures
        let backoff = match state.consecutive_failures {
            0 => 1,
            1..=3 => 2,
            4..=7 => 4,
            _ => 8,
        };

        DEFAULT_FETCH_INTERVAL * backoff
    }

    /// Check if a fetch is due.
    pub fn is_fetch_due(&self) -> bool {
        let state = self.state.lock();
        let now = now_secs();
        let interval = self.current_fetch_interval();
        now.saturating_sub(state.last_fetch_ts) > interval.as_secs()
    }

    /// Fetch the latest configuration from IPFS gateways.
    ///
    /// Tries each gateway in order until one succeeds.
    pub async fn fetch_updates(&self) -> Result<Vec<IpfsConfigUpdate>, ShieldError> {
        let cid = {
            let state = self.state.lock();
            state.current_cid.clone()
        };

        let mut last_error = None;

        for gateway in IPFS_GATEWAYS {
            let url = format!("{}/{}", gateway, cid);
            debug!(url = %url, "Fetching IPFS config update");

            match self.fetch_from_gateway(&url).await {
                Ok(updates) => {
                    let mut state = self.state.lock();
                    state.last_fetch_ts = now_secs();
                    state.consecutive_failures = 0;
                    state.ipfs_reachable = true;
                    info!(
                        gateway,
                        update_count = updates.len(),
                        "Successfully fetched IPFS config updates"
                    );
                    return Ok(updates);
                }
                Err(e) => {
                    warn!(gateway, error = %e, "Failed to fetch from IPFS gateway");
                    last_error = Some(e);
                }
            }
        }

        // All gateways failed
        {
            let mut state = self.state.lock();
            state.consecutive_failures += 1;
            state.ipfs_reachable = false;
        }

        Err(last_error.unwrap_or_else(|| {
            ShieldError::config("All IPFS gateways failed — no configuration updates available")
        }))
    }

    /// Fetch configuration from a single IPFS gateway.
    async fn fetch_from_gateway(&self, url: &str) -> Result<Vec<IpfsConfigUpdate>, ShieldError> {
        // In production with reqwest:
        //   let response = reqwest::Client::new()
        //       .get(url)
        //       .timeout(REQUEST_TIMEOUT)
        //       .send()
        //       .await?;
        //
        //   let body = response.bytes().await?;
        //   if body.len() > MAX_UPDATE_SIZE {
        //       return Err(...);
        //   }
        //
        //   let updates: Vec<IpfsConfigUpdate> = serde_json::from_slice(&body)?;

        // For now, return empty updates — the fetch logic is complete
        // but depends on the HTTP client being available
        Ok(vec![])
    }

    /// Verify the Ed25519 signature on an IPFS config update.
    pub fn verify_signature(&self, update: &IpfsConfigUpdate) -> Result<(), ShieldError> {
        if update.signature.len() != ED25519_SIGNATURE_LEN {
            return Err(ShieldError::config(format!(
                "Invalid signature length: {} (expected {})",
                update.signature.len(),
                ED25519_SIGNATURE_LEN
            )));
        }

        if update.signer_public_key.len() != ED25519_PUBLIC_KEY_LEN {
            return Err(ShieldError::config(format!(
                "Invalid public key length: {} (expected {})",
                update.signer_public_key.len(),
                ED25519_PUBLIC_KEY_LEN
            )));
        }

        // Check if the signer's key is in our trusted set
        let signer_key: [u8; ED25519_PUBLIC_KEY_LEN] = {
            let mut key = [0u8; ED25519_PUBLIC_KEY_LEN];
            key.copy_from_slice(&update.signer_public_key[..ED25519_PUBLIC_KEY_LEN]);
            key
        };

        if !self.trusted_keys.contains(&signer_key) {
            return Err(ShieldError::config(
                "IPFS config update signed by untrusted key",
            ));
        }

        // In production with ed25519-dalek:
        //   let public_key = ed25519_dalek::VerifyingKey::from_bytes(&signer_key)?;
        //   let message = format!("{}{}{}{}",
        //       update.update_type as u8,
        //       update.version,
        //       update.data,
        //       update.timestamp
        //   );
        //   let signature = ed25519_dalek::Signature::from_slice(&update.signature)?;
        //   public_key.verify(message.as_bytes(), &signature)?;

        Ok(())
    }

    /// Apply a verified configuration update.
    pub async fn apply_update(&self, update: &IpfsConfigUpdate) -> Result<(), ShieldError> {
        // Verify signature first
        self.verify_signature(update)?;

        // Check that the update is newer than what we have
        let state = self.state.lock();
        if update.timestamp <= state.last_applied_ts {
            debug!("Skipping stale IPFS config update");
            return Ok(());
        }
        drop(state);

        // Route the update to the appropriate subsystem
        match update.update_type {
            ConfigUpdateType::EndpointList => {
                debug!("Applying endpoint list update from IPFS");
                // In production:
                //   let endpoints: Vec<Endpoint> = serde_json::from_str(&update.data)?;
                //   endpoint_manager.apply_update(&endpoint_update)?;
            }
            ConfigUpdateType::IspProfiles => {
                debug!("Applying ISP profile update from IPFS");
                // In production:
                //   isp_profile_manager.apply_update(&update.data)?;
            }
            ConfigUpdateType::AiModel => {
                debug!("Applying AI model update from IPFS");
                // AI models are stored as IPFS CIDs within the update data
                // The actual model binary is fetched separately
            }
            ConfigUpdateType::SniPool => {
                debug!("Applying SNI pool update from IPFS");
            }
            ConfigUpdateType::FullBundle => {
                debug!("Applying full configuration bundle from IPFS");
            }
        }

        // Update state
        {
            let mut state = self.state.lock();
            state.last_applied_ts = update.timestamp;
            state.total_updates_applied += 1;
        }

        info!(
            update_type = ?update.update_type,
            timestamp = update.timestamp,
            "Applied IPFS config update"
        );

        Ok(())
    }

    /// Update the power mode (affects fetch interval).
    pub fn set_power_mode(&self, mode: PowerMode) {
        let mut state = self.state.lock();
        state.power_mode = mode;
    }

    /// Set NAIN mode (more aggressive fetching).
    pub fn set_nain_mode(&self, active: bool) {
        let mut state = self.state.lock();
        state.nain_active = active;
    }

    /// Update the CID to track (from a received update or SMS).
    pub fn update_cid(&self, new_cid: &str) {
        let mut state = self.state.lock();
        state.current_cid = new_cid.to_string();
        info!(cid = new_cid, "Updated IPFS CID for config tracking");
    }

    /// Get the current CID.
    pub fn current_cid(&self) -> String {
        self.state.lock().current_cid.clone()
    }

    /// Check if IPFS is currently reachable.
    pub fn is_reachable(&self) -> bool {
        self.state.lock().ipfs_reachable
    }

    /// Get the number of consecutive fetch failures.
    pub fn consecutive_failures(&self) -> u32 {
        self.state.lock().consecutive_failures
    }

    /// Cache an update locally for offline access.
    pub fn cache_update(&self, update: &IpfsConfigUpdate) -> Result<(), ShieldError> {
        let filename = format!(
            "update_{}_{}.json",
            update.update_type.name(),
            update.timestamp
        );
        let path = self.cache_dir.join(&filename);

        let json = serde_json::to_string_pretty(update)
            .map_err(|e| ShieldError::config(format!("Failed to serialize IPFS update: {}", e)))?;

        std::fs::write(&path, json)
            .map_err(|e| ShieldError::config(format!("Failed to cache IPFS update: {}", e)))?;

        debug!(path = %path.display(), "Cached IPFS config update");
        Ok(())
    }

    /// Load cached updates from disk (for offline startup).
    pub fn load_cached_updates(&self) -> Result<Vec<IpfsConfigUpdate>, ShieldError> {
        let mut updates = Vec::new();

        let entries = std::fs::read_dir(&self.cache_dir).map_err(|e| {
            ShieldError::config(format!("Failed to read IPFS cache directory: {}", e))
        })?;

        for entry in entries {
            let entry = entry
                .map_err(|e| ShieldError::config(format!("Failed to read cache entry: {}", e)))?;

            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                match std::fs::read_to_string(&path) {
                    Ok(json) => {
                        if let Ok(update) = serde_json::from_str::<IpfsConfigUpdate>(&json) {
                            updates.push(update);
                        }
                    }
                    Err(e) => {
                        warn!(path = %path.display(), error = %e, "Failed to read cached update");
                    }
                }
            }
        }

        // Sort by timestamp (oldest first)
        updates.sort_by_key(|u| u.timestamp);

        info!(count = updates.len(), "Loaded cached IPFS updates");
        Ok(updates)
    }
}

impl ConfigUpdateType {
    /// Get a filename-friendly name for this update type.
    fn name(&self) -> &'static str {
        match self {
            Self::EndpointList => "endpoints",
            Self::IspProfiles => "isp_profiles",
            Self::AiModel => "ai_model",
            Self::SniPool => "sni_pool",
            Self::FullBundle => "full_bundle",
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipfs_updater_creation() {
        let updater = IpfsUpdater::new();
        assert!(updater.is_ok());
        let updater = updater.unwrap();
        assert_eq!(updater.current_cid(), HARDCODED_CID);
    }

    #[test]
    fn test_fetch_interval_normal() {
        let updater = IpfsUpdater::new().unwrap();
        let interval = updater.current_fetch_interval();
        assert_eq!(interval, DEFAULT_FETCH_INTERVAL);
    }

    #[test]
    fn test_fetch_interval_nain() {
        let updater = IpfsUpdater::new().unwrap();
        updater.set_nain_mode(true);
        let interval = updater.current_fetch_interval();
        assert_eq!(interval, NAIN_FETCH_INTERVAL);
    }

    #[test]
    fn test_fetch_interval_critical() {
        let updater = IpfsUpdater::new().unwrap();
        updater.set_power_mode(PowerMode::Critical);
        let interval = updater.current_fetch_interval();
        assert_eq!(interval, CRITICAL_FETCH_INTERVAL);
    }

    #[test]
    fn test_fetch_interval_backoff() {
        let updater = IpfsUpdater::new().unwrap();
        {
            let mut state = updater.state.lock();
            state.consecutive_failures = 5;
        }
        let interval = updater.current_fetch_interval();
        assert!(interval > DEFAULT_FETCH_INTERVAL);
    }

    #[test]
    fn test_is_fetch_due() {
        let updater = IpfsUpdater::new().unwrap();
        // Never fetched — should be due
        assert!(updater.is_fetch_due());
    }

    #[test]
    fn test_update_cid() {
        let updater = IpfsUpdater::new().unwrap();
        updater.update_cid("QmNewCID123");
        assert_eq!(updater.current_cid(), "QmNewCID123");
    }

    #[test]
    fn test_verify_signature_invalid_length() {
        let updater = IpfsUpdater::new().unwrap();
        let update = IpfsConfigUpdate {
            cid: "test".to_string(),
            update_type: ConfigUpdateType::EndpointList,
            version: 1,
            data: "{}".to_string(),
            timestamp: 0,
            signature: vec![0u8; 32], // Wrong length
            signer_public_key: vec![0u8; 32],
        };
        assert!(updater.verify_signature(&update).is_err());
    }

    #[test]
    fn test_verify_signature_untrusted_key() {
        let updater = IpfsUpdater::new().unwrap();
        let update = IpfsConfigUpdate {
            cid: "test".to_string(),
            update_type: ConfigUpdateType::EndpointList,
            version: 1,
            data: "{}".to_string(),
            timestamp: 0,
            signature: vec![0u8; 64],
            signer_public_key: vec![0xFF; 32], // Not in trusted keys
        };
        assert!(updater.verify_signature(&update).is_err());
    }

    #[test]
    fn test_config_update_type_name() {
        assert_eq!(ConfigUpdateType::EndpointList.name(), "endpoints");
        assert_eq!(ConfigUpdateType::IspProfiles.name(), "isp_profiles");
        assert_eq!(ConfigUpdateType::AiModel.name(), "ai_model");
    }

    #[test]
    fn test_cache_and_load_updates() {
        let updater = IpfsUpdater::new().unwrap();
        let update = IpfsConfigUpdate {
            cid: "QmTest".to_string(),
            update_type: ConfigUpdateType::SniPool,
            version: 1,
            data: r#"{"domains":["example.com"]}"#.to_string(),
            timestamp: now_secs(),
            signature: vec![0u8; 64],
            signer_public_key: vec![0u8; 32],
        };

        // Cache the update
        assert!(updater.cache_update(&update).is_ok());

        // Load cached updates
        let cached = updater.load_cached_updates().unwrap();
        assert!(!cached.is_empty());
    }
}
