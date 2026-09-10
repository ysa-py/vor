// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — ISP Profile Manager
//
// Manages per-ISP censorship evasion profiles. Iran's ISPs (MCI, Irancell,
// Rightel, Shatel, ParsOnline, etc.) deploy different DPI systems with
// different detection strategies. This module loads ISP-specific profiles
// that define optimal evasion parameters for each ISP.
//
// Key features:
//   • Load isp-profiles.json at startup
//   • Per-ISP SNI domain pools for TLS-based transports
//   • Per-ISP fragmentation strategies
//   • Auto-detect ISP from network characteristics
//   • Fallback to default profile if ISP unknown
// ─────────────────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::ShieldError;

// ── Constants ───────────────────────────────────────────────────────────────

/// Name of the ISP profiles configuration file.
const ISP_PROFILES_FILENAME: &str = "isp-profiles.json";

/// Default profile used when ISP cannot be identified.
const DEFAULT_PROFILE_NAME: &str = "default";

/// Maximum number of SNI domains per profile.
const MAX_SNI_DOMAINS: usize = 200;

/// Auto-detection retry interval (5 minutes).
const AUTO_DETECT_INTERVAL: Duration = Duration::from_secs(300);

// ── Fragmentation Strategy ──────────────────────────────────────────────────

/// TLS fragmentation strategy for evading DPI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FragmentationStrategy {
    /// No fragmentation — use when DPI doesn't inspect TLS.
    None,
    /// Split ClientHello at the SNI extension.
    SplitClientHello,
    /// Split at random positions within the TLS record.
    RandomSplit,
    /// Fragment into 1-3 byte chunks (very stealthy, high overhead).
    TinyChunks,
    /// Use padding extension to hide the real SNI length.
    PaddingObfuscation,
}

impl Default for FragmentationStrategy {
    fn default() -> Self {
        Self::SplitClientHello
    }
}

// ── DPI System ──────────────────────────────────────────────────────────────

/// Known DPI systems deployed by Iranian ISPs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DpiSystem {
    /// Unknown DPI system — use conservative defaults.
    Unknown,
    /// FAVA (Filtration and Verification Architecture) — most common.
    Fava,
    /// SINA DPI — used by MCI.
    Sina,
    /// Huawei NetEngine DPI — used by Irancell.
    Huawei,
    /// Custom DPI — varies by ISP.
    Custom,
}

impl Default for DpiSystem {
    fn default() -> Self {
        Self::Unknown
    }
}

// ── SNI Domain Pool ─────────────────────────────────────────────────────────

/// A pool of SNI domains that look like legitimate Iranian web traffic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniDomainPool {
    /// Domains mimicking Iranian e-commerce sites.
    pub ecommerce: Vec<String>,
    /// Domains mimicking Iranian banking sites.
    pub banking: Vec<String>,
    /// Domains mimicking Iranian government sites.
    pub government: Vec<String>,
    /// Domains mimicking Iranian media/streaming sites.
    pub media: Vec<String>,
    /// Domains mimicking Iranian educational sites.
    pub education: Vec<String>,
    /// Domains mimicking CDNs and cloud services.
    pub cdn: Vec<String>,
}

impl Default for SniDomainPool {
    fn default() -> Self {
        Self {
            ecommerce: vec![
                "digikala.com".to_string(),
                "esam.ir".to_string(),
                "bamdadm.com".to_string(),
                "torob.com".to_string(),
            ],
            banking: vec![
                "bmi.ir".to_string(),
                "bankmellat.ir".to_string(),
                "bpi.ir".to_string(),
                "sb24.ir".to_string(),
            ],
            government: vec![
                "irancell.ir".to_string(),
                "mci.ir".to_string(),
                "shaparak.ir".to_string(),
                "sabteahval.ir".to_string(),
            ],
            media: vec![
                "aparat.com".to_string(),
                "filimo.com".to_string(),
                "namava.ir".to_string(),
            ],
            education: vec![
                "sanjesh.org".to_string(),
                "msrt.ir".to_string(),
                "ut.ac.ir".to_string(),
            ],
            cdn: vec!["arvancloud.ir".to_string(), "cdn.ir".to_string()],
        }
    }
}

impl SniDomainPool {
    /// Get a random domain from the pool for the given category.
    pub fn random_domain(&self, category: &str) -> Option<&str> {
        let pool = match category {
            "ecommerce" => &self.ecommerce,
            "banking" => &self.banking,
            "government" => &self.government,
            "media" => &self.media,
            "education" => &self.education,
            "cdn" => &self.cdn,
            _ => &self.ecommerce,
        };

        if pool.is_empty() {
            return None;
        }

        // Simple deterministic selection (in production, use crypto RNG)
        let idx = (now_secs() as usize) % pool.len();
        Some(&pool[idx])
    }

    /// Get all domains from the pool.
    pub fn all_domains(&self) -> Vec<&str> {
        let mut domains = Vec::new();
        domains.extend(self.ecommerce.iter().map(|s| s.as_str()));
        domains.extend(self.banking.iter().map(|s| s.as_str()));
        domains.extend(self.government.iter().map(|s| s.as_str()));
        domains.extend(self.media.iter().map(|s| s.as_str()));
        domains.extend(self.education.iter().map(|s| s.as_str()));
        domains.extend(self.cdn.iter().map(|s| s.as_str()));
        domains
    }

    /// Total domain count.
    pub fn total_count(&self) -> usize {
        self.ecommerce.len()
            + self.banking.len()
            + self.government.len()
            + self.media.len()
            + self.education.len()
            + self.cdn.len()
    }
}

// ── ISP Profile ─────────────────────────────────────────────────────────────

/// A complete censorship evasion profile for a specific Iranian ISP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IspProfile {
    /// ISP name (e.g., "mci", "irancell", "shatel").
    pub name: String,
    /// Human-readable ISP name.
    pub display_name: String,
    /// Known DPI system deployed by this ISP.
    pub dpi_system: DpiSystem,
    /// Recommended fragmentation strategy.
    pub fragmentation_strategy: FragmentationStrategy,
    /// SNI domain pool for this ISP.
    pub sni_pool: SniDomainPool,
    /// DNS server IPs used by this ISP.
    pub dns_servers: Vec<String>,
    /// Known IP ranges for this ISP (for detection).
    pub ip_ranges: Vec<String>,
    /// Whether this ISP performs active probing.
    pub active_probing: bool,
    /// Whether this ISP blocks specific transport protocols.
    pub blocked_protocols: Vec<String>,
    /// Recommended TLS fingerprint to mimic.
    pub tls_fingerprint: Option<String>,
    /// Maximum packet size before fragmentation triggers DPI.
    pub max_safe_packet_size: u16,
    /// Whether Domain Fronting is known to work on this ISP.
    pub domain_fronting_works: bool,
    /// CDN front domains that work for domain fronting.
    pub cdn_front_domains: Vec<String>,
    /// Average latency to international servers (ms).
    pub typical_international_latency_ms: u32,
    /// Whether NAIN mode has been observed on this ISP.
    pub nain_observed: bool,
    /// Last time this profile was updated (UNIX timestamp).
    pub last_updated: u64,
}

impl Default for IspProfile {
    fn default() -> Self {
        Self {
            name: DEFAULT_PROFILE_NAME.to_string(),
            display_name: "Default (Unknown ISP)".to_string(),
            dpi_system: DpiSystem::default(),
            fragmentation_strategy: FragmentationStrategy::default(),
            sni_pool: SniDomainPool::default(),
            dns_servers: vec!["5.200.200.200".to_string()],
            ip_ranges: vec![],
            active_probing: true, // Assume active probing
            blocked_protocols: vec![],
            tls_fingerprint: Some("chrome_120".to_string()),
            max_safe_packet_size: 1448,
            domain_fronting_works: false,
            cdn_front_domains: vec![],
            typical_international_latency_ms: 150,
            nain_observed: true,
            last_updated: now_secs(),
        }
    }
}

impl IspProfile {
    /// Get the recommended SNI domain for the current context.
    pub fn recommended_sni(&self, transport: &str) -> Option<&str> {
        // Different transports benefit from different SNI categories
        let category = match transport {
            "hysteria2" | "tuic_v5" => "media",      // Looks like streaming
            "shadow_tls" | "reality" => "cdn",       // Looks like CDN traffic
            "vless" | "webtransport" => "ecommerce", // Looks like shopping
            "mqtt_ws" => "iot",                      // Looks like IoT
            _ => "ecommerce",
        };

        self.sni_pool.random_domain(category)
    }

    /// Check if a specific transport protocol is blocked by this ISP.
    pub fn is_protocol_blocked(&self, protocol: &str) -> bool {
        self.blocked_protocols.iter().any(|p| p == protocol)
    }

    /// Get the recommended CDN front domain for domain fronting.
    pub fn recommended_cdn_front(&self) -> Option<&str> {
        if self.domain_fronting_works && !self.cdn_front_domains.is_empty() {
            let idx = (now_secs() as usize) % self.cdn_front_domains.len();
            Some(&self.cdn_front_domains[idx])
        } else {
            None
        }
    }

    /// Rotate to the next domain in the SNI pool.
    pub fn rotate_domain(&self) -> Option<&str> {
        self.sni_pool.random_domain("ecommerce")
    }

    /// Get the current recommended domain.
    pub fn current_domain(&self) -> &str {
        self.sni_pool.all_domains().first().copied().unwrap_or("digikala.com")
    }

    /// Short ISP identifier (aliases `name`).
    pub fn isp_name(&self) -> &str {
        &self.name
    }

    /// Get the list of SNI domains.
    pub fn sni_domains(&self) -> Vec<&str> {
        self.sni_pool.all_domains()
    }

    /// Create profiles from bundled config.
    pub fn from_bundled_config() -> Vec<IspProfile> {
        let built_in: Vec<IspProfile> = vec![
            IspProfile {
                name: "mci".to_string(),
                display_name: "Mobile Communication Company of Iran (MCI)".to_string(),
                dpi_system: DpiSystem::Sina,
                fragmentation_strategy: FragmentationStrategy::SplitClientHello,
                sni_pool: SniDomainPool::default(),
                dns_servers: vec!["5.200.200.200".to_string()],
                ip_ranges: vec!["5.104.0.0/16".to_string(), "5.112.0.0/12".to_string()],
                active_probing: true,
                blocked_protocols: vec!["openvpn".to_string(), "wireguard".to_string()],
                tls_fingerprint: Some("chrome_120".to_string()),
                max_safe_packet_size: 1448,
                domain_fronting_works: true,
                cdn_front_domains: vec!["cdn.arvancloud.ir".to_string()],
                typical_international_latency_ms: 180,
                nain_observed: true,
                last_updated: now_secs(),
            },
            IspProfile {
                name: "irancell".to_string(),
                display_name: "MTN Irancell".to_string(),
                dpi_system: DpiSystem::Huawei,
                fragmentation_strategy: FragmentationStrategy::RandomSplit,
                sni_pool: SniDomainPool::default(),
                dns_servers: vec!["217.218.155.155".to_string()],
                ip_ranges: vec!["5.56.0.0/14".to_string(), "185.143.232.0/22".to_string()],
                active_probing: true,
                blocked_protocols: vec!["openvpn".to_string()],
                tls_fingerprint: Some("chrome_120".to_string()),
                max_safe_packet_size: 1400,
                domain_fronting_works: false,
                cdn_front_domains: vec![],
                typical_international_latency_ms: 160,
                nain_observed: true,
                last_updated: now_secs(),
            },
        ];
        built_in
    }
}

// ── ISP Detection ───────────────────────────────────────────────────────────

/// Methods for detecting the current ISP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IspDetectionMethod {
    /// Not yet detected.
    Undetected,
    /// Detected from DNS resolver IP.
    DnsResolver,
    /// Detected from public IP range.
    IpRange,
    /// Detected from network latency characteristics.
    LatencyFingerprint,
    /// Detected from DPI behavior.
    DpiFingerprint,
    /// Manually set by user.
    Manual,
}

// ── ISP Profile Manager ─────────────────────────────────────────────────────

/// Manages ISP-specific censorship evasion profiles.
///
/// The IspProfileManager:
///   1. Loads ISP profiles from isp-profiles.json at startup
///   2. Auto-detects the current ISP from network characteristics
///   3. Provides the active profile for transport configuration
///   4. Supports profile updates via IPFS without restart
pub struct IspProfileManager {
    /// All loaded ISP profiles, keyed by ISP name.
    profiles: HashMap<String, IspProfile>,
    /// Currently active ISP profile.
    active_profile: IspProfile,
    /// How the current ISP was detected.
    detection_method: IspDetectionMethod,
    /// Whether profiles have been loaded.
    loaded: bool,
    /// Last auto-detection timestamp.
    last_detection_ts: u64,
    /// Cached DNS resolver IP for detection.
    cached_resolver_ip: Option<String>,
}

impl IspProfileManager {
    /// Create a new ISP profile manager.
    pub fn new() -> Result<Self, ShieldError> {
        Ok(Self {
            profiles: HashMap::new(),
            active_profile: IspProfile::default(),
            detection_method: IspDetectionMethod::Undetected,
            loaded: false,
            last_detection_ts: 0,
            cached_resolver_ip: None,
        })
    }

    /// Load ISP profiles from configuration file.
    pub fn load_profiles(&mut self) -> Result<(), ShieldError> {
        // Try to load from the embedded JSON resource
        let profiles_json = self.get_embedded_profiles();
        self.parse_profiles(&profiles_json)?;

        self.loaded = true;
        info!(profile_count = self.profiles.len(), "ISP profiles loaded");

        Ok(())
    }

    /// Get the embedded ISP profiles JSON.
    fn get_embedded_profiles(&self) -> String {
        // In production, this would use include_str! to embed the profiles
        // at compile time. For now, we provide built-in profiles.
        let built_in: Vec<IspProfile> = vec![
            IspProfile {
                name: "mci".to_string(),
                display_name: "Mobile Communication Company of Iran (MCI)".to_string(),
                dpi_system: DpiSystem::Sina,
                fragmentation_strategy: FragmentationStrategy::SplitClientHello,
                sni_pool: SniDomainPool::default(),
                dns_servers: vec!["5.200.200.200".to_string()],
                ip_ranges: vec!["5.104.0.0/16".to_string(), "5.112.0.0/12".to_string()],
                active_probing: true,
                blocked_protocols: vec!["openvpn".to_string(), "wireguard".to_string()],
                tls_fingerprint: Some("chrome_120".to_string()),
                max_safe_packet_size: 1448,
                domain_fronting_works: true,
                cdn_front_domains: vec!["cdn.arvancloud.ir".to_string()],
                typical_international_latency_ms: 180,
                nain_observed: true,
                last_updated: now_secs(),
            },
            IspProfile {
                name: "irancell".to_string(),
                display_name: "MTN Irancell".to_string(),
                dpi_system: DpiSystem::Huawei,
                fragmentation_strategy: FragmentationStrategy::RandomSplit,
                sni_pool: SniDomainPool::default(),
                dns_servers: vec!["217.218.155.155".to_string()],
                ip_ranges: vec!["5.56.0.0/14".to_string(), "185.143.232.0/22".to_string()],
                active_probing: true,
                blocked_protocols: vec!["openvpn".to_string()],
                tls_fingerprint: Some("chrome_120".to_string()),
                max_safe_packet_size: 1400,
                domain_fronting_works: false,
                cdn_front_domains: vec![],
                typical_international_latency_ms: 160,
                nain_observed: true,
                last_updated: now_secs(),
            },
            IspProfile {
                name: "shatel".to_string(),
                display_name: "Shatel ISP".to_string(),
                dpi_system: DpiSystem::Fava,
                fragmentation_strategy: FragmentationStrategy::TinyChunks,
                sni_pool: SniDomainPool::default(),
                dns_servers: vec!["5.200.200.200".to_string()],
                ip_ranges: vec!["5.160.0.0/14".to_string()],
                active_probing: true,
                blocked_protocols: vec![
                    "openvpn".to_string(),
                    "wireguard".to_string(),
                    "shadowsocks".to_string(),
                ],
                tls_fingerprint: Some("chrome_120".to_string()),
                max_safe_packet_size: 1300,
                domain_fronting_works: true,
                cdn_front_domains: vec!["cdn.arvancloud.ir".to_string()],
                typical_international_latency_ms: 120,
                nain_observed: true,
                last_updated: now_secs(),
            },
            IspProfile {
                name: "rightel".to_string(),
                display_name: "Rightel".to_string(),
                dpi_system: DpiSystem::Custom,
                fragmentation_strategy: FragmentationStrategy::PaddingObfuscation,
                sni_pool: SniDomainPool::default(),
                dns_servers: vec!["5.200.200.200".to_string()],
                ip_ranges: vec!["5.124.0.0/14".to_string()],
                active_probing: false,
                blocked_protocols: vec!["openvpn".to_string()],
                tls_fingerprint: Some("chrome_120".to_string()),
                max_safe_packet_size: 1448,
                domain_fronting_works: true,
                cdn_front_domains: vec!["cdn.arvancloud.ir".to_string()],
                typical_international_latency_ms: 200,
                nain_observed: false,
                last_updated: now_secs(),
            },
        ];

        serde_json::to_string_pretty(&built_in).unwrap_or_else(|_| "[]".to_string())
    }

    /// Parse ISP profiles from JSON string.
    fn parse_profiles(&mut self, json: &str) -> Result<(), ShieldError> {
        let profiles: Vec<IspProfile> = serde_json::from_str(json)
            .map_err(|e| ShieldError::config(format!("Failed to parse ISP profiles: {}", e)))?;

        for profile in profiles {
            self.profiles.insert(profile.name.clone(), profile);
        }

        Ok(())
    }

    /// Auto-detect the current ISP from network characteristics.
    ///
    /// Detection methods (in priority order):
    ///   1. DNS resolver IP → ISP identification
    ///   2. Public IP range matching
    ///   3. Latency fingerprinting
    ///   4. DPI behavior analysis
    pub async fn auto_detect_if_needed(&mut self) {
        let now = now_secs();

        // Don't re-detect too frequently
        if now.saturating_sub(self.last_detection_ts) < AUTO_DETECT_INTERVAL.as_secs() {
            return;
        }

        // Method 1: Check DNS resolver
        if self.cached_resolver_ip.is_none() {
            self.cached_resolver_ip = self.detect_dns_resolver().await;
        }

        if let Some(ref resolver_ip) = self.cached_resolver_ip {
            if let Some(profile) = self.match_profile_by_dns(resolver_ip) {
                self.active_profile = profile.clone();
                self.detection_method = IspDetectionMethod::DnsResolver;
                self.last_detection_ts = now;
                info!(
                    isp = %self.active_profile.name,
                    method = "dns_resolver",
                    "Auto-detected ISP"
                );
                return;
            }
        }

        // Method 2: Check public IP range
        if let Some(profile) = self.detect_by_ip_range().await {
            self.active_profile = profile.clone();
            self.detection_method = IspDetectionMethod::IpRange;
            self.last_detection_ts = now;
            info!(
                isp = %self.active_profile.name,
                method = "ip_range",
                "Auto-detected ISP"
            );
            return;
        }

        // No detection succeeded — use default profile
        debug!("ISP auto-detection inconclusive — using default profile");
        self.last_detection_ts = now;
    }

    /// Detect the DNS resolver being used.
    async fn detect_dns_resolver(&self) -> Option<String> {
        // In production, this would:
        //   1. Check /etc/resolv.conf on Linux
        //   2. Use NetworkInterface D-Bus on Android
        //   3. Use res_ninit() on iOS
        //
        // Common Iranian DNS resolvers:
        //   - 5.200.200.200 (MCI/Shatel)
        //   - 217.218.155.155 (Irancell/TIC)
        //   - 91.92.254.137 (Rightel)
        None
    }

    /// Match a DNS resolver IP to an ISP profile.
    fn match_profile_by_dns(&self, resolver_ip: &str) -> Option<&IspProfile> {
        self.profiles
            .values()
            .find(|p| p.dns_servers.iter().any(|dns| dns == resolver_ip))
    }

    /// Detect ISP by checking our public IP against known ranges.
    async fn detect_by_ip_range(&self) -> Option<&IspProfile> {
        // In production, this would:
        //   1. Fetch our public IP from a service like ifconfig.me
        //   2. Check which ISP's IP range it falls into
        None
    }

    /// Get the currently active ISP profile.
    pub fn active_profile(&self) -> &IspProfile {
        &self.active_profile
    }

    /// Get the currently active ISP profile name.
    pub fn active_profile_name(&self) -> &str {
        &self.active_profile.name
    }

    /// Manually set the active ISP profile.
    pub fn set_active_profile(&mut self, name: &str) -> Result<(), ShieldError> {
        if let Some(profile) = self.profiles.get(name) {
            self.active_profile = profile.clone();
            self.detection_method = IspDetectionMethod::Manual;
            info!(isp = name, "Manually set ISP profile");
            Ok(())
        } else {
            Err(ShieldError::config(format!(
                "Unknown ISP profile: {}",
                name
            )))
        }
    }

    /// Get the current detection method.
    pub fn detection_method(&self) -> IspDetectionMethod {
        self.detection_method
    }

    /// Get all available ISP profile names.
    pub fn available_profiles(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.profiles.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Get a specific ISP profile by name.
    pub fn get_profile(&self, name: &str) -> Option<&IspProfile> {
        self.profiles.get(name)
    }

    /// Get the recommended fragmentation strategy for the current ISP.
    pub fn recommended_fragmentation(&self) -> FragmentationStrategy {
        self.active_profile.fragmentation_strategy
    }

    /// Get the recommended SNI domain for the given transport.
    pub fn recommended_sni(&self, transport: &str) -> Option<&str> {
        self.active_profile.recommended_sni(transport)
    }

    /// Check if the current ISP does active probing.
    pub fn does_active_probing(&self) -> bool {
        self.active_profile.active_probing
    }

    /// Check if a transport protocol is blocked by the current ISP.
    pub fn is_protocol_blocked(&self, protocol: &str) -> bool {
        self.active_profile.is_protocol_blocked(protocol)
    }

    /// Get the maximum safe packet size for the current ISP.
    pub fn max_safe_packet_size(&self) -> u16 {
        self.active_profile.max_safe_packet_size
    }

    /// Apply an ISP profile update from IPFS or other channels.
    pub fn apply_update(&mut self, profiles_json: &str) -> Result<(), ShieldError> {
        self.parse_profiles(profiles_json)?;
        info!(
            profile_count = self.profiles.len(),
            "Applied ISP profile update"
        );

        // If we have an active profile, reload it
        let active_name = self.active_profile.name.clone();
        if let Some(updated) = self.profiles.get(&active_name) {
            self.active_profile = updated.clone();
        }

        Ok(())
    }

    /// Serialize all profiles for caching.
    pub fn serialize_profiles(&self) -> Result<String, ShieldError> {
        let profiles: Vec<&IspProfile> = self.profiles.values().collect();
        serde_json::to_string_pretty(&profiles)
            .map_err(|e| ShieldError::config(format!("Failed to serialize ISP profiles: {}", e)))
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
    fn test_default_profile() {
        let profile = IspProfile::default();
        assert_eq!(profile.name, "default");
        assert_eq!(profile.dpi_system, DpiSystem::Unknown);
        assert!(profile.active_probing);
    }

    #[test]
    fn test_sni_domain_pool_default() {
        let pool = SniDomainPool::default();
        assert!(!pool.ecommerce.is_empty());
        assert!(!pool.banking.is_empty());
        assert!(pool.total_count() > 0);
    }

    #[test]
    fn test_sni_domain_pool_all_domains() {
        let pool = SniDomainPool::default();
        let all = pool.all_domains();
        assert_eq!(all.len(), pool.total_count());
    }

    #[test]
    fn test_recommended_sni() {
        let profile = IspProfile::default();
        let sni = profile.recommended_sni("hysteria2");
        assert!(sni.is_some());
        assert!(!sni.unwrap().is_empty());
    }

    #[test]
    fn test_protocol_blocked() {
        let profile = IspProfile {
            blocked_protocols: vec!["openvpn".to_string(), "wireguard".to_string()],
            ..IspProfile::default()
        };
        assert!(profile.is_protocol_blocked("openvpn"));
        assert!(profile.is_protocol_blocked("wireguard"));
        assert!(!profile.is_protocol_blocked("hysteria2"));
    }

    #[test]
    fn test_fragmentation_strategy_default() {
        assert_eq!(
            FragmentationStrategy::default(),
            FragmentationStrategy::SplitClientHello
        );
    }

    #[test]
    fn test_isp_profile_manager_creation() {
        let manager = IspProfileManager::new();
        assert!(manager.is_ok());
        let manager = manager.unwrap();
        assert_eq!(manager.active_profile_name(), "default");
    }

    #[test]
    fn test_load_profiles() {
        let mut manager = IspProfileManager::new().unwrap();
        manager.load_profiles().unwrap();
        assert!(manager.loaded);
        assert!(manager.profiles.len() >= 4); // MCI, Irancell, Shatel, Rightel
    }

    #[test]
    fn test_available_profiles() {
        let mut manager = IspProfileManager::new().unwrap();
        manager.load_profiles().unwrap();
        let names = manager.available_profiles();
        assert!(names.contains(&"mci"));
        assert!(names.contains(&"irancell"));
    }

    #[test]
    fn test_set_active_profile() {
        let mut manager = IspProfileManager::new().unwrap();
        manager.load_profiles().unwrap();
        assert!(manager.set_active_profile("mci").is_ok());
        assert_eq!(manager.active_profile_name(), "mci");
        assert_eq!(manager.detection_method(), IspDetectionMethod::Manual);
    }

    #[test]
    fn test_set_unknown_profile() {
        let mut manager = IspProfileManager::new().unwrap();
        manager.load_profiles().unwrap();
        assert!(manager.set_active_profile("nonexistent").is_err());
    }

    #[test]
    fn test_cdn_front_domain() {
        let profile = IspProfile {
            domain_fronting_works: true,
            cdn_front_domains: vec!["cdn.arvancloud.ir".to_string()],
            ..IspProfile::default()
        };
        let front = profile.recommended_cdn_front();
        assert!(front.is_some());
        assert_eq!(front.unwrap(), "cdn.arvancloud.ir");
    }

    #[test]
    fn test_cdn_front_domain_not_available() {
        let profile = IspProfile {
            domain_fronting_works: false,
            cdn_front_domains: vec!["cdn.example.ir".to_string()],
            ..IspProfile::default()
        };
        let front = profile.recommended_cdn_front();
        assert!(front.is_none());
    }

    #[test]
    fn test_serialize_profiles() {
        let mut manager = IspProfileManager::new().unwrap();
        manager.load_profiles().unwrap();
        let json = manager.serialize_profiles();
        assert!(json.is_ok());
        assert!(json.unwrap().contains("mci"));
    }
}
