# P2P Serverless Relay — UnifiedShield NextGen

## Motivation

When all VPN servers are blocked (as happens during Iran's internet shutdowns), users need a way to reach the open internet. P2P relay networks solve this by allowing users to route traffic through each other.

## Architecture

```
┌──────────┐     ┌──────────┐     ┌──────────┐
│  Peer A  │────▶│  Peer B  │────▶│  Peer C  │────▶ Internet
│  (Iran)  │     │  (Turkey) │     │(Germany) │
└──────────┘     └──────────┘     └──────────┘
     │                │                │
     └────────────────┴────────────────┘
              DHT Discovery Layer
              (libp2p Kademlia)
```

### Node Types

1. **Source Peer** (Iran): Initiates connection, sends traffic
2. **Relay Peer** (any country): Forwards traffic between peers
3. **Exit Peer** (free country): Has internet access, acts as final hop

### Key Properties

- **No Central Server**: Uses DHT for peer discovery
- **Encrypted End-to-End**: Each hop adds encryption layer (onion routing)
- **NAT Traversal**: Uses libp2p's hole-punching for NAT bypass
- **Incentive System**: Relay providers earn priority for their own traffic
- **Trust Scoring**: Peers build reputation over time

## Protocol Stack

```
┌─────────────────────────────┐
│     Application Layer        │
│  (HTTP, DNS, etc.)          │
├─────────────────────────────┤
│     Onion Encryption         │
│  (Layer per relay hop)      │
├─────────────────────────────┤
│     Relay Protocol           │
│  (Hop-by-hop messaging)     │
├─────────────────────────────┤
│     libp2p Transport         │
│  (Noise encrypted channels) │
├─────────────────────────────┤
│     Network                  │
│  (TCP/QUIC/WebSocket)       │
└─────────────────────────────┘
```

## DHT Discovery

Uses Kademlia DHT for decentralized peer discovery:

1. **Bootstrap**: Connect to known bootstrap nodes
2. **Lookup**: Query DHT for peers near target key
3. **Routing**: Find optimal relay path through DHT
4. **Refresh**: Periodically refresh routing table

```rust
// DHT discovery flow
async fn discover_peers(target: PeerId) -> Vec<PeerInfo> {
    let mut peers = vec![];
    for bootstrap in BOOTSTRAP_NODES {
        let result = kademlia.get_closest_peers(target).await;
        peers.extend(result);
    }
    peers.sort_by(|a, b| a.latency.cmp(&b.latency));
    peers
}
```

## NAT Traversal

Most Iranian users are behind NAT. The P2P network supports:

### Hole Punching (UDP)
1. Both peers send UDP packets to each other simultaneously
2. NAT devices create forwarding rules
3. Direct connection established

### Relay Fallback
1. If hole punching fails, use a relay peer
2. Relay peer has public IP address
3. Both source and exit connect to relay
4. Traffic flows: Source → Relay → Exit

### TURN-like Protocol
1. For restrictive NATs (symmetric NAT)
2. Relay acts as full proxy
3. Higher latency but guaranteed connectivity

## Relay Path Selection

The system selects relay paths using multiple criteria:

| Factor | Weight | Description |
|--------|--------|-------------|
| Latency | 30% | Lower is better |
| Bandwidth | 25% | Higher is better |
| Trust Score | 20% | Based on history |
| Uptime | 15% | Stability metric |
| Geographic | 10% | Path diversity |

### Path Example
```
Iran (Source) → Turkey (Relay, 35ms) → Germany (Exit, 70ms) → Internet
Total latency: ~105ms (acceptable for browsing)
```

## Encryption

### Onion Routing
Each relay hop adds a layer of encryption:

```
Original payload: "GET https://example.com"

Layer 3 (Exit decrypts):  "GET https://example.com"
Layer 2 (Relay decrypts): [Encrypted for Exit]
Layer 1 (Source encrypts): [Encrypted for Relay [Encrypted for Exit]]

Each relay only knows:
- Previous hop (where data came from)
- Next hop (where to forward)
- Cannot read payload content
```

### Key Exchange
- **Protocol**: X25519 ECDH
- **Symmetric**: ChaCha20-Poly1305 AEAD
- **Key Rotation**: Every 5 minutes or 1MB of data

## Incentive System

Relay providers earn tokens that give them priority:

| Action | Tokens Earned |
|--------|---------------|
| Relay 1MB of traffic | 1 token |
| Stay online 1 hour | 5 tokens |
| Provide exit node | 3 tokens/hour |

| Priority Level | Tokens Required |
|----------------|----------------|
| Low | 0 tokens (best effort) |
| Medium | 50 tokens |
| High | 200 tokens |
| Critical | 500 tokens |

## Safety Considerations

### For Relay Operators
- Never see unencrypted traffic (onion encryption)
- Cannot determine traffic source or destination
- Legal protection: operating a relay is like operating a VPN
- Can set bandwidth limits and operating hours

### For Source Users
- Traffic is end-to-end encrypted
- No single relay can identify you
- If one relay is compromised, other layers protect you
- Exit node sees destination but not source

### Anti-Abuse
- Content filtering at exit nodes (block illegal content)
- Rate limiting per source peer
- Reputation system for bad actors
- Blacklist for known malicious peers

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| DHT Discovery | ✅ Done | Kademlia via libp2p |
| NAT Traversal | ✅ Done | Hole punching + relay |
| Onion Encryption | ✅ Done | 3-layer ChaCha20 |
| Path Selection | ✅ Done | Multi-criteria scoring |
| Incentive System | 🔄 In Progress | Token system |
| Exit Node Rotation | ✅ Done | Every 10 minutes |
| Bandwidth Limiting | ✅ Done | Per-peer limits |
| Mobile Support | ✅ Done | Flutter + libp2p |

## Configuration

```json
{
  "p2p": {
    "enabled": true,
    "actAsRelay": true,
    "maxRelayConnections": 10,
    "bandwidthLimitKB": 5120,
    "bootstrapNodes": [
      "/dns4/p2p1.unifiedshield.io/tcp/4001/p2p/QmBoot1",
      "/dns4/p2p2.unifiedshield.io/tcp/4001/p2p/QmBoot2"
    ],
    "preferredCountries": ["TR", "DE", "NL", "AE"],
    "onionLayers": 3,
    "keyRotationInterval": 300,
    "exitNodeRotation": 600
  }
}
```
