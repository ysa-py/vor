# Vor Merge Plan — 100% Feature Accounting

This document accounts for **every feature of all 8 source repositories** in the
`ysa-py/vor1` collection and states where each one ended up in the unified
**Vor** platform. Per the engineering constraint, nothing is silently dropped:
each feature is marked **[PORTED]** (carried as-is), **[ADAPTED]** (carried with
changes, change described), or **[FLAGGED]** (not portable into a release
artifact — with the reason — and where the preserved source lives).

Legend:

| Mark | Meaning |
|---|---|
| `[PORTED]` | Feature runs in Vor exactly as in the source repo |
| `[ADAPTED]` | Feature runs in Vor with modifications (described) |
| `[FLAGGED]` | Feature cannot ship in Vor release binaries; source preserved under `extras/`; reason stated |

Top-level mapping of the 8 sources:

| Source repo | Disposition in Vor |
|---|---|
| **V2RayEZ** (MIT) | **Base of `android/`** — imported whole; app re-branded "Vor", applicationId `com.vor.app`, license gate + adaptive AI + guard ports added on top |
| **EasySNI / "V2RayEz" by MacanDev** (MIT) | **Base of `desktop/`** — imported whole; re-branded "Vor Desktop", UAC pattern engine + license gate + Aethon dashboard merged in |
| **MasterDnsVPN** (per repo LICENSE) | **`dns-tunnel/`** — imported whole (Go client+server); client exposed as a selectable DNS-tunnel engine |
| **MICAFP** (GPL-3.0) | Split: real algorithmic core → **`core/vor-core/`** (shared Rust library, single source); training pipeline → `core/ai-models/`; data → `core/vor-core/data/`; OpenWrt C package → **`openwrt/`**; full source preserved at `extras/micafp/` |
| **AetherGUI / "Aethon"** (AGPL-3.0) | Dashboard + telemetry + lifecycle features merged into `desktop/` UI and `android/`; full source preserved at `extras/aethon/` |
| **UAC-SNI-Spoofer-Android** (MPL-2.0 components) | Kotlin engines (fragmenter, edge bridge, adaptive planner, edge discovery) **[PORTED]** into `android/`; sources preserved at `extras/uac-sni-spoofer-android/` |
| **UAC-SNI-Spoofer-Windows** (GPL-3.0 via patterniha) | Fragment strategy matrix + carrier presets + wrong-sequence injection technique **[ADAPTED]** (Go port) into `desktop/internal/pattern/`; sources preserved at `extras/uac-sni-spoofer-windows/` |
| **MSN-GUARD** (AGPL-3.0) | Guard features (verified-connect, watchdog/auto-reconnect hardening, kill switch, Psiphon ladder) **[PORTED]** into `android/`; sources preserved at `extras/msn-guard/` |

---

## 1. V2RayEZ → `android/` (base shell)

The entire repository is carried as the Android base (this is the base the task
designates). Everything below is **[PORTED]** unless noted; the only changes are
the re-brand (label `V2RayEz` → `Vor`, applicationId `com.v2rayez.app` →
`com.vor.app`, version reset to 1.0.0), the removal of the bundled
`google-services.json` hard requirement (Firebase telemetry now degrades
gracefully when no `GOOGLE_SERVICES_JSON` secret is configured — needed because
Vor's maintainer is pseudonymous and may not run a Firebase project), and the
additions described in §9.

- **Protocols/cores:** VLESS, VMess (AEAD), Trojan, Shadowsocks (SIP002/SIP003),
  REALITY (pbk/sid/spx, XTLS Vision), uTLS fingerprints; transports TCP, WS,
  gRPC, HTTP/2, HTTP; in-process Xray AAR with in-core TUN **[PORTED]**; sing-box
  and mihomo/Clash process cores via `ProcessProxyCore` **[PORTED]**; SSH,
  WireGuard (`wg://`/`[Interface]` .conf), dnstt, psiphon URI schemes **[PORTED]**.
- **Engines:** Tor (embedded `libtor.so`, obfs4/lyrebird, snowflake, webtunnel
  bridges + MOAT-style bridge list asset) **[PORTED]**; Psiphon engine
  (psiphon-tunnel-core addon pack) **[PORTED]**; ByeDpi (ciadpi `libbyedpi.so`)
  **[PORTED]**; dnstt DNS-tunnel addon **[PORTED]** (see §8 — MasterDnsVPN is
  added as a *second*, selectable DNS-tunnel engine); MITM domain-fronting
  engine (local Xray proxy-only + CA) **[PORTED]**; domain-front dialer
  (UAC-style local SOCKS dialer, 3 Java files) **[PORTED]**.
- **VPN service:** foreground `specialUse` VpnService, per-mode tunnels, mutex +
  generation counter, connectivity probes (generate_204 × 3 URLs), 1 Hz stats
  loop (4 Hz battery-saver), 1 s death-watchdog with 3-sample streak +
  auto-reconnect, always-on/lockdown reflection, dual notification channels,
  boot receiver honoring `bootAutoConnect`, QS tile, 1×1 + 2×2 widgets,
  shortcuts **[PORTED]**.
- **Config/UI:** Room v8 (55-column servers, subscriptions, sessions,
  daily_traffic), DataStore JSON settings, Compose Material3 UI with 24 routes
  × 967 strings in en/fa/ru, onboarding wizard, server editor, free-server
  aggregator, QR import/generation, Clash YAML subscriptions, per-app proxy,
  Iran routing/geosite auto-config, statistics, logs, diagnostics, speed test,
  browser WebView with presets, BPB panel, hotspot sharing, core manager,
  donation page **[PORTED]**.
- **Addon pack system:** GitHub-Release addon zips (tor/lyrebird/snowflake/
  webtunnel/byedpi/psiphon/dnsteam) with sha256+ELF validation, `pack-addons.sh`
  CI conventions, checksum-pinned vendor sources **[PORTED]** (packaged from the
  committed `jniLibs/`, matching upstream conventions).
- **CI:** `android.yml` build+test+lint, tag-driven signed release with
  `SHA256SUMS.txt` **[PORTED]** (re-triggered by Vor's coordinated pipeline,
  see §10).

## 2. MICAFP → `core/vor-core/`, `openwrt/`, `core/ai-models/`, `extras/micafp/`

MICAFP is a prior mega-merge (itself claiming 13 merged projects) whose useful
core is split into Vor's **shared library** (as the task requires: "port the
AI-assisted DPI-evasion logic as a shared library — do not fork it into
platform-specific copies"). Full source is preserved at `extras/micafp/` for
every [FLAGGED] item.

- **AI — UCB1 endpoint/strategy selection** (`daemon/src/ai/ucb_bandit.rs`, real
  implementation) **[PORTED]** → `core/vor-core/src/bandit.rs`.
- **AI — Q-learning transport selector** (906-line `rl_transport_selector.rs`:
  experience replay, ε-greedy, state discretization, persisted Q-table)
  **[ADAPTED]** → `core/vor-core/src/selector.rs` (same algorithm, smaller state
  schema shared across platforms via JSON state files).
- **AI — traffic feature extractor** (15 rolling-window features: IAT stats,
  size stats, TLS record-type ratios, burst metrics) **[PORTED]** →
  `core/vor-core/src/features.rs`.
- **AI — DPI classifier / traffic predictor / WGAN ONNX inference**
  **[FLAGGED]**: upstream ships **no trained model binaries** — `ai-models/`
  contains only the Python *training* pipeline and the Rust inference is
  hardcoded-stubbed (`onnx_runtime.rs` returns `vec![0.85; 8]`). The training
  pipeline is preserved at `core/ai-models/` **[PORTED as tooling]** and the
  adaptive engine (bandit + Q-learning + threat ladder) selects strategies from
  **real measured outcomes** instead of a nonexistent model. Honest statement:
  the "AI models" in MICAFP were never real model files; Vor's AI is on-device
  adaptive learning (bandit/RL over live probes), not a shipped neural net.
- **AI — adversarial traffic profiles** (`adversarial_traffic.rs`: youtube_hd/
  netflix_4k/large_download packet-size/IAT/burst presets used by the traffic
  shaper) **[PORTED]** → `core/vor-core/data/traffic-profiles.json`.
- **Iran ISP profiles + DPI signatures + IP ranges** (`configs/isp-profiles.json`
  11 ISPs with ASN/blocked/working protocols; `dpi-signatures.json` FAVA RST
  timing 95–320 ms, poison IPs 10.10.34.34/35; `iran-ip-ranges.json`)
  **[PORTED]** → `core/vor-core/data/` (+ copied into `android/` assets and
  embedded in the desktop binary). This is the Fingerprinted-ISP → strategy map
  used by every platform.
- **TLS ClientHello fragmentation strategies** (`tls_fragment.rs`: SniSplit —
  "defeats ~60% of FAVA v2", RecordSplit — exploits FAVA v1 reassembly limit,
  RandomSplit) with per-ISP strategy selection and ThreatLevel ladder
  (None→Passive→ActiveV1→ActiveV2→CompleteBlackout) and 3-layer
  ObfuscationCoordinator (fragment always-on for Iranian ISPs; shaper when DPI
  detected; transform under aggressive threat) with battery-aware degradation
  **[PORTED]** → `core/vor-core/src/fragment.rs`, `threat.rs`, `coordinator.rs`.
- **Traffic shaping (GAN-profile-driven padding/IAT/burst)** (`traffic_shaper.rs`)
  **[PORTED]** → `core/vor-core/src/shaper.rs` (uses the JSON traffic profiles).
- **Orphaned obfuscation modules** (utls_fingerprint.rs, timing_jitter.rs,
  packet_size_normalizer.rs, http3_masquerade.rs, steganographic_header.rs,
  websocket_tunnel.rs — present upstream but **never declared in mod.rs, i.e.
  never compiled**) **[ADAPTED]** → the genuinely useful ones are wired into
  `core/vor-core` (uTLS-style per-domain-hour deterministic fingerprint rotation,
  timing jitter with anti-correlation, TLS padding). Upstream defect fixed
  rather than carried forward.
- **23-transport ladder** (VLESS, Reality, Shadow-TLS, Hysteria2, TUIC v5,
  NaïveProxy, Chinese-CDN worker relays, domain fronting, meek, DoH/DoQ tunnels,
  WebRTC, WebTransport, MQTT×2, ICMP tunnel, pluggable transports, multihop
  onion, dual-mode QUIC/onion) **[ADAPTED]**: the transports users actually run
  against servers today are provided in Vor through the multi-core architecture
  (Xray AAR + sing-box process core cover VLESS/Reality/Hysteria2/TUIC/Naive/
  WS/gRPC; domain fronting via MITM engine; Tor bridges incl. meek/snowflake/
  webtunnel; DoH via core DNS settings; DNS-tunnel via MasterDnsVPN §8).
  The exotic covert-channel transports are **[FLAGGED]** to `extras/micafp/`:
  Chinese-CDN/Arvan serverless relays, MQTT brokers, ICMP raw-socket tunnel,
  WebRTC relay, WebTransport, acoustic 18–22 kHz OFDM, BLE mesh, Wi-Fi Aware,
  SMS bootstrap, NTP covert channel, libp2p/yggdrasil/I2P overlays, dual-mode
  v70 engine — each requires matching server infrastructure or hardware
  capabilities a consumer VPN client cannot assume; preserved as reference
  implementations for future engines (see `docs/ARCHITECTURE.md` — the engine
  abstraction is where they would plug in).
- **9 core adapters** (xray, singbox, hiddify, psiphon, lantern, amneziavpn,
  defyx, mahsang, moav) **[ADAPTED]**: xray/singbox/psiphon are live in Vor via
  V2RayEZ's cores; hiddify/lantern/amnezia/defyx/mahsang/moav are separate
  third-party products with their own licensing/distribution — **[FLAGGED]** as
  external process cores a user can add via the process-core mechanism, not
  vendored.
- **Quantum modules** (ML-KEM hybrid handshake, PQ double ratchet, lattice
  onion, QKD simulation, homomorphic routing) **[FLAGGED]**: no server
  ecosystem interoperates; preserved at `extras/micafp/`; noted as future work.
- **NAIN (National Intranet) detection + fallback routing** (shutdown detector
  with screen-state-aware probe intervals, intranet detection, fallback ladder)
  **[ADAPTED]** → the NAIN detector heuristic + offline-mode awareness is ported
  into `core/vor-core/src/nain.rs` and surfaced as a status flag; the covert
  channel fallback ladder is **[FLAGGED]** with the transports above.
- **OpenWrt C package** (netifd proto handler + UCI config + LuCI app +
  `iran_ip_ranges.txt`) **[PORTED]** → `openwrt/` (see §7; now linking
  `libvor-core` for strategy selection).
- **zig-openwrt** **[FLAGGED]**: upstream is broken as committed — `build.zig`
  exists but `src/main.zig` is **missing**, so it cannot build. Preserved as-is
  at `extras/micafp/zig-openwrt/` with a note; Vor's OpenWrt artifact is the C
  package + Rust musl cross-build.
- **Linux/Windows C++ desktop apps** (tun interface, systemd service, tray,
  metrics overlay, named-pipe IPC, Inno Setup installer) **[ADAPTED]**: their
  roles are fulfilled by `desktop/` (Go) — TUN via sing-box/tun2socks, service
  via deb/systemd packaging, metrics via the dashboard, installer via NSIS +
  portable zip; preserved at `extras/micafp/linux`, `extras/micafp/windows`.
- **iOS Swift code** (PacketTunnelProvider patterns) **[ADAPTED]** → informs
  `ios/` **[PORTED as reference]**; preserved at `extras/micafp/ios`.
- **Next.js admin dashboard** (20 API routes, 18 panels) **[FLAGGED]**:
  server-side admin panel conflicts with Vor's no-server constraint; its
  *views* (DPI status, transport health, scanner, network analyzer, session
  analytics) are represented in the unified desktop dashboard where they apply
  to a client; full source preserved at `extras/micafp/dashboard`.
- **Browser extensions + 9 CDN workers + WASM obfuscator** **[FLAGGED]**:
  separate distribution products requiring user deployment; preserved at
  `extras/micafp/extensions`, `extras/micafp/workers`,
  `extras/micafp/wasm-obfuscator`.
- **Flutter apps** (2 of them) **[FLAGGED]**: superseded by the Compose base
  (V2RayEZ) per the task's designation; preserved at `extras/micafp/flutter*`.
- **Censorship simulation test rig** (scapy FAVA simulator, Suricata rules,
  docker-compose firewall sim) **[PORTED]** → `core/tests/censorship-sim/`
  (used by CI to validate fragment strategies against simulated FAVA v1/v2
  behavior).
- **Ops scripts** (build-all, cross-compile matrix, ISP detect, checksums,
  signing, IP-range updates) **[ADAPTED]** → the applicable ones are folded into
  `scripts/` and CI workflows.

## 3. EasySNI (Go, "V2RayEz" by MacanDev) → `desktop/` (base)

The whole repository is carried as the cross-platform desktop engine (it is not
Windows-only — it builds for windows/macOS/linux, amd64+arm64). Re-branded
"Vor Desktop". All features **[PORTED]** except the merged-in additions in §9:

- **SNI Tunnel proxy**: transparent (fake-SNI TLS termination), passthrough,
  CDN-fronting (front SNI + Host rewrite), multi-SNI random rotation,
  LAN sharing **[PORTED]**.
- **DPI desync**: ClientHello SNI-boundary fragmentation (configurable chunk,
  inter-write delay), fake ClientHello injection (11 fingerprint presets),
  wrong-checksum / wrong-seq fake segments, WinDivert (Windows) + raw-socket
  (Linux) injection with graceful fallback to fragmentation **[PORTED]** — this
  is the **default Windows engine** (see `docs/ENGINES.md` tradeoff analysis).
- **Client-side domain-fronting MITM** (local CA + per-host leaves, rule map,
  blind passthrough, fronted DoH presets CF/Google/Quad9, CA export
  .pem/.crt/.mobileconfig, phone config helper) **[PORTED]**.
- **Google Tunnel (Fastly)** GAS+CF-Worker relay, **SPlus tunnel**
  (SOCKS5-over-LiveKit inside a SoroushPlus call), **Psiphon-over-MITM** chain,
  embedded Psiphon (build tag), **Tor** subprocess with obfs4/meek/snowflake/
  webtunnel + bridge rotation **[PORTED]**.
- **Xray/sing-box engines + TUN**: locate/download/run xray, sing-box system
  TUN VPN, tun2socks, system proxy set/clear (Win/macOS/Linux GNOME) **[PORTED]**.
- **Scanners**: single SNI probe, relay timing, mass SNI (captcha-domain
  preset), Cloudflare clean-IP, CDN edge, site scanner, CDN-Configs builder
  (TLS/TTFB/throughput/colo scoring), mass URI via real xray **[PORTED]**.
- **EzBPB** one-click BPB Worker Panel deploy, edge-tunnel link builder, CF
  worker maker **[PORTED]**.
- **Dashboard**: embedded SPA, ~90 JSON endpoints, SSE live logs, EN/FA RTL,
  dark/light, chromeless app-window, auto-saved UI state, config library
  (~190 paste-ready links), updater (github.com releases without
  api.github.com) **[PORTED]** — now also carries the merged Aethon home view
  and license gate (§9).
- **Telegram Mini App** clean-IP scanner (`scanner.html`) **[PORTED]**.

## 4. AetherGUI ("Aethon") → merged into `desktop/` + `android/`

Aethon's transport logic is the external Aether core + sing-box; its
*product-level* features are merged:

- **Connection orb home view** (animated state-colored orb, status pill with
  states disconnected/connecting/scanning/reconnecting/connected/error, live
  status messages) **[ADAPTED]** → implemented as the new Home tab of the
  unified desktop dashboard (vanilla JS matching EasySNI's no-framework
  frontend; Aethon's orb CSS ported with its reduced-motion support).
- **Telemetry widgets: session upload/download totals, ping, VPN exit location
  (flag + city via GeoIP through tunnel), session elapsed** **[ADAPTED]** →
  Home tab + periodic probe endpoints (Cloudflare trace via the live tunnel,
  same technique as Aethon's `vpn_probe`).
- **Connection modes**: Device VPN (TUN), Smart Connect (auto protocol
  benchmark), SOCKS proxy **[ADAPTED]** → Device VPN = sing-box TUN mode
  (EasySNI) with Aethon's DNS-leak protection (strict_route + DoH + hijack-dns)
  and bypass-local route computation; SOCKS = xray SOCKS mode; Smart Connect =
  Vor's **AI engine selector** (§9) which benchmarks engines the way Aethon
  benchmarked protocols.
- **MASQUE / WireGuard / gool protocols, scan modes Turbo/Balanced/Thorough/
  Stealth/Ironclad, MASQUE H3→H2 fallback, obfuscation profiles, MTU setting,
  route exclusions** **[FLAGGED]**: these live in the Aether core (CluvexStudio,
  AGPL, trademark-protected name). Aethon is preserved whole at
  `extras/aethon/` including its Android app and fetch scripts; the engine
  abstraction documents how an Aether sidecar could be added later.
- **Split tunneling UI with Start-Menu app picker + icons + add-executable,
  per-app include/exclude** **[ADAPTED]** → desktop split-tunnel tab (process
  paths on Windows, binary picker).
- **Windows lifecycle engineering**: elevated routing helper with named-mutex
  guard, `0.0.0.0/1 + 128.0.0.0/1` route takeover, session recovery
  (`recovery.json`), stale-adapter cleanup, `--repair-network` CLI + NSIS
  pre-uninstall hook, single-instance, close-to-tray behavior, update flow with
  SHA-256 verification **[ADAPTED]** → equivalent logic re-implemented for the
  Go desktop (repair CLI, route takeover, recovery state, single-instance
  mutex, NSIS hooks for cleanup) where the Go stack needs it; preserved
  reference at `extras/aethon/src-tauri/`.
- **Android app** (Aethon full Java VPN client with HEV bridge, Smart Connect
  benchmark, update manager with same-signer verification, MASQUE fallback)
  **[FLAGGED]** as a build (superseded by the V2RayEZ base), but its
  same-signing-certificate APK verification is **[ADAPTED]** into Vor's Android
  updater (§6).

## 5. UAC-SNI-Spoofer-Android → engines ported into `android/`

The app shell is superseded (V2RayEZ is the base); its **engines** are ported
as Kotlin sources into the Vor app under
`android/app/src/main/java/com/v2rayez/app/data/uac/`:

- **`MciFragmenter`** — pure-Kotlin ClientHello surgery: full SNI parser +
  9 strategies (FINALMASK_TLS_HELLO, FULL5/10/20, SNI_BOUNDARY, SNI_SPLIT,
  TLS_RECORD_FRAG, TLS_SNI_RECORDS, HALF, RAW) **[PORTED]** — runs in front of
  any core (core-independent external finalmask).
- **`MciEdgeBridge`** — 127.0.0.1:40443 loopback relay performing the
  ClientHello TLS-record split (external finalmask) with session limits and
  bind retry **[PORTED]**.
- **`finalmask` Xray config emission** (`streamSettings.finalmask`
  tlshello/length/delay/maxSplit + keepalive sockopt + edge substitution
  preserving SNI/Host/Path/ALPN/fingerprint) **[PORTED]** → added to Vor's
  `ConfigBuilder` as an opt-in per-server "Fragment/FinalMask" advanced option
  (works with the stock Xray AAR core when driven through the Edge Bridge; the
  patched Xray-core fork from upstream is NOT required — see below).
- **Patched Xray-core fork (v26.7.28 `finalmask` binary, 153 MB across ABIs)**
  **[FLAGGED]**: not vendored into Vor (size + maintenance); the same
  capability is provided core-independently by the Edge Bridge, and users of
  the process-core mechanism can point at a finalmask-enabled xray binary.
  Preserved upstream at `extras/uac-sni-spoofer-android/app/src/main/jniLibs/`
  (source tree only, binaries not committed — they are upstream release
  artifacts).
- **`AdaptiveConnection`** — network fingerprinting (transport/carrier/ASN/
  provider), candidate planning (champion → learned winner → last-good →
  backup → fresh pool), failure cooldowns, per-fingerprint learning store
  **[PORTED]** → `android/.../data/ai/adaptive/` (feeds the AI engine
  selector, §9).
- **`CloudflareEdgeDiscovery`** — scanning CF IP ranges for clean edges (TLS
  preflight, suitability, history store, phases/progress) **[PORTED]** →
  `android/.../data/ai/edge/`.
- **Route Speed Test matrix** (Edge × DNS × Fragment × MTU staged competition)
  **[ADAPTED]** → exposed as Vor's "Route Lab" diagnostics screen using the
  ported engines + the existing speed-test infrastructure.
- **DoH catalog (CF/Google/Quad9/AdGuard/OpenDNS + bootstrap IPs)** **[PORTED]**
  → Vor DNS settings.
- **Tor/WebTunnel engine mode + exit-country picker** **[FLAGGED]**: Vor's
  base already ships a Tor engine with webtunnel bridges and geoip; UAC's
  variant is not duplicated. Its per-country relay-count data is **[PORTED]**
  into the Tor settings screen data.
- **UI (drawer, SNI Maker, TV variant, Persian fonts/hero art)** **[FLAGGED]**
  (superseded by the base app UI; preserved in `extras/`).

## 6. UAC-SNI-Spoofer-Windows → Go port in `desktop/internal/pattern/`

The Python/PySide6 app is superseded by the Go desktop (single binary, no
150–250 MB Python footprint — full tradeoff analysis in `docs/ENGINES.md`).
Its engines are ported:

- **Fragment strategy matrix + carrier racing** (`FragmentProxy` +
  `tls_tools.fragments()`: full5/10/20, sni_boundary, sni_split,
  tls_record_frag, tls_sni_records, half, raw, multi64, sni_chars; strategy/
  host/carrier racing with early-stop, preference caches, cooldowns, warm
  sockets) **[PORTED]** → `desktop/internal/pattern/fragment.go` (the
  **alternate** Windows engine).
- **Wrong-sequence fake-SNI injection** (patterniha technique: WinDivert SNIFF
  on handshake packets; after the client's final handshake ACK, inject a
  ClientHello with the fake SNI at `seq = syn_seq + 1 − len(fake)` so the DPI
  middlebox parses the fake SNI while the server discards the out-of-window
  segment; verify by exact-ACK match; graceful fallback to fragmentation;
  hybrid fake+fragment mode for MCI) **[ADAPTED]** → re-implemented from the
  published technique description in Go (`desktop/internal/pattern/inject.go`)
  with attribution; the upstream Python (GPL-3.0) source is preserved at
  `extras/uac-sni-spoofer-windows/uac_desktop/pattern_core/` and
  `third_party/patterniha_sni_spoofing/`. The Python app itself (assistant,
  gateway, Qt UI) is **[FLAGGED]** as a build.
- **Carrier tuning presets** (MCI: edges 104.18.1.1/172.66.0.1, fake SNI
  www.speedtest.net, compatibility profile; Irancell: 104.19.229.21, fake SNI
  chatgpt.com, maximum profile; Auto cross-carrier racing) **[PORTED]** →
  shared JSON `core/vor-core/data/carrier-presets.json` consumed by all
  platforms.
- **517-byte padded ClientHello template** (fake hello indistinguishable size)
  **[PORTED]** → pattern engine template.
- **Windows system proxy mode, sing-box TUN mode with Job-Object guarding,
  Mobile Gateway LAN sharing (forwarding + ARP + DNS rewrite + MSS clamp)**
  **[ADAPTED]**: system proxy + TUN already exist in the Go base (EasySNI's
  sysproxy + singbox); Job-Object kill-on-close is **[PORTED]** via Windows
  Job Objects in the Go engine wrapper; Mobile Gateway ARP-based sharing is
  **[FLAGGED]** (requires Npcap + Administrator, niche; preserved in extras).
- **Verified country profiles (SPOOF-NNN)** **[PORTED]** → pattern engine
  presets JSON.

## 7. MSN-GUARD → guard module in `android/`

MSN-GUARD is a full VPN client (superseded as an app by the V2RayEZ base); its
**guard features** — the reason it is in the merge — are ported into
`android/.../data/guard/` as an optional add-on layer ("Vor Guard"):

- **Verified-connect gate** (never show "Connected" until ≥ 4 KiB real RX bytes
  crossed the tunnel — kills "fake-connected" states) **[PORTED]** → integrated
  into the VPN service connect sequence.
- **Tunnel watchdog** (30 s liveness poll of local components; deliberately not
  an HTTP probe — own-package-exclusion makes such probes meaningless) **[PORTED]**
  (replaces/augments the base's death-watchdog with local-liveness semantics).
- **Auto-reconnect ladder with backoff 5/15/30/60/120 s**, distinguishing
  user-initiated vs spontaneous disconnects, foreground-service retained across
  reconnects **[PORTED]**.
- **Kill switch** (on unexpected drop, re-establish a blocking TUN so traffic
  cannot leak in plaintext) **[PORTED]** → optional setting, off by default,
  enabled from Guard settings.
- **DNS guard** (force public resolvers, filter carrier DNS, alternate-port
  resolvers OpenDNS :5353 / Quad9 :9953 to defeat UDP/53 hijack) **[PORTED]**
  → VPN DNS builder option.
- **Psiphon 5-rung strategy ladder + region phase + winner memory** (fronted/
  direct/chainable protocol sets, 25 s region budget, rung attribution)
  **[PORTED]** → wraps the existing Psiphon engine.
- **Monthly traffic accounting + session stats** **[PORTED]** → statistics
  screen.
- **Psiphon embedded server entries (429), Tor regions with relay counts,
  PSIPHON_ALTERNATE_DNS, LAN bypass with CGNAT awareness** **[PORTED]** → data
  files + routing helper.
- **Aether-core transports (MASQUE/WG/gool chain ladder)** **[FLAGGED]** (same
  disposition as §4 — external core with trademark constraints; preserved at
  `extras/msn-guard/core/`).
- **Orbit UI (dial, glass widgets)** **[FLAGGED]** (superseded by the Compose
  UI; the design language is acknowledged; sources preserved).
- **Zero Trust enrolment (Cloudflare Access email code → token in KeyStore)**
  **[FLAGGED]** (requires a Cloudflare Access subscription — conflicts with
  free-tier/pseudonymous constraint; preserved in extras).
- **`build-tor.sh` pinned toolchain build + filtered GeoIP** **[PORTED]** →
  `scripts/vendor/` for CI Tor rebuilds.

## 8. MasterDnsVPN → `dns-tunnel/` + engine integration

The complete Go module (client + server) is imported **[PORTED]** and exposed
in two ways:

- **Standalone server** — operator deploys on their delegated domain (docs
  carried over, incl. the Persian README) **[PORTED]**.
- **Client as a Vor engine** — the DNS-tunnel client (SOCKS5 listener at
  127.0.0.1:18000, TXT-qname uplink, ARQ, 8 resolver balancers, local DNS
  cache) is spawnable from the desktop app (process engine) and from Android
  (process engine via `ProcessProxyCore` conventions, same as sing-box/mihomo)
  **[ADAPTED]**: added `vor` build tag entry points + a small adapter config
  that maps Vor's unified config to the TOML schema. It appears in the engine
  selector as "DNS Tunnel (MasterDns)" alongside V2RayEZ's dnstt.
- All protocol features (base36 label encoding, 6 crypto methods, ZSTD/LZ4/
  ZLIB compression, ARQ, MLQ priorities, packed control blocks, multipath
  resolver balancing, MTU sync, policy clamps, SOCKS4/5, UDP ASSOCIATE, local
  DNS server, docker images, systemd installer, benchmark harness)
  **[PORTED]** — unmodified.
- Relationship to V2RayEZ's `dnstt` addon: both are kept (constraint: no
  feature removal); the engine selector lists both, with a short description
  of the tradeoff (dnstt: simpler, well-known; MasterDns: ARQ+multipath, lower
  header overhead, needs your own server).

## 9. New unified features (from the task requirements themselves)

- **License gate (first screen)** on Android + desktop + iOS + OpenWrt LuCI +
  License Manager companion Android app — `license/`, implementations per
  platform, all sharing `license/vectors.json` conformance vectors.
- **Transport engine selector** — UI on every platform: **Auto (AI)** or manual
  pick (Xray core, sing-box, SNI Tunnel, Pattern/WrongSeq engine, DNS-tunnel
  dnstt, DNS-tunnel MasterDns, Tor, Psiphon, MITM fronting…). Auto mode runs
  the shared adaptive engine (ISP profile → candidate plan → probes → bandit
  update), the single source of decision logic being `core/vor-core` with
  vector-tested ports.
- **Unified dashboard** — one consistent desktop UI: Aethon-style Home (orb,
  status, traffic/ping/location) + EasySNI's tool tabs, with the SSE log
  console as the shared telemetry view (§3 + §4 above).
- **Iran-grade adaptive AI** (user requirement): on-device, offline, dynamic —
  ISP fingerprinting, threat ladder, UCB1/Q-learning strategy selection,
  per-connection strategy rotation, clean-edge discovery. No cloud AI calls,
  ever (a deliberate divergence from MICAFP's `aiproviders/` cloud-LLM
  registry, which is **[FLAGGED]** — cloud LLM calls from a censorship-
  circumvention client are a privacy/availability anti-pattern under Iran-grade
  filtering; preserved in extras).

## 10. CI / release mapping

One tag `vor-v*` produces all five artifacts through the coordinated pipeline
(`docs/ARCHITECTURE.md` §CI): Android signed APK (V2RayEZ conventions +
license gate), Windows portable+NSIS exe (Go), Linux binary + deb + AppImage
(Go), OpenWrt `.ipk` arch matrix (C + libvor-core musl builds), iOS unsigned
`.ipa` for TrollStore (macOS runner). Every platform job runs its unit tests +
lint and fails the release loudly.

---

## Resolved conflicts

- **EasySNI (Go) vs UAC-SNI-Spoofer-Windows (Python)** — both implement Windows
  SNI spoofing. Tradeoffs (performance, size, dependencies, detection
  resistance) analyzed in `docs/ENGINES.md`. Decision: **EasySNI's Go desync
  engine is the default**; the UAC pattern engine (fragment matrix + wrong-seq
  injection, Go port) is the runtime-switchable alternate; the Xray finalmask
  path is a third option via config emission. Nothing dropped.
- **Two Android apps (V2RayEZ, MSN-GUARD, UAC-Android, Aethon-Android, MICAFP
  android, 2 Flutter apps)** — V2RayEZ is the designated base; the others
  contribute engines/features per above and are preserved in full at `extras/`.
- **Two dashboards (EasySNI web panel, Aethon home)** — merged into one desktop
  UI (Home + tools tabs), per the task.
- **Two DNS tunnels (dnstt, MasterDnsVPN)** — both selectable engines.
- **AI: "models" that don't exist vs real adaptive logic** — resolved honestly:
  ship the real adaptive engine; carry the training pipeline as tooling; state
  plainly that no neural-net binaries exist upstream.
