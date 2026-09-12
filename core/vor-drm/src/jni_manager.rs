//! JNI surface for the **manager core** — Kotlin object `com.vor.drm.VorDrmManager`.
//!
//! ```kotlin
//! object VorDrmManager {
//!     init { System.loadLibrary("vor_drm") }   // libvor_drm.so (manager flavor)
//!     external fun nativeVersion(): String
//!     external fun nativeGenerateKey(): ByteArray          // fresh 32-byte seed
//!     external fun nativePublicKeyOf(seed: ByteArray): String  // b64url
//!     external fun nativeIssueV2(seed: ByteArray, paramsJson: String): String
//!     external fun nativeIssueV2Batch(seed: ByteArray, batchJson: String): String
//!     external fun nativeVaultSeal(seed: ByteArray, passphrase: String): String
//!     external fun nativeVaultOpen(envelope: String, passphrase: String): ByteArray?
//!     external fun nativeSelfTest(seed: ByteArray): String
//! }
//! ```
//!
//! Seed handling: seeds cross the boundary as `byte[]` (never hex strings
//! in JVM immutable memory), are consumed into [`zeroize::Zeroizing`]
//! buffers on the Rust side, and are scrubbed on every exit path. The
//! Kotlin caller is expected to zero its own copy immediately after the
//! call (same discipline as the existing vault code).
//!
//! The escalation contract: repeated anti-analysis detections while a seed
//! is in use abort the process (see [`crate::manager::escalate_if_needed`])
//! — the session dies, nothing at rest is touched, nothing gets signed.

use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jbyteArray, jstring};
use jni::JNIEnv;
use zeroize::Zeroizing;

use crate::manager::{self, IssueSpec, SEED_LEN};

fn read_str(env: &mut JNIEnv, value: &JString) -> Option<String> {
    match env.get_string(value) {
        Ok(text) => Some(text.into()),
        Err(_) => None,
    }
}

fn write_json_str(env: &mut JNIEnv, text: String) -> jstring {
    match env.new_string(text) {
        Ok(output) => output.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn error_json(message: &str) -> String {
    format!(r#"{{"ok":false,"error":"{}"}}"#, message)
}

/// Read a 32-byte seed from a Java byte array into a zeroized buffer.
fn read_seed(env: &mut JNIEnv, bytes: &JByteArray) -> Option<Zeroizing<[u8; SEED_LEN]>> {
    let raw = env.convert_byte_array(bytes).ok()?;
    if raw.len() != SEED_LEN {
        return None;
    }
    let mut seed = Zeroizing::new([0u8; SEED_LEN]);
    for (slot, value) in seed.iter_mut().zip(raw) {
        *slot = value;
    }
    Some(seed)
}

/// JNI: library version + build info.
#[no_mangle]
pub extern "system" fn Java_com_vor_drm_VorDrmManager_nativeVersion(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        format!(
            r#"{{"ok":true,"version":"{}","core":"vor-drm","manager":true}}"#,
            crate::VERSION
        )
    }))
    .unwrap_or_else(|_| error_json("panic"));
    write_json_str(&mut env, body)
}

/// JNI: fresh Ed25519 seed (32 random bytes) for the caller to wrap into its
/// Keystore vault immediately.
#[no_mangle]
pub extern "system" fn Java_com_vor_drm_VorDrmManager_nativeGenerateKey(
    env: JNIEnv,
    _class: JClass,
) -> jbyteArray {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let fresh = manager::generate_seed()?;
        // Zeroize the Vec copy immediately after handing it to the JVM.
        let seed = Zeroizing::new(fresh);
        let mut raw = seed.to_vec();
        let array = env.byte_array_from_slice(&raw).ok()?;
        for slot in raw.iter_mut() {
            *slot = 0;
        }
        Some(array)
    }));
    match result {
        Ok(Some(array)) => array.into_raw(),
        _ => std::ptr::null_mut(),
    }
}

/// JNI: public key (base64url) of a seed.
#[no_mangle]
pub extern "system" fn Java_com_vor_drm_VorDrmManager_nativePublicKeyOf(
    mut env: JNIEnv,
    _class: JClass,
    seed_bytes: JByteArray,
) -> jstring {
    let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        match read_seed(&mut env, &seed_bytes) {
            Some(seed) => format!(
                r#"{{"ok":true,"public_b64url":"{}"}}"#,
                manager::public_key_b64(&seed)
            ),
            None => error_json("seed must be exactly 32 bytes"),
        }
    }))
    .unwrap_or_else(|_| error_json("panic"));
    write_json_str(&mut env, body)
}

/// JNI: issue one VOR2 token.
///
/// `paramsJson`: `{"id","tier","issued_at","expires_at","bandwidth_mib",
/// "platforms":[…],"hwid_hex"}` (epochs as numbers or RFC 3339 strings).
#[no_mangle]
pub extern "system" fn Java_com_vor_drm_VorDrmManager_nativeIssueV2(
    mut env: JNIEnv,
    _class: JClass,
    seed_bytes: JByteArray,
    params_json: JString,
) -> jstring {
    let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(seed) = read_seed(&mut env, &seed_bytes) else {
            return error_json("seed must be exactly 32 bytes");
        };
        let Some(params) = read_str(&mut env, &params_json) else {
            return error_json("missing params");
        };
        let Some(spec) = manager::parse_spec_json(&params) else {
            return error_json("params did not parse");
        };
        match manager::issue(&seed, &spec) {
            Ok(token) => format!(r#"{{"ok":true,"token":"{token}"}}"#),
            Err(error) => error_json(&error.reason),
        }
    }))
    .unwrap_or_else(|_| error_json("panic"));
    write_json_str(&mut env, body)
}

/// JNI: issue a batch.
///
/// `batchJson`: `{"specs":[<spec>,…]}` (same spec shape as the single issue;
/// the first invalid row names itself in the error).
#[no_mangle]
pub extern "system" fn Java_com_vor_drm_VorDrmManager_nativeIssueV2Batch(
    mut env: JNIEnv,
    _class: JClass,
    seed_bytes: JByteArray,
    batch_json: JString,
) -> jstring {
    let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(seed) = read_seed(&mut env, &seed_bytes) else {
            return error_json("seed must be exactly 32 bytes");
        };
        let Some(batch) = read_str(&mut env, &batch_json) else {
            return error_json("missing batch");
        };
        let Some(root) = crate::json::Json::parse(&batch) else {
            return error_json("batch did not parse");
        };
        let Some(items) = root.get("specs").and_then(crate::json::Json::as_arr) else {
            return error_json("batch must carry a specs array");
        };
        let mut specs: Vec<IssueSpec> = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            let parsed = match manager::spec_from_json(item) {
                Some(spec) => spec,
                None => return error_json(&format!("row {} did not parse", index + 1)),
            };
            specs.push(parsed);
        }
        match manager::issue_batch(&seed, &specs) {
            Ok(tokens) => {
                let joined: Vec<String> = tokens.iter().map(|t| format!("\"{t}\"")).collect();
                format!(r#"{{"ok":true,"tokens":[{}]}}"#, joined.join(","))
            }
            Err(error) => error_json(&format!("row {}: {}", error.index, error.reason)),
        }
    }))
    .unwrap_or_else(|_| error_json("panic"));
    write_json_str(&mut env, body)
}

/// JNI: seal a seed into a passphrase-protected portable vault envelope.
#[no_mangle]
pub extern "system" fn Java_com_vor_drm_VorDrmManager_nativeVaultSeal(
    mut env: JNIEnv,
    _class: JClass,
    seed_bytes: JByteArray,
    passphrase: JString,
) -> jstring {
    let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(seed) = read_seed(&mut env, &seed_bytes) else {
            return error_json("seed must be exactly 32 bytes");
        };
        let Some(pass) = read_str(&mut env, &passphrase) else {
            return error_json("missing passphrase");
        };
        match manager::vault_seal(&seed, &pass) {
            Some(envelope) => format!(r#"{{"ok":true,"envelope":"{envelope}"}}"#),
            None => error_json("seal failed"),
        }
    }))
    .unwrap_or_else(|_| error_json("panic"));
    write_json_str(&mut env, body)
}

/// JNI: open a portable vault envelope (null when the passphrase is wrong
/// or the envelope is corrupt).
#[no_mangle]
pub extern "system" fn Java_com_vor_drm_VorDrmManager_nativeVaultOpen(
    mut env: JNIEnv,
    _class: JClass,
    envelope: JString,
    passphrase: JString,
) -> jbyteArray {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let text = read_str(&mut env, &envelope)?;
        let pass = read_str(&mut env, &passphrase)?;
        manager::vault_open(&text, &pass)
    }));
    match result {
        Ok(Some(seed)) => match env.byte_array_from_slice(&seed) {
            Ok(array) => array.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        _ => std::ptr::null_mut(),
    }
}

/// JNI: sign-and-verify self test.
#[no_mangle]
pub extern "system" fn Java_com_vor_drm_VorDrmManager_nativeSelfTest(
    mut env: JNIEnv,
    _class: JClass,
    seed_bytes: JByteArray,
) -> jstring {
    let body = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        match read_seed(&mut env, &seed_bytes) {
            Some(seed) => {
                let (ok, detail) = manager::self_test(&seed);
                format!(
                    r#"{{"ok":{ok},"signs_and_verifies":{ok},"detail":"{}"}}"#,
                    detail
                )
            }
            None => error_json("seed must be exactly 32 bytes"),
        }
    }))
    .unwrap_or_else(|_| error_json("panic"));
    write_json_str(&mut env, body)
}

#[cfg(test)]
mod tests {
    use crate::json::Json;

    #[test]
    fn spec_from_json_accepts_parsed_rows() {
        let row = Json::parse(
            r#"{"id":"x","tier":"vip","issued_at":100,"expires_at":200,"bandwidth_mib":0,"platforms":[]}"#,
        )
        .expect("parse");
        assert!(crate::manager::spec_from_json(&row).is_some());
        assert!(crate::manager::spec_from_json(&Json::Null).is_none());
    }
}
