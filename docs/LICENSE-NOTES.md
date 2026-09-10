# Licensing Notes

Vor is a consolidation; each imported component keeps its own license, and
new shared code declares its license per module. No proprietary component
is included; no paid infrastructure is required.

| Component | Directory | License | Notes |
|---|---|---|---|
| V2RayEZ base app | `android/` (base) | MIT | © V2RayEZ authors |
| EasySNI / "V2RayEz" desktop base | `desktop/` (base) | MIT | © MacanDev; bundled WinDivert 2.2.2 (LGPL/GPL, upstream) |
| vor-core (shared library) | `core/vor-core/` | GPL-3.0 | derived from MICAFP (GPL-3.0) algorithms + shared data |
| MICAFP full source | `extras/micafp/` | GPL-3.0 | preserved as-is |
| Aethon (AetherGUI) | `extras/aethon/` | AGPL-3.0 | preserved; "Aether" is a CluvexStudio trademark — Vor uses its own name and does not use Aether branding |
| MSN-GUARD | `extras/msn-guard/` | AGPL-3.0 | preserved; guard features ported to `android/` as noted in MERGE-PLAN |
| UAC-SNI-Spoofer-Android | `extras/uac-sni-spoofer-android/` | mixed (Xray MPL-2.0; MIT components) | preserved; Kotlin engines ported |
| UAC-SNI-Spoofer-Windows | `extras/uac-sni-spoofer-windows/` | GPL-3.0 (patterniha) | preserved; pattern engine re-implemented in Go (`desktop/internal/pattern`, GPL-3.0 headers) |
| MasterDnsVPN | `dns-tunnel/` | per repo LICENSE | imported whole |
| Vor license system | `license/`, ports | CC0-style reference + code under each platform's module | spec + vectors are data; the ports follow their module licenses |
| OpenWrt package | `openwrt/vor/` | GPL-3.0 | matches MICAFP openwrt lineage |
| iOS target | `ios/` | GPL-3.0 | shared-code lineage |

If you redistribute Vor, keep every directory's LICENSE/NOTICE with it.
Trademark note: names and logos of the source projects (V2RayEZ, Aether,
Psiphon, Tor, …) belong to their owners; Vor ships no third-party branding.
