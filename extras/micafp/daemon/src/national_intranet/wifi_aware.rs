// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — WiFi Aware / NAN Mesh
//
// WiFi Aware (Neighbor Awareness Networking) enables device-to-device
// communication without traditional WiFi infrastructure. This module provides
// the Rust FFI bridge for Android's NanBridge.kt and manages peer discovery.
//
// Key features:
//   • Rust FFI bridge for Android NanBridge.kt (WiFi Aware API)
//   • Receives discovered peers via JNI callback
//   • Adds peers to the peer_exchange system
//   • Auto-routes traffic through peers with international access
//
// Battery optimization:
//   • Only scan when screen is ON or when NAIN CompleteBlackout detected
//   • Scan interval: 30s when active, 300s when background
//   • Stop scanning entirely when battery < 15%
//   • WiFi Aware uses ~15mA during active scanning
// ─────────────────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::error::{ErrorCode, ShieldError};

// ── WiFi Aware Peer ─────────────────────────────────────────────────────────

/// A peer discovered via WiFi Aware / NAN.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WifiAwarePeer {
    /// Unique peer identifier (from NAN service discovery).
    pub peer_id: String,
    /// NAN service-specific info (may contain encrypted endpoint data).
    pub service_info: Vec<u8>,
    /// Signal strength (dBm) of the last discovery.
    pub signal_strength_dbm: i16,
    /// Whether this peer has confirmed international internet access.
    pub has_international_access: bool,
    /// Timestamp when this peer was last seen.
    pub last_seen: u64,
    /// Number of times this peer has been discovered.
    pub discovery_count: u32,
}

// ── Scanning State ──────────────────────────────────────────────────────────

/// Current scanning state of the WiFi Aware mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanState {
    /// Not scanning — WiFi Aware is inactive.
    Inactive,
    /// Actively scanning for peers (foreground).
    ActiveScanning,
    /// Background scanning at reduced interval.
    BackgroundScanning,
    /// Scanning suspended due to low battery.
    BatterySuspended,
}

// ── WiFi Aware Configuration ────────────────────────────────────────────────

/// Configuration for WiFi Aware scanning behavior.
#[derive(Debug, Clone)]
pub struct WifiAwareConfig {
    /// Service name for NAN service discovery.
    pub service_name: String,
    /// Active scan interval (when screen on or NAIN blackout).
    pub active_scan_interval: Duration,
    /// Background scan interval (when screen off).
    pub background_scan_interval: Duration,
    /// Battery threshold below which scanning stops entirely.
    pub battery_stop_threshold: u8,
    /// Battery threshold below which scanning goes to background mode.
    pub battery_background_threshold: u8,
    /// Maximum number of peers to track.
    pub max_peers: usize,
    /// Peer expiry timeout — peers not seen for this long are removed.
    pub peer_expiry: Duration,
}

impl Default for WifiAwareConfig {
    fn default() -> Self {
        Self {
            service_name: "micafp-shield-v6".to_string(),
            active_scan_interval: Duration::from_secs(30),
            background_scan_interval: Duration::from_secs(300),
            battery_stop_threshold: 15,
            battery_background_threshold: 30,
            max_peers: 50,
            peer_expiry: Duration::from_secs(600),
        }
    }
}

// ── WiFi Aware Mesh ─────────────────────────────────────────────────────────

/// WiFi Aware mesh network manager.
///
/// Provides the Rust side of the FFI bridge to Android's NanBridge.kt
/// for WiFi Aware (NAN) peer discovery and data exchange.
pub struct WifiAwareMesh {
    /// Configuration.
    config: WifiAwareConfig,
    /// Discovered peers, keyed by peer ID.
    peers: Arc<RwLock<HashMap<String, WifiAwarePeer>>>,
    /// Current scanning state.
    scan_state: Mutex<ScanState>,
    /// Whether WiFi Aware is available on this device.
    available: Arc<std::sync::atomic::AtomicBool>,
}

impl WifiAwareMesh {
    /// Create a new WiFi Aware mesh manager.
    pub fn new() -> Result<Self, ShieldError> {
        Ok(Self {
            config: WifiAwareConfig::default(),
            peers: Arc::new(RwLock::new(HashMap::new())),
            scan_state: Mutex::new(ScanState::Inactive),
            available: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Create with custom configuration.
    pub fn with_config(config: WifiAwareConfig) -> Result<Self, ShieldError> {
        Ok(Self {
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            scan_state: Mutex::new(ScanState::Inactive),
            available: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Check if WiFi Aware is available on this device.
    pub fn is_available(&self) -> bool {
        self.available.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Set WiFi Aware availability (called from JNI when Android reports status).
    pub fn set_available(&self, available: bool) {
        self.available
            .store(available, std::sync::atomic::Ordering::Relaxed);
        if !available {
            info!("WiFi Aware not available on this device");
        }
    }

    /// Start scanning for WiFi Aware peers.
    pub async fn start_scanning(&self) -> Result<(), ShieldError> {
        if !self.is_available() {
            warn!("WiFi Aware not available — cannot start scanning");
            return Err(ShieldError::nain_mode(
                ErrorCode::NainWifiAwareUnavailable,
                "WiFi Aware hardware not available on this device",
            ));
        }

        {
            let mut state = self.scan_state.lock();
            if *state == ScanState::ActiveScanning {
                debug!("WiFi Aware already actively scanning");
                return Ok(());
            }
            *state = ScanState::ActiveScanning;
        }

        info!(
            service_name = %self.config.service_name,
            "Starting WiFi Aware peer scanning"
        );

        // In production, this calls NanBridge.kt via JNI:
        //   NanBridge.startDiscovery(serviceName, callback)
        // The Kotlin callback will invoke on_peer_discovered() below.

        Ok(())
    }

    /// Stop scanning for WiFi Aware peers.
    pub async fn stop_scanning(&self) -> Result<(), ShieldError> {
        {
            let mut state = self.scan_state.lock();
            if *state == ScanState::Inactive {
                return Ok(());
            }
            *state = ScanState::Inactive;
        }

        info!("Stopping WiFi Aware peer scanning");

        // In production, this calls NanBridge.kt via JNI:
        //   NanBridge.stopDiscovery()

        Ok(())
    }

    /// Switch to background scanning mode (reduced frequency).
    pub async fn enter_background_scan(&self) -> Result<(), ShieldError> {
        {
            let mut state = self.scan_state.lock();
            *state = ScanState::BackgroundScanning;
        }

        debug!("WiFi Aware entering background scan mode");

        // In production, this adjusts the WorkManager periodic work interval
        // from active_scan_interval to background_scan_interval.

        Ok(())
    }

    /// Suspend scanning due to low battery.
    pub async fn suspend_for_battery(&self) -> Result<(), ShieldError> {
        {
            let mut state = self.scan_state.lock();
            *state = ScanState::BatterySuspended;
        }

        info!("WiFi Aware scanning suspended due to low battery");

        // In production, this calls NanBridge.stopDiscovery() via JNI.

        Ok(())
    }

    /// Get the current scan state.
    pub fn scan_state(&self) -> ScanState {
        *self.scan_state.lock()
    }

    /// Get the current number of discovered peers.
    pub async fn peer_count(&self) -> u32 {
        self.peers.read().await.len() as u32
    }

    /// Get all discovered peers.
    pub async fn get_peers(&self) -> Vec<WifiAwarePeer> {
        self.peers.read().await.values().cloned().collect()
    }

    /// Get peers that have confirmed international access.
    pub async fn get_internet_peers(&self) -> Vec<WifiAwarePeer> {
        self.peers
            .read()
            .await
            .values()
            .filter(|p| p.has_international_access)
            .cloned()
            .collect()
    }

    // ── JNI Callbacks ───────────────────────────────────────────────────

    /// Called from JNI (NanBridge.kt) when a new peer is discovered.
    ///
    /// The peer information is passed from the Android WiFi Aware API
    /// through the Kotlin bridge to this Rust function.
    pub async fn on_peer_discovered(
        &self,
        peer_id: String,
        service_info: Vec<u8>,
        signal_strength_dbm: i16,
    ) -> Result<(), ShieldError> {
        debug!(
            peer_id = %peer_id,
            signal = signal_strength_dbm,
            info_len = service_info.len(),
            "WiFi Aware peer discovered"
        );

        let mut peers = self.peers.write().await;

        // Check peer limit
        if peers.len() >= self.config.max_peers && !peers.contains_key(&peer_id) {
            // Remove the oldest peer to make room
            if let Some(oldest_key) = peers
                .iter()
                .min_by_key(|(_, p)| p.last_seen)
                .map(|(k, _)| k.clone())
            {
                peers.remove(&oldest_key);
            }
        }

        // Update or insert the peer
        let peer = peers
            .entry(peer_id.clone())
            .and_modify(|existing| {
                existing.service_info = service_info.clone();
                existing.signal_strength_dbm = signal_strength_dbm;
                existing.last_seen = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
                existing.discovery_count = existing.discovery_count.saturating_add(1);
            })
            .or_insert_with(|| WifiAwarePeer {
                peer_id: peer_id.clone(),
                service_info: service_info.clone(),
                signal_strength_dbm,
                has_international_access: false, // Will be confirmed via probe
                last_seen: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
                discovery_count: 1,
            });

        // If the service info contains a flag indicating international access,
        // parse it. The service info format is:
        //   [1B: flags] [rest: encrypted endpoint data]
        //   flags bit 0: has_international_access
        if !peer.service_info.is_empty() {
            let flags = peer.service_info[0];
            peer.has_international_access = (flags & 0x01) != 0;
        }

        Ok(())
    }

    /// Called from JNI when a peer is lost (no longer reachable).
    pub async fn on_peer_lost(&self, peer_id: &str) {
        debug!(peer_id, "WiFi Aware peer lost");

        let mut peers = self.peers.write().await;
        peers.remove(peer_id);
    }

    // ── Data Exchange ───────────────────────────────────────────────────

    /// Send data to a specific peer via WiFi Aware NAN data path.
    ///
    /// This is used for exchanging peer lists and configuration data
    /// between nearby devices.
    pub async fn send_to_peer(&self, peer_id: &str, data: &[u8]) -> Result<(), ShieldError> {
        let peers = self.peers.read().await;
        if !peers.contains_key(peer_id) {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainWifiAwareUnavailable,
                format!("Peer {} not found in discovered peers", peer_id),
            ));
        }

        debug!(
            peer_id,
            data_len = data.len(),
            "Sending data to WiFi Aware peer"
        );

        // In production, this calls NanBridge.sendData(peerId, data) via JNI.
        // The NAN data path (NDP) provides reliable data transfer.

        Ok(())
    }

    /// Broadcast data to all discovered peers.
    pub async fn broadcast(&self, data: &[u8]) -> Result<usize, ShieldError> {
        let peers = self.peers.read().await;
        let mut sent_count = 0;

        for peer_id in peers.keys() {
            match self.send_to_peer(peer_id, data).await {
                Ok(()) => sent_count += 1,
                Err(e) => {
                    warn!(peer_id, error = %e, "Failed to send to WiFi Aware peer");
                }
            }
        }

        debug!(
            sent_count,
            total = peers.len(),
            "WiFi Aware broadcast completed"
        );
        Ok(sent_count)
    }

    // ── Maintenance ─────────────────────────────────────────────────────

    /// Remove expired peers that haven't been seen recently.
    pub async fn prune_expired_peers(&self) {
        let mut peers = self.peers.write().await;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let expiry = self.config.peer_expiry;

        let expired: Vec<String> = peers
            .iter()
            .filter(|(_, p)| Duration::from_millis(now_ms.saturating_sub(p.last_seen)) > expiry)
            .map(|(k, _)| k.clone())
            .collect();

        for key in expired {
            peers.remove(&key);
        }
    }

    /// Update battery state and adjust scanning accordingly.
    pub async fn update_battery_state(&self, battery_level: u8, is_charging: bool) {
        let current_state = self.scan_state();

        if is_charging {
            // Resume active scanning when charging
            if current_state == ScanState::BatterySuspended {
                if let Err(e) = self.start_scanning().await {
                    warn!(error = %e, "Failed to resume WiFi Aware scanning");
                }
            }
            return;
        }

        if battery_level < self.config.battery_stop_threshold {
            // Battery critically low — stop scanning
            if current_state != ScanState::BatterySuspended {
                if let Err(e) = self.suspend_for_battery().await {
                    warn!(error = %e, "Failed to suspend WiFi Aware scanning");
                }
            }
        } else if battery_level < self.config.battery_background_threshold {
            // Battery low — switch to background scanning
            if current_state == ScanState::ActiveScanning {
                if let Err(e) = self.enter_background_scan().await {
                    warn!(error = %e, "Failed to enter background WiFi Aware scanning");
                }
            }
        }
    }

    /// Get the recommended scan interval for the current state.
    pub fn recommended_scan_interval(&self) -> Duration {
        match self.scan_state() {
            ScanState::ActiveScanning => self.config.active_scan_interval,
            ScanState::BackgroundScanning => self.config.background_scan_interval,
            ScanState::BatterySuspended | ScanState::Inactive => Duration::MAX,
        }
    }
}

// ── JNI FFI Bridge ──────────────────────────────────────────────────────────

/// FFI callback structure for Android NanBridge.kt.
///
/// These functions are called from Kotlin/JNI to report WiFi Aware events
/// back to the Rust daemon.
#[cfg(target_os = "android")]
pub mod jni_bridge {
    use super::*;

    /// JNI callback: WiFi Aware peer discovered.
    ///
    /// # Safety
    /// Called from JNI. `peer_id_ptr` and `service_info_ptr` must be valid
    /// pointers with the specified lengths.
    #[no_mangle]
    pub unsafe extern "C" fn Java_org_micafp_shield_wifi_1aware_NanBridge_onPeerDiscovered(
        _env: *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
        mesh_ptr: usize,
        peer_id_ptr: *const u8,
        peer_id_len: usize,
        service_info_ptr: *const u8,
        service_info_len: usize,
        signal_strength: i16,
    ) {
        if peer_id_ptr.is_null() || service_info_ptr.is_null() {
            return;
        }

        let peer_id_slice = std::slice::from_raw_parts(peer_id_ptr, peer_id_len);
        let peer_id = match std::str::from_utf8(peer_id_slice) {
            Ok(s) => s.to_string(),
            Err(_) => return,
        };

        let service_info = std::slice::from_raw_parts(service_info_ptr, service_info_len).to_vec();

        let mesh = unsafe { &*(mesh_ptr as *const WifiAwareMesh) };

        let rt = tokio::runtime::Handle::current();
        let _ = rt.block_on(async {
            mesh.on_peer_discovered(peer_id, service_info, signal_strength)
                .await
        });
    }

    /// JNI callback: WiFi Aware peer lost.
    #[no_mangle]
    pub unsafe extern "C" fn Java_org_micafp_shield_wifi_1aware_NanBridge_onPeerLost(
        _env: *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
        mesh_ptr: usize,
        peer_id_ptr: *const u8,
        peer_id_len: usize,
    ) {
        if peer_id_ptr.is_null() {
            return;
        }

        let peer_id_slice = std::slice::from_raw_parts(peer_id_ptr, peer_id_len);
        let peer_id = match std::str::from_utf8(peer_id_slice) {
            Ok(s) => s.to_string(),
            Err(_) => return,
        };

        let mesh = unsafe { &*(mesh_ptr as *const WifiAwareMesh) };
        let rt = tokio::runtime::Handle::current();
        let _ = rt.block_on(async { mesh.on_peer_lost(&peer_id).await });
    }

    /// JNI callback: WiFi Aware availability changed.
    #[no_mangle]
    pub unsafe extern "C" fn Java_org_micafp_shield_wifi_1aware_NanBridge_onAvailabilityChanged(
        _env: *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
        mesh_ptr: usize,
        available: bool,
    ) {
        let mesh = unsafe { &*(mesh_ptr as *const WifiAwareMesh) };
        mesh.set_available(available);
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wifi_aware_mesh_creation() {
        let mesh = WifiAwareMesh::new().unwrap();
        assert!(!mesh.is_available());
        assert_eq!(mesh.scan_state(), ScanState::Inactive);
        assert_eq!(mesh.peer_count().await, 0);
    }

    #[tokio::test]
    async fn test_peer_discovery() {
        let mesh = WifiAwareMesh::new().unwrap();
        mesh.set_available(true);

        // Simulate peer discovery
        mesh.on_peer_discovered(
            "peer-001".to_string(),
            vec![0x01, 0x02, 0x03], // flags: has_international_access
            -55,
        )
        .await
        .unwrap();

        assert_eq!(mesh.peer_count().await, 1);

        let internet_peers = mesh.get_internet_peers().await;
        assert_eq!(internet_peers.len(), 1);
        assert!(internet_peers[0].has_international_access);
    }

    #[tokio::test]
    async fn test_peer_lost() {
        let mesh = WifiAwareMesh::new().unwrap();
        mesh.set_available(true);

        mesh.on_peer_discovered("peer-001".to_string(), vec![0x00], -55)
            .await
            .unwrap();
        assert_eq!(mesh.peer_count().await, 1);

        mesh.on_peer_lost("peer-001").await;
        assert_eq!(mesh.peer_count().await, 0);
    }

    #[tokio::test]
    async fn test_max_peers_limit() {
        let config = WifiAwareConfig {
            max_peers: 2,
            ..Default::default()
        };
        let mesh = WifiAwareMesh::with_config(config).unwrap();
        mesh.set_available(true);

        mesh.on_peer_discovered("peer-001".to_string(), vec![0x00], -55)
            .await
            .unwrap();
        mesh.on_peer_discovered("peer-002".to_string(), vec![0x00], -60)
            .await
            .unwrap();
        mesh.on_peer_discovered("peer-003".to_string(), vec![0x00], -65)
            .await
            .unwrap();

        // Should still have at most 2 peers (oldest removed)
        assert_eq!(mesh.peer_count().await, 2);
    }
}
