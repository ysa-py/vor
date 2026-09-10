//! Conformance tests: run the shared cross-platform vectors against the
//! Rust reference implementation.
//!
//! * `tests/vectors/decision-vectors.json` — decision/threat/reward/fragment
//!   vectors shared with the Kotlin (android) and Go (desktop) interpreters.
//! * `../../../license/vectors.json` — license vectors shared with every
//!   platform's license verifier.

use serde_json::Value;
use std::fs;
use vor_core::selector;
use vor_core::threat::{ThreatLevel, ThreatObservation};

fn vectors() -> Value {
    let text = fs::read_to_string("tests/vectors/decision-vectors.json").expect("vectors file");
    serde_json::from_str(&text).expect("vectors json")
}

fn license_vector_data() -> Value {
    let text = fs::read_to_string("../../license/vectors.json").expect("license vectors file");
    serde_json::from_str(&text).expect("license vectors json")
}

fn dev_public_key() -> String {
    fs::read_to_string("../../license/keys/dev/VOR_LICENSE_PUBLIC_KEY.txt")
        .expect("dev public key")
        .trim()
        .to_string()
}

fn client_hello() -> Vec<u8> {
    // Deterministic ClientHello with SNI "example.com" followed by a dummy
    // supported_versions extension (mirrors fragment.rs test helper).
    let name = b"example.com";
    let mut ext = Vec::new();
    let list_len = (1 + 2 + name.len()) as u16;
    ext.extend_from_slice(&list_len.to_be_bytes());
    ext.push(0x00);
    ext.extend_from_slice(&(name.len() as u16).to_be_bytes());
    ext.extend_from_slice(name);
    // dummy supported_versions extension (type 0x002B, 2 bytes)
    let mut trailing = Vec::new();
    trailing.extend_from_slice(&0x002Bu16.to_be_bytes());
    trailing.extend_from_slice(&2u16.to_be_bytes());
    trailing.extend_from_slice(&[0x03, 0x04]);
    let ext_total = ext.len() + 4 + trailing.len();
    let mut handshake = Vec::new();
    handshake.extend_from_slice(&[0x03, 0x03]);
    handshake.extend_from_slice(&[0u8; 32]);
    handshake.push(0);
    handshake.extend_from_slice(&2u16.to_be_bytes());
    handshake.extend_from_slice(&[0x13, 0x01]);
    handshake.push(1);
    handshake.push(0x00);
    handshake.extend_from_slice(&(ext_total as u16).to_be_bytes());
    handshake.extend_from_slice(&0x0000u16.to_be_bytes());
    handshake.extend_from_slice(&(ext.len() as u16).to_be_bytes());
    handshake.extend_from_slice(&ext);
    handshake.extend_from_slice(&trailing);
    let mut record = Vec::new();
    record.push(0x16);
    record.extend_from_slice(&[0x03, 0x01]);
    record.extend_from_slice(&((handshake.len() + 4) as u16).to_be_bytes());
    record.push(0x01);
    record.extend_from_slice(&handshake.len().to_be_bytes()[1..4]);
    record.extend_from_slice(&handshake);
    record
}

#[test]
fn reward_vectors() {
    let root = vectors();
    let tolerance = root["metadata"]["reward_tolerance"].as_f64().unwrap();
    for case in root["reward_cases"].as_array().unwrap() {
        let outcome: selector::Outcome = serde_json::from_value(case["outcome"].clone()).unwrap();
        let expected = case["expected_reward"].as_f64().unwrap();
        let actual = selector::reward_for(&outcome);
        assert!(
            (actual - expected).abs() <= tolerance,
            "case {}: reward {} != {}",
            case["name"].as_str().unwrap(),
            actual,
            expected
        );
    }
}

#[test]
fn threat_vectors() {
    let root = vectors();
    for case in root["threat_cases"].as_array().unwrap() {
        let observation: ThreatObservation =
            serde_json::from_value(case["observation"].clone()).unwrap();
        let expected = ThreatLevel::from_str_value(case["expected_level"].as_str().unwrap());
        assert_eq!(
            observation.level(),
            expected,
            "case {}",
            case["name"].as_str().unwrap()
        );
    }
}

#[test]
fn decision_vectors() {
    let root = vectors();
    for case in root["decision_cases"].as_array().unwrap() {
        let state: selector::SelectorState = match &case["state"] {
            Value::Null => selector::SelectorState::default(),
            value => serde_json::from_value(value.clone()).unwrap(),
        };
        let context: selector::DecisionContext =
            serde_json::from_value(case["context"].clone()).unwrap();
        let decision = selector::decide(&state, &context);
        let expected = &case["expected"];
        let name = case["name"].as_str().unwrap();

        assert_eq!(
            decision.engine,
            expected["engine"].as_str().unwrap(),
            "case {name}: engine"
        );
        if let Some(first) = expected["first_strategy"].as_str() {
            assert_eq!(decision.strategies[0], first, "case {name}: first strategy");
        }
        if let Some(strategies) = expected["strategies"].as_array() {
            let want: Vec<String> = strategies
                .iter()
                .map(|s| s.as_str().unwrap().to_string())
                .collect();
            assert_eq!(decision.strategies, want, "case {name}: strategies");
        }
        if let Some(delay) = expected["fragment_delay_ms"].as_u64() {
            assert_eq!(
                decision.fragment_delay_ms as u64, delay,
                "case {name}: delay"
            );
        }
        if let Some(fake) = expected["fake_sni"].as_str() {
            assert_eq!(decision.fake_sni, fake, "case {name}: fake sni");
        }
        if let Some(count) = expected["edge_count"].as_u64() {
            assert_eq!(
                decision.edges.len() as u64,
                count,
                "case {name}: edge count"
            );
        }
        if let Some(first_edge) = expected["first_edge"].as_object() {
            assert_eq!(
                decision.edges[0]["ip"].as_str().unwrap(),
                first_edge["ip"].as_str().unwrap()
            );
            assert_eq!(
                decision.edges[0]["port"].as_u64().unwrap(),
                first_edge["port"].as_u64().unwrap()
            );
            assert_eq!(
                decision.edges[0]["max_split"].as_u64().unwrap(),
                first_edge["max_split"].as_u64().unwrap()
            );
        }
        if let Some(shaping) = expected["traffic_shaping"].as_bool() {
            assert_eq!(decision.traffic_shaping, shaping, "case {name}: shaping");
        }
        if let Some(level) = expected["shaper_level"].as_str() {
            assert_eq!(decision.shaper_level, level, "case {name}: shaper level");
        }
        if let Some(level) = expected["threat_level"].as_str() {
            assert_eq!(decision.threat_level, level, "case {name}: threat level");
        }
        if let Some(champion) = expected["is_champion"].as_bool() {
            assert_eq!(decision.is_champion, champion, "case {name}: is champion");
        }
    }
}

#[test]
fn fragment_vectors() {
    use vor_core::fragment::{plan, Strategy};
    let root = vectors();
    let record = client_hello();
    for case in root["fragment_cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let strategy = Strategy::from_name(case["strategy"].as_str().unwrap());
        let delay = case["delay_ms"].as_u64().unwrap_or(0) as u32;
        let fragment = plan(&record, strategy, delay);
        if let Some(writes) = case["expected_writes"].as_u64() {
            assert_eq!(
                fragment.writes.len() as u64,
                writes,
                "case {name}: write count"
            );
        }
        if let Some(min_writes) = case["expected_min_writes"].as_u64() {
            assert!(
                fragment.writes.len() as u64 >= min_writes,
                "case {name}: min writes (got {})",
                fragment.writes.len()
            );
        }
        if let Some(end) = case["expected_first_write_end"].as_u64() {
            assert_eq!(
                fragment.writes[0].end as u64, end,
                "case {name}: first write end"
            );
        }
        if let Some(delay) = case["expected_first_delay"].as_u64() {
            assert_eq!(
                fragment.writes[0].delay_ms as u64, delay,
                "case {name}: first delay"
            );
        }
        if let Some(delay) = case["expected_second_delay"].as_u64() {
            assert_eq!(
                fragment.writes[1].delay_ms as u64, delay,
                "case {name}: second delay"
            );
        }
        if let Some(chunk) = case["expected_chunk_bytes"].as_u64() {
            for write in &fragment.writes[..fragment.writes.len() - 1] {
                assert_eq!(
                    (write.end - write.start) as u64,
                    chunk,
                    "case {name}: chunk size"
                );
            }
        }
        // invariants that must hold for every plan
        let mut cursor = 0;
        for write in &fragment.writes {
            assert_eq!(write.start, cursor, "case {name}: contiguity");
            cursor = write.end;
        }
        assert_eq!(cursor, record.len(), "case {name}: coverage");
    }
}

#[test]
fn license_conformance_vectors() {
    let root = license_vector_data();
    let now =
        vor_core::license::parse_rfc3339_epoch(root["spec"]["now"].as_str().unwrap()).unwrap();
    let public_key = dev_public_key();
    assert_eq!(root["public_key"].as_str().unwrap(), public_key);
    for case in root["cases"].as_array().unwrap() {
        let token = case["token"].as_str().unwrap();
        let expected = case["expected"].as_str().unwrap();
        let result = vor_core::license::verify(&public_key, token, now);
        assert_eq!(
            result.status.as_str(),
            expected,
            "case {}",
            case["name"].as_str().unwrap()
        );
    }
}

#[test]
fn embedded_data_is_valid_json() {
    for text in [
        vor_core::ISP_PROFILES_JSON,
        vor_core::DPI_SIGNATURES_JSON,
        vor_core::CARRIER_PRESETS_JSON,
        vor_core::TRAFFIC_PROFILES_JSON,
    ] {
        serde_json::from_str::<Value>(text).expect("embedded json must be valid");
    }
}
