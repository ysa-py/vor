# Windows SNI-Spoofing Engines — Tradeoff Analysis & Decision

The source collection contains **two independent Windows SNI-spoofing
implementations** (plus a third technique inside Xray configs). The
engineering task forbids silently dropping either, so this document
evaluates both and explains the default/alternate split shipped in Vor.

## The candidates

| | **EasySNI** (Go) — `desktop/` base | **UAC-SNI-Spoofer-Windows** (Python) |
|---|---|---|
| Runtime | single static Go binary (~15–40 MB, ~30–60 MB RSS) | PySide6 + Python runtime (~150–250 MB extracted, ~200–400 MB RSS) |
| Per-connection path | native goroutines; userspace writes; WinDivert via syscalls only for fake-segment injection | every connection transits a Python relay (blocking `select()` / asyncio); strategy/host/carrier racing in Python |
| Techniques | ClientHello SNI-boundary fragmentation, fake ClientHello (11 presets), wrong-checksum / wrong-seq segments, multi-SNI rotation, CDN fronting, MITM domain fronting + fronted DoH | fragment matrix (full5/10/20, sni_boundary, sni_split, tls_record_frag, tls_sni_records, multi64, sni_chars…), Patterniha wrong-seq injection, carrier edge racing, fake-SNI probes, hybrid fake+fragment |
| Carrier knowledge | ISP-agnostic knobs | deep MCI/Irancell presets: edges (104.18.1.1 / 104.19.229.21 / 172.66.0.1), fake SNIs (www.speedtest.net / chatgpt.com), maxSplit tuning, timing presets |
| Dependencies | none beyond the binary + WinDivert.dll/.sys (already bundled) | Python + PySide6 + psutil + pydivert + scapy(+Npcap for gateway) + xray.exe + sing-box + libcronet |
| Detection resistance | network-shape based; layered | network-shape based; deeper per-carrier timing + wrong-seq choreography; 517-byte padded fake hello |
| Install friction | one exe; UAC prompt for WinDivert | PyInstaller bundle known to attract AV heuristics; uac_admin exe |
| Testability | `go test` — fast, in CI | pytest; heavier to keep green |
| License | MIT | GPL-3.0 (patterniha lineage) |

## The decision

1. **Default engine: the Go desync engine (EasySNI).** Performance and
   footprint win for the common case; the binary already embeds the whole
   toolkit (xray/sing-box/tor/scanners); MIT licensing keeps the desktop
   bundle simple.
2. **Alternate engine: the pattern engine (UAC), ported to Go**
   (`desktop/internal/pattern`). The port keeps the *techniques* that made
   upstream effective — the full fragment matrix, carrier presets
   (shared JSON with every platform), the wrong-sequence injection
   choreography, hybrid fallback — while dropping the Python runtime.
   Selectable at runtime in the engine selector (UI chips +
   `/api/engine/select`); Windows-only (WinDivert).
3. **Third path: Xray `finalmask`** — emitted through the config builder
   and implemented core-independently by the Edge Bridge
   (`android/.../data/uac/EdgeBridge.kt`, `openwrt/vor` C daemon), so the
   stock Xray core gets external-finalmask behavior without shipping
   UAC's 153 MB patched-Xray fork. Users running a finalmask-enabled Xray
   as a process core keep the native path.

Nothing is dropped: EasySNI's engine runs by default; UAC's engine is a
runtime-switchable alternate with its carrier tuning folded into the
shared carrier-presets JSON consumed by ALL platforms (Android, desktop,
OpenWrt); the upstream Python app itself is preserved whole at
`extras/uac-sni-spoofer-windows/`.

## Why the pattern engine is a Go re-implementation, not a vendored copy

The upstream implementation is GPL-3.0 via the patterniha component, and
its value is the *technique* (documented in the upstream README, preserved
in `extras/…/third_party/patterniha_sni_spoofing/`). The Go port
re-implements the published choreography (WinDivert SNIFF on handshake
packets → inject fake-SNI ClientHello at `seq = syn_seq + 1 − len(fake)`
after the final handshake ACK → verify the server ACK stays at
`syn_seq + 1` → relay; fall back to fragmentation on any anomaly) with
attribution and the upstream source preserved for reference. Licensing
implications of the lineage are documented in `docs/LICENSE-NOTES.md` —
the pattern package carries GPL-3.0 headers.

---

# SlipNet-equivalent tunnel coverage — license audit & implementation

The maintainer asked for tunnel-type coverage equivalent to
`github.com/anonvector/SlipNet`, integrated into the existing Vor app
under the Vor name with V2RayEZ's UI. SlipNet is AGPL-3.0 **and** the
maintainer's instruction is stricter than the license: treat it as
reference-only, never port its app code. What follows is the per-library
license audit and where each capability ended up.

## License audit of SlipNet's underlying libraries

| Library | License | Verdict |
|---|---|---|
| `dnstt` (David Fifield, via V2RayEZ addons) | permissive upstream | already integrated as the DNSTT addon (pre-existing) |
| `anonvector/noizdns` | AGPL-3.0 | **concepts only** (cover traffic, stealth labels, fan-out) — reimplemented, zero code copied |
| `anonvector/slipstream-rust` | Apache-2.0 | protocol reference only (Rust); QUIC coverage delivered via the Xray QUIC transport |
| `Fox-Fig/slipstream-rust-plus` | GPL-3.0 | not used |
| `anonvector/vaydns`, `vaydns-mobile` | repo unavailable at audit time | **flagged**; equivalent coverage exists via MasterDns (own implementation, richer: 8 balancer strategies, 6-lane MLQ, ARQ) |
| `meek-mobile`, `snowflake-mobile` (in SlipNet's tree) | **no license files present** | **flagged, not used**; upstream Tor meek/snowflake are usable separately if bridge support is extended |
| `lyrebird` (Tor PT) | Tor Project | not integrated (bridge config support only; PT binaries are a roadmap item) |
| NaiveProxy (klzgrad) | BSD-3 | protocol implemented from public docs; the Chromium C++ client is not embeddable in the Go binary — see caveats |
| SSH / TLS / WebSocket / HTTP CONNECT / DoH / DoT | IETF RFCs | generic techniques, implemented from scratch |

## What shipped (all from scratch, per platform)

**Desktop (Go)** — `desktop/internal/sshchain`, `dnsprobe`, `naive`:
- **ssh-chain** engine: standalone SSH, SSH-over-TLS (custom SNI), SSH-over-WebSocket (RFC 6455 client framing — CDN-compatible), SSH-over-HTTP-CONNECT (custom Host), pre-handshake payload injection, cipher selection (aes128-gcm / chacha20-poly1305 / aes128-ctr), local SOCKS5 front. Password + PEM key auth.
- **naive-https** engine: padded HTTPS CONNECT (X-Padding length randomization) over TLS.
- **dnsprobe**: resolver scanner/scorer (latency median-of-3, EDNS0 probe, NXDOMAIN-hijack detection via a random `.invalid` label, 0-100 composite score) + resolution transports UDP / DoT / DoH (RFC 8484). Dashboard endpoints `/api/dns/scan`, `/api/dns/transports`, `/api/sshchain/*`, `/api/naive/*` — license-gated.

**Android (Kotlin, V2RayEZ UI)** — `data/dns/ResolverScanner` + `DnsScannerScreen`:
- The scanner as a Tools screen (latency / EDNS / hijack columns, scored list), surfaced from the server-profile editor's DNS-tunnel resolver field (`dnsscan_open_from_editor`). Same semantics and scoring formula as the desktop reference. Strings in en/fa/ru (parity gate PASS).

**iOS** — `SocksProxy.swift`:
- Real SOCKS5 engine (RFC 1928 subset) with a **selectable resolution transport** (system DNS or DoH via cloudflare-dns.com JSON API) for upstream dials; the previous release shipped placeholder pseudo-code that never compiled (see the ipa fix commit).

**OpenWrt (universal ipk)** — UCI schema + LuCI:
- `config engine / ssh_chain / naive_https / dns_transport` sections and the matching LuCI CBI panel (engine selector + per-tunnel options). The C daemon keeps the SNI-relay strategies; DNS-tunnel coverage on routers comes from the per-architecture MasterDns binaries built in the same release; full Go tunnels on OpenWrt are documented as the follow-up (the ipk currently packages the C daemon + LuCI only).

## Honest caveats (not silently glossed)

1. **Chromium TLS fingerprinting**: the naive-https engine approximates the
   fingerprint (Chrome UA + modern TLS 1.3 defaults). Byte-perfect
   Chromium ClientHello mimicry needs the uTLS library — flagged as an
   enhancement, not claimed as shipped.
2. **QUIC/Slipstream**: delivered through the existing Xray QUIC transport
   (process core) rather than a native QUIC tunnel; native interop with
   slipstream servers is not claimed.
3. **NoizDNS wire compatibility**: the cover-traffic / stealth-label
   concepts are reimplemented for Vor's own DNS tunnels; wire
   compatibility with anonvector's NoizDNS server is not claimed (and its
   AGPL makes direct reuse unattractive for this codebase's license mix).
4. **Tor pluggable transports** (meek/snowflake/lyrebird): bridge-line
   configuration flows to the existing Tor engine; PT binaries are not
   bundled this release.
5. **iOS scope**: no PacketTunnelProvider (unsigned TrollStore builds
   cannot carry the network-extension entitlement); coverage = SOCKS5 +
   DoH resolution transport + the license gate.
