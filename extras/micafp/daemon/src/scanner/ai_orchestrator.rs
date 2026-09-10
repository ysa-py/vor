//! AI Orchestration & Deterministic Safety Gate
//!
//! Provides a provider-agnostic interface (Remote AI, Local Heuristic, Deterministic Fallback)
//! to interpret network telemetry, generate exploration hypotheses, and rank candidate transports.
//! Enforces deterministic safety gating: all AI recommendations MUST be verified via live probes
//! before promotion to active routing.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::scanner::candidate_graph::{CandidateNode, CandidateSource, ProtocolClassification};

/// Structured AI Policy Suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPolicySuggestion {
    pub recommended_transport: String,
    pub recommended_ports: Vec<u16>,
    pub candidate_snis: Vec<String>,
    pub tls_fragmentation: bool,
    pub utls_fingerprint: String,
    pub rationale: String,
    pub confidence_score: f32, // 0.0 to 1.0
}

/// Provider-agnostic AI interface
#[async_trait]
pub trait AiPolicyProvider: Send + Sync {
    async fn evaluate_network(
        &self,
        isp: &str,
        recent_loss: f32,
        avg_latency: u64,
        dpi_detected: bool,
        censorship_score: u8,
    ) -> anyhow::Result<AiPolicySuggestion>;

    fn provider_name(&self) -> &'static str;
}

/// Local Deterministic / Rule-based Heuristic AI Provider (Always available offline)
pub struct LocalHeuristicAiProvider;

#[async_trait]
impl AiPolicyProvider for LocalHeuristicAiProvider {
    async fn evaluate_network(
        &self,
        isp: &str,
        recent_loss: f32,
        avg_latency: u64,
        dpi_detected: bool,
        censorship_score: u8,
    ) -> anyhow::Result<AiPolicySuggestion> {
        let isp_lower = isp.to_lowercase();

        let (transport, ports, snis, frag, fp) = if censorship_score > 70 || dpi_detected {
            if isp_lower.contains("irancell") || isp_lower.contains("mtn") {
                (
                    "vless_reality".to_string(),
                    vec![443, 8443, 2053],
                    vec![
                        "www.speedtest.net".to_string(),
                        "c.whatsapp.net".to_string(),
                    ],
                    true,
                    "chrome_124".to_string(),
                )
            } else if isp_lower.contains("mci") || isp_lower.contains("hamrah") {
                (
                    "hysteria2".to_string(),
                    vec![443, 8443, 2087],
                    vec!["www.speedtest.net".to_string()],
                    false,
                    "ios_17_2".to_string(),
                )
            } else {
                (
                    "shadow_tls".to_string(),
                    vec![443, 8443],
                    vec![
                        "www.speedtest.net".to_string(),
                        "telewebion.com".to_string(),
                    ],
                    true,
                    "safari_17_0".to_string(),
                )
            }
        } else if recent_loss > 0.15 || avg_latency > 250 {
            (
                "hysteria2".to_string(),
                vec![443, 2083, 2096],
                vec!["www.speedtest.net".to_string()],
                false,
                "chrome_124".to_string(),
            )
        } else {
            (
                "vless_reality".to_string(),
                vec![443, 8443],
                vec!["www.speedtest.net".to_string()],
                false,
                "chrome_124".to_string(),
            )
        };

        Ok(AiPolicySuggestion {
            recommended_transport: transport,
            recommended_ports: ports,
            candidate_snis: snis,
            tls_fragmentation: frag,
            utls_fingerprint: fp,
            rationale: format!(
                "Deterministic heuristic tailored for {} with severity {}",
                isp, censorship_score
            ),
            confidence_score: 0.88,
        })
    }

    fn provider_name(&self) -> &'static str {
        "LocalHeuristicEngine"
    }
}

/// AI Orchestrator that coordinates AI providers with strict deterministic safety gating
pub struct AiOrchestrator {
    provider: Box<dyn AiPolicyProvider>,
}

impl AiOrchestrator {
    pub fn new() -> Self {
        Self {
            provider: Box::new(LocalHeuristicAiProvider),
        }
    }

    pub fn with_provider(provider: Box<dyn AiPolicyProvider>) -> Self {
        Self { provider }
    }

    /// Generate verified exploration hypotheses safely
    pub async fn generate_hypotheses(
        &self,
        isp: &str,
        recent_loss: f32,
        avg_latency: u64,
        dpi_detected: bool,
        censorship_score: u8,
    ) -> Vec<CandidateNode> {
        let suggestion = match self
            .provider
            .evaluate_network(
                isp,
                recent_loss,
                avg_latency,
                dpi_detected,
                censorship_score,
            )
            .await
        {
            Ok(s) => s,
            Err(e) => {
                warn!(
                    "AI suggestion generation error: {}. Falling back to local deterministic.",
                    e
                );
                LocalHeuristicAiProvider
                    .evaluate_network(
                        isp,
                        recent_loss,
                        avg_latency,
                        dpi_detected,
                        censorship_score,
                    )
                    .await
                    .unwrap()
            }
        };

        let mut hypotheses = Vec::new();
        let protocol = match suggestion.recommended_transport.as_str() {
            "vless_reality" => ProtocolClassification::VlessReality,
            "hysteria2" => ProtocolClassification::Hysteria2,
            "shadow_tls" => ProtocolClassification::ShadowTlsV3,
            "tuic" => ProtocolClassification::TuicV5,
            _ => ProtocolClassification::VlessReality,
        };

        for port in suggestion.recommended_ports {
            for sni in &suggestion.candidate_snis {
                let mut node = CandidateNode::new(
                    sni,
                    port,
                    &suggestion.recommended_transport,
                    protocol,
                    CandidateSource::AiHypothesis,
                );
                node.utls_fingerprint = suggestion.utls_fingerprint.clone();
                node.sni_candidate = sni.clone();
                node.operator_affinity = Some(isp.to_string());
                hypotheses.push(node);
            }
        }

        hypotheses
    }
}
