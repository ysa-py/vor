// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield 6.0 — Build Script
//
// Compiles protobuf definitions and sets platform-specific cfg flags.
// Embeds resource JSON files at compile time via include_str!.
// ─────────────────────────────────────────────────────────────────────────────

use std::env;
use std::path::PathBuf;

fn main() {
    // ── Determine output directory ─────────────────────────────────────────
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    // ── Compile protobuf definitions ───────────────────────────────────────
    let proto_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("proto");

    if proto_dir.exists() {
        let proto_files: Vec<PathBuf> = walk_proto_dir(&proto_dir);
        if !proto_files.is_empty() {
            prost_build::Config::new()
                .out_dir(&out_dir)
                .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
                .compile_protos(
                    &proto_files.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
                    &[proto_dir.as_path()],
                )
                .expect("Failed to compile protobuf definitions");
        }
    }

    // ── Embed resource JSON at compile time ────────────────────────────────
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let cdn_endpoints_path = manifest_dir.join("resources/cdn-endpoints.json");
    let p2p_peers_path = manifest_dir.join("resources/p2p-bootstrap-peers.json");

    // Validate that embedded resources are valid JSON at build time
    if cdn_endpoints_path.exists() {
        let content = std::fs::read_to_string(&cdn_endpoints_path)
            .expect("Failed to read cdn-endpoints.json");
        serde_json::from_str::<serde_json::Value>(&content)
            .expect("cdn-endpoints.json is not valid JSON");
        println!(
            "cargo:rustc-env=CDN_ENDPOINTS_JSON={}",
            cdn_endpoints_path.display()
        );
    }

    if p2p_peers_path.exists() {
        let content = std::fs::read_to_string(&p2p_peers_path)
            .expect("Failed to read p2p-bootstrap-peers.json");
        serde_json::from_str::<serde_json::Value>(&content)
            .expect("p2p-bootstrap-peers.json is not valid JSON");
        println!(
            "cargo:rustc-env=P2P_BOOTSTRAP_PEERS_JSON={}",
            p2p_peers_path.display()
        );
    }

    // ── Platform detection and cfg flags ───────────────────────────────────
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    match target_os.as_str() {
        "linux" => {
            // Check if targeting Android (Linux + android feature)
            if env::var("CARGO_FEATURE_PLATFORM_ANDROID").is_ok() {
                println!("cargo:rustc-cfg=platform_android");
                println!("cargo:rustc-cfg=platform_mobile");
            } else if env::var("CARGO_FEATURE_PLATFORM_OPENWRT").is_ok() {
                println!("cargo:rustc-cfg=platform_openwrt");
                println!("cargo:rustc-cfg=platform_embedded");
            } else {
                println!("cargo:rustc-cfg=platform_linux");
                println!("cargo:rustc-cfg=platform_desktop");
            }
        }
        "windows" => {
            println!("cargo:rustc-cfg=platform_windows");
            println!("cargo:rustc-cfg=platform_desktop");
        }
        "macos" => {
            println!("cargo:rustc-cfg=platform_macos");
            println!("cargo:rustc-cfg=platform_desktop");
        }
        "ios" => {
            println!("cargo:rustc-cfg=platform_ios");
            println!("cargo:rustc-cfg=platform_mobile");
        }
        "android" => {
            println!("cargo:rustc-cfg=platform_android");
            println!("cargo:rustc-cfg=platform_mobile");
        }
        _ => {
            println!("cargo:rustc-cfg=platform_unknown");
        }
    }

    match target_arch.as_str() {
        "x86_64" | "x86" => {
            println!("cargo:rustc-cfg=arch_x86");
        }
        "aarch64" | "arm" => {
            println!("cargo:rustc-cfg=arch_arm");
        }
        "riscv64" | "riscv32" => {
            println!("cargo:rustc-cfg=arch_riscv");
        }
        _ => {}
    }

    // ── Feature-based cfg flags ────────────────────────────────────────────
    if env::var("CARGO_FEATURE_AI_INFERENCE").is_ok() {
        println!("cargo:rustc-cfg=feature_ai_inference");
    }
    if env::var("CARGO_FEATURE_USERSPACE_TUNNEL").is_ok() {
        println!("cargo:rustc-cfg=feature_userspace_tunnel");
    }
    if env::var("CARGO_FEATURE_HARDENED_MEMORY").is_ok() {
        println!("cargo:rustc-cfg=feature_hardened_memory");
    }
    if env::var("CARGO_FEATURE_POST_QUANTUM").is_ok() {
        println!("cargo:rustc-cfg=feature_post_quantum");
    }

    // ── Linker hints for size-optimised builds ─────────────────────────────
    if target_os == "linux" && env::var("PROFILE").unwrap_or_default() == "release" {
        println!("cargo:rustc-link-arg=-s"); // Strip symbols
    }

    // ── Re-run if resources change ─────────────────────────────────────────
    println!("cargo:rerun-if-changed=resources/cdn-endpoints.json");
    println!("cargo:rerun-if-changed=resources/p2p-bootstrap-peers.json");
    println!("cargo:rerun-if-changed=proto/");
    println!("cargo:rerun-if-changed=build.rs");
}

/// Recursively collect .proto files from a directory.
fn walk_proto_dir(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                result.extend(walk_proto_dir(&path));
            } else if path.extension().is_some_and(|e| e == "proto") {
                result.push(path);
            }
        }
    }
    result
}
