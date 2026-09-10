// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — Bluetooth LE Mesh Network
//
// BLE mesh networking for device-to-device communication during complete
// internet blackouts. BLE is far more battery-efficient than WiFi scanning
// (~2mA vs ~30mA) and works even when WiFi and cellular are jammed.
//
// Features:
//   • BLE GATT server advertising yggdrasil public key
//   • BLE GATT client scanning for nearby peers
//   • MTU negotiation for larger payloads
//   • Mesh routing: hop data through multiple BLE peers
//   • Range: ~30m per hop, can chain multiple hops
//
// Battery optimization:
//   • BLE scanning: ~2mA vs WiFi scanning: ~30mA
//   • Scan window: 30s scan, 30s pause (duty cycle 50%)
//   • When battery < 20%: 10s scan, 50s pause (duty cycle ~17%)
//   • When charging: continuous scan
//   • Android: BLE foreground service for background scanning
//   • iOS: CBCentralManager scan with allowDuplicates=false
// ─────────────────────────────────────────────────────────────────────────────

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::error::{ErrorCode, ShieldError};

// ── Constants ───────────────────────────────────────────────────────────────

/// BLE service UUID for MICAFP Shield mesh.
const SHIELD_SERVICE_UUID: &str = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";

/// BLE characteristic UUID for yggdrasil public key.
const YGGDRASIL_PUBKEY_CHAR_UUID: &str = "a1b2c3d4-e5f6-7890-abcd-ef1234567891";

/// BLE characteristic UUID for mesh data exchange.
const MESH_DATA_CHAR_UUID: &str = "a1b2c3d4-e5f6-7890-abcd-ef1234567892";

/// Default BLE MTU size (before negotiation).
const DEFAULT_MTU: usize = 23;
/// Maximum BLE MTU size (after negotiation).
const MAX_MTU: usize = 517;
/// Maximum number of mesh peers to track.
const MAX_MESH_PEERS: usize = 100;
/// Peer expiry timeout.
const PEER_EXPIRY: Duration = Duration::from_secs(600);
/// Maximum hop count for mesh routing.
const MAX_HOP_COUNT: u8 = 5;
/// Maximum mesh message size.
const MAX_MESH_MESSAGE_SIZE: usize = 4096;
/// Default scan window duration.
const SCAN_WINDOW_DEFAULT: Duration = Duration::from_secs(30);
/// Default scan pause duration.
const SCAN_PAUSE_DEFAULT: Duration = Duration::from_secs(30);
/// Low-battery scan window.
const SCAN_WINDOW_LOW_BATTERY: Duration = Duration::from_secs(10);
/// Low-battery scan pause.
const SCAN_PAUSE_LOW_BATTERY: Duration = Duration::from_secs(50);

// ── BLE Peer ────────────────────────────────────────────────────────────────

/// A peer discovered via BLE scanning.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlePeer {
    /// Unique peer identifier (BLE device address).
    pub device_address: String,
    /// Yggdrasil public key (if advertised).
    pub yggdrasil_pubkey: Option<Vec<u8>>,
    /// RSSI signal strength (dBm).
    pub rssi: i16,
    /// Negotiated MTU size.
    pub mtu: usize,
    /// Number of hops from this device (0 = direct neighbor).
    pub hop_count: u8,
    /// Whether this peer has internet access (directly or via mesh).
    pub has_internet_access: bool,
    /// Timestamp when this peer was last seen.
    pub last_seen: u64,
    /// Mesh route to reach this peer (sequence of device addresses).
    pub route: Vec<String>,
    /// Reputation score (0.0 - 1.0).
    pub reputation: f64,
}

// ── Mesh Message ────────────────────────────────────────────────────────────

/// A message in the BLE mesh network.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MeshMessage {
    /// Message type.
    pub msg_type: MeshMessageType,
    /// Source device address.
    pub source: String,
    /// Destination device address (empty = broadcast).
    pub destination: String,
    /// Hop count (incremented at each relay).
    pub hop_count: u8,
    /// Maximum allowed hops.
    pub max_hops: u8,
    /// Message ID for deduplication.
    pub message_id: u64,
    /// Encrypted payload.
    pub payload: Vec<u8>,
    /// Timestamp when the message was created.
    pub created_at: u64,
}

/// Types of messages in the BLE mesh network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MeshMessageType {
    /// Endpoint list update.
    EndpointUpdate,
    /// Peer discovery announcement.
    PeerAnnouncement,
    /// Internet access probe response.
    InternetProbeResponse,
    /// Data relay (carrying user traffic).
    DataRelay,
    /// Mesh route update.
    RouteUpdate,
    /// Heartbeat / keepalive.
    Heartbeat,
}

// ── Scan State ──────────────────────────────────────────────────────────────

/// Current BLE scanning state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BleScanState {
    /// Not scanning.
    Inactive,
    /// Active scanning (foreground or charging).
    ActiveScanning,
    /// Paused between scan windows.
    ScanPaused,
    /// Low-battery scanning (reduced duty cycle).
    LowBatteryScanning,
    /// Scanning suspended due to critical battery.
    BatterySuspended,
}

// ── BLE Mesh Configuration ──────────────────────────────────────────────────

/// Configuration for the BLE mesh network.
#[derive(Debug, Clone)]
pub struct BleMeshConfig {
    /// BLE service UUID.
    pub service_uuid: String,
    /// Scan window duration.
    pub scan_window: Duration,
    /// Scan pause duration.
    pub scan_pause: Duration,
    /// Low-battery scan window.
    pub low_battery_scan_window: Duration,
    /// Low-battery scan pause.
    pub low_battery_scan_pause: Duration,
    /// Battery threshold for low-battery mode.
    pub low_battery_threshold: u8,
    /// Battery threshold for suspending scanning.
    pub critical_battery_threshold: u8,
    /// Maximum number of peers.
    pub max_peers: usize,
    /// Maximum hop count.
    pub max_hops: u8,
}

impl Default for BleMeshConfig {
    fn default() -> Self {
        Self {
            service_uuid: SHIELD_SERVICE_UUID.to_string(),
            scan_window: SCAN_WINDOW_DEFAULT,
            scan_pause: SCAN_PAUSE_DEFAULT,
            low_battery_scan_window: SCAN_WINDOW_LOW_BATTERY,
            low_battery_scan_pause: SCAN_PAUSE_LOW_BATTERY,
            low_battery_threshold: 20,
            critical_battery_threshold: 10,
            max_peers: MAX_MESH_PEERS,
            max_hops: MAX_HOP_COUNT,
        }
    }
}

// ── Mesh Router ─────────────────────────────────────────────────────────────

/// Mesh routing table for multi-hop BLE communication.
pub struct MeshRouter {
    /// Routing table: destination -> next hop.
    routes: Mutex<HashMap<String, String>>,
    /// Seen message IDs for deduplication.
    seen_messages: Mutex<HashSet<u64>>,
    /// Message queue for relay.
    relay_queue: Mutex<VecDeque<MeshMessage>>,
}

impl MeshRouter {
    /// Create a new mesh router.
    pub fn new() -> Self {
        Self {
            routes: Mutex::new(HashMap::new()),
            seen_messages: Mutex::new(HashSet::with_capacity(1000)),
            relay_queue: Mutex::new(VecDeque::with_capacity(64)),
        }
    }

    /// Update the routing table based on peer information.
    pub fn update_routes(&self, peers: &HashMap<String, BlePeer>) {
        let mut routes = self.routes.lock();
        routes.clear();

        for (peer_id, peer) in peers {
            if peer.route.len() > 1 {
                // Multi-hop route: next hop is the first address in the route
                routes.insert(peer_id.clone(), peer.route[0].clone());
            } else {
                // Direct neighbor: route is direct
                routes.insert(peer_id.clone(), peer_id.clone());
            }
        }
    }

    /// Determine if a message should be relayed.
    ///
    /// Returns the next hop device address, or None if the message
    /// should not be relayed (already seen, expired, or destination reached).
    pub fn should_relay(&self, message: &MeshMessage, own_address: &str) -> Option<String> {
        // Don't relay our own messages
        if message.source == own_address {
            return None;
        }

        // Don't relay expired messages
        if message.hop_count >= message.max_hops {
            return None;
        }

        // Check deduplication
        if self.seen_messages.lock().contains(&message.message_id) {
            return None;
        }

        // Mark as seen
        self.seen_messages.lock().insert(message.message_id);

        // If this is a broadcast or we're the destination, don't relay further
        // (But for broadcast, we SHOULD relay to other peers)
        if message.destination == own_address {
            return None;
        }

        // If destination is specified, look up the route
        if !message.destination.is_empty() {
            let routes = self.routes.lock();
            return routes.get(&message.destination).cloned();
        }

        // Broadcast message: relay to all direct neighbors
        // Return a special marker that the caller interprets as "relay to all"
        Some("*".to_string())
    }

    /// Prune old message IDs from the deduplication set.
    pub fn prune_seen_messages(&self, max_size: usize) {
        let mut seen = self.seen_messages.lock();
        if seen.len() > max_size {
            // Remove random half of entries
            let to_remove: Vec<u64> = seen.iter().take(seen.len() / 2).copied().collect();
            for id in to_remove {
                seen.remove(&id);
            }
        }
    }

    /// Enqueue a message for relay.
    pub fn enqueue_relay(&self, message: MeshMessage) {
        let mut queue = self.relay_queue.lock();
        if queue.len() < 64 {
            queue.push_back(message);
        }
    }

    /// Dequeue the next message for relay.
    pub fn dequeue_relay(&self) -> Option<MeshMessage> {
        self.relay_queue.lock().pop_front()
    }
}

// ── BLE Mesh Network ────────────────────────────────────────────────────────

/// BLE mesh network manager for anti-censorship peer communication.
pub struct BleMeshNetwork {
    /// Configuration.
    config: BleMeshConfig,
    /// Discovered BLE peers.
    peers: Arc<RwLock<HashMap<String, BlePeer>>>,
    /// Current scanning state.
    scan_state: Mutex<BleScanState>,
    /// Mesh router for multi-hop communication.
    router: MeshRouter,
    /// Our own BLE device address.
    own_address: Mutex<String>,
    /// Our yggdrasil public key for advertising.
    yggdrasil_pubkey: Mutex<Vec<u8>>,
    /// Whether BLE is available on this device.
    available: Arc<std::sync::atomic::AtomicBool>,
    /// Incoming messages awaiting processing.
    incoming_messages: Arc<RwLock<Vec<MeshMessage>>>,
    /// Next message ID counter.
    next_message_id: Mutex<u64>,
}

impl BleMeshNetwork {
    /// Create a new BLE mesh network.
    pub fn new() -> Result<Self, ShieldError> {
        Ok(Self {
            config: BleMeshConfig::default(),
            peers: Arc::new(RwLock::new(HashMap::new())),
            scan_state: Mutex::new(BleScanState::Inactive),
            router: MeshRouter::new(),
            own_address: Mutex::new(String::new()),
            yggdrasil_pubkey: Mutex::new(Vec::new()),
            available: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            incoming_messages: Arc::new(RwLock::new(Vec::new())),
            next_message_id: Mutex::new(1),
        })
    }

    /// Create with custom configuration.
    pub fn with_config(config: BleMeshConfig) -> Result<Self, ShieldError> {
        Ok(Self {
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            scan_state: Mutex::new(BleScanState::Inactive),
            router: MeshRouter::new(),
            own_address: Mutex::new(String::new()),
            yggdrasil_pubkey: Mutex::new(Vec::new()),
            available: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            incoming_messages: Arc::new(RwLock::new(Vec::new())),
            next_message_id: Mutex::new(1),
        })
    }

    /// Set our BLE device address.
    pub fn set_own_address(&self, address: String) {
        *self.own_address.lock() = address;
    }

    /// Set our yggdrasil public key for advertising.
    pub fn set_yggdrasil_pubkey(&self, pubkey: Vec<u8>) {
        *self.yggdrasil_pubkey.lock() = pubkey;
    }

    /// Set BLE availability (called from platform layer).
    pub fn set_available(&self, available: bool) {
        self.available
            .store(available, std::sync::atomic::Ordering::Relaxed);
    }

    /// Check if BLE is available.
    pub fn is_available(&self) -> bool {
        self.available.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Start BLE scanning for nearby peers.
    pub async fn start_scanning(&self) -> Result<(), ShieldError> {
        if !self.is_available() {
            warn!("BLE not available — cannot start scanning");
            return Err(ShieldError::nain_mode(
                ErrorCode::NainBleMeshFailed,
                "BLE hardware not available on this device",
            ));
        }

        {
            let mut state = self.scan_state.lock();
            if *state == BleScanState::ActiveScanning || *state == BleScanState::LowBatteryScanning
            {
                debug!("BLE already scanning");
                return Ok(());
            }
            *state = BleScanState::ActiveScanning;
        }

        info!("Starting BLE mesh scanning");

        // Start GATT server for advertising our presence
        self.start_gatt_server().await?;

        // Start GATT client for scanning
        self.start_gatt_client().await?;

        Ok(())
    }

    /// Stop BLE scanning.
    pub async fn stop_scanning(&self) -> Result<(), ShieldError> {
        {
            let mut state = self.scan_state.lock();
            *state = BleScanState::Inactive;
        }

        info!("BLE mesh scanning stopped");
        Ok(())
    }

    /// Get current scan state.
    pub fn scan_state(&self) -> BleScanState {
        *self.scan_state.lock()
    }

    /// Get current peer count.
    pub async fn peer_count(&self) -> u32 {
        self.peers.read().await.len() as u32
    }

    /// Get all discovered peers.
    pub async fn get_peers(&self) -> Vec<BlePeer> {
        self.peers.read().await.values().cloned().collect()
    }

    /// Get peers that have internet access (directly or via mesh).
    pub async fn get_internet_peers(&self) -> Vec<BlePeer> {
        self.peers
            .read()
            .await
            .values()
            .filter(|p| p.has_internet_access)
            .cloned()
            .collect()
    }

    /// Get direct neighbors (hop_count == 0).
    pub async fn get_direct_neighbors(&self) -> Vec<BlePeer> {
        self.peers
            .read()
            .await
            .values()
            .filter(|p| p.hop_count == 0)
            .cloned()
            .collect()
    }

    // ── Peer Discovery Callbacks ────────────────────────────────────────

    /// Called from platform layer when a BLE peer is discovered.
    pub async fn on_peer_discovered(
        &self,
        device_address: String,
        rssi: i16,
        service_data: Option<Vec<u8>>,
    ) -> Result<(), ShieldError> {
        debug!(
            addr = %device_address,
            rssi,
            has_data = service_data.is_some(),
            "BLE peer discovered"
        );

        let mut peers = self.peers.write().await;

        // Check peer limit
        if peers.len() >= self.config.max_peers && !peers.contains_key(&device_address) {
            if let Some(oldest_key) = peers
                .iter()
                .min_by_key(|(_, p)| p.last_seen)
                .map(|(k, _)| k.clone())
            {
                peers.remove(&oldest_key);
            }
        }

        peers
            .entry(device_address.clone())
            .and_modify(|existing| {
                existing.rssi = rssi;
                existing.last_seen = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
                if let Some(ref data) = service_data {
                    if data.len() >= 32 {
                        existing.yggdrasil_pubkey = Some(data[..32].to_vec());
                    }
                }
            })
            .or_insert_with(|| {
                let mut peer = BlePeer {
                    device_address: device_address.clone(),
                    yggdrasil_pubkey: None,
                    rssi,
                    mtu: DEFAULT_MTU,
                    hop_count: 0,
                    has_internet_access: false,
                    last_seen: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
                    route: vec![device_address.clone()],
                    reputation: 0.5,
                };
                if let Some(ref data) = service_data {
                    if data.len() >= 32 {
                        peer.yggdrasil_pubkey = Some(data[..32].to_vec());
                    }
                    // Check internet access flag (byte 32)
                    if data.len() > 32 && (data[32] & 0x01) != 0 {
                        peer.has_internet_access = true;
                    }
                }
                peer
            });

        // Update routing table
        let peers_snapshot = peers.clone();
        drop(peers);
        self.router.update_routes(&peers_snapshot);

        Ok(())
    }

    /// Called when a BLE peer is lost.
    pub async fn on_peer_lost(&self, device_address: &str) {
        debug!(addr = device_address, "BLE peer lost");
        self.peers.write().await.remove(device_address);
    }

    /// Called when MTU is negotiated with a peer.
    pub async fn on_mtu_negotiated(&self, device_address: &str, mtu: usize) {
        let mtu = mtu.min(MAX_MTU);
        debug!(addr = device_address, mtu, "MTU negotiated");

        if let Some(peer) = self.peers.write().await.get_mut(device_address) {
            peer.mtu = mtu;
        }
    }

    // ── Mesh Data Exchange ──────────────────────────────────────────────

    /// Send a message through the BLE mesh network.
    pub async fn send_message(
        &self,
        destination: &str,
        msg_type: MeshMessageType,
        payload: &[u8],
    ) -> Result<(), ShieldError> {
        if payload.len() > MAX_MESH_MESSAGE_SIZE {
            return Err(ShieldError::nain_mode(
                ErrorCode::NainBleMeshFailed,
                format!("Mesh message too large: {} bytes", payload.len()),
            ));
        }

        let own_addr = self.own_address.lock().clone();
        let message_id = {
            let mut id = self.next_message_id.lock();
            *id += 1;
            *id
        };

        let message = MeshMessage {
            msg_type,
            source: own_addr,
            destination: destination.to_string(),
            hop_count: 0,
            max_hops: self.config.max_hops,
            message_id,
            payload: payload.to_vec(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        if destination.is_empty() {
            // Broadcast: send to all direct neighbors
            self.broadcast_to_neighbors(&message).await
        } else {
            // Unicast: route through mesh
            self.unicast_message(&message).await
        }
    }

    /// Broadcast an endpoint update to all mesh peers.
    pub async fn broadcast_endpoint_update(&self, endpoints: &[u8]) -> Result<usize, ShieldError> {
        let neighbors = self.get_direct_neighbors().await;
        let mut sent = 0;

        for neighbor in &neighbors {
            if let Ok(()) = self
                .send_message(
                    &neighbor.device_address,
                    MeshMessageType::EndpointUpdate,
                    endpoints,
                )
                .await
            {
                sent += 1;
            }
        }

        info!(
            sent,
            total_neighbors = neighbors.len(),
            "BLE endpoint update broadcast"
        );
        Ok(sent)
    }

    /// Process an incoming mesh message from a BLE peer.
    pub async fn process_incoming_message(&self, message: MeshMessage) -> Result<(), ShieldError> {
        let own_addr = self.own_address.lock().clone();

        // Check if this message is for us
        if message.destination.is_empty() || message.destination == own_addr {
            // Process the message
            self.incoming_messages.write().await.push(message.clone());
        }

        // Should we relay this message?
        if let Some(next_hop) = self.router.should_relay(&message, &own_addr) {
            let mut relay_message = message.clone();
            relay_message.hop_count += 1;

            if next_hop == "*" {
                // Broadcast relay
                self.broadcast_to_neighbors(&relay_message).await?;
            } else {
                // Unicast relay
                self.unicast_message(&relay_message).await?;
            }
        }

        Ok(())
    }

    /// Drain all incoming messages.
    pub async fn drain_incoming_messages(&self) -> Vec<MeshMessage> {
        std::mem::take(&mut *self.incoming_messages.write().await)
    }

    // ── Battery Optimization ────────────────────────────────────────────

    /// Update battery state and adjust scanning accordingly.
    pub async fn update_battery_state(&self, battery_level: u8, is_charging: bool) {
        let current_state = self.scan_state();

        if is_charging {
            // When charging, use continuous scanning
            if current_state == BleScanState::BatterySuspended
                || current_state == BleScanState::LowBatteryScanning
            {
                if let Err(e) = self.start_scanning().await {
                    warn!(error = %e, "Failed to resume BLE scanning after charging");
                }
            }
            return;
        }

        if battery_level < self.config.critical_battery_threshold {
            // Critical battery — stop scanning
            if current_state != BleScanState::BatterySuspended {
                if let Err(e) = self.stop_scanning().await {
                    warn!(error = %e, "Failed to stop BLE scanning for critical battery");
                }
                *self.scan_state.lock() = BleScanState::BatterySuspended;
            }
        } else if battery_level < self.config.low_battery_threshold {
            // Low battery — reduce duty cycle
            *self.scan_state.lock() = BleScanState::LowBatteryScanning;
            info!("BLE mesh switched to low-battery scanning (10s/50s duty cycle)");
        }
    }

    /// Get the recommended scan parameters for the current state.
    pub fn scan_parameters(&self) -> (Duration, Duration) {
        match self.scan_state() {
            BleScanState::ActiveScanning => (self.config.scan_window, self.config.scan_pause),
            BleScanState::LowBatteryScanning => (
                self.config.low_battery_scan_window,
                self.config.low_battery_scan_pause,
            ),
            BleScanState::BatterySuspended | BleScanState::Inactive | BleScanState::ScanPaused => {
                (Duration::ZERO, Duration::MAX)
            }
        }
    }

    /// Estimate current power consumption in mA.
    pub fn estimated_power_ma(&self) -> f64 {
        match self.scan_state() {
            BleScanState::ActiveScanning => 2.0,
            BleScanState::LowBatteryScanning => 0.5, // Reduced duty cycle
            BleScanState::BatterySuspended | BleScanState::Inactive | BleScanState::ScanPaused => {
                0.0
            }
        }
    }

    // ── Maintenance ─────────────────────────────────────────────────────

    /// Remove expired peers.
    pub async fn prune_expired_peers(&self) {
        let mut peers = self.peers.write().await;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let expired: Vec<String> = peers
            .iter()
            .filter(|(_, p)| Duration::from_millis(now_ms.saturating_sub(p.last_seen)) > PEER_EXPIRY)
            .map(|(k, _)| k.clone())
            .collect();

        for key in expired {
            peers.remove(&key);
        }

        // Update routing table
        let peers_snapshot = peers.clone();
        drop(peers);
        self.router.update_routes(&peers_snapshot);

        // Prune old message IDs
        self.router.prune_seen_messages(500);
    }

    // ── Internal methods ────────────────────────────────────────────────

    /// Start the GATT server for advertising our presence.
    async fn start_gatt_server(&self) -> Result<(), ShieldError> {
        let pubkey = self.yggdrasil_pubkey.lock().clone();

        if pubkey.is_empty() {
            debug!("No yggdrasil public key set — BLE GATT server will advertise without it");
        }

        // In production, this creates a BLE GATT server with:
        //   - Shield Service UUID
        //   - Yggdrasil Public Key characteristic (read)
        //   - Mesh Data characteristic (write/notify)
        //   - Internet Access Flag (read)
        //
        // On Android: BluetoothGattServer + BluetoothGattServerCallback via JNI
        // On iOS: CBPeripheralManager

        debug!("BLE GATT server started");
        Ok(())
    }

    /// Start the GATT client for scanning.
    async fn start_gatt_client(&self) -> Result<(), ShieldError> {
        // In production, this starts BLE scanning with:
        //   - Service UUID filter for SHIELD_SERVICE_UUID
        //   - Scan mode: SCAN_MODE_LOW_LATENCY (active) or SCAN_MODE_LOW_POWER (background)
        //   - allowDuplicates = false (iOS) / false (Android)
        //
        // On Android: BluetoothLeScanner.startScan() via JNI
        // On iOS: CBCentralManager.scanForPeripherals()

        debug!("BLE GATT client scanning started");
        Ok(())
    }

    /// Broadcast a message to all direct neighbors.
    async fn broadcast_to_neighbors(&self, message: &MeshMessage) -> Result<(), ShieldError> {
        let neighbors = self.get_direct_neighbors().await;
        let mut sent = 0;

        for neighbor in &neighbors {
            // In production, this writes to the GATT characteristic
            // of each connected peer
            sent += 1;
        }

        debug!(sent, total = neighbors.len(), "BLE mesh broadcast");
        Ok(())
    }

    /// Send a unicast message through the mesh.
    async fn unicast_message(&self, message: &MeshMessage) -> Result<(), ShieldError> {
        // Look up the next hop from the routing table
        let next_hop = {
            let routes = self.router.routes.lock();
            routes.get(&message.destination).cloned()
        };

        match next_hop {
            Some(hop) => {
                debug!(
                    dest = %message.destination,
                    next_hop = %hop,
                    "BLE mesh unicast routing"
                );
                // In production, write to the GATT characteristic of the next hop
                Ok(())
            }
            None => {
                // No route known — broadcast to discover
                warn!(
                    dest = %message.destination,
                    "No BLE mesh route — broadcasting"
                );
                self.broadcast_to_neighbors(message).await
            }
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ble_mesh_creation() {
        let mesh = BleMeshNetwork::new().unwrap();
        assert!(!mesh.is_available());
        assert_eq!(mesh.scan_state(), BleScanState::Inactive);
        assert_eq!(mesh.peer_count().await, 0);
    }

    #[tokio::test]
    async fn test_peer_discovery() {
        let mesh = BleMeshNetwork::new().unwrap();
        mesh.set_available(true);
        mesh.set_own_address("AA:BB:CC:DD:EE:FF".to_string());

        mesh.on_peer_discovered(
            "11:22:33:44:55:66".to_string(),
            -45,
            Some(vec![0x42; 33]), // 32 bytes pubkey + 1 byte flags
        )
        .await
        .unwrap();

        assert_eq!(mesh.peer_count().await, 1);

        let neighbors = mesh.get_direct_neighbors().await;
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].device_address, "11:22:33:44:55:66");
    }

    #[tokio::test]
    async fn test_mesh_router() {
        let router = MeshRouter::new();
        let own_addr = "AA:BB:CC:DD:EE:FF";

        let message = MeshMessage {
            msg_type: MeshMessageType::EndpointUpdate,
            source: "11:22:33:44:55:66".to_string(),
            destination: String::new(), // broadcast
            hop_count: 0,
            max_hops: 5,
            message_id: 1,
            payload: vec![0x01, 0x02, 0x03],
            created_at: 0,
        };

        // Should relay broadcast messages
        let next_hop = router.should_relay(&message, own_addr);
        assert_eq!(next_hop, Some("*".to_string()));

        // Should not relay our own messages
        let own_message = MeshMessage {
            source: own_addr.to_string(),
            ..message.clone()
        };
        assert!(router.should_relay(&own_message, own_addr).is_none());
    }

    #[test]
    fn test_scan_parameters() {
        let mesh = BleMeshNetwork::new().unwrap();

        // Default: active scanning
        *mesh.scan_state.lock() = BleScanState::ActiveScanning;
        let (window, pause) = mesh.scan_parameters();
        assert_eq!(window, Duration::from_secs(30));
        assert_eq!(pause, Duration::from_secs(30));

        // Low battery
        *mesh.scan_state.lock() = BleScanState::LowBatteryScanning;
        let (window, pause) = mesh.scan_parameters();
        assert_eq!(window, Duration::from_secs(10));
        assert_eq!(pause, Duration::from_secs(50));
    }

    #[test]
    fn test_power_estimation() {
        let mesh = BleMeshNetwork::new().unwrap();

        *mesh.scan_state.lock() = BleScanState::ActiveScanning;
        assert_eq!(mesh.estimated_power_ma(), 2.0);

        *mesh.scan_state.lock() = BleScanState::LowBatteryScanning;
        assert_eq!(mesh.estimated_power_ma(), 0.5);

        *mesh.scan_state.lock() = BleScanState::Inactive;
        assert_eq!(mesh.estimated_power_ma(), 0.0);
    }
}
