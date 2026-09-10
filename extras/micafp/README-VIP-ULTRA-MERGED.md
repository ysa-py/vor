# 🛡️ MICAFP-UnifiedShield-vip-ultra

> **نسخه نهایی Unified — ادغام کامل ۸ پروژه + قابلیت‌های پیشرفته جدید**
> Final merged edition — all 8 projects fully combined, zero features removed, advanced capabilities added.

---

## 📦 Merge Summary (خلاصه ادغام)

This project is the **complete, automatic merge** of all 8 source projects:

| Project | Files | Key Contributions |
|---------|-------|-------------------|
| `MICAFP-UnifiedShield-&` | 361 | Base VIP merged from *, nextgen$, nextgen@ |
| `MICAFP-UnifiedShield-*` | 267 | VIP Ultra — comprehensive Rust daemon |
| `MICAFP-UnifiedShield-+` | 494 | **Base for this merge** — most complete prior merge |
| `MICAFP-UnifiedShield-¢` | 94  | Scanner, GoodbyeDPI/Zapret, ICMP, Covert channels |
| `MICAFP-UnifiedShield-£` | 121 | Post-quantum KEX, BLE mesh, WASM obfuscator, Boringtun |
| `MICAFP-UnifiedShield-€` | 106 | Setup scripts, iOS Info.plist, Arvan CDN |
| `unifiedshield-nextgen$` | 266 | Dashboard UI components (60+), full browser extensions |
| `unifiedshield-nextgen@` | 249 | iOS Swift, Linux C++, Windows C++ native apps |

**Zero features were removed.** Every module from all 8 projects is present in the merged output.

---

## 🆕 New Advanced Features (قابلیت‌های پیشرفته جدید)

### 1. 📊 Monitoring & Prometheus Metrics (`daemon/src/monitoring/`)
- **Prometheus exporter** — exposes `/metrics` on port 9090, compatible with Grafana
- **Health checker** — continuous self-diagnostics of all 8 subsystems (IPC, AI, P2P, transport, NAIN, battery, PQ-KEX, scanner)
- **Latency tracker** — per-transport P50/P95/P99 RTT histograms with 1000-sample rolling window
- **Alert manager** — threshold-based alerting with automatic escalation (Warning → Critical after 3 occurrences)
- **Grafana dashboard** — pre-built JSON at `docs/grafana/shield-dashboard.json`

### 2. 🕸️ Mesh Network Coordinator (`daemon/src/mesh/`)
- **Mesh coordinator** — orchestrates WiFi Aware + BLE mesh + Yggdrasil + I2P channels
- **Topology manager** — Dijkstra shortest-path routing across the mesh graph
- **Gossip protocol** — epidemic peer discovery, converges in O(log N) rounds, max 5 hops
- **Mesh cryptography** — per-hop X25519 ECDH + ChaCha20-Poly1305 with forward secrecy
- **Android BLE Manager** — `MeshNetworkManager.kt` with WiFi Aware NAN + BLE scan
- **iOS BLE Manager** — `BleMeshManager.swift` with CBCentralManager + CBPeripheralManager

### 3. 🔄 Resilience Subsystem (`daemon/src/resilience/`)
- **Circuit breaker** — Closed → Open → Half-Open state machine per transport, prevents cascade failures
- **Retry policy** — exponential backoff with full jitter (`initial=500ms`, `multiplier=2×`, `max=30s`)
- **Fallback chain** — 8-strategy ordered fallback: Primary → CDN → P2P → DoH → ICMP → Mesh → Snowflake → Meek
- **Watchdog** — heartbeat monitoring of all daemon tasks, auto-restart on timeout, escalation after `max_restarts`
- **Android `ResilienceManager.kt`** — `CircuitBreaker` + `FallbackChain` for Android app layer

### 4. 📈 Advanced Analytics Dashboard (`dashboard/src/`)
- **`/api/advanced-analytics`** — aggregates transport latency, AI engine, mesh, and resilience metrics in one response
- **`/api/mesh-network`** — live mesh topology with nodes and edges
- **`/api/resilience`** — circuit breaker states, fallback chain position, watchdog status
- **`AdvancedAnalyticsPanel`** — React component with auto-refresh every 5 seconds

### 5. 📱 Flutter Advanced Security Screen
- **`advanced_security_screen.dart`** — full UI for post-quantum KEX, anti-forensics, mesh peers, and resilience chain visualization

### 6. 🖥️ Desktop Advanced Tools
- **Linux** — `prometheus_client.cpp`: CLI live metrics overlay with `--watch` mode
- **Windows** — `metrics_overlay.cpp`: WinHTTP-based tray tooltip with live DPI risk + NAIN status
- **OpenWrt** — `advanced.htm`: LuCI view for resilience chain and AI engine status

### 7. 🔧 Advanced Scripts (`scripts/`)
- `health-check.sh` — verifies daemon, IPC socket, Prometheus, dashboard, DNS, CDN, Rust, Node
- `benchmark-transports.sh` — queries Prometheus for per-transport P50/P95 latency
- `rotate-identity.sh` — forces immediate ephemeral identity rotation via IPC

---

## 🏗️ Complete Architecture

```
MICAFP-UnifiedShield-vip-ultra/
├── daemon/                          # Rust core daemon (shield_daemon crate)
│   └── src/
│       ├── ai/                      # UCB1 bandit, ONNX DPI, GAN, RL selector
│       │   ├── adversarial_traffic.rs
│       │   ├── dpi_classifier.rs
│       │   ├── feature_extractor.rs
│       │   ├── mod.rs
│       │   ├── onnx_runtime.rs
│       │   ├── rl_transport_selector.rs
│       │   ├── traffic_predictor.rs
│       │   └── ucb_bandit.rs
│       ├── battery/                 # Adaptive duty cycle, coalesced timers
│       ├── config/                  # Endpoint manager, IPFS updater, ISP profiles
│       ├── cores/                   # 9 VPN cores: hiddify, xray, singbox, amneziawg,
│       │                            #   defyx, moav, lantern, mahsang, psiphon
│       ├── ipc/                     # Unix socket + named pipe IPC
│       ├── mesh/          ★ NEW     # Mesh coordinator, topology, gossip, crypto
│       ├── monitoring/    ★ NEW     # Prometheus exporter, health, latency, alerts
│       ├── national_intranet/       # NAIN detector, acoustic/NTP/SMS/WiFi/BLE covert
│       ├── obfuscation/             # uTLS, HTTP/3, traffic shaping, WASM, steganographic
│       ├── p2p/                     # libp2p, NAT traversal, I2P, Yggdrasil, relay
│       ├── platform/                # Android, iOS, Linux (Zapret), Windows (GoodbyeDPI)
│       ├── resilience/    ★ NEW     # Circuit breaker, retry, fallback chain, watchdog
│       ├── scanner/                 # DNS, DPI, port, network assessor
│       ├── security/                # Anti-forensics, device secret, ephemeral ID, PQ-KEX
│       ├── transport/               # 13 transports: VLESS, ShadowTLS, Reality, Hysteria2,
│       │                            #   TUICv5, NaïveProxy, CDN Worker, DoQ, WebTransport,
│       │                            #   Meek, MQTT-WS, ICMP tunnel, Chinese CDN
│       └── tunnel/                  # AmneziaWG, WireGuard, Boringtun, split tunnel, TUN
│
├── dashboard/                       # Next.js 15 admin dashboard
│   └── src/
│       ├── app/api/                 # 19 API routes (health, cores, AI, OTA, kill-switch,
│       │   │                        #   geo-router, network-analyzer, threat-intel,
│       │   │                        #   security-audit, intranet-mode, p2p-peers,
│       │   │                        #   obfuscation, auto-reconnect, advanced-analytics ★,
│       │   │                        #   mesh-network ★, resilience ★)
│       ├── components/              # 40+ shadcn/ui components + advanced panels ★
│       └── lib/                     # Store, types, geo-router, network-analyzer, security
│
├── flutter/                         # Flutter app (FA/EN bilingual, full RTL)
├── flutter_app/                     # Flutter app (alternative structure)
│   └── lib/screens/advanced/        # Advanced security screen ★
│
├── android/                         # Native Android (Kotlin)
│   └── app/src/main/kotlin/com/unifiedshield/
│       ├── security/                # MeshNetworkManager.kt ★, ResilienceManager.kt ★
│       └── (CoreBridge, DPI, ISP, KillSwitch, OTA, SplitTunnel, VPN)
│
├── ios/                             # Native iOS (Swift)
│   ├── AdvancedSecurityBridge.swift ★
│   ├── BleMeshManager.swift         ★
│   └── ShieldPacketTunnel/
│
├── linux/                           # Native Linux C++ app
│   └── src/advanced/prometheus_client.cpp ★
│
├── windows/                         # Native Windows C++ app
│   └── src/advanced/metrics_overlay.cpp ★
│
├── extensions/                      # Browser extensions
│   ├── chrome/                      # MV3 Chrome extension (background, popup, options)
│   ├── firefox/                     # MV2 Firefox extension
│   ├── shared/                      # crypto-utils, dpi-signatures, iran-ip-ranges, ISP-DB
│   └── wasm-obfuscator/             # Rust WASM obfuscator
│
├── workers/                         # Edge CDN workers
│   ├── alibaba-cdn/                 # Alibaba Cloud FC (obfuscator + rate-limiter + relay)
│   ├── arvan-cdn/                   # ArvanCloud (Iran-friendly CDN)
│   ├── baidu-cdn/                   # Baidu Cloud CFC
│   ├── bytedance-cdn/               # ByteDance CDN
│   ├── cloudflare/                  # Cloudflare Workers (backup)
│   ├── deno-relay/                  # Deno Deploy relay
│   ├── huawei-cdn/                  # Huawei Cloud
│   ├── tencent-cdn/                 # Tencent SCF
│   └── universal/                   # Universal worker template
│
├── ai-models/                       # ONNX model training + quantization
├── configs/                         # CDN endpoints, ISP profiles, DPI signatures
├── docs/grafana/shield-dashboard.json ★  # Grafana dashboard
├── openwrt/                         # OpenWrt LuCI package + C source
├── scripts/                         # Build, cross-compile, health-check ★, benchmark ★
├── tests/                           # Censorship simulation (Docker), unit tests
├── wasm-obfuscator/                 # Root-level Rust WASM obfuscator
└── zig-openwrt/                     # Zig-based OpenWrt build
```

---

## 🚀 Quick Start

```bash
# Clone and build
git clone https://github.com/micafp/MICAFP-UnifiedShield-vip-ultra
cd MICAFP-UnifiedShield-vip-ultra

# Run health check
./scripts/health-check.sh

# Build daemon (Linux)
cd daemon && cargo build --release --features=full

# Build with all features
make all

# Run censorship simulation tests
cd tests && docker-compose -f censorship-simulation/docker-compose.yml up

# View live metrics
./scripts/benchmark-transports.sh

# Start dashboard
cd dashboard && bun install && bun dev

# Rotate identity immediately
./scripts/rotate-identity.sh
```

---

## 🇮🇷 Iran-Specific Features

The platform is optimized for Iran's censorship environment:

- **Zero-VPS architecture** — uses Chinese CDN relays (Alibaba, Tencent, Baidu, Huawei) that are not blocked in Iran
- **12 ISP profiles** — MCI/همراه اول (ASN 41689), Irancell/ایرانسل (ASN 39074), Rightel/رایتل (ASN 48434), Shatel, MCI, and 7 others
- **National Intranet Mode** — automatic detection of international internet severing, fallback cascade: CDN → DoH → Snowflake → ICMP → BLE Mesh
- **Covert channel bootstrap** — SMS, NTP timing, acoustic, BLE mesh for bootstrapping when all conventional channels are blocked
- **GoodbyeDPI/Zapret** — platform-level DPI bypass on Windows/Linux without VPN overhead

---

## 📜 License

GPL-3.0-or-later — See LICENSE file.
