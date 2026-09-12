//! JNI surface for the **client core** — Kotlin object `com.vor.drm.VorDrmClient`.
//!
//! ```kotlin
//! object VorDrmClient {
//!     init { System.loadLibrary("vor_drm") }   // libvor_drm.so
//!     external fun nativeVersion(): String
//!     external fun nativeHwidHex(drmId: String, androidId: String, fingerprint: String): String
//!     external fun nativeVerify(
//!         token: String, publicKeyB64: String,
//!         persistedRatchet: Long, trusted: Long, deviceWall: Long,
//!         hwidDrmId: String, hwidAndroidId: String, hwidFingerprint: String,
//!     ): String   // JSON verdict
//!     external fun nativeTriage(force: Boolean): Boolean
//! }
//! ```
//!
//! Contract rules (magnifying-glass edition):
//!
//! * **never panics across the boundary** — every entry point is wrapped in
//!   `catch_unwind` and degrades to an `{"ok":false,…}` JSON string;
//! * **never blocks** — the frida port probe is the slowest path and is
//!   bounded to ~120 ms, cached for a minute;
//! * **stateless** — no globals mutated besides the guard/cache state, so
//!   the object is safe from any thread Kotlin calls it on;
//! * all results are JSON strings the Kotlin bridge maps into typed data.

use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jlong, jstring};
use jni::JNIEnv;

use crate::client::{self, VerifyInputs, VerifyOutcome};
use crate::guard;
use crate::hwid;

fn read_str(env: &mut JNIEnv, value: &JString) -> Option<String> {
    match env.get_string(value) {
        Ok(text) => Some(text.into()),
        Err(_) => None,
    }
}

/// Render a pre-built JSON document (or the shared error shape) into a Java
/// string; on (catastrophic) allocation failure returns null — the Kotlin
/// bridge treats null as "unavailable" and falls back.
fn write_json_str(env: &mut JNIEnv, text: String) -> jstring {
    match env.new_string(text) {
        Ok(output) => output.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn error_json(message: &str) -> String {
    format!(r#"{{"ok":false,"error":"{}"}}"#, message)
}

/// Outcome -> JSON (the Kotlin bridge's contract shape).
fn outcome_json(outcome: &VerifyOutcome) -> String {
    let mut json = String::with_capacity(320);
    json.push_str(r#"{"ok":true,"status":""#);
    json.push_str(outcome.status.as_str());
    json.push_str(r#"","now":"#);
    json.push_str(&outcome.now_used.to_string());
    json.push_str(r#","new_ratchet":"#);
    json.push_str(&outcome.new_ratchet.to_string());
    json.push_str(r#","rolled_back":"#);
    json.push_str(if outcome.rolled_back { "true" } else { "false" });
    json.push_str(r#","tampered":"#);
    json.push_str(if outcome.tampered { "true" } else { "false" });
    if let Some(info) = &outcome.info {
        json.push_str(r#","payload":{"id":""#);
        json.push_str(&json_escape(&info.id));
        json.push_str(r#"","issued_at":""#);
        json.push_str(&json_escape(&info.issued_at));
        json.push_str(r#"","expires_at":""#);
        json.push_str(&json_escape(&info.expires_at));
        json.push_str(r#"","expires_at_epoch":"#);
        json.push_str(&info.expires_at_epoch.to_string());
        json.push_str(r#","bandwidth_limit_mib":"#);
        json.push_str(&info.bandwidth_limit_mib.to_string());
        json.push_str(r#","hwid_locked":"#);
        json.push_str(if info.hwid_locked { "true" } else { "false" });
        json.push_str(r#","generation":"#);
        json.push_str(&info.generation.to_string());
        if let Some(tier) = &info.tier {
            json.push_str(r#","tier":""#);
            json.push_str(&json_escape(tier));
            json.push('"');
        }
        if !info.platforms.is_empty() {
            json.push_str(r#","platforms":["#);
            for (index, platform) in info.platforms.iter().enumerate() {
                if index > 0 {
                    json.push(',');
                }
                json.push('"');
                json.push_str(&json_escape(platform));
                json.push('"');
            }
            json.push(']');
        }
        json.push('}');
    }
    json.push('}');
    json
}

/// Minimal JSON string escaping (control chars + quotes + backslash).
fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// JNI: library version + build info.
#[no_mangle]
pub extern "system" fn Java_com_vor_drm_VorDrmClient_nativeVersion(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        format!(
            r#"{{"ok":true,"version":"{}","core":"vor-drm","generation":2}}"#,
            crate::VERSION
        )
    }))
    .unwrap_or_else(|_| error_json("panic"));
    write_json_str(&mut env, body)
}

/// JNI: derive the hardware fingerprint digest (64 lowercase hex chars).
#[no_mangle]
pub extern "system" fn Java_com_vor_drm_VorDrmClient_nativeHwidHex(
    mut env: JNIEnv,
    _class: JClass,
    drm_id: JString,
    android_id: JString,
    fingerprint: JString,
) -> jstring {
    let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let drm = read_str(&mut env, &drm_id).unwrap_or_default();
        let aid = read_str(&mut env, &android_id).unwrap_or_default();
        let fp = read_str(&mut env, &fingerprint).unwrap_or_default();
        let digest = hwid::derive(&drm, &aid, &fp);
        format!(r#"{{"ok":true,"hwid":"{}"}}"#, hwid::to_hex(&digest))
    }))
    .unwrap_or_else(|_| error_json("panic"));
    write_json_str(&mut env, body)
}

/// JNI: verify a token (the full enforcement ladder — see `client` docs).
///
/// `publicKeyB64` empty = use the baked-in build key. The Kotlin bridge
/// normally passes the BuildConfig key so gradle stays the single source
/// of truth in CI.
#[no_mangle]
pub extern "system" fn Java_com_vor_drm_VorDrmClient_nativeVerify(
    mut env: JNIEnv,
    _class: JClass,
    token: JString,
    public_key_b64: JString,
    persisted_ratchet: jlong,
    trusted: jlong,
    device_wall: jlong,
    hwid_drm_id: JString,
    hwid_android_id: JString,
    hwid_fingerprint: JString,
) -> jstring {
    let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(token) = read_str(&mut env, &token) else {
            return error_json("missing token");
        };
        let key = read_str(&mut env, &public_key_b64).unwrap_or_default();
        let drm = read_str(&mut env, &hwid_drm_id).unwrap_or_default();
        let aid = read_str(&mut env, &hwid_android_id).unwrap_or_default();
        let fp = read_str(&mut env, &hwid_fingerprint).unwrap_or_default();
        let inputs = VerifyInputs {
            token: &token,
            public_key_b64: &key,
            device_wall,
            persisted_ratchet,
            trusted,
            boot: 0, // read BOOTTIME natively (eluded: clock::boottime_secs)
            device_hwid: Some((&drm, &aid, &fp)),
            platform: "android",
        };
        outcome_json(&client::verify(&inputs))
    }))
    .unwrap_or_else(|_| error_json("panic"));
    write_json_str(&mut env, body)
}

/// JNI: anti-analysis triage (diagnostics / pre-flight gate in the UI).
#[no_mangle]
pub extern "system" fn Java_com_vor_drm_VorDrmClient_nativeTriage(
    _env: JNIEnv,
    _class: JClass,
    force: jboolean,
) -> jboolean {
    let tampered = guard::environment_tampered(force != 0);
    // Best-effort dump hardening once per process while we are here.
    guard::harden_process();
    if tampered {
        guard::register_strike();
    }
    jboolean::from(tampered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::LicenseStatus;
    #[test]
    fn json_escape_handles_hostile_strings() {
        assert_eq!(json_escape("plain"), "plain");
        assert_eq!(json_escape("a\"b"), "a\\\"b");
        assert_eq!(json_escape("a\\b"), "a\\\\b");
        assert_eq!(json_escape("a\nb"), "a\\nb");
        assert_eq!(json_escape("\u{1}"), "\\u0001");
        assert_eq!(json_escape("مرحبا"), "مرحبا");
    }

    #[test]
    fn outcome_json_shape_is_parseable() {
        let outcome = VerifyOutcome {
            status: LicenseStatus::Valid,
            info: Some(crate::client::LicenseInfo {
                id: "x\"y".to_string(),
                issued_at: "2026-01-01T00:00:00Z".to_string(),
                expires_at: "2027-01-01T00:00:00Z".to_string(),
                expires_at_epoch: 1_798_761_600,
                tier: Some("vip".to_string()),
                platforms: vec!["android".to_string()],
                bandwidth_limit_mib: 1024,
                hwid_locked: true,
                generation: 2,
            }),
            now_used: 1_500_000,
            new_ratchet: 1_500_000,
            rolled_back: false,
            tampered: false,
        };
        let json = outcome_json(&outcome);
        let parsed = crate::json::Json::parse(&json).expect("valid JSON");
        assert_eq!(parsed.obj_str("status"), Some("VALID"));
        let payload = parsed.get("payload").expect("payload");
        assert_eq!(payload.obj_str("id"), Some("x\"y"));
        assert_eq!(payload.obj_int("expires_at_epoch"), Some(1_798_761_600));
        assert_eq!(payload.obj_str("tier"), Some("vip"));
    }
}
