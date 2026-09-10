#!/usr/bin/env python3
"""
Feature Engineering for DPI Classifier — 47 features

Extracts 47 numerical features from raw network flow data
for training the DPI classifier and traffic predictor.
"""

import numpy as np
from typing import List, Dict, Any
from dataclasses import dataclass


@dataclass
class FlowFeatures:
    """47 features extracted from a network flow"""

    # ─── Packet Inter-Arrival Time (4) ───
    iat_mean: float        # 0: Mean inter-arrival time (ms)
    iat_std: float         # 1: Std dev of inter-arrival time
    iat_min: float         # 2: Min inter-arrival time
    iat_max: float         # 3: Max inter-arrival time

    # ─── Packet Size (4) ───
    size_mean: float       # 4: Mean packet size (bytes)
    size_std: float        # 5: Std dev of packet size
    size_min: float        # 6: Min packet size
    size_max: float        # 7: Max packet size

    # ─── Flow Meta (5) ───
    duration: float        # 8: Flow duration (ms)
    total_packets: int     # 9: Total packets in flow
    total_bytes: int       # 10: Total bytes in flow
    fwd_packets: int       # 11: Forward (client→server) packets
    bwd_packets: int       # 12: Backward (server→client) packets

    # ─── TCP Flags (5) ───
    tcp_syn: int           # 13: SYN flag count
    tcp_ack: int           # 14: ACK flag count
    tcp_fin: int           # 15: FIN flag count
    tcp_rst: int           # 16: RST flag count
    tcp_window: int        # 17: TCP window size

    # ─── TLS Features (6) ───
    tls_version: int       # 18: TLS version (0x0301=TLS1.0, 0x0303=TLS1.2)
    tls_cipher_count: int  # 19: Number of cipher suites
    tls_ext_count: int     # 20: Number of extensions
    tls_sni_len: int       # 21: SNI string length
    tls_alpn_count: int    # 22: ALPN protocol count
    tls_session_id_len: int  # 23: Session ID length

    # ─── Connection (4) ───
    rtt_mean: float        # 24: Mean RTT (ms)
    rtt_std: float         # 25: RTT standard deviation
    retransmits: int       # 26: Retransmission count
    out_of_order: int      # 27: Out-of-order packet count

    # ─── DPI Signatures (6) ───
    rst_after_syn: int     # 28: RST received after SYN (0/1)
    rst_timing: float      # 29: RST timing in ms (95-320 = FAVA)
    http_status: int       # 30: HTTP response status code
    dns_rcode: int         # 31: DNS response code
    dns_answer_count: int  # 32: DNS answer count
    dns_poisoned: int      # 33: DNS response contains poison IP (0/1)
    sni_filtered: int      # 34: SNI was filtered by DPI (0/1)

    # ─── Statistical (3) ───
    entropy_size: float    # 35: Shannon entropy of packet sizes
    entropy_iat: float     # 36: Shannon entropy of inter-arrival times
    burst_count: int       # 37: Number of burst periods

    # ─── Extended (9) ───
    tcp_psh: int           # 38: PSH flag count
    tcp_urg: int           # 39: URG flag count
    size_median: float     # 40: Median packet size
    bytes_per_sec: float   # 41: Bytes per second
    packets_per_sec: float # 42: Packets per second
    active_duration: float # 43: Active time (ms)
    idle_duration: float   # 44: Idle time (ms)
    fwd_bytes: int         # 45: Forward bytes
    bwd_bytes: int         # 46: Backward bytes

    def to_array(self) -> np.ndarray:
        """Convert to numpy array of 47 features"""
        return np.array([
            self.iat_mean, self.iat_std, self.iat_min, self.iat_max,
            self.size_mean, self.size_std, self.size_min, self.size_max,
            self.duration, self.total_packets, self.total_bytes,
            self.fwd_packets, self.bwd_packets,
            self.tcp_syn, self.tcp_ack, self.tcp_fin, self.tcp_rst, self.tcp_window,
            self.tls_version, self.tls_cipher_count, self.tls_ext_count,
            self.tls_sni_len, self.tls_alpn_count, self.tls_session_id_len,
            self.rtt_mean, self.rtt_std, self.retransmits, self.out_of_order,
            self.rst_after_syn, self.rst_timing, self.http_status,
            self.dns_rcode, self.dns_answer_count, self.dns_poisoned,
            self.sni_filtered,
            self.entropy_size, self.entropy_iat, self.burst_count,
            self.tcp_psh, self.tcp_urg, self.size_median,
            self.bytes_per_sec, self.packets_per_sec,
            self.active_duration, self.idle_duration,
            self.fwd_bytes, self.bwd_bytes,
        ], dtype=np.float32)


def compute_shannon_entropy(values: List[float]) -> float:
    """Compute Shannon entropy of a list of values"""
    if not values or len(values) < 2:
        return 0.0

    hist, _ = np.histogram(values, bins=min(20, len(values)))
    probs = hist / hist.sum()
    probs = probs[probs > 0]

    return float(-np.sum(probs * np.log2(probs)))


def extract_features_from_packets(
    packets: List[Dict[str, Any]],
    metadata: Dict[str, Any],
) -> FlowFeatures:
    """
    Extract 47 features from a list of packet records.

    Each packet should have:
        - timestamp: float (epoch ms)
        - size: int (bytes)
        - direction: 'fwd' | 'bwd'
        - tcp_flags: dict (syn, ack, fin, rst, psh, urg)
        - tcp_window: int
    """

    if not packets:
        return FlowFeatures(
            iat_mean=0, iat_std=0, iat_min=0, iat_max=0,
            size_mean=0, size_std=0, size_min=0, size_max=0,
            duration=0, total_packets=0, total_bytes=0,
            fwd_packets=0, bwd_packets=0,
            tcp_syn=0, tcp_ack=0, tcp_fin=0, tcp_rst=0, tcp_window=0,
            tls_version=0, tls_cipher_count=0, tls_ext_count=0,
            tls_sni_len=0, tls_alpn_count=0, tls_session_id_len=0,
            rtt_mean=0, rtt_std=0, retransmits=0, out_of_order=0,
            rst_after_syn=0, rst_timing=0, http_status=0,
            dns_rcode=0, dns_answer_count=0, dns_poisoned=0, sni_filtered=0,
            entropy_size=0, entropy_iat=0, burst_count=0,
            tcp_psh=0, tcp_urg=0, size_median=0,
            bytes_per_sec=0, packets_per_sec=0,
            active_duration=0, idle_duration=0,
            fwd_bytes=0, bwd_bytes=0,
        )

    # Sort by timestamp
    packets = sorted(packets, key=lambda p: p["timestamp"])

    # Inter-arrival times
    timestamps = [p["timestamp"] for p in packets]
    iats = [timestamps[i+1] - timestamps[i] for i in range(len(timestamps) - 1)]

    # Packet sizes
    sizes = [p["size"] for p in packets]

    # Directions
    fwd_pkts = [p for p in packets if p.get("direction") == "fwd"]
    bwd_pkts = [p for p in packets if p.get("direction") == "bwd"]

    # TCP flags
    tcp_syn = sum(1 for p in packets if p.get("tcp_flags", {}).get("syn", False))
    tcp_ack = sum(1 for p in packets if p.get("tcp_flags", {}).get("ack", False))
    tcp_fin = sum(1 for p in packets if p.get("tcp_flags", {}).get("fin", False))
    tcp_rst = sum(1 for p in packets if p.get("tcp_flags", {}).get("rst", False))
    tcp_psh = sum(1 for p in packets if p.get("tcp_flags", {}).get("psh", False))
    tcp_urg = sum(1 for p in packets if p.get("tcp_flags", {}).get("urg", False))

    # Duration
    duration = timestamps[-1] - timestamps[0] if len(timestamps) > 1 else 0

    # Throughput
    bytes_per_sec = (sum(sizes) / (duration / 1000)) if duration > 0 else 0
    packets_per_sec = (len(packets) / (duration / 1000)) if duration > 0 else 0

    # Burst detection (2+ packets within 10ms)
    burst_count = 0
    in_burst = False
    for i, iat in enumerate(iats):
        if iat < 10:
            if not in_burst:
                burst_count += 1
                in_burst = True
        else:
            in_burst = False

    # DPI-specific
    rst_after_syn = metadata.get("rst_after_syn", 0)
    rst_timing = metadata.get("rst_timing_ms", 0)
    http_status = metadata.get("http_status_code", 0)

    return FlowFeatures(
        iat_mean=np.mean(iats) if iats else 0,
        iat_std=np.std(iats) if len(iats) > 1 else 0,
        iat_min=min(iats) if iats else 0,
        iat_max=max(iats) if iats else 0,

        size_mean=np.mean(sizes),
        size_std=np.std(sizes) if len(sizes) > 1 else 0,
        size_min=min(sizes),
        size_max=max(sizes),

        duration=duration,
        total_packets=len(packets),
        total_bytes=sum(sizes),
        fwd_packets=len(fwd_pkts),
        bwd_packets=len(bwd_pkts),

        tcp_syn=tcp_syn,
        tcp_ack=tcp_ack,
        tcp_fin=tcp_fin,
        tcp_rst=tcp_rst,
        tcp_window=packets[0].get("tcp_window", 0) if packets else 0,

        tls_version=metadata.get("tls_version", 0),
        tls_cipher_count=metadata.get("tls_cipher_count", 0),
        tls_ext_count=metadata.get("tls_ext_count", 0),
        tls_sni_len=metadata.get("tls_sni_len", 0),
        tls_alpn_count=metadata.get("tls_alpn_count", 0),
        tls_session_id_len=metadata.get("tls_session_id_len", 0),

        rtt_mean=metadata.get("rtt_mean", 0),
        rtt_std=metadata.get("rtt_std", 0),
        retransmits=metadata.get("retransmits", 0),
        out_of_order=metadata.get("out_of_order", 0),

        rst_after_syn=rst_after_syn,
        rst_timing=rst_timing,
        http_status=http_status,
        dns_rcode=metadata.get("dns_rcode", 0),
        dns_answer_count=metadata.get("dns_answer_count", 0),
        dns_poisoned=metadata.get("dns_poisoned", 0),
        sni_filtered=metadata.get("sni_filtered", 0),

        entropy_size=compute_shannon_entropy(sizes),
        entropy_iat=compute_shannon_entropy(iats),
        burst_count=burst_count,

        tcp_psh=tcp_psh,
        tcp_urg=tcp_urg,
        size_median=float(np.median(sizes)),
        bytes_per_sec=bytes_per_sec,
        packets_per_sec=packets_per_sec,
        active_duration=metadata.get("active_duration", duration),
        idle_duration=metadata.get("idle_duration", 0),
        fwd_bytes=sum(p["size"] for p in fwd_pkts),
        bwd_bytes=sum(p["size"] for p in bwd_pkts),
    )


# Feature names for reference
FEATURE_NAMES = [
    "iat_mean", "iat_std", "iat_min", "iat_max",
    "size_mean", "size_std", "size_min", "size_max",
    "duration", "total_packets", "total_bytes",
    "fwd_packets", "bwd_packets",
    "tcp_syn", "tcp_ack", "tcp_fin", "tcp_rst", "tcp_window",
    "tls_version", "tls_cipher_count", "tls_ext_count",
    "tls_sni_len", "tls_alpn_count", "tls_session_id_len",
    "rtt_mean", "rtt_std", "retransmits", "out_of_order",
    "rst_after_syn", "rst_timing", "http_status",
    "dns_rcode", "dns_answer_count", "dns_poisoned", "sni_filtered",
    "entropy_size", "entropy_iat", "burst_count",
    "tcp_psh", "tcp_urg", "size_median",
    "bytes_per_sec", "packets_per_sec",
    "active_duration", "idle_duration",
    "fwd_bytes", "bwd_bytes",
]

assert len(FEATURE_NAMES) == 47, f"Expected 47 features, got {len(FEATURE_NAMES)}"
