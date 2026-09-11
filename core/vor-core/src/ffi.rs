//! C ABI — the shared-library surface every native host uses.
//!
//! All functions are **stateless**: state travels in and out as JSON strings
//! owned by the caller. No handles, no global mutable state, no locks — the
//! host (Android JNI layer, Go cgo wrapper, OpenWrt C daemon) manages
//! persistence and threading. This keeps the ABI trivially stable and
//! thread-safe.
//!
//! Error contract: every function returns a JSON object; on failure the
//! object is `{"ok":false,"error":"..."}` — hosts check `ok` first.

use crate::fragment::{self, Strategy};
use crate::license;
use crate::selector;
use crate::threat;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

/// Returns the library version (static string, do NOT free).
#[no_mangle]
pub extern "C" fn vor_core_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

/// Returns the embedded engine catalog JSON (static, do NOT free).
#[no_mangle]
pub extern "C" fn vor_engine_catalog() -> *const c_char {
    // Leaked once — the catalog never changes during a process lifetime.
    static CATALOG: std::sync::OnceLock<CString> = std::sync::OnceLock::new();
    CATALOG
        .get_or_init(|| CString::new(crate::engine::catalog_json()).expect("catalog cstring"))
        .as_ptr()
}

/// Look up an ISP profile by ASN ("AS41689"). Caller frees with
/// [`vor_free_string`].
#[no_mangle]
pub extern "C" fn vor_isp_lookup(asn: *const c_char) -> *mut c_char {
    let asn = unsafe_cstr(asn).unwrap_or("");
    let profile = crate::isp::isp_by_asn(asn);
    return_json(serde_json::to_value(&profile).unwrap_or_default())
}

/// Look up a carrier preset by carrier key or ASN. Caller frees.
#[no_mangle]
pub extern "C" fn vor_carrier_lookup(key_or_asn: *const c_char) -> *mut c_char {
    let key = unsafe_cstr(key_or_asn).unwrap_or("");
    match crate::isp::carrier_by_key_or_asn(key) {
        Some(preset) => return_json(serde_json::to_value(&preset).unwrap_or_default()),
        None => return_json(serde_json::json!({"ok": false, "error": "carrier not found"})),
    }
}

/// Compute the next decision. `context_json` is a `DecisionContext`
/// (see `selector.rs`). Caller frees.
#[no_mangle]
pub extern "C" fn vor_decide(
    state_json: *const c_char,
    context_json: *const c_char,
) -> *mut c_char {
    let state_text = unsafe_cstr(state_json).unwrap_or("");
    let context_text = unsafe_cstr(context_json).unwrap_or("");
    let state: selector::SelectorState = if state_text.trim().is_empty() {
        selector::SelectorState::default()
    } else {
        match serde_json::from_str(state_text) {
            Ok(value) => value,
            Err(error) => return error_json(format!("bad state json: {error}")),
        }
    };
    let context: selector::DecisionContext = match serde_json::from_str(context_text) {
        Ok(value) => value,
        Err(error) => return error_json(format!("bad context json: {error}")),
    };
    let decision = selector::decide(&state, &context);
    return_json(serde_json::to_value(&decision).unwrap_or_default())
}

/// Fold a probe outcome into the state; returns the new state JSON.
/// Caller frees.
#[no_mangle]
pub extern "C" fn vor_observe(
    state_json: *const c_char,
    outcome_json: *const c_char,
) -> *mut c_char {
    let state_text = unsafe_cstr(state_json).unwrap_or("");
    let outcome_text = unsafe_cstr(outcome_json).unwrap_or("");
    let state: selector::SelectorState = if state_text.trim().is_empty() {
        selector::SelectorState::default()
    } else {
        match serde_json::from_str(state_text) {
            Ok(value) => value,
            Err(error) => return error_json(format!("bad state json: {error}")),
        }
    };
    let outcome: selector::Outcome = match serde_json::from_str(outcome_text) {
        Ok(value) => value,
        Err(error) => return error_json(format!("bad outcome json: {error}")),
    };
    let next = selector::observe(&state, &outcome);
    return_json(serde_json::to_value(&next).unwrap_or_default())
}

/// Build a fragmentation plan for a ClientHello. `record_hex` is the full
/// TLS record hex-encoded; `strategy` is a strategy name; `delay_ms` is the
/// inter-write delay. Caller frees.
#[no_mangle]
pub extern "C" fn vor_fragment_plan(
    record_hex: *const c_char,
    strategy: *const c_char,
    delay_ms: c_int,
) -> *mut c_char {
    let hex = unsafe_cstr(record_hex).unwrap_or("");
    let strategy_name = unsafe_cstr(strategy).unwrap_or("raw");
    let record = match hex_decode(hex) {
        Some(bytes) => bytes,
        None => return error_json("bad hex input".to_string()),
    };
    let plan = fragment::plan(
        &record,
        Strategy::from_name(strategy_name),
        delay_ms.max(0) as u32,
    );
    return_json(serde_json::to_value(&plan).unwrap_or_default())
}

/// Classify a threat level from a `ThreatObservation` JSON. Caller frees.
#[no_mangle]
pub extern "C" fn vor_threat_level(observation_json: *const c_char) -> *mut c_char {
    let text = unsafe_cstr(observation_json).unwrap_or("{}");
    let observation: threat::ThreatObservation = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(error) => return error_json(format!("bad observation json: {error}")),
    };
    let level = observation.level();
    serde_json::json!({
        "level": level.as_str_value(),
        "requires_obfuscation": level.requires_obfuscation(),
        "requires_aggressive": level.requires_aggressive(),
        "min_hops": level.min_hops(),
    })
    .to_string()
    .into_c_json()
}

/// Verify a license token. `public_key_b64` is the base64url Ed25519 public
/// key; `now_epoch` is unix seconds (0 = use the device clock). Caller frees.
#[no_mangle]
pub extern "C" fn vor_license_verify(
    public_key_b64: *const c_char,
    token: *const c_char,
    now_epoch: i64,
) -> *mut c_char {
    let key = unsafe_cstr(public_key_b64).unwrap_or("");
    let token = unsafe_cstr(token).unwrap_or("");
    let now = if now_epoch == 0 {
        license::now_epoch_secs()
    } else {
        now_epoch
    };
    let result = license::verify(key, token, now);
    serde_json::json!({
        "status": result.status.as_str(),
        "payload": result.payload,
    })
    .to_string()
    .into_c_json()
}

/// Free any string returned by this library (except `vor_core_version` /
/// `vor_engine_catalog`, which are static).
///
/// # Safety
/// `string` must be a pointer returned by one of the functions in this
/// module, and must not already have been freed.
#[no_mangle]
pub unsafe extern "C" fn vor_free_string(string: *mut c_char) {
    if !string.is_null() {
        drop(CString::from_raw(string));
    }
}

// ---------------------------------------------------------------- helpers

trait IntoCJson {
    fn into_c_json(self) -> *mut c_char;
}

impl IntoCJson for String {
    fn into_c_json(self) -> *mut c_char {
        match CString::new(self) {
            Ok(cstring) => cstring.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    }
}

fn unsafe_cstr(pointer: *const c_char) -> Option<&'static str> {
    if pointer.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(pointer).to_str().ok() }
}

fn return_json(value: serde_json::Value) -> *mut c_char {
    value.to_string().into_c_json()
}

fn error_json(message: String) -> *mut c_char {
    serde_json::json!({"ok": false, "error": message})
        .to_string()
        .into_c_json()
}

fn hex_decode(text: &str) -> Option<Vec<u8>> {
    let text: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    let bytes = text.as_bytes();
    for pair in bytes.chunks(2) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn to_c(text: &str) -> *const c_char {
        CString::new(text).unwrap().into_raw() as *const c_char
    }

    fn from_c(pointer: *mut c_char) -> String {
        unsafe { CStr::from_ptr(pointer).to_string_lossy().into_owned() }
    }

    #[test]
    fn version_string_is_static() {
        let version = vor_core_version();
        let text = unsafe { CStr::from_ptr(version).to_str().unwrap() };
        // Must always match the crate version — never a stale hardcode.
        assert_eq!(text, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn decide_ffi_roundtrip() {
        let context = serde_json::json!({
            "carrier": "mci",
            "fingerprint": "test",
            "threat": {},
            "available_engines": ["xray", "sni-tunnel"],
        })
        .to_string();
        let context_c = to_c(&context);
        let pointer = vor_decide(std::ptr::null(), context_c);
        assert!(!pointer.is_null());
        let decision = from_c(pointer);
        let parsed: serde_json::Value = serde_json::from_str(&decision).unwrap();
        assert!(parsed["engine"].is_string());
        assert!(!parsed["strategies"].as_array().unwrap().is_empty());
        unsafe { vor_free_string(pointer) };
        unsafe { vor_free_string(context_c as *mut c_char) };
    }

    #[test]
    fn fragment_plan_ffi() {
        // 0x16 record header + junk that is not a valid hello -> passthrough
        let record = format!("16030100{}", "00".repeat(20));
        let record_c = to_c(&record);
        let strategy_c = to_c("record_split");
        let pointer = vor_fragment_plan(record_c, strategy_c, 0);
        let plan = from_c(pointer);
        let parsed: serde_json::Value = serde_json::from_str(&plan).unwrap();
        assert!(parsed["writes"].is_array());
        unsafe { vor_free_string(pointer) };
        unsafe { vor_free_string(record_c as *mut c_char) };
        unsafe { vor_free_string(strategy_c as *mut c_char) };
    }

    #[test]
    fn license_ffi_rejects_garbage() {
        let key = crate::b64::encode(&[0u8; 32]);
        let key_c = to_c(&key);
        let token_c = to_c("VOR1.junk.junk");
        let pointer = vor_license_verify(key_c, token_c, 1789041600);
        let result = from_c(pointer);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["status"], "INVALID");
        unsafe { vor_free_string(pointer) };
        unsafe { vor_free_string(key_c as *mut c_char) };
        unsafe { vor_free_string(token_c as *mut c_char) };
    }
}
