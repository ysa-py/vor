# UnifiedShield NextGen — Architecture

## Overview

UnifiedShield NextGen is a multi-platform anti-censorship system designed specifically for Iran's internet landscape. It combines 9 anti-DPI cores, P2P relay networking, national intranet mode, and AI-powered orchestration into a unified experience.

```
┌─────────────────────────────────────────────────────────────────┐
│                    UNIFIEDSHIELD NEXTGEN                        │
├──────────────┬──────────────┬──────────────┬────────────────────┤
│  Flutter App │  Next.js     │  Rust Daemon │  P2P Network       │
│  (Cross-Plat)│  Dashboard   │  (Core Engine)│  (libp2p)         │
├──────────────┴──────────────┴──────────────┴────────────────────┤
│                     IPC / WebSocket Layer                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────┐ ┌──────┐ ┌─────┐ ┌────┐ ┌─────┐ ┌──────┐ ┌────────┐ │
│  │XTLS │ │Hyste-│ │TUIC │ │SS  │ │VLESS│ │Wire- │ │Trojan  │ │
│  │Real-│ │ria2  │ │v5   │ │    │ │     │ │Guard │ │        │ │
│  │ity  │ │      │ │     │ │    │ │     │ │      │ │        │ │
│  └─────┘ └──────┘ └─────┘ └────┘ └─────┘ └──────┘ └────────┘ │
│  ┌──────────┐ ┌──────────┐                                      │
│  │NaïveProxy│ │P2P-Relay │                                      │
│  └──────────┘ └──────────┘                                      │
├─────────────────────────────────────────────────────────────────┤
│                     UCB1 Bandit Selector                         │
├─────────────────────────────────────────────────────────────────┤
│  Kill Switch │ Auto-Reconnect │ Obfuscation │ Geo-Router        │
├─────────────────────────────────────────────────────────────────┤
│                     Platform VPN Interface                       │
│  Android VpnService │ iOS NetworkExtension │ TUN/TAP (Desktop)  │
└─────────────────────────────────────────────────────────────────┘
```

## Components

### 1. Flutter Cross-Platform App
- **Platforms**: Android, iOS, Windows, macOS, Linux
- **State Management**: Riverpod
- **Localization**: Persian (فارسی) primary, English
- **Key Features**:
  - One-tap VPN connection with UCB1 core selection
  - Real-time speed gauge and connection stats
  - Core management with 9 anti-censorship protocols
  - National intranet mode
  - Security audit (DPI test, DNS leak, IP leak)
  - OTA updates from Chinese CDN mirrors

### 2. Next.js Dashboard
- **Stack**: Next.js 16, React 19, shadcn/ui, Zustand, Prisma, Recharts
- **API Routes**: 16 API endpoints for all subsystems
- **Real-time**: WebSocket for live stats
- **Database**: PostgreSQL via Prisma ORM
- **Features**:
  - Real-time speed/latency visualization
  - Core performance comparison charts
  - Threat intelligence dashboard
  - P2P network topology view
  - Intranet mode management
  - OTA update management

### 3. Rust Daemon
- **IPC**: Unix domain socket + gRPC
- **Functions**: VPN tunnel management, packet routing, DPI evasion
- **Native**: Cross-compiled for all target platforms
- **Security**: Sandboxed, minimal permissions

### 4. P2P Network
- **Protocol**: libp2p
- **Features**: DHT discovery, NAT traversal, encrypted relay
- **Use Case**: Serverless connectivity when all servers are blocked

## 9 Core Protocols

| # | Core | Protocol | Port | Best For | DPI Resistance |
|---|------|----------|------|----------|---------------|
| 1 | XTLS-Reality | XTLS | 443 | Hardened environments | ★★★★★ |
| 2 | Hysteria2 | QUIC | 8443 | High-speed, UDP traffic | ★★★★☆ |
| 3 | TUICv5 | QUIC | 8443 | UDP relay, gaming | ★★★★☆ |
| 4 | Shadowsocks | SOCKS5+AEAD | 8388 | Compatibility | ★★★☆☆ |
| 5 | VLESS | XTLS | 443 | Lightweight | ★★★★☆ |
| 6 | WireGuard | WireGuard | 51820 | Speed, kernel-level | ★★☆☆☆ |
| 7 | Trojan | TLS | 443 | HTTPS mimicry | ★★★★☆ |
| 8 | NaïveProxy | HTTP/2 | 443 | Chrome fingerprint | ★★★★★ |
| 9 | P2P-Relay | libp2p | Dynamic | Serverless fallback | ★★★☆☆ |

## UCB1 Bandit Algorithm

The UCB1 (Upper Confidence Bound 1) algorithm selects the optimal core by balancing **exploration** (testing untried cores) and **exploitation** (using known-good cores):

```
UCB1(core) = μ(core) + c × √(ln(N) / n(core))
```

Where:
- `μ(core)` = average success rate of the core
- `N` = total connection attempts across all cores
- `n(core)` = connection attempts for this specific core
- `c` = exploration factor (default: √2 ≈ 1.414)

This ensures that:
- New cores are explored (high exploration bonus when n is small)
- Proven cores are exploited (high μ when success rate is high)
- The system adapts to changing network conditions over time

## Iran-Specific Design Decisions

### Cloudflare is BLOCKED
All Cloudflare-dependent infrastructure is avoided. Chinese CDNs serve as primary mirrors:
- **Alibaba Cloud OSS** (Shanghai + Hong Kong) — PRIMARY
- **Tencent COS** (Hong Kong) — PRIMARY
- **GitHub Releases** — SECONDARY (may be slow/throttled)

### DNS Configuration
- AliDNS (223.5.5.5) and DNSPod (119.29.29.29) as upstream resolvers
- Electro (78.157.42.100) and Shecan (178.22.122.100) as Iranian alternatives
- DNS-over-HTTPS enabled by default

### ISP-Specific Routing
Different Iranian ISPs have different censorship profiles:
- **MCI (AS197207)**: Protocol-based throttling, active probing
- **Irancell (AS44244)**: QUIC-friendly, throttles WireGuard
- **Rightel (AS49581)**: Less aggressive DPI
- **Shatel (AS31549)**: Protocol-based throttling
- **ParsOnline (AS16322)**: Relatively open
- **Mokhaberat (AS58224)**: Heavy DPI, active probing

## Data Flow

```
User taps "Connect"
    │
    ▼
Flutter App → IPC → Rust Daemon
    │
    ▼
UCB1 Selector picks best core
    │
    ▼
Core connects to server (via optimal ISP route)
    │
    ▼
Obfuscation layer activates (TLS camo + padding)
    │
    ▼
VPN tunnel established → Platform VPN interface
    │
    ▼
Kill switch activates (block non-VPN traffic)
    │
    ▼
Stats stream back → UI updates in real-time
```
