//! Android JNI bindings (`feature = "jni"`).
//!
//! Kotlin class `com.vor.core.VorCoreNative`:
//!
//! ```kotlin
//! object VorCoreNative {
//!     init { System.loadLibrary("vor_core") }   // libvor_core.so
//!     external fun coreVersion(): String
//!     external fun engineCatalog(): String
//!     external fun ispLookup(asn: String): String
//!     external fun carrierLookup(keyOrAsn: String): String
//!     external fun decide(stateJson: String, contextJson: String): String
//!     external fun observe(stateJson: String, outcomeJson: String): String
//!     external fun fragmentPlan(recordHex: String, strategy: String, delayMs: Int): String
//!     external fun threatLevel(observationJson: String): String
//!     external fun licenseVerify(publicKeyB64: String, token: String, nowEpoch: Long): String
//!     external fun licenseNowEpoch(): Long
//! }
//! ```
//!
//! All arguments and returns are JSON strings (mirroring the C ABI), so the
//! Kotlin side stays a thin adapter. The same decision model also exists as
//! a pure-Kotlin interpreter (`android` JVM tests) — the JNI native path is
//! the production fast path, the interpreter is the fallback when the
//! `.so` is unavailable and the cross-check in unit tests.

use jni::objects::{JClass, JString};
use jni::sys::{jint, jlong, jstring};
use jni::JNIEnv;

fn read_string(env: &mut JNIEnv, input: &JString) -> Option<String> {
    match env.get_string(input) {
        Ok(text) => Some(text.into()),
        Err(_) => None,
    }
}

fn write_json(env: &mut JNIEnv, value: serde_json::Value) -> jstring {
    let text = value.to_string();
    match env.new_string(text) {
        Ok(output) => output.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn error_json(env: &mut JNIEnv, message: &str) -> jstring {
    write_json(env, serde_json::json!({"ok": false, "error": message}))
}

/// JNI: library version.
#[no_mangle]
pub extern "system" fn Java_com_vor_core_VorCoreNative_coreVersion(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    write_json(&mut env, serde_json::json!(crate::VERSION))
}

/// JNI: engine catalog JSON.
#[no_mangle]
pub extern "system" fn Java_com_vor_core_VorCoreNative_engineCatalog(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    write_json(&mut env, serde_json::json!(crate::engine::catalog_json()))
}

/// JNI: ISP profile lookup by ASN.
#[no_mangle]
pub extern "system" fn Java_com_vor_core_VorCoreNative_ispLookup(
    mut env: JNIEnv,
    _class: JClass,
    asn: JString,
) -> jstring {
    let Some(asn) = read_string(&mut env, &asn) else {
        return error_json(&mut env, "missing asn");
    };
    let profile = crate::isp::isp_by_asn(&asn);
    write_json(&mut env, serde_json::to_value(&profile).unwrap_or_default())
}

/// JNI: carrier preset lookup by key or ASN.
#[no_mangle]
pub extern "system" fn Java_com_vor_core_VorCoreNative_carrierLookup(
    mut env: JNIEnv,
    _class: JClass,
    key: JString,
) -> jstring {
    let Some(key) = read_string(&mut env, &key) else {
        return error_json(&mut env, "missing carrier key");
    };
    match crate::isp::carrier_by_key_or_asn(&key) {
        Some(preset) => write_json(&mut env, serde_json::to_value(&preset).unwrap_or_default()),
        None => error_json(&mut env, "carrier not found"),
    }
}

/// JNI: compute the next adaptive decision.
#[no_mangle]
pub extern "system" fn Java_com_vor_core_VorCoreNative_decide(
    mut env: JNIEnv,
    _class: JClass,
    state_json: JString,
    context_json: JString,
) -> jstring {
    let (Some(state_text), Some(context_text)) = (
        read_string(&mut env, &state_json),
        read_string(&mut env, &context_json),
    ) else {
        return error_json(&mut env, "missing arguments");
    };
    let state: crate::selector::SelectorState = if state_text.trim().is_empty() {
        crate::selector::SelectorState::default()
    } else {
        match serde_json::from_str(&state_text) {
            Ok(value) => value,
            Err(error) => return error_json(&mut env, &format!("bad state json: {error}")),
        }
    };
    let context: crate::selector::DecisionContext = match serde_json::from_str(&context_text) {
        Ok(value) => value,
        Err(error) => return error_json(&mut env, &format!("bad context json: {error}")),
    };
    let decision = crate::selector::decide(&state, &context);
    write_json(
        &mut env,
        serde_json::to_value(&decision).unwrap_or_default(),
    )
}

/// JNI: fold a probe outcome into the state; returns the new state JSON.
#[no_mangle]
pub extern "system" fn Java_com_vor_core_VorCoreNative_observe(
    mut env: JNIEnv,
    _class: JClass,
    state_json: JString,
    outcome_json: JString,
) -> jstring {
    let (Some(state_text), Some(outcome_text)) = (
        read_string(&mut env, &state_json),
        read_string(&mut env, &outcome_json),
    ) else {
        return error_json(&mut env, "missing arguments");
    };
    let state: crate::selector::SelectorState = if state_text.trim().is_empty() {
        crate::selector::SelectorState::default()
    } else {
        match serde_json::from_str(&state_text) {
            Ok(value) => value,
            Err(error) => return error_json(&mut env, &format!("bad state json: {error}")),
        }
    };
    let outcome: crate::selector::Outcome = match serde_json::from_str(&outcome_text) {
        Ok(value) => value,
        Err(error) => return error_json(&mut env, &format!("bad outcome json: {error}")),
    };
    let next = crate::selector::observe(&state, &outcome);
    write_json(&mut env, serde_json::to_value(&next).unwrap_or_default())
}

/// JNI: ClientHello fragmentation plan (hex record in, plan JSON out).
#[no_mangle]
pub extern "system" fn Java_com_vor_core_VorCoreNative_fragmentPlan(
    mut env: JNIEnv,
    _class: JClass,
    record_hex: JString,
    strategy: JString,
    delay_ms: jint,
) -> jstring {
    use crate::fragment::Strategy;
    let (Some(hex), Some(strategy_name)) = (
        read_string(&mut env, &record_hex),
        read_string(&mut env, &strategy),
    ) else {
        return error_json(&mut env, "missing arguments");
    };
    let record = match hex_to_bytes(&hex) {
        Some(bytes) => bytes,
        None => return error_json(&mut env, "bad hex input"),
    };
    let plan = crate::fragment::plan(
        &record,
        Strategy::from_name(&strategy_name),
        delay_ms.max(0) as u32,
    );
    write_json(&mut env, serde_json::to_value(&plan).unwrap_or_default())
}

/// JNI: threat level classification from an observation JSON.
#[no_mangle]
pub extern "system" fn Java_com_vor_core_VorCoreNative_threatLevel(
    mut env: JNIEnv,
    _class: JClass,
    observation_json: JString,
) -> jstring {
    let Some(text) = read_string(&mut env, &observation_json) else {
        return error_json(&mut env, "missing observation");
    };
    let observation: crate::threat::ThreatObservation =
        serde_json::from_str(&text).unwrap_or_default();
    let level = observation.level();
    write_json(
        &mut env,
        serde_json::json!({
            "level": level.as_str_value(),
            "requires_obfuscation": level.requires_obfuscation(),
            "requires_aggressive": level.requires_aggressive(),
            "min_hops": level.min_hops(),
        }),
    )
}

/// JNI: offline license verification.
#[no_mangle]
pub extern "system" fn Java_com_vor_core_VorCoreNative_licenseVerify(
    mut env: JNIEnv,
    _class: JClass,
    public_key_b64: JString,
    token: JString,
    now_epoch: jlong,
) -> jstring {
    let (Some(key), Some(token)) = (
        read_string(&mut env, &public_key_b64),
        read_string(&mut env, &token),
    ) else {
        return error_json(&mut env, "missing arguments");
    };
    let now = if now_epoch == 0 {
        crate::license::now_epoch_secs()
    } else {
        now_epoch
    };
    let result = crate::license::verify(&key, &token, now);
    write_json(
        &mut env,
        serde_json::json!({
            "status": result.status.as_str(),
            "payload": result.payload,
        }),
    )
}

/// JNI: current unix epoch seconds (for clock-skew-aware callers).
#[no_mangle]
pub extern "system" fn Java_com_vor_core_VorCoreNative_licenseNowEpoch(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    crate::license::now_epoch_secs()
}

fn hex_to_bytes(text: &str) -> Option<Vec<u8>> {
    let text: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    for pair in text.as_bytes().chunks(2) {
        let high = hex_value(pair[0])?;
        let low = hex_value(pair[1])?;
        out.push((high << 4) | low);
    }
    Some(out)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
