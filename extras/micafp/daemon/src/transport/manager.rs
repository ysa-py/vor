//! Transport Manager with UCB1 Bandit Endpoint Selection
//!
//! Maintains a prioritized list of transports and uses the UCB1 (Upper
//! Confidence Bound) algorithm to select the best endpoint for each
//! connection. Implements auto-rotation, exponential backoff, battery-aware
//! probing, and endpoint refresh via IPFS/acoustic/NTP/SMS channels.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use rand::Rng;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use super::{
    exponential_backoff_with_jitter, BatteryState, EndpointStats, ShieldError, Transport,
    TransportConnection,
};

// ── Constants ───────────────────────────────────────────────────────────────

/// Maximum number of retry attempts before switching transport.
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (milliseconds).
const BACKOFF_BASE_MS: u64 = 500;

/// Maximum backoff cap (milliseconds).
const BACKOFF_MAX_MS: u64 = 30_000;

/// Duration after which an endpoint is deprioritized (24 hours).
const DEPRIORITIZE_THRESHOLD: Duration = Duration::from_secs(24 * 60 * 60);

/// How often to check endpoint health (normal mode).
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(30);

/// How often to check endpoint health (battery critical).
const HEALTH_CHECK_INTERVAL_CRITICAL: Duration = Duration::from_secs(150);

/// UCB1 exploration factor (higher = more exploration).
const UCB1_EXPLORATION: f64 = 2.0;

/// Number of endpoints to probe per health check cycle.
const PROBE_BATCH_SIZE: usize = 3;

// ── Refresh channel types ───────────────────────────────────────────────────

/// Channels through which endpoint lists can be refreshed without restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefreshChannel {
    /// IPFS distributed hash table.
    Ipfs,
    /// Acoustic near-field communication.
    Acoustic,
    /// NTP packet injection.
    Ntp,
    /// SMS-based endpoint distribution.
    Sms,
}

impl RefreshChannel {
    /// All available refresh channels, in preference order.
    pub fn all() -> Vec<RefreshChannel> {
        vec![
            RefreshChannel::Ipfs,
            RefreshChannel::Acoustic,
            RefreshChannel::Ntp,
            RefreshChannel::Sms,
        ]
    }
}

// ── Transport entry (internal) ──────────────────────────────────────────────

/// Internal representation of a registered transport.
struct TransportEntry {
    transport: Arc<dyn Transport>,
    /// Per-endpoint statistics for this transport.
    endpoint_stats: HashMap<SocketAddr, EndpointStats>,
    /// Number of total selections (for UCB1 denominator).
    total_selections: u64,
}

impl TransportEntry {
    fn new(transport: Arc<dyn Transport>) -> Self {
        Self {
            transport,
            endpoint_stats: HashMap::new(),
            total_selections: 0,
        }
    }
}

// ── Connection result ───────────────────────────────────────────────────────

/// Result of a connection attempt, including metadata for the bandit algorithm.
pub struct ConnectionResult {
    pub connection: Box<dyn TransportConnection>,
    pub latency_ms: f64,
    pub endpoint: SocketAddr,
}

// ── Transport Manager ───────────────────────────────────────────────────────

/// Central transport manager that selects the best transport and endpoint
/// using a UCB1 bandit algorithm.
///
/// # Algorithm
///
/// The UCB1 score for each endpoint is:
/// ```text
/// score = success_rate + sqrt(EXPLORATION * ln(total_selections) / endpoint_attempts)
/// ```
///
/// This balances exploitation (choosing known-good endpoints) with
/// exploration (trying less-used endpoints that might be better).
///
/// # Auto-Rotation
///
/// When the current transport fails `MAX_RETRIES` times consecutively,
/// the manager automatically rotates to the next available transport
/// in priority order.
///
/// # Battery Integration
///
/// Probing frequency is reduced when the battery module reports
/// Low or Critical state.
pub struct TransportManager {
    /// Registered transports, sorted by priority.
    transports: Arc<RwLock<Vec<TransportEntry>>>,
    /// Index of the currently active transport.
    current_transport_idx: RwLock<usize>,
    /// Current battery state.
    battery_state: Arc<RwLock<BatteryState>>,
    /// Consecutive failure count for current transport.
    consecutive_failures: RwLock<u32>,
    /// Background health check task handle.
    health_check_handle: RwLock<Option<JoinHandle<()>>>,
    /// Whether the manager has been shut down.
    shutdown: Arc<RwLock<bool>>,
    /// Refresh channel endpoints (IPFS CID, phone numbers, etc.).
    refresh_config: RwLock<HashMap<RefreshChannel, String>>,
    /// Callback for UI status updates (connected/disconnected only).
    status_callback: RwLock<Option<Arc<dyn Fn(bool) + Send + Sync>>>,
}

impl TransportManager {
    /// Create a new transport manager with no transports registered.
    pub fn new() -> Self {
        Self {
            transports: Arc::new(RwLock::new(Vec::new())),
            current_transport_idx: RwLock::new(0),
            battery_state: Arc::new(RwLock::new(BatteryState::Unknown)),
            consecutive_failures: RwLock::new(0),
            health_check_handle: RwLock::new(None),
            shutdown: Arc::new(RwLock::new(false)),
            refresh_config: RwLock::new(HashMap::new()),
            status_callback: RwLock::new(None),
        }
    }

    /// Register a transport with the manager.
    ///
    /// Transports are automatically sorted by priority after registration.
    pub async fn register_transport(&self, transport: Arc<dyn Transport>) {
        let mut transports = self.transports.write().await;
        transports.push(TransportEntry::new(transport));
        // Sort by priority (lowest = highest priority)
        transports.sort_by_key(|e| e.transport.priority());
    }

    /// Set the battery state (called by the battery module).
    pub async fn set_battery_state(&self, state: BatteryState) {
        *self.battery_state.write().await = state;
    }

    /// Set a callback for UI status updates.
    ///
    /// The callback receives `true` when connected, `false` when disconnected.
    /// Endpoint URLs are NEVER exposed to the UI.
    pub async fn set_status_callback(&self, callback: Arc<dyn Fn(bool) + Send + Sync>) {
        *self.status_callback.write().await = Some(callback);
    }

    /// Configure a refresh channel for endpoint list updates.
    pub async fn configure_refresh_channel(&self, channel: RefreshChannel, config: String) {
        self.refresh_config.write().await.insert(channel, config);
    }

    /// Start the background health check loop.
    pub async fn start_health_checks(&self) {
        let manager_ptr = self as *const Self as usize;
        let shutdown = Arc::clone(&self.shutdown);
        let transports = Arc::clone(&self.transports);
        let battery_state = Arc::clone(&self.battery_state);

        let handle = tokio::spawn(async move {
            loop {
                // Check shutdown flag
                if *shutdown.read().await {
                    break;
                }

                // Determine interval based on battery state
                let battery = *battery_state.read().await;
                let interval = match battery {
                    BatteryState::Critical => HEALTH_CHECK_INTERVAL_CRITICAL,
                    BatteryState::Low => Duration::from_secs(
                        (HEALTH_CHECK_INTERVAL.as_secs() as f64 * battery.probe_throttle()) as u64,
                    ),
                    _ => HEALTH_CHECK_INTERVAL,
                };

                // Probe a batch of endpoints
                {
                    let mut transports_guard = transports.write().await;
                    for entry in transports_guard.iter_mut() {
                        let available_endpoints: Vec<SocketAddr> =
                            entry.endpoint_stats.keys().cloned().collect();

                        // Probe only PROBE_BATCH_SIZE endpoints per cycle
                        let batch: Vec<SocketAddr> = available_endpoints
                            .into_iter()
                            .take(PROBE_BATCH_SIZE)
                            .collect();

                        for addr in batch {
                            if let Some(stats) = entry.endpoint_stats.get_mut(&addr) {
                                // Check deprioritization
                                stats.check_deprioritization();

                                // Quick TCP connect probe
                                let start = Instant::now();
                                match tokio::net::TcpStream::connect(&addr).await {
                                    Ok(_) => {
                                        let latency = start.elapsed().as_millis() as f64;
                                        stats.record_success(latency);
                                    }
                                    Err(_) => {
                                        stats.record_failure();
                                    }
                                }
                            }
                        }
                    }
                }

                tokio::time::sleep(interval).await;
            }
        });

        *self.health_check_handle.write().await = Some(handle);
    }

    /// Calculate the UCB1 score for an endpoint.
    ///
    /// Score = success_rate + sqrt(EXPLORATION * ln(total) / endpoint_attempts)
    ///
    /// Higher score = more likely to be selected.
    fn ucb1_score(stats: &EndpointStats, total_selections: u64) -> f64 {
        if stats.total_attempts == 0 {
            return f64::MAX; // Always try unexplored endpoints
        }

        let success_rate = stats.success_rate();

        // Latency penalty: normalize latency to [0, 1] range
        // 0ms -> 0.0, 1000ms+ -> 1.0
        let latency_penalty = if stats.latency_ms == f64::MAX {
            1.0
        } else {
            (stats.latency_ms / 1000.0).min(1.0)
        };

        // Recency bonus: recently seen endpoints get a small boost
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let age_ms = now.saturating_sub(stats.last_seen);
        let recency_bonus = if stats.last_seen == 0 {
            0.0
        } else {
            // 5-minute recency window
            let recency_factor = 1.0 - (age_ms as f64 / 300_000.0).min(1.0);
            recency_factor * 0.1
        };

        // Deprioritization penalty
        let deprioritize_factor = if stats.deprioritized { 0.5 } else { 1.0 };

        // UCB1 exploration term
        let exploration = if total_selections == 0 {
            0.0
        } else {
            (UCB1_EXPLORATION * (total_selections as f64).ln() / stats.total_attempts as f64)
                .sqrt()
        };

        let base_score = success_rate - (latency_penalty * 0.3) + recency_bonus + exploration;
        base_score * deprioritize_factor
    }

    /// Select the best endpoint for the current transport using UCB1.
    async fn select_endpoint(&self) -> Result<(usize, SocketAddr), ShieldError> {
        let mut transports = self.transports.write().await;
        let current_idx = *self.current_transport_idx.read().await;

        if transports.is_empty() {
            return Err(ShieldError::AllEndpointsExhausted);
        }

        // Try transports in priority order starting from current
        let total_transports = transports.len();
        for offset in 0..total_transports {
            let idx = (current_idx + offset) % total_transports;
            let entry = &mut transports[idx];

            // Check if this transport is available
            if !entry.transport.is_available().await {
                continue;
            }

            // If no endpoints registered yet, use the transport directly
            if entry.endpoint_stats.is_empty() {
                return Ok((idx, "0.0.0.0:0".parse().unwrap()));
            }

            // Select best endpoint using UCB1
            let total = entry.total_selections;
            let mut best_score = f64::MIN;
            let mut best_addr: Option<SocketAddr> = None;

            for (addr, stats) in entry.endpoint_stats.iter() {
                let score = Self::ucb1_score(stats, total);
                if score > best_score {
                    best_score = score;
                    best_addr = Some(*addr);
                }
            }

            if let Some(addr) = best_addr {
                entry.total_selections += 1;
                return Ok((idx, addr));
            }
        }

        Err(ShieldError::AllEndpointsExhausted)
    }

    /// Add an endpoint to a specific transport's statistics.
    pub async fn add_endpoint(&self, transport_name: &str, addr: SocketAddr) {
        let mut transports = self.transports.write().await;
        for entry in transports.iter_mut() {
            if entry.transport.name() == transport_name {
                entry
                    .endpoint_stats
                    .entry(addr)
                    .or_insert_with(|| EndpointStats::new());
                break;
            }
        }
    }

    /// Remove an endpoint from a specific transport.
    pub async fn remove_endpoint(&self, transport_name: &str, addr: &SocketAddr) {
        let mut transports = self.transports.write().await;
        for entry in transports.iter_mut() {
            if entry.transport.name() == transport_name {
                entry.endpoint_stats.remove(addr);
                break;
            }
        }
    }

    /// Refresh endpoint lists from all configured channels.
    ///
    /// This downloads new endpoint lists via IPFS/acoustic/NTP/SMS
    /// without requiring a restart.
    pub async fn refresh_endpoints(&self) -> Result<(), ShieldError> {
        let config = self.refresh_config.read().await;

        for channel in RefreshChannel::all() {
            if let Some(channel_config) = config.get(&channel) {
                let new_endpoints = self
                    .fetch_endpoints_from_channel(channel, channel_config)
                    .await?;
                for (transport_name, addr) in new_endpoints {
                    self.add_endpoint(&transport_name, addr).await;
                }
            }
        }

        Ok(())
    }

    /// Fetch endpoints from a specific refresh channel.
    async fn fetch_endpoints_from_channel(
        &self,
        channel: RefreshChannel,
        config: &str,
    ) -> Result<Vec<(String, SocketAddr)>, ShieldError> {
        match channel {
            RefreshChannel::Ipfs => self.fetch_from_ipfs(config).await,
            RefreshChannel::Acoustic => self.fetch_from_acoustic(config).await,
            RefreshChannel::Ntp => self.fetch_from_ntp(config).await,
            RefreshChannel::Sms => self.fetch_from_sms(config).await,
        }
    }

    /// Fetch endpoints from IPFS distributed hash table.
    async fn fetch_from_ipfs(&self, cid: &str) -> Result<Vec<(String, SocketAddr)>, ShieldError> {
        // IPFS gateway endpoint resolution
        // Connect to local IPFS node and resolve CID
        let gateway_urls = ["https://ipfs.io/ipfs/", "https://dweb.link/ipfs/"];

        for gateway in &gateway_urls {
            let url = format!("{}{}", gateway, cid);
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .map_err(|e| ShieldError::Config(format!("HTTP client: {}", e)))?;

            if let Ok(resp) = client.get(&url).send().await {
                if let Ok(body) = resp.text().await {
                    return self.parse_endpoint_list(&body);
                }
            }
        }

        Ok(vec![])
    }

    /// Fetch endpoints via acoustic near-field communication.
    async fn fetch_from_acoustic(
        &self,
        _config: &str,
    ) -> Result<Vec<(String, SocketAddr)>, ShieldError> {
        // Acoustic modem: decode FSK/OFDM signal from microphone
        // This is a placeholder for the acoustic receiver
        // In production, this would use the cpal crate for audio I/O
        // and a custom FSK demodulator
        Ok(vec![])
    }

    /// Fetch endpoints via NTP packet injection.
    async fn fetch_from_ntp(
        &self,
        ntp_server: &str,
    ) -> Result<Vec<(String, SocketAddr)>, ShieldError> {
        // NTP-based covert channel: endpoints encoded in NTP packet fields
        // (reference timestamp, originate timestamp carry encoded data)
        use tokio::net::UdpSocket;

        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| ShieldError::Io(e.to_string()))?;

        socket
            .connect((ntp_server, 123))
            .await
            .map_err(|e| ShieldError::EndpointUnreachable(format!("NTP: {}", e)))?;

        // Craft NTP client request with request-for-endpoints flag
        // in the reference timestamp field
        let mut ntp_packet = [0u8; 48];
        ntp_packet[0] = 0x1B; // LI=0, VN=3, Mode=3 (client)
        ntp_packet[40] = 0xDE; // Magic marker for endpoint request
        ntp_packet[41] = 0xAD;
        ntp_packet[42] = 0xBE;
        ntp_packet[43] = 0xEF;

        socket
            .send(&ntp_packet)
            .await
            .map_err(|e| ShieldError::Io(e.to_string()))?;

        let mut buf = [0u8; 48];
        match tokio::time::timeout(Duration::from_secs(5), socket.recv(&mut buf)).await {
            Ok(Ok(len)) if len >= 48 => {
                // Decode endpoint data from NTP response fields
                // Transmit timestamp and receive timestamp carry endpoint data
                let mut endpoints = Vec::new();

                // Each NTP timestamp field is 8 bytes: 4 bytes IP + 2 bytes port + 2 bytes transport_id
                for offset in &[32, 40] {
                    let ip_bytes = &buf[*offset..*offset + 4];
                    let port_bytes = &buf[*offset + 4..*offset + 6];
                    let transport_id = u16::from_be_bytes([buf[*offset + 6], buf[*offset + 7]]);

                    let ip = std::net::IpAddr::from(std::net::Ipv4Addr::new(
                        ip_bytes[0],
                        ip_bytes[1],
                        ip_bytes[2],
                        ip_bytes[3],
                    ));
                    let port = u16::from_be_bytes([port_bytes[0], port_bytes[1]]);

                    if port > 0 {
                        let transport_name = match transport_id {
                            0 => "vless",
                            1 => "shadow-tls-v3",
                            2 => "reality",
                            3 => "hysteria2",
                            4 => "tuic-v5",
                            5 => "naive-proxy",
                            6 => "cdn-worker",
                            7 => "doq-tunnel",
                            8 => "webtransport",
                            9 => "meek",
                            10 => "mqtt-ws",
                            11 => "icmp-tunnel",
                            _ => continue,
                        };
                        endpoints.push((transport_name.to_string(), SocketAddr::new(ip, port)));
                    }
                }

                Ok(endpoints)
            }
            _ => Ok(vec![]),
        }
    }

    /// Fetch endpoints via SMS-based distribution.
    async fn fetch_from_sms(
        &self,
        phone_number: &str,
    ) -> Result<Vec<(String, SocketAddr)>, ShieldError> {
        // SMS-based endpoint distribution
        // In production, this would use AT modem commands or
        // Android SMS API to read encoded endpoint messages
        // Format: SHIELD:<base64-encoded-endpoint-list>
        let _ = phone_number; // Phone number to send SMS request to
        Ok(vec![])
    }

    /// Parse an endpoint list from a text body (IPFS/acoustic/SMS).
    fn parse_endpoint_list(&self, body: &str) -> Result<Vec<(String, SocketAddr)>, ShieldError> {
        let mut endpoints = Vec::new();

        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Format: transport_name,address:port
            let parts: Vec<&str> = line.splitn(2, ',').collect();
            if parts.len() != 2 {
                continue;
            }

            let transport_name = parts[0].trim();
            if let Ok(addr) = parts[1].trim().parse::<SocketAddr>() {
                endpoints.push((transport_name.to_string(), addr));
            }
        }

        Ok(endpoints)
    }

    /// Connect to the destination using the best available transport.
    ///
    /// This is the main entry point for establishing connections.
    /// It uses the UCB1 bandit algorithm to select the best transport
    /// and endpoint, with automatic rotation on failure.
    pub async fn connect_best(&self, dest: &SocketAddr) -> Result<ConnectionResult, ShieldError> {
        if *self.shutdown.read().await {
            return Err(ShieldError::TransportUnavailable(
                "Manager is shut down".into(),
            ));
        }

        // Notify UI: connecting
        if let Some(cb) = self.status_callback.read().await.as_ref() {
            cb(false);
        }

        let mut last_error: Option<ShieldError> = None;
        let mut retry_count = 0;

        loop {
            // Select best transport + endpoint using UCB1
            let (transport_idx, endpoint) = match self.select_endpoint().await {
                Ok(result) => result,
                Err(e) => {
                    // All transports exhausted
                    if let Some(cb) = self.status_callback.read().await.as_ref() {
                        cb(false);
                    }
                    return Err(e);
                }
            };

            // Get the transport
            let transport = {
                let transports = self.transports.read().await;
                transports[transport_idx].transport.clone()
            };

            // Update current transport index
            *self.current_transport_idx.write().await = transport_idx;

            // Attempt connection with exponential backoff
            let start = Instant::now();

            match transport.connect(dest).await {
                Ok(conn) => {
                    let latency_ms = start.elapsed().as_millis() as f64;

                    // Record success in endpoint stats
                    {
                        let mut transports = self.transports.write().await;
                        if let Some(stats) =
                            transports[transport_idx].endpoint_stats.get_mut(&endpoint)
                        {
                            stats.record_success(latency_ms);
                        }
                    }

                    // Reset consecutive failure counter
                    *self.consecutive_failures.write().await = 0;

                    // Notify UI: connected
                    if let Some(cb) = self.status_callback.read().await.as_ref() {
                        cb(true);
                    }

                    return Ok(ConnectionResult {
                        connection: conn,
                        latency_ms,
                        endpoint,
                    });
                }
                Err(e) => {
                    last_error = Some(e.clone());

                    // Record failure in endpoint stats
                    {
                        let mut transports = self.transports.write().await;
                        if let Some(stats) =
                            transports[transport_idx].endpoint_stats.get_mut(&endpoint)
                        {
                            stats.record_failure();
                        }
                    }

                    // Increment consecutive failures
                    let mut failures = self.consecutive_failures.write().await;
                    *failures += 1;

                    if *failures >= MAX_RETRIES {
                        // Auto-rotate to next transport
                        *failures = 0;
                        drop(failures); // Release borrow

                        let total = self.transports.read().await.len();
                        if total > 0 {
                            let mut idx = self.current_transport_idx.write().await;
                            *idx = (*idx + 1) % total;
                        }

                        retry_count += 1;
                        if retry_count >= 3 {
                            // Tried rotating through all transports multiple times
                            if let Some(cb) = self.status_callback.read().await.as_ref() {
                                cb(false);
                            }
                            return Err(last_error.unwrap_or(ShieldError::AllEndpointsExhausted));
                        }
                    } else {
                        drop(failures);
                    }

                    // Exponential backoff with jitter
                    let backoff = exponential_backoff_with_jitter(
                        *self.consecutive_failures.read().await,
                        BACKOFF_BASE_MS,
                        BACKOFF_MAX_MS,
                    );
                    tokio::time::sleep(backoff).await;

                    // Try rotating SNI domain on current transport
                    let _ = transport.rotate_sni_domain().await;
                }
            }
        }
    }

    /// Get the name of the currently active transport.
    pub async fn current_transport_name(&self) -> String {
        let transports = self.transports.read().await;
        let idx = *self.current_transport_idx.read().await;
        if idx < transports.len() {
            transports[idx].transport.name().to_string()
        } else {
            "none".to_string()
        }
    }

    /// Get aggregated statistics for all transports.
    ///
    /// Returns (total_endpoints, available_endpoints, avg_latency_ms).
    pub async fn get_stats(&self) -> (usize, usize, f64) {
        let transports = self.transports.read().await;
        let mut total = 0;
        let mut available = 0;
        let mut total_latency = 0.0;
        let mut latency_count = 0;

        for entry in transports.iter() {
            for stats in entry.endpoint_stats.values() {
                total += 1;
                if !stats.deprioritized && stats.success_count > 0 {
                    available += 1;
                    if stats.latency_ms != f64::MAX {
                        total_latency += stats.latency_ms;
                        latency_count += 1;
                    }
                }
            }
        }

        let avg_latency = if latency_count > 0 {
            total_latency / latency_count as f64
        } else {
            0.0
        };

        (total, available, avg_latency)
    }

    /// Check whether any transport is currently available.
    pub async fn is_connected(&self) -> bool {
        let transports = self.transports.read().await;
        let idx = *self.current_transport_idx.read().await;
        if idx < transports.len() {
            transports[idx].transport.is_available().await
        } else {
            false
        }
    }

    /// Shut down the transport manager and all registered transports.
    pub async fn shutdown_manager(&self) {
        *self.shutdown.write().await = true;

        // Stop health check task
        if let Some(handle) = self.health_check_handle.write().await.take() {
            handle.abort();
        }

        // Shut down all transports
        let transports = self.transports.read().await;
        for entry in transports.iter() {
            let _ = entry.transport.shutdown().await;
        }
    }
}

impl Default for TransportManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ucb1_score_unexplored() {
        let stats = EndpointStats::new();
        let score = TransportManager::ucb1_score(&stats, 100);
        assert_eq!(score, f64::MAX); // Unexplored endpoints get max score
    }

    #[test]
    fn test_ucb1_score_success_rate() {
        let mut stats = EndpointStats::new();
        stats.success_count = 80;
        stats.fail_count = 20;
        stats.latency_ms = 100.0;
        stats.last_seen = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let score = TransportManager::ucb1_score(&stats, 1000);
        assert!(score > 0.0);
        assert!(score < 2.0); // Reasonable range
    }

    #[test]
    fn test_endpoint_deprioritization() {
        let mut stats = EndpointStats::new();
        stats.success_count = 1;
        stats.last_seen = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
            - 25 * 60 * 60 * 1000; // 25 hours ago
        stats.check_deprioritization();
        assert!(stats.deprioritized);
    }

    #[test]
    fn test_exponential_backoff() {
        let d0 = exponential_backoff_with_jitter(0, 500, 30_000);
        let d1 = exponential_backoff_with_jitter(1, 500, 30_000);
        let d2 = exponential_backoff_with_jitter(2, 500, 30_000);
        assert!(d0 <= d1 || d0 <= d2); // Jitter may cause overlap
    }
}
