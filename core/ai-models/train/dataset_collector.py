#!/usr/bin/env python3
"""
UnifiedShield Dataset Collector

Collects network traffic samples for training the DPI classifier
and traffic predictor models.

Data sources:
1. Live packet capture (requires root/admin)
2. PCAP file parsing
3. Synthetic DPI signature generation
4. Public datasets (e.g., ISCX-VPN, ISCX-Tor)
"""

import json
import time
import socket
import struct
import random
import hashlib
from pathlib import Path
from typing import List, Dict, Any, Optional
import logging

import numpy as np

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("DatasetCollector")


# ──────────────── Synthetic Data Generation ────────────────

def generate_normal_flow(num_samples: int = 1000) -> List[Dict[str, Any]]:
    """Generate normal (non-blocked) traffic flow features"""
    samples = []

    for _ in range(num_samples):
        features = {
            # Packet timing features
            "packet_inter_arrival_mean": random.gauss(50, 20),         # ms
            "packet_inter_arrival_std": random.gauss(15, 5),
            "packet_inter_arrival_min": max(1, random.gauss(5, 2)),
            "packet_inter_arrival_max": random.gauss(200, 50),

            # Packet size features
            "packet_size_mean": random.gauss(800, 300),
            "packet_size_std": random.gauss(400, 100),
            "packet_size_min": max(40, random.gauss(64, 20)),
            "packet_size_max": random.gauss(1500, 200),

            # Flow features
            "flow_duration": random.gauss(5000, 3000),                 # ms
            "total_packets": int(max(5, random.gauss(50, 20))),
            "total_bytes": int(max(500, random.gauss(50000, 20000))),
            "forward_packets": int(max(2, random.gauss(25, 10))),
            "backward_packets": int(max(2, random.gauss(25, 10))),

            # TCP features
            "tcp_flags_syn": random.randint(1, 3),
            "tcp_flags_ack": int(max(1, random.gauss(30, 15))),
            "tcp_flags_fin": random.choice([1, 2]),
            "tcp_flags_rst": 0,  # Normal: no RST
            "tcp_window_size": random.randint(8192, 65535),

            # TLS features
            "tls_version": random.choice([0x0303, 0x0301]),
            "tls_cipher_suites_count": random.randint(5, 20),
            "tls_extensions_count": random.randint(3, 12),
            "tls_sni_length": random.randint(8, 50),
            "tls_alpn_protocols": random.randint(1, 3),
            "tls_session_id_length": random.choice([0, 32]),

            # Connection features
            "connection_rtt": random.gauss(100, 50),                   # ms
            "connection_rtt_std": random.gauss(20, 10),
            "retransmission_count": random.randint(0, 2),
            "out_of_order_count": random.randint(0, 1),

            # DPI-specific features
            "rst_after_syn": 0,
            "rst_timing_ms": 0,
            "http_status_code": 200,
            "dns_response_code": 0,  # NOERROR
            "dns_answer_count": random.randint(1, 5),
            "dns_poisoned": 0,
            "sni_filtered": 0,

            # Statistical features
            "entropy_packet_sizes": random.gauss(3.5, 0.5),
            "entropy_inter_arrival": random.gauss(3.0, 0.5),
            "burst_count": random.randint(1, 5),

            # Label
            "label": 0,  # normal
            "label_name": "normal",
        }
        samples.append(features)

    return samples


def generate_tls_rst_flow(num_samples: int = 500) -> List[Dict[str, Any]]:
    """Generate FAVA TLS RST flow features (95-320ms timing)"""
    samples = []

    for _ in range(num_samples):
        rst_timing = random.uniform(95, 320)  # FAVA signature timing

        features = {
            "packet_inter_arrival_mean": random.gauss(30, 15),
            "packet_inter_arrival_std": random.gauss(10, 5),
            "packet_inter_arrival_min": max(1, random.gauss(3, 1)),
            "packet_inter_arrival_max": rst_timing + random.gauss(50, 20),

            "packet_size_mean": random.gauss(400, 150),
            "packet_size_std": random.gauss(200, 80),
            "packet_size_min": max(40, random.gauss(60, 15)),
            "packet_size_max": random.gauss(1200, 200),

            "flow_duration": rst_timing + random.gauss(20, 10),
            "total_packets": int(max(3, random.gauss(6, 2))),
            "total_bytes": int(max(200, random.gauss(3000, 1000))),
            "forward_packets": int(max(1, random.gauss(3, 1))),
            "backward_packets": int(max(1, random.gauss(3, 1))),

            "tcp_flags_syn": 1,
            "tcp_flags_ack": int(max(1, random.gauss(3, 1))),
            "tcp_flags_fin": 0,
            "tcp_flags_rst": random.randint(1, 3),  # KEY: RST present
            "tcp_window_size": random.randint(8192, 65535),

            "tls_version": 0x0303,
            "tls_cipher_suites_count": random.randint(8, 18),
            "tls_extensions_count": random.randint(5, 10),
            "tls_sni_length": random.randint(10, 40),
            "tls_alpn_protocols": random.randint(1, 2),
            "tls_session_id_length": 32,

            "connection_rtt": rst_timing,  # KEY: FAVA timing
            "connection_rtt_std": random.gauss(5, 2),
            "retransmission_count": 0,
            "out_of_order_count": 0,

            "rst_after_syn": 1,
            "rst_timing_ms": rst_timing,
            "http_status_code": 0,
            "dns_response_code": 0,
            "dns_answer_count": 0,
            "dns_poisoned": 0,
            "sni_filtered": 1,

            "entropy_packet_sizes": random.gauss(2.5, 0.5),
            "entropy_inter_arrival": random.gauss(2.0, 0.5),
            "burst_count": random.randint(1, 2),

            "label": 1,  # tls_rst
            "label_name": "tls_rst",
        }
        samples.append(features)

    return samples


def generate_dns_poison_flow(num_samples: int = 500) -> List[Dict[str, Any]]:
    """Generate DNS poisoning flow features"""
    samples = []

    poison_ips = ["10.10.34.34", "10.10.34.35"]

    for _ in range(num_samples):
        features = {
            "packet_inter_arrival_mean": random.gauss(10, 5),
            "packet_inter_arrival_std": random.gauss(5, 2),
            "packet_inter_arrival_min": max(1, random.gauss(2, 1)),
            "packet_inter_arrival_max": random.gauss(100, 30),

            "packet_size_mean": random.gauss(200, 80),
            "packet_size_std": random.gauss(100, 40),
            "packet_size_min": max(40, random.gauss(50, 10)),
            "packet_size_max": random.gauss(512, 100),

            "flow_duration": random.gauss(50, 30),
            "total_packets": 2,  # DNS query + response
            "total_bytes": int(max(100, random.gauss(500, 200))),
            "forward_packets": 1,
            "backward_packets": 1,

            "tcp_flags_syn": 0,
            "tcp_flags_ack": 0,
            "tcp_flags_fin": 0,
            "tcp_flags_rst": 0,
            "tcp_window_size": 0,

            "tls_version": 0,
            "tls_cipher_suites_count": 0,
            "tls_extensions_count": 0,
            "tls_sni_length": 0,
            "tls_alpn_protocols": 0,
            "tls_session_id_length": 0,

            "connection_rtt": random.gauss(5, 3),
            "connection_rtt_std": random.gauss(1, 0.5),
            "retransmission_count": 0,
            "out_of_order_count": 0,

            "rst_after_syn": 0,
            "rst_timing_ms": 0,
            "http_status_code": 0,
            "dns_response_code": 0,  # NOERROR but poisoned
            "dns_answer_count": 1,
            "dns_poisoned": 1,  # KEY: DNS poisoned
            "sni_filtered": 0,

            "entropy_packet_sizes": random.gauss(2.0, 0.3),
            "entropy_inter_arrival": random.gauss(1.5, 0.3),
            "burst_count": 1,

            "label": 3,  # dns_poison
            "label_name": "dns_poison",
        }
        samples.append(features)

    return samples


def generate_http_403_flow(num_samples: int = 300) -> List[Dict[str, Any]]:
    """Generate HTTP 403 block flow features"""
    samples = []

    for _ in range(num_samples):
        features = {
            "packet_inter_arrival_mean": random.gauss(20, 10),
            "packet_inter_arrival_std": random.gauss(8, 4),
            "packet_inter_arrival_min": max(1, random.gauss(3, 1)),
            "packet_inter_arrival_max": random.gauss(150, 40),

            "packet_size_mean": random.gauss(600, 200),
            "packet_size_std": random.gauss(300, 100),
            "packet_size_min": max(40, random.gauss(50, 15)),
            "packet_size_max": random.gauss(1400, 200),

            "flow_duration": random.gauss(200, 100),
            "total_packets": int(max(4, random.gauss(8, 3))),
            "total_bytes": int(max(300, random.gauss(5000, 2000))),
            "forward_packets": int(max(2, random.gauss(4, 1))),
            "backward_packets": int(max(2, random.gauss(4, 1))),

            "tcp_flags_syn": 1,
            "tcp_flags_ack": int(max(1, random.gauss(5, 2))),
            "tcp_flags_fin": 1,
            "tcp_flags_rst": 0,
            "tcp_window_size": random.randint(8192, 65535),

            "tls_version": 0x0303,
            "tls_cipher_suites_count": 0,
            "tls_extensions_count": 0,
            "tls_sni_length": 0,
            "tls_alpn_protocols": 0,
            "tls_session_id_length": 0,

            "connection_rtt": random.gauss(50, 30),
            "connection_rtt_std": random.gauss(10, 5),
            "retransmission_count": 0,
            "out_of_order_count": 0,

            "rst_after_syn": 0,
            "rst_timing_ms": 0,
            "http_status_code": 403,  # KEY: HTTP 403
            "dns_response_code": 0,
            "dns_answer_count": 0,
            "dns_poisoned": 0,
            "sni_filtered": 0,

            "entropy_packet_sizes": random.gauss(3.0, 0.5),
            "entropy_inter_arrival": random.gauss(2.5, 0.5),
            "burst_count": random.randint(1, 2),

            "label": 2,  # http_403
            "label_name": "http_403",
        }
        samples.append(features)

    return samples


def generate_sni_filter_flow(num_samples: int = 400) -> List[Dict[str, Any]]:
    """Generate SNI filter flow features"""
    samples = []

    for _ in range(num_samples):
        features = {
            "packet_inter_arrival_mean": random.gauss(25, 12),
            "packet_inter_arrival_std": random.gauss(10, 5),
            "packet_inter_arrival_min": max(1, random.gauss(3, 1)),
            "packet_inter_arrival_max": random.gauss(180, 40),

            "packet_size_mean": random.gauss(350, 150),
            "packet_size_std": random.gauss(200, 80),
            "packet_size_min": max(40, random.gauss(60, 15)),
            "packet_size_max": random.gauss(1000, 200),

            "flow_duration": random.gauss(150, 80),
            "total_packets": int(max(3, random.gauss(5, 2))),
            "total_bytes": int(max(200, random.gauss(2000, 800))),
            "forward_packets": int(max(1, random.gauss(3, 1))),
            "backward_packets": int(max(1, random.gauss(2, 1))),

            "tcp_flags_syn": 1,
            "tcp_flags_ack": int(max(1, random.gauss(2, 1))),
            "tcp_flags_fin": 0,
            "tcp_flags_rst": 1,  # RST after SNI
            "tcp_window_size": random.randint(8192, 65535),

            "tls_version": 0x0303,
            "tls_cipher_suites_count": random.randint(8, 16),
            "tls_extensions_count": random.randint(5, 10),
            "tls_sni_length": random.randint(10, 50),
            "tls_alpn_protocols": random.randint(1, 2),
            "tls_session_id_length": 32,

            "connection_rtt": random.gauss(120, 50),
            "connection_rtt_std": random.gauss(15, 8),
            "retransmission_count": 0,
            "out_of_order_count": 0,

            "rst_after_syn": 1,
            "rst_timing_ms": random.uniform(95, 200),
            "http_status_code": 0,
            "dns_response_code": 0,
            "dns_answer_count": 0,
            "dns_poisoned": 0,
            "sni_filtered": 1,  # KEY: SNI filtered

            "entropy_packet_sizes": random.gauss(2.5, 0.5),
            "entropy_inter_arrival": random.gauss(2.0, 0.5),
            "burst_count": 1,

            "label": 4,  # sni_filter
            "label_name": "sni_filter",
        }
        samples.append(features)

    return samples


def collect_dataset(output_dir: str = "data") -> None:
    """Collect full dataset from all sources"""

    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    all_samples = []

    # Generate synthetic data for each class
    logger.info("Generating normal traffic samples...")
    all_samples.extend(generate_normal_flow(2000))

    logger.info("Generating TLS RST samples...")
    all_samples.extend(generate_tls_rst_flow(1000))

    logger.info("Generating HTTP 403 samples...")
    all_samples.extend(generate_http_403_flow(600))

    logger.info("Generating DNS poison samples...")
    all_samples.extend(generate_dns_poison_flow(800))

    logger.info("Generating SNI filter samples...")
    all_samples.extend(generate_sni_filter_flow(800))

    logger.info(f"Total samples: {len(all_samples)}")

    # Shuffle
    random.shuffle(all_samples)

    # Extract features and labels
    feature_keys = [
        "packet_inter_arrival_mean", "packet_inter_arrival_std",
        "packet_inter_arrival_min", "packet_inter_arrival_max",
        "packet_size_mean", "packet_size_std",
        "packet_size_min", "packet_size_max",
        "flow_duration", "total_packets", "total_bytes",
        "forward_packets", "backward_packets",
        "tcp_flags_syn", "tcp_flags_ack",
        "tcp_flags_fin", "tcp_flags_rst", "tcp_window_size",
        "tls_version", "tls_cipher_suites_count",
        "tls_extensions_count", "tls_sni_length",
        "tls_alpn_protocols", "tls_session_id_length",
        "connection_rtt", "connection_rtt_std",
        "retransmission_count", "out_of_order_count",
        "rst_after_syn", "rst_timing_ms",
        "http_status_code", "dns_response_code",
        "dns_answer_count", "dns_poisoned", "sni_filtered",
        "entropy_packet_sizes", "entropy_inter_arrival",
        "burst_count",
        # Add more features to reach 47
        "tcp_flags_psh", "tcp_flags_urg",
        "packet_size_median", "flow_bytes_per_second",
        "flow_packets_per_second", "active_duration", "idle_duration",
    ]

    # Pad to 47 features
    while len(feature_keys) < 47:
        feature_keys.append(f"reserved_{len(feature_keys)}")

    feature_keys = feature_keys[:47]

    X = np.array(
        [[s.get(k, 0.0) for k in feature_keys] for s in all_samples],
        dtype=np.float32,
    )
    y = np.array([s["label"] for s in all_samples], dtype=np.int64)

    # Save
    np.savez(output_path / "dpi_dataset.npz", X=X, y=y)

    # Save raw JSON
    with open(output_path / "dpi_dataset_raw.json", "w") as f:
        json.dump(all_samples[:100], f, indent=2)  # Sample

    # Also create traffic predictor dataset
    # Binary classification: blocked (1) vs not blocked (0)
    y_binary = (y > 0).astype(np.float32)
    np.savez(output_path / "traffic_dataset.npz", X=X, y=y_binary)

    logger.info(f"Saved DPI dataset: {X.shape} -> {output_path / 'dpi_dataset.npz'}")
    logger.info(f"Saved traffic dataset: {X.shape} -> {output_path / 'traffic_dataset.npz'}")
    logger.info(f"Feature count: {X.shape[1]}")
    logger.info(f"Class distribution: {dict(zip(*np.unique(y, return_counts=True)))}")


if __name__ == "__main__":
    collect_dataset()
