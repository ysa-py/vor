// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Dual-Mode Test Suite (Directive v70)
//
// Validation & Verification:
//   1. Unit: Path scoring & composite formula
//   2. Unit: Scheduler dynamic rebalancing
//   3. Unit: PMTUD step-down and probe behavior
//   4. Unit: Congestion control state tracking
//   5. Unit: Mode A single-layer AEAD correctness
//   6. Unit: Mode B 5-layer nested onion encapsulation & decapsulation
//   7. Integration: Mode A packet loss injection & automatic path rerouting
//   8. Integration: Mode B fail-closed assertion on unreachable hop
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use std::sync::Arc;
    use std::time::Duration;

    use crate::transport::dual_mode::mode_a_fast::{
        ModeAFastConfig, ModeAFastEngine, PmtudController,
    };
    use crate::transport::dual_mode::mode_b_layered::{
        ModeBLayeredConfig, ModeBLayeredEngine, MODE_B_HOP_COUNT,
    };
    use crate::transport::dual_mode::scheduler::{
        AdaptiveMultipathScheduler, CongestionState, PathMetrics, PathScheduler,
    };
    use crate::transport::dual_mode::tun_pipeline::{
        DualModeCoreConfig, DualModeCoreEngine, DualTransportMode,
    };

    // ── 1. Unit: Path scoring formula verification ────────────────────────────
    #[test]
    fn test_path_scoring_logic() {
        let scheduler = AdaptiveMultipathScheduler::new(3);

        let mut good_path = PathMetrics::default();
        good_path.path_id = 0;
        good_path.rtt_ms = 15.0;
        good_path.packet_loss_rate = 0.0;
        good_path.available_bandwidth_kbps = 100_000;

        let mut poor_path = PathMetrics::default();
        poor_path.path_id = 1;
        poor_path.rtt_ms = 180.0;
        poor_path.packet_loss_rate = 0.12;
        poor_path.congestion_state = CongestionState::Loss;

        let score_good = scheduler.score_path(&good_path);
        let score_poor = scheduler.score_path(&poor_path);

        assert!(
            score_good.composite_score > score_poor.composite_score,
            "Good path composite score ({}) must exceed poor path score ({})",
            score_good.composite_score,
            score_poor.composite_score
        );
    }

    // ── 2. Unit: Scheduler dynamic rebalancing ────────────────────────────────
    #[test]
    fn test_scheduler_rebalance_on_metric_update() {
        let scheduler = AdaptiveMultipathScheduler::new(2);

        let mut p0 = PathMetrics::default();
        p0.path_id = 0;
        p0.rtt_ms = 30.0;

        let mut p1 = PathMetrics::default();
        p1.path_id = 1;
        p1.rtt_ms = 30.0;

        scheduler.register_path(p0);
        scheduler.register_path(p1);

        // Degrade path 0
        scheduler.update_metrics(
            0,
            Box::new(|m| {
                m.rtt_ms = 400.0;
                m.packet_loss_rate = 0.25;
            }),
        );
        scheduler.rebalance();

        // Sample scheduling over 100 packets: Path 1 should win majority
        let mut p1_wins = 0;
        for _ in 0..100 {
            if let Some(pid) = scheduler.schedule_packet(1024, None) {
                if pid == 1 {
                    p1_wins += 1;
                }
            }
        }
        assert!(
            p1_wins > 70,
            "Path 1 should receive >70% of traffic after Path 0 degradation, got {}",
            p1_wins
        );
    }

    // ── 3. Unit: PMTUD step-down and probe sizing ─────────────────────────────
    #[test]
    fn test_pmtud_behavior() {
        let mut pmtud = PmtudController::new(1420, 1280);
        assert_eq!(pmtud.current_pmtu, 1420);

        // Step down to explicit advertised MTU
        let new_mtu = pmtud.handle_packet_too_big(Some(1360));
        assert_eq!(new_mtu, 1360);
        assert_eq!(pmtud.current_pmtu, 1360);

        // Step down without advertised MTU (fallback)
        let fallback_mtu = pmtud.handle_packet_too_big(None);
        assert_eq!(fallback_mtu, 1280);
    }

    // ── 4. Unit: Congestion control state tracking ────────────────────────────
    #[test]
    fn test_congestion_control_transitions() {
        let scheduler = AdaptiveMultipathScheduler::new(1);
        let mut p0 = PathMetrics::default();
        p0.path_id = 0;
        scheduler.register_path(p0);

        scheduler.on_packet_loss(0);
        let metrics = scheduler.get_all_metrics();
        assert_eq!(metrics[0].congestion_state, CongestionState::Loss);
        assert!(metrics[0].packet_loss_rate > 0.0);

        // ACK recovers path
        scheduler.on_packet_ack(0, Duration::from_millis(20));
        let recovered = scheduler.get_all_metrics();
        assert!(recovered[0].is_alive);
    }

    // ── 5. Unit: Mode A single-layer AEAD correctness ────────────────────────
    #[tokio::test]
    async fn test_mode_a_single_layer_roundtrip() {
        let engine = ModeAFastEngine::new(ModeAFastConfig::default()).unwrap();
        let payload = Bytes::from_static(b"HELLO-QUANTUM-FAST-PACKET-MODE-A-12345");

        let (path_id, encrypted) = engine
            .dispatch_tun_packet(payload.clone(), Some(42))
            .await
            .unwrap();
        assert_ne!(encrypted.as_ref(), payload.as_ref());

        let decrypted = engine
            .handle_incoming_datagram(path_id, encrypted)
            .await
            .unwrap();
        assert_eq!(decrypted, payload);
    }

    // ── 6. Unit: Mode B 5-layer nested onion encapsulation & decapsulation ───
    #[test]
    fn test_mode_b_5_hop_nested_onion_roundtrip() {
        let engine = ModeBLayeredEngine::new(ModeBLayeredConfig::default()).unwrap();
        let payload = b"TOP-SECRET-5-HOP-ONION-MIXNET-PAYLOAD-007";

        // 1. Client encapsulates through all 5 hops
        let onion_packet = engine.encapsulate_5_hops(payload).unwrap();
        assert_ne!(onion_packet.as_ref(), payload);

        // 2. Simulate progressive decapsulation hop by hop (Hop 0 -> 1 -> 2 -> 3 -> 4)
        let mut current_buffer = onion_packet.to_vec();
        for hop_idx in 0..MODE_B_HOP_COUNT {
            let (next_payload, next_hop_addr) = engine
                .decapsulate_hop_layer(hop_idx, &current_buffer)
                .unwrap();
            if hop_idx < MODE_B_HOP_COUNT - 1 {
                assert!(
                    next_hop_addr.is_some(),
                    "Hops 0-3 must specify next hop address"
                );
            } else {
                assert!(
                    next_hop_addr.is_none(),
                    "Exit hop 4 must terminate chain without next hop"
                );
            }
            current_buffer = next_payload.to_vec();
        }

        // 3. Final unwrapped output must exactly equal original payload
        assert_eq!(&current_buffer, payload);
    }

    // ── 7. Integration: Mode A packet loss failover reroute ───────────────────
    #[tokio::test]
    async fn test_mode_a_packet_loss_reroute_integration() {
        let mut config = ModeAFastConfig::default();
        config.path_count = 3;
        let engine = ModeAFastEngine::new(config).unwrap();
        let test_pkt = Bytes::from_static(b"STREAM-PACKET-DATA");

        // Fail Path 0
        engine.record_path_loss(0);
        engine.record_path_loss(0);
        engine.record_path_loss(0);
        engine.scheduler().mark_path_failed(0);

        // Verify dispatch immediately picks alternative active path (Path 1 or 2)
        for _ in 0..10 {
            let (chosen_pid, _enc) = engine
                .dispatch_tun_packet(test_pkt.clone(), None)
                .await
                .unwrap();
            assert_ne!(
                chosen_pid, 0,
                "Failed Path 0 must not be chosen by scheduler"
            );
        }
    }

    // ── 8. Integration: Mode B fail-closed assertion ─────────────────────────
    #[test]
    fn test_mode_b_fail_closed_on_unreachable_hop() {
        let mut config = ModeBLayeredConfig::default();
        config.fail_closed = true;
        let engine = ModeBLayeredEngine::new(config).unwrap();
        let payload = b"CONFIDENTIAL-DATA";

        // Initial state is healthy
        assert!(engine.encapsulate_5_hops(payload).is_ok());

        // Hop 2 loses connectivity
        engine.set_hop_liveness(2, false);

        // Assertion: Circuit MUST fail-closed and refuse packet dispatch
        let result = engine.encapsulate_5_hops(payload);
        assert!(
            result.is_err(),
            "Mode B must fail closed and drop packets when a hop in the chain fails"
        );
    }

    // ── 9. Integration: Dual-Mode Core runtime switching ─────────────────────
    #[tokio::test]
    async fn test_dual_mode_core_runtime_switch() {
        let config = DualModeCoreConfig::default();
        let core = DualModeCoreEngine::new(config).unwrap();

        assert_eq!(core.current_mode(), DualTransportMode::Fast);
        let test_pkt = Bytes::from_static(b"DUAL-MODE-INGRESS-PACKET");

        // Outbound via Fast Mode
        let (_ep, out_fast) = core
            .process_outbound_tun_packet(test_pkt.clone(), None)
            .await
            .unwrap();
        assert!(!out_fast.is_empty());

        // Switch to Layered Mode at runtime
        core.set_transport_mode(DualTransportMode::Layered);
        assert_eq!(core.current_mode(), DualTransportMode::Layered);

        // Outbound via Layered Mode
        let (_ep, out_layered) = core
            .process_outbound_tun_packet(test_pkt.clone(), None)
            .await
            .unwrap();
        assert!(!out_layered.is_empty());
        assert_ne!(out_fast, out_layered);
    }
}
