//! Comprehensive Autonomous Scanner and Anti-Censorship Test Suite

use micafp_unified_shield_daemon::scanner::adaptive_engine::{
    AdaptiveScanScheduler, ResourceBudget, ScanLevel,
};
use micafp_unified_shield_daemon::scanner::ai_orchestrator::{
    AiOrchestrator, LocalHeuristicAiProvider,
};
use micafp_unified_shield_daemon::scanner::autonomous_engine::AutonomousScannerEngine;
use micafp_unified_shield_daemon::scanner::candidate_graph::{
    CandidateGraph, CandidateNode, CandidateSource, CandidateState, ProbeOutcome,
    ProtocolClassification,
};
use micafp_unified_shield_daemon::scanner::evidence_fusion::{
    CensorshipRootCause, EvidenceFusionEngine, EvidenceSignal,
};
use micafp_unified_shield_daemon::scanner::network_assessor::NetworkState;
use micafp_unified_shield_daemon::scanner::self_healing_failover::SelfHealingEngine;
use std::sync::Arc;

#[test]
fn test_candidate_graph_lifecycle() {
    let graph = CandidateGraph::new(50);

    let mut node1 = CandidateNode::new(
        "104.16.1.1",
        443,
        "vless_reality",
        ProtocolClassification::VlessReality,
        CandidateSource::BootstrapSeed,
    );
    node1.reputation_score = 50.0;

    graph.add_or_merge(node1);
    assert_eq!(graph.count(), 1);
    assert_eq!(graph.verified_count(), 0);

    // Record verified probe success
    graph.record_success("104.16.1.1:443:vless_reality", 45, 15, 30);
    assert_eq!(graph.verified_count(), 1);

    let top = graph.get_top_verified(1);
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].recent_latency_ms, Some(45));
    assert!(top[0].reputation_score > 60.0);

    // Record consecutive failures -> should trigger quarantine
    let id = "104.16.1.1:443:vless_reality";
    graph.record_failure(id, ProbeOutcome::ConnectionTimeout { timeout_ms: 2000 });
    graph.record_failure(id, ProbeOutcome::ConnectionTimeout { timeout_ms: 2000 });
    graph.record_failure(id, ProbeOutcome::DpiTcpReset { reset_delay_ms: 12 });

    let verified_after_fail = graph.get_top_verified(1);
    assert_eq!(verified_after_fail.len(), 0); // Quarantined node removed from top verified
}

#[test]
fn test_adaptive_scan_scheduler_transitions() {
    let mut scheduler = AdaptiveScanScheduler::new();

    // Normal state with high verified count -> Focused
    let level1 = scheduler.calculate_optimal_level(NetworkState::Normal, 6, false, true);
    assert_eq!(level1, ScanLevel::Focused);

    // Degraded state with scarce candidates -> Deep
    let level2 = scheduler.calculate_optimal_level(NetworkState::Degraded, 1, true, true);
    assert_eq!(level2, ScanLevel::Deep);

    // Domestic Only (National Intranet) -> BlackoutRecovery
    let level3 = scheduler.calculate_optimal_level(NetworkState::DomesticOnly, 0, false, false);
    assert_eq!(level3, ScanLevel::BlackoutRecovery);

    // Thermal throttling downgrade
    scheduler.update_resource_budget(ResourceBudget {
        battery_percent: 80,
        is_charging: true,
        is_thermal_throttled: true,
        is_metered: false,
        cpu_load_factor: 0.8,
    });

    let level4 = scheduler.calculate_optimal_level(NetworkState::Normal, 2, false, false);
    assert_eq!(level4, ScanLevel::MicroProbe);
}

#[test]
fn test_evidence_fusion_national_intranet() {
    let nin_signal = EvidenceSignal {
        tcp_rst_observed: false,
        tcp_rst_delta_ms: None,
        sni_block_count: 0,
        dns_injection_detected: false,
        quic_drop_rate: 1.0,
        domestic_probe_alive: true,
        international_probe_alive: false,
        control_probe_passed: true,
    };

    let assessment = EvidenceFusionEngine::fuse(&nin_signal);
    assert_eq!(
        assessment.primary_cause,
        CensorshipRootCause::NationalIntranetIsolation
    );
    assert!(assessment.confidence_percentage >= 90);
    assert!(assessment.mitigation.fallback_to_domestic_reverse_relay);
    assert!(assessment.mitigation.halt_international_probes);
}

#[test]
fn test_evidence_fusion_fava_dpi_rst() {
    let dpi_signal = EvidenceSignal {
        tcp_rst_observed: true,
        tcp_rst_delta_ms: Some(11), // 11ms fast middlebox injection
        sni_block_count: 3,
        dns_injection_detected: true,
        quic_drop_rate: 0.4,
        domestic_probe_alive: true,
        international_probe_alive: true,
        control_probe_passed: true,
    };

    let assessment = EvidenceFusionEngine::fuse(&dpi_signal);
    assert_eq!(
        assessment.primary_cause,
        CensorshipRootCause::TransportFamilyFingerprinted
    );
    assert!(assessment.mitigation.enable_tls_fragmentation);
    assert!(assessment.mitigation.switch_to_reality_vless);
}

#[test]
fn test_self_healing_predictive_failover() {
    let graph = Arc::new(CandidateGraph::new(50));

    let node1 = CandidateNode::new(
        "1.1.1.1",
        443,
        "vless_reality",
        ProtocolClassification::VlessReality,
        CandidateSource::LearnedSuccess,
    );
    let node2 = CandidateNode::new(
        "8.8.8.8",
        8443,
        "hysteria2",
        ProtocolClassification::Hysteria2,
        CandidateSource::LearnedSuccess,
    );

    graph.add_or_merge(node1.clone());
    graph.add_or_merge(node2.clone());

    graph.record_success("1.1.1.1:443:vless_reality", 50, 15, 35);
    graph.record_success("8.8.8.8:8443:hysteria2", 60, 20, 40);

    let engine = SelfHealingEngine::new(Arc::clone(&graph));
    engine.set_active_node(node1);
    engine.refresh_standby_pool();

    // High packet loss triggers failover to node2
    let switched = engine.evaluate_health_and_heal(250, 0.45, 2);
    assert!(switched.is_some());
    assert_eq!(switched.unwrap().id, "8.8.8.8:8443:hysteria2");
    assert_eq!(engine.get_failover_count(), 1);
}

#[tokio::test]
async fn test_end_to_end_autonomous_cycle() {
    let engine = AutonomousScannerEngine::new();
    let status = engine.run_autonomous_cycle("Irancell (MTN)").await;
    assert!(status.is_ok());

    let res = status.unwrap();
    assert!(res.total_candidates > 0);
    assert!(res.censorship_confidence > 0);
}
