# ENGINEERING DIRECTIVE v70 — DUAL-MODE TRANSPORT ARCHITECTURE

## 1. Data Flow Architecture Diagrams

### Mode A: Parallel Multipath Transport ("fast" — Default)
```
+-------------------------------------------------------------------------+
|                              TUN Interface                              |
|                   (Raw IP Packets / Zero-Copy Buffers)                  |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
|                  Adaptive Path Scheduler (Trait Impl)                   |
|       Metrics: RTT, Jitter, Loss %, BW, Congestion State, Battery       |
|                  Score -> Schedule -> Rebalance Loop                    |
+-------------------------------------------------------------------------+
          |                         |                         |
 (Path 1: Top Score)       (Path 2: Secondary)       (Path 3: Backup)
          v                         v                         v
+-------------------+     +-------------------+     +-------------------+
|  1-Layer AEAD Enc |     |  1-Layer AEAD Enc |     |  1-Layer AEAD Enc |
|  ChaCha20 / GCM   |     |  ChaCha20 / GCM   |     |  ChaCha20 / GCM   |
|   PMTUD Probing   |     |   PMTUD Probing   |     |   PMTUD Probing   |
+-------------------+     +-------------------+     +-------------------+
          |                         |                         |
          v                         v                         v
+-------------------+     +-------------------+     +-------------------+
| QUIC Path 1 (UDP) |     | QUIC Path 2 (UDP) |     | QUIC Path 3 (UDP) |
|  Relay: EU-West   |     | Relay: SG-East    |     | Relay: HK-China   |
|     (BBR CC)      |     |     (BBR CC)      |     |    (CUBIC CC)     |
+-------------------+     +-------------------+     +-------------------+
          \                         |                         /
           \                        |                        /
            v                       v                       v
+-------------------------------------------------------------------------+
|                            Target Internet                              |
+-------------------------------------------------------------------------+
```

### Mode B: 5-Hop Nested Layered Encryption ("layered" — Opt-in)
```
+-------------------------------------------------------------------------+
|                              TUN Interface                              |
|                   (Raw IP Packets / Zero-Copy Buffers)                  |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
|                  5-Layer Nested Encapsulation Engine                    |
|       (Inside-Out: Hop 5 -> Hop 4 -> Hop 3 -> Hop 2 -> Hop 1)           |
|                Sphinx-format / Noise Protocol State Machine             |
|          [Fail-Closed Policy: Drops packet if any hop is down]          |
+-------------------------------------------------------------------------+
                                    |
                     (Layer 1 Encapsulation: Hop 1)
                                    v
+-------------------------------------------------------------------------+
|               Hop 1: Entry Gateway (EU-West / Frankfurt)                |
|                    [Peels Layer 1 -> Discovers Hop 2]                   |
+-------------------------------------------------------------------------+
                                    |
                     (Layer 2 Encapsulation: Hop 2)
                                    v
+-------------------------------------------------------------------------+
|               Hop 2: Mixnet Relay (SG-East / Singapore)                 |
|                    [Peels Layer 2 -> Discovers Hop 3]                   |
+-------------------------------------------------------------------------+
                                    |
                     (Layer 3 Encapsulation: Hop 3)
                                    v
+-------------------------------------------------------------------------+
|               Hop 3: Anonymity Core (US-East / Virginia)                |
|                    [Peels Layer 3 -> Discovers Hop 4]                   |
+-------------------------------------------------------------------------+
                                    |
                     (Layer 4 Encapsulation: Hop 4)
                                    v
+-------------------------------------------------------------------------+
|               Hop 4: Bridge Relay (JP-Central / Tokyo)                  |
|                    [Peels Layer 4 -> Discovers Hop 5]                   |
+-------------------------------------------------------------------------+
                                    |
                     (Layer 5 Encapsulation: Hop 5)
                                    v
+-------------------------------------------------------------------------+
|               Hop 5: Exit Gateway (CH-Alibaba / Hong Kong)              |
|                   [Peels Layer 5 -> Sends Clear Packet]                 |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
|                            Target Internet                              |
+-------------------------------------------------------------------------+
```

---

## 2. iOS Memory-Safety Checklist

| Item | Requirement | Implementation in v70 |
|---|---|---|
| Hard OS Limit | 15–50 MB RAM operational ceiling | Active allocation cap at 35 MB |
| Async Runtime | Single-threaded `rt-current-thread` | Compiled with `tokio/rt` single-threaded on `cfg(target_os = "ios")` |
| Allocator | Lightweight allocator | `mimalloc` / system default on iOS |
| Release Profile | Small binary & zero-cost unwinding | `opt-level = "z"`, `lto = true`, `panic = "abort"`, `strip = true` |
| Crypto State Budget | Mode A vs Mode B state budgeting | Mode A (~12 MB) / Mode B (~22 MB) actively monitored |

---

## 3. FFI Blueprint

- **Android**: `CoreBridge.nativeInitDualModeTransport`, `CoreBridge.nativeSwitchTransportMode`, `CoreBridge.nativeGetDualModeTelemetry`, `CoreBridge.nativeRunDualModeBenchmark`.
- **iOS**: `ios_dual_mode_init`, `ios_get_memory_usage_bytes`, `ios_switch_mode`.
