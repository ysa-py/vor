//! Cross-implementation conformance: the shared `license/vectors.json`.
//!
//! Every Vor port (Python reference, Kotlin verifier, iOS, desktop Go)
//! verifies this exact vector set; this file pins the Rust `vor-drm`
//! client core to the same answers, so a VOR1 license issued by any port
//! expires at the same instant on every platform.
//!
//! The vectors live OUTSIDE the crate (`license/vectors.json`, repo root)
//! because they are the cross-platform contract, not crate-local data.

#[cfg(feature = "client")]
mod vectors {
    use vor_drm::client::{self, LicenseStatus, VerifyInputs};

    /// Repo-root license directory, resolved from CARGO_MANIFEST_DIR
    /// (core/vor-drm -> ../../license).
    fn repo_path(relative: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }

    fn epoch_of_rfc3339(text: &str) -> i64 {
        vor_drm::rfc3339::parse_epoch(text).expect("vector spec.now must parse")
    }

    #[derive(Default)]
    struct Tally {
        passed: usize,
        failed: Vec<String>,
    }

    #[test]
    fn all_shared_vectors_agree() {
        let raw = std::fs::read_to_string(repo_path("license/vectors.json"))
            .expect("license/vectors.json must be readable from the crate");
        let root = vor_drm::json::Json::parse(&raw).expect("vectors.json must be valid JSON");

        let spec = root.get("spec").expect("spec object");
        let now_epoch = epoch_of_rfc3339(spec.obj_str("now").expect("spec.now"));
        let public_key = root.obj_str("public_key").expect("public_key").to_string();
        // The vector key must match the committed dev key — otherwise the
        // vectors and the baked default have drifted apart.
        let dev_key_file = repo_path("license/keys/dev/VOR_LICENSE_PUBLIC_KEY.txt");
        let dev_key = std::fs::read_to_string(&dev_key_file)
            .expect("license/keys/dev/VOR_LICENSE_PUBLIC_KEY.txt must exist")
            .trim()
            .to_string();
        assert_eq!(public_key, dev_key, "vector key != committed dev key");

        let cases = root
            .get("cases")
            .and_then(vor_drm::json::Json::as_arr)
            .expect("cases array");
        assert!(!cases.is_empty(), "conformance vector set is empty");

        let mut tally = Tally::default();
        for case in cases {
            let name = case.obj_str("name").unwrap_or("<unnamed>");
            let token = case.obj_str("token").unwrap_or("");
            let expected = case.obj_str("expected").unwrap_or("");
            let wanted = match expected {
                "VALID" => LicenseStatus::Valid,
                "EXPIRED" => LicenseStatus::Expired,
                "INVALID" => LicenseStatus::Invalid,
                other => panic!("unknown expected status {other:?} in vector {name:?}"),
            };

            // A fixed (wall, boot) pair for every case: the watch never
            // sees movement, so only the signed payload decides.
            let inputs = VerifyInputs {
                token,
                public_key_b64: &public_key,
                device_wall: now_epoch,
                persisted_ratchet: 0,
                trusted: 0,
                boot: 4242,
                device_hwid: None,
                platform: "android",
            };
            let outcome = client::verify(&inputs);
            let got = outcome.status;
            if got == wanted {
                tally.passed += 1;
            } else {
                tally.failed.push(format!(
                    "{name}: got {}, want {}",
                    got.as_str(),
                    wanted.as_str()
                ));
            }
        }

        assert!(
            tally.failed.is_empty(),
            "{} of {} vectors FAILED:\n{}",
            tally.failed.len(),
            cases.len(),
            tally.failed.join("\n")
        );
        eprintln!(
            "conformance: {}/{} vectors passed",
            tally.passed,
            cases.len()
        );
    }

    /// The vector key must be a VALID Ed25519 point — a corrupt committed
    /// key would fail every verification at release time.
    #[test]
    fn vector_public_key_is_a_valid_point() {
        let raw = std::fs::read_to_string(repo_path("license/vectors.json")).unwrap();
        let root = vor_drm::json::Json::parse(&raw).unwrap();
        let public_key = root.obj_str("public_key").unwrap();
        let bytes = vor_drm::b64::decode(public_key).expect("public key must be base64url");
        let array: [u8; 32] = bytes.try_into().expect("public key must be 32 bytes");
        assert!(ed25519_dalek::VerifyingKey::from_bytes(&array).is_ok());
    }

    /// spec.now of the vector file must round-trip through our RFC 3339
    /// codec — the same instant on every platform, bit for bit.
    #[test]
    fn spec_now_parses_to_the_canonical_epoch() {
        let raw = std::fs::read_to_string(repo_path("license/vectors.json")).unwrap();
        let root = vor_drm::json::Json::parse(&raw).unwrap();
        let now = root
            .get("spec")
            .and_then(|spec| spec.obj_str("now"))
            .expect("spec.now");
        assert_eq!(now, "2026-09-10T12:00:00Z");
        assert_eq!(
            vor_drm::rfc3339::parse_epoch(now),
            Some(1_789_041_600),
            "spec.now epoch drifted from the canonical value"
        );
        // And format_utc is its exact inverse (whole-second, always Z).
        assert_eq!(vor_drm::rfc3339::format_utc(1_789_041_600), now);
    }
}
