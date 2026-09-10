//! Engine catalog — the registry every platform's engine selector renders.
//!
//! Engines are the runtime-switchable transports of the merged product
//! (requirement: "multiple selectable VPN/obfuscation engines, not force
//! one"). The catalog entry describes origin, availability and which
//! platform builds ship it, so the UI can render an honest list.

use serde::Serialize;

/// A catalog engine entry.
#[derive(Debug, Clone, Serialize)]
pub struct EngineInfo {
    /// Engine id (selector vocabulary).
    pub id: &'static str,
    /// Human label (English).
    pub label: &'static str,
    /// Human label (Persian).
    pub label_fa: &'static str,
    /// One-line description of what it does / where it came from.
    pub description: &'static str,
    /// Platforms where the engine can run.
    pub platforms: &'static [&'static str],
    /// Whether this engine needs a user-supplied server.
    pub needs_server: bool,
    /// Origin repository.
    pub origin: &'static str,
}

/// The full catalog.
pub const ENGINES: &[EngineInfo] = &[
    EngineInfo {
        id: "xray",
        label: "Xray core",
        label_fa: "هسته Xray",
        description: "In-process Xray (VLESS/VMess/Trojan/SS/REALITY) with in-core TUN. The default full-device engine.",
        platforms: &["android", "windows", "linux"],
        needs_server: true,
        origin: "V2RayEZ",
    },
    EngineInfo {
        id: "singbox",
        label: "sing-box core",
        label_fa: "هسته sing-box",
        description: "sing-box process core: Hysteria2, TUIC v5, NaiveProxy, ShadowTLS and more.",
        platforms: &["android", "windows", "linux"],
        needs_server: true,
        origin: "V2RayEZ / EasySNI",
    },
    EngineInfo {
        id: "sni-tunnel",
        label: "SNI Tunnel (desync)",
        label_fa: "تونل SNI (desync)",
        description: "EasySNI Go engine: SNI spoofing, ClientHello fragmentation, fake ClientHello, wrong checksum/seq. Default Windows engine.",
        platforms: &["windows", "linux"],
        needs_server: false,
        origin: "EasySNI",
    },
    EngineInfo {
        id: "pattern",
        label: "Pattern engine (wrong-seq)",
        label_fa: "موتور الگو (wrong-seq)",
        description: "UAC pattern engine: WinDivert wrong-sequence fake-SNI injection + fragment matrix + carrier racing. Alternate Windows engine.",
        platforms: &["windows"],
        needs_server: false,
        origin: "UAC-SNI-Spoofer-Windows",
    },
    EngineInfo {
        id: "dns-tunnel-dnstt",
        label: "DNS tunnel (dnstt)",
        label_fa: "تونل DNS (dnstt)",
        description: "dnstt DNS tunnel addon engine.",
        platforms: &["android", "windows", "linux"],
        needs_server: true,
        origin: "V2RayEZ",
    },
    EngineInfo {
        id: "dns-tunnel-masterdns",
        label: "DNS tunnel (MasterDns)",
        label_fa: "تونل DNS (MasterDns)",
        description: "MasterDnsVPN DNS tunnel: TXT-qname uplink, ARQ, multipath resolvers, low header overhead.",
        platforms: &["android", "windows", "linux"],
        needs_server: true,
        origin: "MasterDnsVPN",
    },
    EngineInfo {
        id: "tor",
        label: "Tor",
        label_fa: "تور",
        description: "Embedded Tor with obfs4/snowflake/webtunnel bridges and exit-country selection.",
        platforms: &["android", "windows", "linux"],
        needs_server: false,
        origin: "V2RayEZ / MSN-GUARD",
    },
    EngineInfo {
        id: "psiphon",
        label: "Psiphon",
        label_fa: "سایفون",
        description: "Psiphon tunnel-core engine with the MSN-GUARD strategy ladder and region selection.",
        platforms: &["android", "windows", "linux"],
        needs_server: false,
        origin: "V2RayEZ / MSN-GUARD / EasySNI",
    },
    EngineInfo {
        id: "mitm-fronting",
        label: "MITM domain fronting",
        label_fa: "فرانتینگ MITM",
        description: "Client-side domain fronting MITM with local CA and fronted DoH.",
        platforms: &["android", "windows", "linux"],
        needs_server: false,
        origin: "V2RayEZ / EasySNI",
    },
    EngineInfo {
        id: "openwrt-proto",
        label: "OpenWrt netifd proto",
        label_fa: "پروتکل netifd اپن‌ورت",
        description: "OpenWrt netifd protocol handler driving the edge bridge + fragment planner from libvor-core.",
        platforms: &["openwrt"],
        needs_server: true,
        origin: "MICAFP (openwrt) + Vor",
    },
];

/// Serialize the catalog to JSON (used by FFI/JNI and the LuCI page).
pub fn catalog_json() -> String {
    serde_json::to_string_pretty(ENGINES).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_serializes() {
        let text = catalog_json();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();
        assert!(parsed.len() >= 10);
        assert!(parsed.iter().any(|e| e["id"] == "sni-tunnel"));
        assert!(parsed.iter().any(|e| e["id"] == "pattern"));
    }

    #[test]
    fn engine_ids_match_selector_vocabulary() {
        for engine in ENGINES {
            assert!(
                crate::selector::KNOWN_ENGINES.contains(&engine.id) || engine.id == "openwrt-proto",
                "catalog id {id} missing from selector vocabulary",
                id = engine.id
            );
        }
    }
}
