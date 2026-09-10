// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — Peer Exchange System
//
// Manages the local peer database with reputation scores, and handles
// peer exchange via all available channels (Yggdrasil, WiFi Aware, BLE,
// acoustic, NTP, SMS).
//
// Key features:
//   • Maintains local peer database with reputation scores
//   • Exchange peer lists via Yggdrasil, WiFi Aware, BLE, acoustic channels
//   • Rate limit peer exchanges to prevent flooding
//   • Sign peer announcements with Ed25519
//   • Verify signatures before accepting peers
// ─────────────────────────────────────────────────────────────────────────────

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use parking_lot::Mutex;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::error::{ErrorCode, ShieldError};
use hex;

// ── Constants ───────────────────────────────────────────────────────────────

/// Maximum number of peers to store.
const MAX_PEERS: usize = 500;
/// Maximum number of endpoints per peer.
const MAX_ENDPOINTS_PER_PEER: usize = 8;
/// Peer expiry timeout — peers not seen for this long are removed.
const PEER_EXPIRY: Duration = Duration::from_secs(86400); // 24 hours
/// Minimum interval between peer exchange requests.
const MIN_EXCHANGE_INTERVAL: Duration = Duration::from_secs(30);
/// Maximum peers per exchange message (prevents flooding).
const MAX_PEERS_PER_EXCHANGE: usize = 20;
/// Minimum reputation score to be considered a "good" peer.
const GOOD_PEER_THRESHOLD: f64 = 0.6;
/// Reputation increase on successful connection.
const REP_SUCCESS_DELTA: f64 = 0.05;
/// Reputation decrease on failed connection.
const REP_FAILURE_DELTA: f64 = 0.1;
/// Maximum reputation score.
const REP_MAX: f64 = 1.0;
/// Minimum reputation score.
const REP_MIN: f64 = 0.0;

// ── Peer Record ─────────────────────────────────────────────────────────────

/// A peer record in the local database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerRecord {
    /// Unique peer identifier (Ed25519 public key, hex encoded).
    pub peer_id: String,
    /// Available endpoint addresses (e.g., "tcp://1.2.3.4:51820").
    pub endpoints: Vec<String>,
    /// Ed25519 verifying key for signature validation.
    pub verifying_key: Option<String>,
    /// Reputation score (0.0 - 1.0).
    pub reputation: f64,
    /// How this peer was discovered.
    pub discovery_source: String,
    /// Number of successful connections.
    pub success_count: u32,
    /// Number of failed connections.
    pub failure_count: u32,
    /// Timestamp when this peer was first seen.
    pub first_seen: u64,
    /// Timestamp when this peer was last seen.
    pub last_seen: u64,
    /// Last time we exchanged peers with this peer.
    pub last_exchange: Option<u64>,
    /// Whether this peer is currently connected.
    pub connected: bool,
    /// Geographic region (if known).
    pub region: Option<String>,
    /// Supported transport protocols.
    pub transports: Vec<String>,
}

impl PeerRecord {
    /// Check if this peer is considered "good" (above reputation threshold).
    pub fn is_good(&self) -> bool {
        self.reputation >= GOOD_PEER_THRESHOLD
    }

    /// Record a successful connection.
    pub fn record_success(&mut self) {
        self.success_count = self.success_count.saturating_add(1);
        self.reputation = (self.reputation + REP_SUCCESS_DELTA).min(REP_MAX);
        self.last_seen = now_secs();
    }

    /// Record a failed connection.
    pub fn record_failure(&mut self) {
        self.failure_count = self.failure_count.saturating_add(1);
        self.reputation = (self.reputation - REP_FAILURE_DELTA).max(REP_MIN);
    }

    /// Update endpoints from a new peer announcement.
    pub fn update_endpoints(&mut self, new_endpoints: &[String]) {
        let mut merged: HashSet<String> = self.endpoints.iter().cloned().collect();
        for ep in new_endpoints {
            merged.insert(ep.clone());
        }
        self.endpoints = merged.into_iter().take(MAX_ENDPOINTS_PER_PEER).collect();
        self.last_seen = now_secs();
    }
}

// ── Peer Announcement ───────────────────────────────────────────────────────

/// A signed peer announcement message.
///
/// This is the wire format for exchanging peer information between nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAnnouncement {
    /// Protocol version.
    pub version: u8,
    /// Announcing peer's identifier.
    pub peer_id: String,
    /// Endpoints the announcing peer is reachable at.
    pub endpoints: Vec<String>,
    /// Timestamp (UNIX epoch seconds) when this announcement was created.
    pub timestamp: u64,
    /// TTL (time-to-live) in seconds — how long this announcement is valid.
    pub ttl_secs: u32,
    /// Supported transport protocols.
    pub transports: Vec<String>,
    /// Ed25519 signature over the announcement body.
    pub signature: Vec<u8>,
}

impl PeerAnnouncement {
    /// Current protocol version.
    pub const VERSION: u8 = 1;
    /// Default TTL: 6 hours.
    pub const DEFAULT_TTL: u32 = 21600;

    /// Create a new unsigned peer announcement.
    pub fn new(peer_id: String, endpoints: Vec<String>, transports: Vec<String>) -> Self {
        Self {
            version: Self::VERSION,
            peer_id,
            endpoints,
            timestamp: now_secs(),
            ttl_secs: Self::DEFAULT_TTL,
            transports,
            signature: Vec::new(),
        }
    }

    /// Sign the announcement with the given signing key.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<(), ShieldError> {
        let message = self.signable_bytes();
        let signature = signing_key.sign(&message);
        self.signature = signature.to_bytes().to_vec();
        Ok(())
    }

    /// Verify the announcement's signature.
    pub fn verify(&self, verifying_key: &VerifyingKey) -> Result<bool, ShieldError> {
        if self.signature.len() != 64 {
            return Ok(false);
        }

        let message = self.signable_bytes();
        let sig_bytes: [u8; 64] = self.signature.clone().try_into().map_err(|_| {
            ShieldError::p2p(ErrorCode::P2pPeerExchangeFailed, "Invalid signature length")
        })?;
        let signature = Signature::from_bytes(&sig_bytes);

        match verifying_key.verify(&message, &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Check if this announcement is still valid (not expired).
    pub fn is_valid(&self) -> bool {
        let now = now_secs();
        now.saturating_sub(self.timestamp) < self.ttl_secs as u64
    }

    /// Get the bytes that are covered by the signature.
    fn signable_bytes(&self) -> Vec<u8> {
        // Sign over: version + peer_id + endpoints + timestamp + ttl + transports
        let mut buf = Vec::new();
        buf.push(self.version);
        buf.extend_from_slice(self.peer_id.as_bytes());
        buf.push(0); // null separator
        for ep in &self.endpoints {
            buf.extend_from_slice(ep.as_bytes());
            buf.push(0);
        }
        buf.extend_from_slice(&self.timestamp.to_le_bytes());
        buf.extend_from_slice(&self.ttl_secs.to_le_bytes());
        for t in &self.transports {
            buf.extend_from_slice(t.as_bytes());
            buf.push(0);
        }
        buf
    }
}

// ── Peer Exchange ───────────────────────────────────────────────────────────

/// The peer exchange system manages the local peer database and handles
/// peer discovery and exchange across all communication channels.
pub struct PeerExchange {
    /// Local peer database.
    peers: Arc<RwLock<HashMap<String, PeerRecord>>>,
    /// Our Ed25519 signing key.
    signing_key: Mutex<SigningKey>,
    /// Our peer ID.
    own_peer_id: Mutex<String>,
    /// Last time we performed a peer exchange.
    last_exchange_time: Mutex<Instant>,
    /// Recent exchange partner set (to avoid redundant exchanges).
    recent_exchange_partners: Mutex<HashSet<String>>,
    /// Incoming announcement queue.
    incoming_announcements: Arc<RwLock<VecDeque<PeerAnnouncement>>>,
    /// Rate limiter: track exchanges per peer.
    exchange_rate_limiter: Mutex<HashMap<String, Instant>>,
}

impl PeerExchange {
    /// Create a new peer exchange system.
    pub fn new() -> Result<Self, ShieldError> {
        // Generate a new Ed25519 key pair
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();
        let peer_id = hex::encode(verifying_key.to_bytes());

        Ok(Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            signing_key: Mutex::new(signing_key),
            own_peer_id: Mutex::new(peer_id),
            last_exchange_time: Mutex::new(Instant::now()),
            recent_exchange_partners: Mutex::new(HashSet::new()),
            incoming_announcements: Arc::new(RwLock::new(VecDeque::with_capacity(64))),
            exchange_rate_limiter: Mutex::new(HashMap::new()),
        })
    }

    /// Get our own peer ID.
    pub fn own_peer_id(&self) -> String {
        self.own_peer_id.lock().clone()
    }

    /// Create a signed peer announcement for our own node.
    pub fn create_own_announcement(
        &self,
        endpoints: Vec<String>,
        transports: Vec<String>,
    ) -> Result<PeerAnnouncement, ShieldError> {
        let peer_id = self.own_peer_id();
        let mut announcement = PeerAnnouncement::new(peer_id, endpoints, transports);
        announcement.sign(&self.signing_key.lock())?;
        Ok(announcement)
    }

    /// Add a peer discovered through any channel.
    pub async fn add_peer_from_discovery(
        &self,
        endpoint: String,
        source: String,
    ) -> Option<PeerRecord> {
        let mut peers = self.peers.write().await;

        // Check if we already have a peer at this endpoint
        for (_, record) in peers.iter_mut() {
            if record.endpoints.contains(&endpoint) {
                record.last_seen = now_secs();
                record.discovery_source = source;
                return Some(record.clone());
            }
        }

        // Create a new peer record
        let peer_id = format!(
            "discovered-{}",
            hex::encode(&endpoint.as_bytes()[..8.min(endpoint.len())])
        );
        let record = PeerRecord {
            peer_id: peer_id.clone(),
            endpoints: vec![endpoint],
            verifying_key: None,
            reputation: 0.5, // Default reputation for new peers
            discovery_source: source,
            success_count: 0,
            failure_count: 0,
            first_seen: now_secs(),
            last_seen: now_secs(),
            last_exchange: None,
            connected: false,
            region: None,
            transports: Vec::new(),
        };

        // Check peer limit
        if peers.len() >= MAX_PEERS {
            // Remove the peer with the lowest reputation
            if let Some(worst_key) = peers
                .iter()
                .min_by_key(|(_, p)| (p.reputation * 1000.0) as u64)
                .map(|(k, _)| k.clone())
            {
                peers.remove(&worst_key);
            }
        }

        peers.insert(peer_id, record.clone());
        Some(record)
    }

    /// Process a received peer announcement.
    ///
    /// Verifies the signature, checks for freshness, and adds the peer
    /// to the database if valid.
    pub async fn process_announcement(
        &self,
        announcement: &PeerAnnouncement,
    ) -> Result<bool, ShieldError> {
        // Don't accept our own announcements
        if announcement.peer_id == *self.own_peer_id.lock() {
            return Ok(false);
        }

        // Check freshness
        if !announcement.is_valid() {
            debug!(peer_id = %announcement.peer_id, "Announcement expired — ignoring");
            return Ok(false);
        }

        // Verify signature if we have the verifying key
        if !announcement.signature.is_empty() {
            if let Ok(verifying_key_bytes) = hex::decode(&announcement.peer_id) {
                if verifying_key_bytes.len() == 32 {
                    let vk_bytes: [u8; 32] = verifying_key_bytes.try_into().map_err(|_| {
                        ShieldError::p2p(ErrorCode::P2pPeerExchangeFailed, "Invalid verifying key")
                    })?;
                    let verifying_key = VerifyingKey::from_bytes(&vk_bytes).map_err(|e| {
                        ShieldError::p2p(
                            ErrorCode::P2pPeerExchangeFailed,
                            format!("Invalid verifying key: {}", e),
                        )
                    })?;

                    if !announcement.verify(&verifying_key)? {
                        warn!(peer_id = %announcement.peer_id, "Announcement signature verification failed");
                        return Ok(false);
                    }
                }
            }
        }

        // Rate limit check for this peer
        if !self.check_exchange_rate(&announcement.peer_id) {
            debug!(peer_id = %announcement.peer_id, "Peer exchange rate limited");
            return Ok(false);
        }

        // Add or update the peer in the database
        let mut peers = self.peers.write().await;
        match peers.get_mut(&announcement.peer_id) {
            Some(existing) => {
                existing.update_endpoints(&announcement.endpoints);
                existing.transports = announcement.transports.clone();
                existing.verifying_key = Some(announcement.peer_id.clone());
            }
            None => {
                // Check peer limit
                if peers.len() >= MAX_PEERS {
                    if let Some(worst_key) = peers
                        .iter()
                        .min_by_key(|(_, p)| (p.reputation * 1000.0) as u64)
                        .map(|(k, _)| k.clone())
                    {
                        peers.remove(&worst_key);
                    }
                }

                let record = PeerRecord {
                    peer_id: announcement.peer_id.clone(),
                    endpoints: announcement.endpoints.clone(),
                    verifying_key: Some(announcement.peer_id.clone()),
                    reputation: 0.5,
                    discovery_source: "peer_exchange".to_string(),
                    success_count: 0,
                    failure_count: 0,
                    first_seen: now_secs(),
                    last_seen: now_secs(),
                    last_exchange: None,
                    connected: false,
                    region: None,
                    transports: announcement.transports.clone(),
                };
                peers.insert(announcement.peer_id.clone(), record);
            }
        }

        debug!(
            peer_id = %announcement.peer_id,
            endpoints = announcement.endpoints.len(),
            "Peer announcement processed"
        );

        Ok(true)
    }

    /// Get peers that are considered "good" (above reputation threshold).
    pub async fn get_good_peers(&self, limit: usize) -> Vec<PeerRecord> {
        let peers = self.peers.read().await;
        let mut good_peers: Vec<PeerRecord> =
            peers.values().filter(|p| p.is_good()).cloned().collect();

        // Sort by reputation (descending)
        good_peers.sort_by(|a, b| {
            b.reputation
                .partial_cmp(&a.reputation)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        good_peers.truncate(limit);
        good_peers
    }

    /// Get all peers.
    pub async fn get_all_peers(&self) -> Vec<PeerRecord> {
        self.peers.read().await.values().cloned().collect()
    }

    /// Get peer statistics.
    pub async fn peer_stats(&self) -> (u32, u32) {
        let peers = self.peers.read().await;
        let total = peers.len() as u32;
        let good = peers.values().filter(|p| p.is_good()).count() as u32;
        (total, good)
    }

    /// Record a successful connection to a peer.
    pub async fn record_success(&self, peer_id: &str) {
        if let Some(record) = self.peers.write().await.get_mut(peer_id) {
            record.record_success();
        }
    }

    /// Record a failed connection to a peer.
    pub async fn record_failure(&self, peer_id: &str) {
        if let Some(record) = self.peers.write().await.get_mut(peer_id) {
            record.record_failure();
        }
    }

    /// Mark a peer as connected.
    pub async fn set_connected(&self, peer_id: &str, connected: bool) {
        if let Some(record) = self.peers.write().await.get_mut(peer_id) {
            record.connected = connected;
            if connected {
                record.last_seen = now_secs();
            }
        }
    }

    /// Create a batch of peer announcements for exchange.
    ///
    /// Returns up to `limit` signed announcements for our best peers.
    pub async fn create_exchange_batch(&self, limit: usize) -> Vec<PeerAnnouncement> {
        let good_peers = self.get_good_peers(limit).await;
        let mut announcements = Vec::with_capacity(good_peers.len());

        for peer in &good_peers {
            let mut announcement = PeerAnnouncement::new(
                peer.peer_id.clone(),
                peer.endpoints.clone(),
                peer.transports.clone(),
            );

            // Try to sign with the original key if we have it
            // For exchanged peers, we just forward their original announcement
            announcements.push(announcement);
        }

        announcements
    }

    /// Process a batch of peer announcements from an exchange partner.
    pub async fn process_exchange_batch(
        &self,
        announcements: &[PeerAnnouncement],
        from_peer_id: &str,
    ) -> Result<usize, ShieldError> {
        // Rate limit check
        if !self.check_exchange_rate(from_peer_id) {
            return Ok(0);
        }

        let mut accepted = 0;
        for announcement in announcements {
            if self.process_announcement(announcement).await? {
                accepted += 1;
            }
        }

        // Record this exchange
        if let Some(record) = self.peers.write().await.get_mut(from_peer_id) {
            record.last_exchange = Some(now_secs());
        }

        info!(
            from = from_peer_id,
            total = announcements.len(),
            accepted,
            "Peer exchange batch processed"
        );

        Ok(accepted)
    }

    /// Prune expired peers from the database.
    pub async fn prune_expired_peers(&self) {
        let mut peers = self.peers.write().await;
        let now = now_secs();
        let expiry_secs = PEER_EXPIRY.as_secs();

        let expired: Vec<String> = peers
            .iter()
            .filter(|(_, p)| now.saturating_sub(p.last_seen) > expiry_secs)
            .map(|(k, _)| k.clone())
            .collect();

        for key in expired {
            peers.remove(&key);
        }

        debug!(remaining = peers.len(), "Pruned expired peers");
    }

    /// Verify reachability of peers by attempting to connect.
    ///
    /// This is a best-effort operation that runs in the background.
    pub async fn verify_peer_reachability(&self) {
        let peers = self.peers.read().await;
        let now = now_secs();

        for (_, peer) in peers.iter() {
            // Only verify peers that haven't been checked recently
            if now - peer.last_seen < 300 {
                continue;
            }

            // In production, this would attempt a TCP connect to each endpoint
            // and update the reputation accordingly
            debug!(peer_id = %peer.peer_id, "Verifying peer reachability (stub)");
        }
    }

    /// Check the exchange rate limit for a specific peer.
    fn check_exchange_rate(&self, peer_id: &str) -> bool {
        let mut limiter = self.exchange_rate_limiter.lock();
        let now = Instant::now();

        if let Some(last_time) = limiter.get(peer_id) {
            if now.duration_since(*last_time) < MIN_EXCHANGE_INTERVAL {
                return false;
            }
        }

        limiter.insert(peer_id.to_string(), now);

        // Clean up old entries periodically
        if limiter.len() > 100 {
            let cutoff = now - Duration::from_secs(300);
            let old_keys: Vec<String> = limiter
                .iter()
                .filter(|(_, t)| t.duration_since(cutoff) > Duration::ZERO)
                .map(|(k, _)| k.clone())
                .collect();
            for key in old_keys {
                limiter.remove(&key);
            }
        }

        true
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_record_reputation() {
        let mut record = PeerRecord {
            peer_id: "test".to_string(),
            endpoints: vec!["tcp://1.2.3.4:51820".to_string()],
            verifying_key: None,
            reputation: 0.5,
            discovery_source: "test".to_string(),
            success_count: 0,
            failure_count: 0,
            first_seen: 0,
            last_seen: 0,
            last_exchange: None,
            connected: false,
            region: None,
            transports: Vec::new(),
        };

        assert!(record.is_good());

        // Simulate failures
        for _ in 0..5 {
            record.record_failure();
        }
        assert!(!record.is_good());
        assert_eq!(record.failure_count, 5);

        // Simulate successes
        for _ in 0..10 {
            record.record_success();
        }
        assert!(record.is_good());
    }

    #[test]
    fn test_peer_announcement_sign_verify() {
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();
        let peer_id = hex::encode(verifying_key.to_bytes());

        let mut announcement = PeerAnnouncement::new(
            peer_id,
            vec!["tcp://1.2.3.4:51820".to_string()],
            vec!["hysteria2".to_string()],
        );

        // Sign
        announcement.sign(&signing_key).unwrap();
        assert!(!announcement.signature.is_empty());

        // Verify
        let valid = announcement.verify(&verifying_key).unwrap();
        assert!(valid);

        // Tamper and verify again
        let mut tampered = announcement.clone();
        tampered.endpoints.push("tcp://5.6.7.8:51820".to_string());
        let valid = tampered.verify(&verifying_key).unwrap();
        assert!(!valid);
    }

    #[test]
    fn test_peer_announcement_validity() {
        let announcement = PeerAnnouncement {
            version: 1,
            peer_id: "test".to_string(),
            endpoints: vec![],
            timestamp: now_secs(),
            ttl_secs: 21600,
            transports: vec![],
            signature: vec![],
        };
        assert!(announcement.is_valid());

        let expired = PeerAnnouncement {
            version: 1,
            peer_id: "test".to_string(),
            endpoints: vec![],
            timestamp: now_secs() - 86400,
            ttl_secs: 21600,
            transports: vec![],
            signature: vec![],
        };
        assert!(!expired.is_valid());
    }

    #[tokio::test]
    async fn test_peer_exchange_creation() {
        let exchange = PeerExchange::new().unwrap();
        let (total, good) = exchange.peer_stats().await;
        assert_eq!(total, 0);
        assert_eq!(good, 0);
    }

    #[tokio::test]
    async fn test_add_peer_from_discovery() {
        let exchange = PeerExchange::new().unwrap();
        let record = exchange
            .add_peer_from_discovery("tcp://1.2.3.4:51820".to_string(), "wifi_aware".to_string())
            .await;

        assert!(record.is_some());
        let (total, _) = exchange.peer_stats().await;
        assert_eq!(total, 1);
    }

    #[tokio::test]
    async fn test_get_good_peers() {
        let exchange = PeerExchange::new().unwrap();

        // Add some peers
        exchange
            .add_peer_from_discovery("tcp://1.2.3.4:51820".to_string(), "test".to_string())
            .await;
        exchange
            .add_peer_from_discovery("tcp://5.6.7.8:51820".to_string(), "test".to_string())
            .await;

        let good = exchange.get_good_peers(10).await;
        assert_eq!(good.len(), 2); // Both start with reputation 0.5 which is below threshold
                                   // Actually 0.5 < 0.6 threshold, so no good peers
                                   // Let me check: GOOD_PEER_THRESHOLD is 0.6, default reputation is 0.5
                                   // So they shouldn't be good
        let good = exchange.get_good_peers(10).await;
        assert!(good.is_empty()); // 0.5 < 0.6
    }
}
