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
