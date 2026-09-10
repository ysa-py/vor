//! Android JNI and UniFFI Bridge for Directive v70 Dual-Mode Transport Core.

use std::sync::Mutex;
use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jint, jlong, jstring};
use crate::benchmarks::DualModeBenchmarkHarness;
use crate::config::{DualModeConfig, TransportMode};
use crate::scheduler::PathMetrics;
use crate::tun_loop::TunDualModeController;

static CONTROLLER: Mutex<Option<TunDualModeController>> = Mutex::new(None);

/// JNI: Initialize Dual-Mode Transport Core with JSON configuration.
#[no_mangle]
pub extern "system" fn Java_com_unifiedshield_CoreBridge_nativeInitDualModeTransport(
    mut env: JNIEnv,
    _class: JClass,
    config_json: JString,
) -> jboolean {
    let json_str: String = match env.get_string(&config_json) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };

    let config: DualModeConfig = serde_json::from_str(&json_str).unwrap_or_default();
    let controller = TunDualModeController::new(config);

    if let Ok(mut lock) = CONTROLLER.lock() {
        *lock = Some(controller);
        1
    } else {
        0
    }
}

/// JNI: Switch active transport mode (0 = Mode A Fast Multipath, 1 = Mode B Layered 5-Hop).
#[no_mangle]
pub extern "system" fn Java_com_unifiedshield_CoreBridge_nativeSwitchTransportMode(
    _env: JNIEnv,
    _class: JClass,
    mode_id: jint,
) -> jint {
    let mode = match mode_id {
        1 => TransportMode::ModeBLayered,
        _ => TransportMode::ModeAFast,
    };

    if let Ok(mut lock) = CONTROLLER.lock() {
        if let Some(ctrl) = lock.as_mut() {
            ctrl.switch_mode(mode);
            return 1;
        }
    }
    0
}

/// JNI: Retrieve live Dual-Mode telemetry JSON.
#[no_mangle]
pub extern "system" fn Java_com_unifiedshield_CoreBridge_nativeGetDualModeTelemetry(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let telemetry_json = if let Ok(lock) = CONTROLLER.lock() {
        if let Some(ctrl) = lock.as_ref() {
            let metrics_a = ctrl.mode_a_engine().get_metrics();
            let hops_b = ctrl.mode_b_engine().get_hops();
            let mode = ctrl.current_mode();

            serde_json::json!({
                "current_mode": match mode {
                    TransportMode::ModeAFast => "mode_a_fast",
                    TransportMode::ModeBLayered => "mode_b_layered",
                },
                "mode_a_paths": metrics_a,
                "mode_b_hops_count": hops_b.len(),
                "mode_b_fail_closed": true,
                "status": "ACTIVE"
            }).to_string()
        } else {
            "{\"status\": \"FALLBACK_KOTLIN\"}".to_string()
        }
    } else {
        "{\"status\": \"ERROR\"}".to_string()
    };

    let output = env.new_string(telemetry_json).unwrap();
    output.into_raw()
}

/// JNI: Run in-engine benchmark harness.
#[no_mangle]
pub extern "system" fn Java_com_unifiedshield_CoreBridge_nativeRunDualModeBenchmark(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let results = DualModeBenchmarkHarness::run_full_suite();
    let json_res = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
    let output = env.new_string(json_res).unwrap();
    output.into_raw()
}
