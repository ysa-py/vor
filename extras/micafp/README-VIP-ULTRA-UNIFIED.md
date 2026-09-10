# MICAFP UnifiedShield VIP-ULTRA 7.0

**Complete 8-Project Unified Merge — Zero Features Removed**

---

## Overview

This is the definitive unified release of the MICAFP UnifiedShield platform, produced by
merging all 8 source project variants into a single, coherent codebase without removing any
feature from any source project. Every module, transport, core, and utility from all 8
variants is present and accessible.

## Source Projects Merged

| # | Project | Unique Contributions |
|---|---------|---------------------|
| 1 | `unifiedshield-nextgen$` | Base daemon, cores (9), dashboard (full), extensions |
| 2 | `unifiedshield-nextgen@` | WebRTC relay, Chrome webrtc-relay.ts, iOS NetworkExtension |
| 3 | `MICAFP-UnifiedShield-&` | Chinese CDN, MQTT tunnel, Cloudflare worker+obfuscator+rate-limiter, Alibaba/Tencent/Baidu CDN workers |
| 4 | `MICAFP-UnifiedShield-*` | cores: defyx/hiddify/lantern/mahsang/moav/psiphon/singbox/xray/amneziavpn, obfuscation: traffic_shaper, packet_size_normalizer |
| 5 | `MICAFP-UnifiedShield-+` | Battery module, scanner module, security module, platform abstraction, RL transport selector, adversarial traffic GAN, BLE mesh, acoustic covert, NTP covert, SMS bootstrap, I2P/Yggdrasil P2P, post-quantum KEx, arvan/bytedance/huawei CDN workers, universal worker, go-bridge, zig-openwrt |
| 6 | `MICAFP-UnifiedShield-¢` | scanner: port/dns/dpi/network assessor, battery optimizer, I2P+Yggdrasil overlay, TLS fragment, security: anti-forensics/ephemeral identity, Android NAN bridge |
| 7 | `MICAFP-UnifiedShield-£` | VLESS, ShadowTLS, Hysteria2, TUIC v5, Reality, NaïveProxy, DoQ, WebTransport, ICMP tunnel, CDN worker, meek, boringtun adapter, iOS acoustic receiver, ShieldPacketTunnel |
| 8 | `MICAFP-UnifiedShield-€` | Full Android manifest + VpnService, iOS Info.plist, Makefile, quickstart.sh, deploy scripts |

## VIP-ULTRA Additions (New in This Release)

The following subsystems are **new additions** not present in any of the 8 source projects:

### 1. Unified Orchestrator (`daemon/src/orchestrator/`)
Central control plane coordinating all subsystems with a 30-second health cycle,
200 ms failover timeout, and battery-aware scheduling. Components:
`control_plane.rs`, `health_monitor.rs`, `failover.rs`.

### 2. Adaptive Load Balancer (`daemon/src/load_balancer/`)
Smooth Weighted Round Robin (SWRR) with EWMA latency scoring distributes traffic
across multiple simultaneous transport connections. Includes session affinity
for long-lived streams. Components: `swrr.rs`, `session_affinity.rs`.

### 3. System Watchdog (`daemon/src/watchdog/mod.rs`)
Cross-platform watchdog monitors each subsystem with configurable heartbeat
intervals. Triggers soft restart of individual subsystems on hang without
killing the entire daemon. Integrates with systemd sd_notify on Linux.

### 4. Prometheus Metrics Exporter (`daemon/src/metrics/mod.rs`)
Exposes `shield_bytes_rx_total`, `shield_bytes_tx_total`, `shield_failover_total`,
`shield_dpi_events_total`, `shield_active_connections`, `shield_health_score`,
and `shield_battery_pct` in Prometheus text exposition format on localhost:9090.

### 5. Differential-Privacy Telemetry (`daemon/src/telemetry/`)
Privacy-preserving censorship telemetry pipeline using the Laplace mechanism
(ε=1.0 default) for differential privacy. No PII is ever transmitted. Reports
are submitted to IPFS/P2P backends in anonymized JSON. Components:
`aggregator.rs`, `dp_noise.rs`, `reporter.rs`.

### 6. Multi-Hop Chain Transport (`daemon/src/transport/multihop_chain.rs`)
Chains 2–5 transport hops with per-hop encryption layering (onion routing),
independent failover, and Dijkstra-based least-cost path selection ensuring
transport-type diversity across hops. Hop count scales automatically with
the threat level classification.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                   Flutter UI / Dashboard                        │
│              (Android · iOS · Windows · Linux · macOS)          │
└───────────────────────┬─────────────────────────────────────────┘
                        │ IPC (Unix socket / Named Pipe)
┌───────────────────────▼─────────────────────────────────────────┐
│              MICAFP UnifiedShield VIP-ULTRA Daemon              │
│                                                                 │
│  ┌────────────────┐  ┌──────────────┐  ┌────────────────────┐  │
│  │  Orchestrator  │  │   Watchdog   │  │  Metrics Exporter  │  │
│  └───────┬────────┘  └──────────────┘  └────────────────────┘  │
│          │                                                      │
│  ┌───────▼──────────────────────────────────────────────────┐  │
│  │               Transport Layer (21 protocols)              │  │
│  │  VLESS · ShadowTLS · Reality · Hysteria2 · TUIC v5       │  │
│  │  NaïveProxy · CDN Worker · DoQ · WebTransport · Meek     │  │
│  │  MQTT-WS · ICMP · Domain Fronting · DOH · WebRTC         │  │
│  │  Chinese CDN · Cloudflare · Alibaba · Tencent · Baidu    │  │
│  │  Arvan · ByteDance · Huawei · Universal Worker           │  │
│  │                  [Multi-Hop Chain] NEW                   │  │
│  └───────────────────────────────────────────────────────────┘  │
│          │                                                      │
│  ┌───────▼──────────────────────────────────────────────────┐  │
│  │                  Core Engines (9 cores)                   │  │
│  │  Hiddify · Xray · sing-box · AmneziaVPN · Defyx          │  │
│  │  MoaV · Lantern · MahsaNG · Psiphon                      │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────────────┐  │
│  │    AI    │ │ Security │ │   P2P    │ │   NAIN (Intranet)  │  │
│  │ DPI-GAN  │ │ Post-QKEx│ │ libp2p   │ │ BLE · WiFi-Aware  │  │
│  │ RL-Sel.  │ │ Ephemeral│ │ I2P      │ │ NTP · SMS · ICMP  │  │
│  │ UCB1     │ │ AntiFors.│ │ Yggdrasil│ │ Acoustic covert   │  │
│  └──────────┘ └──────────┘ └──────────┘ └───────────────────┘  │
│                                                                 │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────────────┐  │
│  │ Battery  │ │ Scanner  │ │ Telemetry│ │  Load Balancer     │  │
│  │ Adaptive │ │ DPI scan │ │ DP noise │ │  SWRR + EWMA      │  │
│  │ Duty     │ │ Port scan│ │ IPFS pub │ │  Session affinity  │  │
│  └──────────┘ └──────────┘ └──────────┘ └───────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Platform Support

| Platform | Status |
|----------|--------|
| Linux (x86_64, aarch64, musl) | ✅ Full |
| Android (arm64, armv7, x86_64) | ✅ Full |
| iOS (arm64) | ✅ Full |
| macOS (x86_64, arm64) | ✅ Full |
| Windows (x86_64) | ✅ Full |
| OpenWrt (mipsel, aarch64) | ✅ Full |

## Build

```bash
# Build everything
make all

# Build daemon only
make daemon

# Cross-compile for Android
make daemon-android

# Cross-compile for iOS
make daemon-ios

# Build Flutter app
make flutter

# Run tests
make test

# Package release
make release
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SHIELD_PROFILE` | `development` | Set to `production` for JSON logs |
| `SHIELD_WIPE_ON_EXIT` | `0` | Set to `1` to trigger emergency wipe on SIGTERM |
| `SHIELD_TELEMETRY` | `0` | Set to `1` to enable privacy-preserving telemetry |
| `SHIELD_IPFS_GATEWAY` | `https://ipfs.io` | IPFS gateway for config/telemetry |
| `RUST_LOG` | `shield_daemon=info` | Tracing filter |

## License

GPL-3.0-or-later
