//! Multi-Signal Censorship & DPI Evidence Fusion Engine
//!
//! Evaluates multi-vector probe signals (TCP reset timing, TLS ClientHello correlation,
//! SNI filtering, DNS poison markers, and differential control probes)
//! to classify network failure domains and emit evidence-backed mitigations.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Root-cause classification of observed network anomalies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CensorshipRootCause {
    /// Local device network interface is down or disconnected
    LocalDeviceOffline,
    /// Only this specific target IP/port is unreachable (normal endpoint drop)
    EndpointSpecificFailure,
    /// Transport protocol family is actively fingerprinted and RST'd (e.g. standard TLS, Plain WS)
    TransportFamilyFingerprinted,
    /// SNI host header blocked by middlebox (FAVA / DPI injection)
    SniBlocklistMatch,
    /// DNS poisoning / NXDOMAIN injection on standard port 53
    DnsPoisoningActive,
    /// QUIC / UDP completely blocked or severely throttled by ISP
    QuicUdpThrottling,
    /// ISP-specific deep packet inspection active (e.g. MCI vs MTN Irancell vs MCI vs TCI)
    IspSpecificDpiInterference,
    /// National Information Network (NIN) isolation — international bearer severed, domestic active
    NationalIntranetIsolation,
    /// Total upstream physical severance
    FullPhysicalSeverance,
    /// Clean, unrestricted network path
    CleanUnrestricted,
}

/// Mitigation Strategy recommended by Evidence Fusion
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvasionMitigation {
    pub enable_tls_fragmentation: bool,
    pub tls_fragment_size: u16,
    pub force_utls_fingerprint: String,
    pub recommended_sni: String,
    pub use_doh_or_doq: bool,
    pub switch_to_reality_vless: bool,
    pub switch_to_hysteria2_quic: bool,
    pub fallback_to_domestic_reverse_relay: bool,
    pub halt_international_probes: bool,
}

/// Evidence Signal record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSignal {
    pub tcp_rst_observed: bool,
    pub tcp_rst_delta_ms: Option<u64>,
    pub sni_block_count: usize,
    pub dns_injection_detected: bool,
    pub quic_drop_rate: f32,
    pub domestic_probe_alive: bool,
    pub international_probe_alive: bool,
    pub control_probe_passed: bool,
}

/// Censorship Evidence Fusion Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionAssessment {
    pub primary_cause: CensorshipRootCause,
    pub confidence_percentage: u8,
    pub severity_score: u8, // 0 to 100
    pub mitigation: EvasionMitigation,
    pub supporting_signals: Vec<String>,
}

pub struct EvidenceFusionEngine;

impl EvidenceFusionEngine {
    /// Fuse multiple observational signals into a high-confidence diagnostic and mitigation
    pub fn fuse(signal: &EvidenceSignal) -> FusionAssessment {
        let mut signals_list = Vec::new();
        let mut severity: u8 = 0;
        let mut confidence: u8 = 50;

        // Check for Physical Bearer Cutoff / NIN first
        if signal.domestic_probe_alive && !signal.international_probe_alive {
            signals_list.push(
                "Domestic Iranian routes operational; international BGP unreachable".to_string(),
            );
            return FusionAssessment {
                primary_cause: CensorshipRootCause::NationalIntranetIsolation,
                confidence_percentage: 96,
                severity_score: 95,
                mitigation: EvasionMitigation {
                    enable_tls_fragmentation: false,
                    tls_fragment_size: 0,
                    force_utls_fingerprint: "chrome_124".to_string(),
                    recommended_sni: "www.aparat.com".to_string(),
                    use_doh_or_doq: true,
                    switch_to_reality_vless: false,
                    switch_to_hysteria2_quic: false,
                    fallback_to_domestic_reverse_relay: true,
                    halt_international_probes: true,
                },
                supporting_signals: signals_list,
            };
        }

        if !signal.domestic_probe_alive && !signal.international_probe_alive {
            signals_list.push("Both domestic and international probes failing".to_string());
            return FusionAssessment {
                primary_cause: CensorshipRootCause::FullPhysicalSeverance,
                confidence_percentage: 92,
                severity_score: 100,
                mitigation: EvasionMitigation {
                    enable_tls_fragmentation: false,
                    tls_fragment_size: 0,
                    force_utls_fingerprint: "chrome_124".to_string(),
                    recommended_sni: "www.speedtest.net".to_string(),
                    use_doh_or_doq: false,
                    switch_to_reality_vless: false,
                    switch_to_hysteria2_quic: false,
                    fallback_to_domestic_reverse_relay: false,
                    halt_international_probes: true,
                },
                supporting_signals: signals_list,
            };
        }

        // Evaluate FAVA DPI RST Injection: Middlebox RSTs usually arrive in < 25ms
        let mut is_fava_rst = false;
        if signal.tcp_rst_observed {
            if let Some(delta) = signal.tcp_rst_delta_ms {
                if delta <= 25 {
                    is_fava_rst = true;
                    signals_list.push(format!(
                        "Early TCP RST injected at {}ms indicating active DPI middlebox",
                        delta
                    ));
                    severity += 35;
                    confidence += 20;
                }
            }
        }

        // Evaluate SNI Blocklist
        if signal.sni_block_count > 0 {
            signals_list.push(format!(
                "{} test domain SNIs intercepted and blocked",
                signal.sni_block_count
            ));
            severity += 25;
            confidence += 15;
        }

        // Evaluate DNS Injection
        if signal.dns_injection_detected {
            signals_list.push("DNS response injection detected on port 53".to_string());
            severity += 20;
            confidence += 10;
        }

        // Evaluate UDP / QUIC
        if signal.quic_drop_rate > 0.6 {
            signals_list.push(format!(
                "High QUIC packet loss ({:.0}%) detected",
                signal.quic_drop_rate * 100.0
            ));
            severity += 15;
        }

        let primary_cause = if is_fava_rst || signal.sni_block_count > 2 {
            CensorshipRootCause::TransportFamilyFingerprinted
        } else if signal.dns_injection_detected && signal.sni_block_count == 0 {
            CensorshipRootCause::DnsPoisoningActive
        } else if signal.quic_drop_rate > 0.8 && signal.control_probe_passed {
            CensorshipRootCause::QuicUdpThrottling
        } else if severity > 40 {
            CensorshipRootCause::IspSpecificDpiInterference
        } else if severity > 10 {
            CensorshipRootCause::EndpointSpecificFailure
        } else {
            CensorshipRootCause::CleanUnrestricted
        };

        let calculated_confidence = confidence.min(98);

        let mitigation = EvasionMitigation {
            enable_tls_fragmentation: primary_cause
                == CensorshipRootCause::TransportFamilyFingerprinted
                || signal.sni_block_count > 0,
            tls_fragment_size: if is_fava_rst { 2 } else { 16 },
            force_utls_fingerprint: if is_fava_rst {
                "chrome_124".to_string()
            } else {
                "safari_17_0".to_string()
            },
            recommended_sni: "www.speedtest.net".to_string(),
            use_doh_or_doq: signal.dns_injection_detected,
            switch_to_reality_vless: primary_cause
                == CensorshipRootCause::TransportFamilyFingerprinted
                || severity > 40,
            switch_to_hysteria2_quic: signal.quic_drop_rate < 0.3 && (severity > 30),
            fallback_to_domestic_reverse_relay: false,
            halt_international_probes: false,
        };

        FusionAssessment {
            primary_cause,
            confidence_percentage: calculated_confidence,
            severity_score: severity.min(100),
            mitigation,
            supporting_signals: signals_list,
        }
    }
}
