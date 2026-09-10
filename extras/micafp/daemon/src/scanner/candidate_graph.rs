//! Candidate Graph and Lifecycle Management for Autonomous Anti-Censorship Scanner
//!
//! Maintains a dynamic graph of connection candidates (IPs, domains, ports, transports)
//! with provenance tracking, reputation decay, independent corroboration,
//! and state transitions (Bootstrap, Discovered, Probing, Verified, Quarantined, Expired, Rehabilitating).

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Source / Provenance of a candidate
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandidateSource {
    /// Built-in bootstrap seed (initial bootstrap only, never permanently dominates)
    BootstrapSeed,
    /// Discovered via clean DNS resolver observation
    DnsObservation,
    /// Discovered from TLS certificate SAN / ALPN inspection
    CertificateInspection,
    /// Dynamic heuristic subnet exploration
    HeuristicSubnetExpansion,
    /// Dynamic port discovery
    DynamicPortSweep,
    /// Learned from previous verified successful connection
    LearnedSuccess,
    /// Discovered from domestic CDN edge mapping (Arvan, SabaIdea, etc.)
    DomesticCdnEdge,
    /// P2P / Authenticated Peer Exchange
    PeerExchange,
    /// AI-generated exploration hypothesis (must pass deterministic verification)
    AiHypothesis,
    /// Test fixture (strictly marked, never promoted to live without verification)
    TestFixture,
}

/// Lifecycle state of a candidate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CandidateState {
    /// Initial bootstrap seed
    #[default]
    Bootstrap,
    /// Newly discovered, awaiting probing
    Discovered,
    /// Currently actively being probed
    Probing,
    /// Verified reachable and protocol-validated with active probe evidence
    Verified,
    /// Temporarily quarantined due to repeated failures or DPI resets
    Quarantined,
    /// In rehabilitation period after quarantine cooldown
    Rehabilitating,
    /// Expired due to age or sustained unreachability
    Expired,
}

/// Protocol classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ProtocolClassification {
    #[default]
    Unknown,
    VlessReality,
    VlessVision,
    VmessWs,
    TrojanTls,
    ShadowTlsV3,
    TuicV5,
    Hysteria2,
    Shadowsocks2022,
    GrpcMulti,
    WsTls,
    Http3Quic,
    DoH,
    DoT,
    DoQ,
    DomesticReverseRelay,
}

/// Fine-grained probe outcome classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeOutcome {
    Success {
        latency_ms: u64,
        tcp_handshake_ms: u64,
        tls_negotiate_ms: u64,
    },
    DpiTcpReset {
        reset_delay_ms: u64,
    },
    TlsHandshakeRejected {
        reason: String,
    },
    ConnectionTimeout {
        timeout_ms: u64,
    },
    ConnectionRefused,
    DnsPoisoned {
        bogus_ip: String,
    },
    QuicUdpBlocked,
    ProtocolMismatch {
        expected: String,
        got: String,
    },
    NetworkUnreachable,
    DomesticOnlyCutoff,
}

/// Individual Candidate Node in the Graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateNode {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub transport_family: String,
    pub protocol: ProtocolClassification,
    pub source: CandidateSource,
    pub state: CandidateState,
    pub discovery_time: DateTime<Utc>,
    pub last_verified_time: Option<DateTime<Utc>>,
    pub last_probe_time: Option<DateTime<Utc>>,
    pub last_outcome: Option<ProbeOutcome>,
    pub success_count: u32,
    pub failure_count: u32,
    pub consecutive_failures: u32,
    pub recent_latency_ms: Option<u64>,
    pub jitter_ms: Option<u64>,
    pub packet_loss_rate: f32,
    pub tcp_handshake_ms: Option<u64>,
    pub tls_negotiate_ms: Option<u64>,
    pub tls_alpn: Vec<String>,
    pub utls_fingerprint: String,
    pub quic_capable: bool,
    pub mux_supported: bool,
    pub sni_candidate: String,
    pub asn: Option<u32>,
    pub operator_affinity: Option<String>,
    pub failure_domain: String,
    pub reputation_score: f32, // 0.0 to 100.0
    pub confidence_score: u8,  // 0 to 100
    pub cooldown_until: Option<DateTime<Utc>>,
}

impl CandidateNode {
    pub fn new(
        host: &str,
        port: u16,
        transport: &str,
        protocol: ProtocolClassification,
        source: CandidateSource,
    ) -> Self {
        let id = format!("{}:{}:{}", host, port, transport);
        let failure_domain = format!(
            "{}:{}",
            host.split('.').take(2).collect::<Vec<_>>().join("."),
            port
        );
        Self {
            id,
            host: host.to_string(),
            port,
            transport_family: transport.to_string(),
            protocol,
            source,
            state: CandidateState::Discovered,
            discovery_time: Utc::now(),
            last_verified_time: None,
            last_probe_time: None,
            last_outcome: None,
            success_count: 0,
            failure_count: 0,
            consecutive_failures: 0,
            recent_latency_ms: None,
            jitter_ms: None,
            packet_loss_rate: 0.0,
            tcp_handshake_ms: None,
            tls_negotiate_ms: None,
            tls_alpn: vec!["h2".to_string(), "http/1.1".to_string()],
            utls_fingerprint: "chrome_124".to_string(),
            quic_capable: protocol == ProtocolClassification::Hysteria2
                || protocol == ProtocolClassification::TuicV5
                || protocol == ProtocolClassification::DoQ,
            mux_supported: true,
            sni_candidate: "www.speedtest.net".to_string(),
            asn: None,
            operator_affinity: None,
            failure_domain,
            reputation_score: 50.0,
            confidence_score: 50,
            cooldown_until: None,
        }
    }

    /// Calculate Composite Quality Score (0.0 to 100.0)
    pub fn calculate_quality_score(&self) -> f32 {
        if self.state == CandidateState::Quarantined || self.state == CandidateState::Expired {
            return 0.0;
        }

        let mut score = self.reputation_score;

        // Latency penalty
        if let Some(lat) = self.recent_latency_ms {
            if lat < 60 {
                score += 15.0;
            } else if lat < 120 {
                score += 8.0;
            } else if lat > 300 {
                score -= 15.0;
            }
        }

        // Handshake responsiveness
        if let (Some(tcp), Some(tls)) = (self.tcp_handshake_ms, self.tls_negotiate_ms) {
            if tcp + tls < 100 {
                score += 10.0;
            }
        }

        // Loss penalty
        score -= self.packet_loss_rate * 30.0;

        // Freshness bonus
        if let Some(last_v) = self.last_verified_time {
            let age_secs = (Utc::now() - last_v).num_seconds().max(0);
            if age_secs < 60 {
                score += 10.0;
            } else if age_secs > 600 {
                score -= 10.0;
            }
        }

        // Source weight
        match self.source {
            CandidateSource::LearnedSuccess => score += 10.0,
            CandidateSource::DomesticCdnEdge => score += 8.0,
            CandidateSource::CertificateInspection => score += 5.0,
            CandidateSource::BootstrapSeed => score += 2.0,
            CandidateSource::AiHypothesis => score -= 2.0, // AI hypotheses require active proof
            CandidateSource::TestFixture => score = score.min(10.0),
            _ => {}
        }

        score.clamp(0.0, 100.0)
    }
}

/// Dynamic Candidate Graph with thread-safe operations, bounded capacity, and decay
pub struct CandidateGraph {
    nodes: Arc<RwLock<HashMap<String, CandidateNode>>>,
    max_capacity: usize,
}

impl CandidateGraph {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::with_capacity(max_capacity))),
            max_capacity,
        }
    }

    /// Add a candidate if not present, or merge metadata if exists
    pub fn add_or_merge(&self, node: CandidateNode) {
        let mut map = self.nodes.write();

        if let Some(existing) = map.get_mut(&node.id) {
            // Merge metadata without overwriting active probe stats
            if node.operator_affinity.is_some() && existing.operator_affinity.is_none() {
                existing.operator_affinity = node.operator_affinity;
            }
            if node.asn.is_some() && existing.asn.is_none() {
                existing.asn = node.asn;
            }
        } else {
            // Check capacity: if full, evict the lowest reputation expired or quarantined node
            if map.len() >= self.max_capacity {
                let to_remove = map
                    .iter()
                    .min_by(|a, b| {
                        a.1.reputation_score
                            .partial_cmp(&b.1.reputation_score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(k, _)| k.clone());

                if let Some(k) = to_remove {
                    map.remove(&k);
                }
            }
            map.insert(node.id.clone(), node);
        }
    }

    /// Record a verified successful probe on a candidate
    pub fn record_success(
        &self,
        id: &str,
        latency_ms: u64,
        tcp_handshake_ms: u64,
        tls_negotiate_ms: u64,
    ) {
        let mut map = self.nodes.write();
        if let Some(node) = map.get_mut(id) {
            node.state = CandidateState::Verified;
            node.last_verified_time = Some(Utc::now());
            node.last_probe_time = Some(Utc::now());
            node.success_count += 1;
            node.consecutive_failures = 0;
            node.recent_latency_ms = Some(latency_ms);
            node.tcp_handshake_ms = Some(tcp_handshake_ms);
            node.tls_negotiate_ms = Some(tls_negotiate_ms);
            node.packet_loss_rate = (node.packet_loss_rate * 0.8).max(0.0);
            node.reputation_score = (node.reputation_score + 12.0).min(100.0);
            node.confidence_score = (node.confidence_score + 10).min(99);
            node.last_outcome = Some(ProbeOutcome::Success {
                latency_ms,
                tcp_handshake_ms,
                tls_negotiate_ms,
            });
            debug!(
                "Candidate {} verified successfully (latency: {}ms)",
                id, latency_ms
            );
        }
    }

    /// Record a failed probe on a candidate with fine-grained outcome
    pub fn record_failure(&self, id: &str, outcome: ProbeOutcome) {
        let mut map = self.nodes.write();
        if let Some(node) = map.get_mut(id) {
            node.last_probe_time = Some(Utc::now());
            node.failure_count += 1;
            node.consecutive_failures += 1;
            node.packet_loss_rate = (node.packet_loss_rate * 0.7 + 0.3).min(1.0);
            node.reputation_score = (node.reputation_score - 15.0).max(0.0);
            node.confidence_score = node.confidence_score.saturating_sub(15);
            node.last_outcome = Some(outcome.clone());

            // Quarantine after 3 consecutive failures
            if node.consecutive_failures >= 3 {
                node.state = CandidateState::Quarantined;
                // Exponential backoff cooldown: 30s * 2^(failures - 3)
                let backoff_secs = 30 * (1 << (node.consecutive_failures - 3).min(6));
                node.cooldown_until = Some(Utc::now() + chrono::Duration::seconds(backoff_secs));
                warn!(
                    "Candidate {} quarantined for {}s due to consecutive failures",
                    id, backoff_secs
                );
            }
        }
    }

    /// Retrieve candidates ready for exploration/probing
    pub fn get_exploration_batch(&self, count: usize) -> Vec<CandidateNode> {
        let map = self.nodes.read();
        let now = Utc::now();

        let mut candidates: Vec<CandidateNode> = map
            .values()
            .filter(|n| {
                match n.state {
                    CandidateState::Discovered | CandidateState::Bootstrap => true,
                    CandidateState::Verified => {
                        // Re-verify if older than 180s
                        n.last_verified_time
                            .map_or(true, |t| (now - t).num_seconds() > 180)
                    }
                    CandidateState::Quarantined => {
                        // Rehabilitate if cooldown elapsed
                        n.cooldown_until.map_or(false, |cd| now > cd)
                    }
                    CandidateState::Rehabilitating => true,
                    CandidateState::Expired | CandidateState::Probing => false,
                }
            })
            .cloned()
            .collect();

        // Sort by exploration priority
        candidates.sort_by(|a, b| {
            b.calculate_quality_score()
                .partial_cmp(&a.calculate_quality_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(count);
        candidates
    }

    /// Retrieve top-ranked verified nodes for connection binding
    pub fn get_top_verified(&self, limit: usize) -> Vec<CandidateNode> {
        let map = self.nodes.read();
        let mut verified: Vec<CandidateNode> = map
            .values()
            .filter(|n| n.state == CandidateState::Verified && n.reputation_score > 40.0)
            .cloned()
            .collect();

        verified.sort_by(|a, b| {
            b.calculate_quality_score()
                .partial_cmp(&a.calculate_quality_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        verified.truncate(limit);
        verified
    }

    /// Apply reputation decay and purge expired candidates
    pub fn decay_and_prune(&self) {
        let mut map = self.nodes.write();
        let now = Utc::now();
        let mut expired_keys = Vec::new();

        for (k, node) in map.iter_mut() {
            // Apply decay: 2% reputation loss per cycle for unprobed nodes
            node.reputation_score = (node.reputation_score * 0.98).max(0.0);

            // Mark expired if quarantined for > 2 hours or discovered with no success for > 1 hour
            let age_secs = (now - node.discovery_time).num_seconds();
            if node.state == CandidateState::Quarantined
                && node.consecutive_failures > 8
                && age_secs > 7200
            {
                expired_keys.push(k.clone());
            } else if node.state == CandidateState::Discovered
                && node.success_count == 0
                && age_secs > 3600
            {
                expired_keys.push(k.clone());
            }
        }

        for k in expired_keys {
            map.remove(&k);
        }
    }

    /// Dynamic Mutation & Branching: derive sibling candidates from verified nodes
    pub fn derive_mutations(&self) -> Vec<CandidateNode> {
        let map = self.nodes.read();
        let mut mutations = Vec::new();

        for node in map.values() {
            if node.state == CandidateState::Verified && node.reputation_score >= 80.0 {
                // Sibling port mutation
                let alternate_ports = match node.port {
                    443 => vec![8443, 2053, 2083, 2087, 2096],
                    8443 => vec![443, 9443],
                    _ => vec![443, 8443],
                };

                for alt_port in alternate_ports {
                    if alt_port != node.port {
                        let mut mut_node = CandidateNode::new(
                            &node.host,
                            alt_port,
                            &node.transport_family,
                            node.protocol,
                            CandidateSource::DynamicPortSweep,
                        );
                        mut_node.operator_affinity = node.operator_affinity.clone();
                        mut_node.asn = node.asn;
                        mutations.push(mut_node);
                    }
                }
            }
        }

        mutations
    }

    /// Total count of candidates
    pub fn count(&self) -> usize {
        self.nodes.read().len()
    }

    /// Count of verified candidates
    pub fn verified_count(&self) -> usize {
        self.nodes
            .read()
            .values()
            .filter(|n| n.state == CandidateState::Verified)
            .count()
    }
}
