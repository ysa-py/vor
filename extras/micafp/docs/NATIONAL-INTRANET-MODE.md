# National Intranet Mode — UnifiedShield NextGen

## Context

Iran has increasingly used "national intranet" (شبکه ملی اطلاعات) as a tool for internet restriction. During protests or political events, the government can:

1. **Total Shutdown**: Cut international internet connectivity entirely
2. **Selective Blocking**: Block specific international services while allowing domestic ones
3. **Throttling**: Severely reduce international bandwidth
4. **DNS Hijacking**: Redirect international domain queries to domestic alternatives

## What is National Intranet Mode?

National Intranet Mode is a survival mode that allows users to:

- Access essential Iranian domestic services when international internet is cut
- Use P2P relay as a fallback for critical external communications
- Maintain security and privacy even in restricted mode
- Automatically detect when national intranet conditions are active

## Mode Levels

### 🟢 Smart Mode (Recommended)
- All `.ir` domains accessible
- P2P fallback for critical external services (Signal, Telegram, WhatsApp)
- DNS-over-HTTPS with domestic resolvers
- Best balance of access and functionality

### 🟡 Essential Mode
- Only banking, government, and health services
- All external access blocked
- Maximum security, minimum functionality
- Use during total shutdowns

### 🔴 Full Mode
- All `.ir` domains accessible
- No external access attempts
- DNS limited to Iranian resolvers
- Emergency information always available

## Supported Domestic Services

### Banking (بانکداری)
| Service | Domain | Purpose |
|---------|--------|---------|
| Bank Melli | bmi.ir | National bank |
| Mellat Bank | bankmellat.ir | Payment services |
| Saman Bank | sb24.ir | Online banking |
| Shaparak | shaparak.ir | Payment network |
| Parsian E-Commerce | pec.ir | E-commerce gateway |

### Government (دولتی)
| Service | Domain | Purpose |
|---------|--------|---------|
| Government Portal | dolat.ir | Government services |
| Irancell | irancell.ir | Telecom services |
| MCI | mci.ir | Mobile operator |
| Post | post.ir | Postal services |

### Education (آموزشی)
| Service | Domain | Purpose |
|---------|--------|---------|
| Sharif University | sharif.edu | University portal |
| Tehran University | ut.ac.ir | University portal |
| Amirkabir University | aut.ac.ir | University portal |
| .ac.ir / .edu.ir | * | All academic domains |

### Health (بهداشت)
| Service | Domain | Purpose |
|---------|--------|---------|
| Social Security | tamin.ir | Insurance/services |
| FDA Iran | fda.ir | Drug/food safety |
| Ministry of Health | behdasht.gov.ir | Health services |

### News (اخبار)
| Service | Domain | Purpose |
|---------|--------|---------|
| ISNA | isna.ir | Student news agency |
| IRNA | irna.ir | Republic news agency |
| Mehr News | mehrnews.com | News agency |
| Tasnim | tasnimnews.com | News agency |

### Essential (ضروری)
| Service | Domain | Purpose |
|---------|--------|---------|
| Digikala | digikala.com | E-commerce |
| Snapp | snapp.ir | Ride-hailing |
| Divar | divar.ir | Classifieds |
| ESAM | esam.ir | E-government |

## P2P Fallback

When national intranet mode is active, P2P relay provides access to critical external services:

| Service | P2P Priority | Purpose |
|---------|-------------|---------|
| Signal | Critical | Secure messaging |
| Telegram | Critical | Group communication |
| WhatsApp | High | International messaging |
| Twitter/X | Medium | Information access |
| YouTube | Low | Video content |

### How P2P Fallback Works

```
1. National intranet detected
2. User enables Smart/Essential mode
3. P2P relay connects to exit nodes in Turkey/Germany
4. Only traffic for critical services routes through P2P
5. Domestic traffic goes through normal internet
6. All P2P traffic is encrypted (3-layer onion)
```

## Auto-Detection

The system can automatically detect national intranet conditions:

### Detection Methods
1. **DNS Test**: Query known-international domains; if they resolve to Iranian IPs → intranet
2. **Connectivity Test**: Try connecting to international servers; if all fail → shutdown
3. **Bandwidth Test**: Measure international bandwidth; if severely throttled → restriction
4. **Community Reports**: Aggregate user reports of connectivity issues

### Detection Flow
```rust
fn detect_intranet_conditions() -> IntranetDetection {
    let dns_ok = test_dns_resolution("google.com");     // Should not resolve to Iranian IP
    let intl_ok = test_international_connectivity();     // Should connect to foreign servers
    let bandwidth_ok = test_international_bandwidth();   // Should be > 1 Mbps

    if !dns_ok && !intl_ok {
        return IntranetDetection::TotalShutdown;        // Full national intranet
    }
    if !dns_ok || !bandwidth_ok {
        return IntranetDetection::PartialRestriction;   // Selective blocking
    }
    return IntranetDetection::Normal;                    // Normal internet
}
```

## DNS Configuration

During national intranet mode, DNS is reconfigured:

### Domestic Resolvers
| Provider | IP | Notes |
|----------|----|----|
| Electro | 78.157.42.100 | Iranian, generally reliable |
| Shecan | 178.22.122.100 | Iranian, bypasses some blocks |

### Chinese Resolvers (Accessible from Iran)
| Provider | IP | Notes |
|----------|----|----|
| AliDNS | 223.5.5.5 | Alibaba Cloud, fast from Iran |
| DNSPod | 119.29.29.29 | Tencent Cloud, reliable |

### ⚠️ BLOCKED Resolvers
| Provider | IP | Status |
|----------|----|----|
| Cloudflare | 1.1.1.1 | ❌ BLOCKED in Iran |
| Google | 8.8.8.8 | ⚠️ Often poisoned |
| Quad9 | 9.9.9.9 | ⚠️ Often poisoned |

## Emergency Information

Always accessible, even in total shutdown:

| Service | Number | Available |
|---------|--------|-----------|
| Police | 110 | ✅ Always |
| Ambulance | 115 | ✅ Always |
| Fire | 125 | ✅ Always |
| Emergency | 112 | ✅ Always |

## UI Flow

```
Home Screen
    │
    ├── Auto-detect button → Checks conditions
    │
    ├── Mode selection
    │   ├── 🟢 Smart (all national + P2P fallback)
    │   ├── 🟡 Essential (banking/gov/health only)
    │   └── 🔴 Full (all .ir only)
    │
    ├── Service categories
    │   ├── 🏦 Banking
    │   ├── 🏛️ Government
    │   ├── 🎓 Education
    │   ├── 🏥 Health
    │   ├── 📰 News
    │   └── 🛒 Essential
    │
    └── Emergency info → Always accessible
```

## Security Considerations

1. **Even in intranet mode, VPN tunnel remains active** for P2P fallback traffic
2. **DNS queries are encrypted** via DoH (AliDNS/DNSPod)
3. **No logging** of accessed domestic services
4. **Kill switch remains active** — if VPN disconnects, P2P traffic is blocked
5. **P2P relay is encrypted** — Iranian ISPs cannot read relayed traffic
