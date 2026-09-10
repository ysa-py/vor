//! Unit and Integration Tests for Directive v70 Dual-Mode Transport Core.

#[cfg(test)]
mod tests {
    use crate::config::{DualModeConfig, GeoHopNode, PathConfig, TransportMode};
    use crate::mode_a_multipath::{ModeAMultipathEngine, QuicPathChannel};
    use crate::mode_b_layered::ModeBLayeredEngine;
    use crate::scheduler::{AdaptiveMultiPathScheduler, PathMetrics, PathScheduler};

    #[test]
    fn test_mode_a_single_layer_encapsulation_and_decapsulation() {
        let path_cfg = PathConfig {
            path_id: 1,
            endpoint: "127.0.0.1:443".to_string(),
            region_tag: "EU-West".to_string(),
            initial_rtt_ms: 30,
            congestion_control: crate::config::CongestionControl::Bbr,
            cipher: crate::config::CipherSuite::ChaCha20Poly1305,
            max_mtu: 1420,
        };
        let channel = QuicPathChannel::new(&path_cfg);
        let raw_payload = b"GET /vpn/stream HTTP/1.1\r\nHost: example.com\r\n\r\n";

        let encapsulated = channel.encapsulate_single_layer(raw_payload);
        assert!(encapsulated.len() > raw_payload.len());
        assert_eq!(encapsulated[0], 1); // Path ID header

        let decapsulated = channel.decapsulate_single_layer(&encapsulated).expect("Decapsulation failed");
        assert_eq!(&decapsulated[..], raw_payload);
    }

    #[test]
    fn test_mode_b_5_layer_encapsulation_and_peel() {
        let config = DualModeConfig::default();
        let engine = ModeBLayeredEngine::new(config);
        let raw_payload = b"CONFIDENTIAL_USER_PACKET_ANONYMITY_SHIELD";

        let encapsulated = engine.encapsulate_5_layers(raw_payload).expect("5-layer encapsulation failed");
        assert!(encapsulated.len() >= raw_payload.len() + (36 * 5));

        // Peel outermost layer (Hop 1)
        let (hop_index, inner_layer_1) = ModeBLayeredEngine::peel_one_layer(&encapsulated).expect("Peel Hop 1 failed");
        assert_eq!(hop_index, 1);

        // Peel layer 2 (Hop 2)
        let (hop_index_2, inner_layer_2) = ModeBLayeredEngine::peel_one_layer(&inner_layer_1).expect("Peel Hop 2 failed");
        assert_eq!(hop_index_2, 2);
    }

    #[test]
    fn test_mode_b_fail_closed_on_unreachable_hop() {
        let mut config = DualModeConfig::default();
        config.fail_closed = true;
        let mut engine = ModeBLayeredEngine::new(config);

        let raw_payload = b"CRITICAL_INTEGRITY_DATA";
        // Initially all hops healthy -> succeeds
        assert!(engine.encapsulate_5_layers(raw_payload).is_ok());

        // Simulate Hop 3 network partition / 95% packet loss
        engine.update_hop_health(3, false, 0.95);

        // Must FAIL CLOSED and drop packet
        let result = engine.encapsulate_5_layers(raw_payload);
        assert!(result.is_err(), "Mode B must drop packet when a hop is down under fail_closed=true");
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Fail-Closed Protection"));
    }

    #[test]
    fn test_mode_a_loss_reroute_adaptive_scheduler() {
        let scheduler = AdaptiveMultiPathScheduler::new(vec![1, 2, 3]);

        // Path 1 has low RTT and zero loss
        let mut m1 = PathMetrics::default();
        m1.path_id = 1;
        m1.rtt_ms = 20;
        m1.packet_loss_rate = 0.0;
        m1.available_bandwidth_kbps = 100_000;
        scheduler.update_metrics(m1);

        // Path 2 has severe loss (45%)
        let mut m2 = PathMetrics::default();
        m2.path_id = 2;
        m2.rtt_ms = 200;
        m2.packet_loss_rate = 0.45;
        m2.available_bandwidth_kbps = 10_000;
        m2.congestion_state = "Loss".to_string();
        scheduler.update_metrics(m2);

        // Path 3 has moderate metrics
        let mut m3 = PathMetrics::default();
        m3.path_id = 3;
        m3.rtt_ms = 40;
        m3.packet_loss_rate = 0.01;
        m3.available_bandwidth_kbps = 75_000;
        scheduler.update_metrics(m3);

        scheduler.rebalance();

        // Sample 50 packet dispatches -> Path 2 should receive minimal to zero dispatches
        let mut path2_dispatches = 0;
        for _ in 0..50 {
            let path = scheduler.select_path(1200).expect("Path selection failed");
            if path == 2 {
                path2_dispatches += 1;
            }
        }

        assert!(path2_dispatches <= 2, "Degraded path 2 should be virtually deprioritized by adaptive scheduler");
    }

    #[test]
    fn test_dynamic_pmtud_probing() {
        let path_cfg = PathConfig {
            path_id: 1,
            endpoint: "127.0.0.1:443".to_string(),
            region_tag: "EU-West".to_string(),
            initial_rtt_ms: 30,
            congestion_control: crate::config::CongestionControl::Bbr,
            cipher: crate::config::CipherSuite::ChaCha20Poly1305,
            max_mtu: 1420,
        };
        let mut channel = QuicPathChannel::new(&path_cfg);

        // Packet loss feedback reduces MTU
        let mtu1 = channel.probe_pmtud(true);
        assert_eq!(mtu1, 1380);

        // Normal feedback increases MTU
        let mtu2 = channel.probe_pmtud(false);
        assert_eq!(mtu2, 1400);
    }
}
