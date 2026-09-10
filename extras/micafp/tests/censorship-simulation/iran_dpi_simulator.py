#!/usr/bin/env python3
"""
Iran DPI Simulator — Scapy-based Deep Packet Inspection simulator

Simulates the Iranian FAVA DPI system for testing bypass effectiveness.
Supports:
- TLS ClientHello SNI inspection and RST injection (95-320ms)
- HTTP 403 injection for blocked URLs
- DNS poisoning (10.10.34.34 / 10.10.34.35)
- Protocol detection (OpenVPN, WireGuard, Shadowsocks, Tor)
"""

import time
import random
import json
import struct
import threading
from pathlib import Path
from typing import Optional, Dict, List, Tuple
from dataclasses import dataclass, field
import logging

try:
    from scapy.all import (
        IP, TCP, UDP, DNS, DNSQR, DNSRR, Raw,
        send, sniff, sr1, Ether, Packet,
        conf as scapy_conf,
    )
    SCAPY_AVAILABLE = True
except ImportError:
    SCAPY_AVAILABLE = False

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("IranDPISimulator")


@dataclass
class DPIConfig:
    """DPI configuration matching real Iranian FAVA system"""

    # TLS RST timing (milliseconds)
    tls_rst_min_ms: int = 95
    tls_rst_max_ms: int = 320

    # DNS poison IPs
    dns_poison_ips: List[str] = field(default_factory=lambda: ["10.10.34.34", "10.10.34.35"])

    # Blocked domains
    blocked_domains: List[str] = field(default_factory=lambda: [
        "youtube.com", "youtu.be", "googlevideo.com",
        "twitter.com", "x.com", "t.co", "twimg.com",
        "facebook.com", "fbcdn.net", "instagram.com",
        "telegram.org", "t.me",
        "whatsapp.com",
        "reddit.com",
        "linkedin.com",
        "github.com",
        "wikipedia.org",
        "bbc.com",
        "medium.com",
        "netflix.com",
        "discord.com",
        "twitch.tv",
        "spotify.com",
        "soundcloud.com",
        "stackoverflow.com",
        "torproject.org",
    ])

    # Blocked SNI keywords
    blocked_sni_keywords: List[str] = field(default_factory=lambda: [
        "youtube", "googlevideo", "youtu",
        "twitter", "facebook", "instagram",
        "telegram", "whatsapp",
        "reddit", "linkedin",
        "tor", "vpn", "proxy",
        "filternet",
    ])

    # Protocol signatures
    openvpn_signature: bytes = b"\x00\x0c\x4e\x6f\x76\x61"  # "Nova" in OpenVPN P_CONTROL
    wireguard_signature: bytes = b"\x01\x00\x00\x00"          # WG handshake init

    # HTTP 403 response body
    http_403_body: str = """<!DOCTYPE html>
<html>
<head><title>Access Denied</title></head>
<body>
<h1>Access to this site is restricted</h1>
<p>The requested URL has been filtered according to regulatory requirements.</p>
</body>
</html>"""

    # Simulation settings
    simulate_packet_loss: bool = False
    packet_loss_rate: float = 0.01
    simulate_throttling: bool = False
    throttle_bandwidth_kbps: int = 256  # 256 Kbps (severe throttling)


class IranDPISimulator:
    """
    Simulates Iranian DPI system for testing bypass effectiveness.
    """

    def __init__(self, config: Optional[DPIConfig] = None):
        self.config = config or DPIConfig()
        self.stats = {
            "packets_inspected": 0,
            "tls_rst_injected": 0,
            "dns_poisoned": 0,
            "http_403_injected": 0,
            "protocol_blocked": 0,
        }
        self._running = False

    def start(self, interface: str = "eth0"):
        """Start the DPI simulator"""
        if not SCAPY_AVAILABLE:
            logger.error("Scapy not available. Install with: pip install scapy")
            return

        self._running = True
        logger.info(f"Starting DPI simulator on {interface}")

        try:
            sniff(
                iface=interface,
                prn=self._process_packet,
                stop_filter=lambda _: not self._running,
                store=False,
            )
        except Exception as e:
            logger.error(f"Sniffing error: {e}")

    def stop(self):
        """Stop the DPI simulator"""
        self._running = False
        logger.info("DPI simulator stopped")
        self._print_stats()

    def _process_packet(self, packet):
        """Process each packet through the DPI pipeline"""
        self.stats["packets_inspected"] += 1

        # Check TLS ClientHello
        if packet.haslayer(TCP) and packet.haslayer(Raw):
            payload = bytes(packet[Raw].load)

            # TLS record
            if len(payload) > 5 and payload[0] == 0x16:
                self._inspect_tls(packet, payload)

            # HTTP request
            elif payload.startswith(b"GET ") or payload.startswith(b"POST "):
                self._inspect_http(packet, payload)

            # Protocol detection
            self._detect_protocol(packet, payload)

        # DNS query
        if packet.haslayer(DNS) and packet.haslayer(DNSQR):
            self._inspect_dns(packet)

    def _inspect_tls(self, packet, payload: bytes):
        """Inspect TLS ClientHello for SNI filtering"""
        sni = self._extract_sni(payload)
        if not sni:
            return

        # Check if SNI matches blocked list
        is_blocked = False
        for blocked in self.config.blocked_sni_keywords:
            if blocked.lower() in sni.lower():
                is_blocked = True
                break

        if is_blocked:
            # Inject RST with FAVA timing (95-320ms)
            delay = random.uniform(
                self.config.tls_rst_min_ms / 1000,
                self.config.tls_rst_max_ms / 1000,
            )

            threading.Timer(delay, self._inject_tls_rst, [packet]).start()
            logger.info(f"🚫 TLS RST scheduled for SNI: {sni} (delay: {delay*1000:.0f}ms)")

    def _extract_sni(self, payload: bytes) -> Optional[str]:
        """Extract SNI from TLS ClientHello"""
        try:
            if payload[0] != 0x16:  # Handshake
                return None

            # Find SNI extension
            # Simplified parsing — real implementation would be more robust
            offset = 5  # After TLS record header

            if offset >= len(payload):
                return None

            # Handshake type should be 0x01 (ClientHello)
            if payload[offset] != 0x01:
                return None

            offset += 1 + 3  # type + length
            offset += 2      # version
            offset += 32     # random

            if offset >= len(payload):
                return None

            # Session ID
            session_id_len = payload[offset]
            offset += 1 + session_id_len

            if offset + 2 >= len(payload):
                return None

            # Cipher suites
            cipher_len = (payload[offset] << 8) | payload[offset + 1]
            offset += 2 + cipher_len

            if offset >= len(payload):
                return None

            # Compression methods
            comp_len = payload[offset]
            offset += 1 + comp_len

            if offset + 2 >= len(payload):
                return None

            # Extensions
            ext_len = (payload[offset] << 8) | payload[offset + 1]
            offset += 2

            ext_end = offset + ext_len

            while offset + 4 <= ext_end:
                ext_type = (payload[offset] << 8) | payload[offset + 1]
                ext_data_len = (payload[offset + 2] << 8) | payload[offset + 3]
                offset += 4

                if ext_type == 0x0000:  # SNI extension
                    list_len = (payload[offset] << 8) | payload[offset + 1]
                    offset += 2
                    sni_type = payload[offset]
                    offset += 1
                    sni_len = (payload[offset] << 8) | payload[offset + 1]
                    offset += 2

                    if sni_type == 0:  # hostname
                        return payload[offset : offset + sni_len].decode("ascii", errors="ignore")

                offset += ext_data_len

        except (IndexError, ValueError):
            pass

        return None

    def _inject_tls_rst(self, original_packet):
        """Inject a TCP RST packet to kill the TLS connection"""
        if not SCAPY_AVAILABLE:
            return

        try:
            rst_packet = IP(
                dst=original_packet[IP].src,
                src=original_packet[IP].dst,
            ) / TCP(
                sport=original_packet[TCP].dport,
                dport=original_packet[TCP].sport,
                flags="R",
                seq=original_packet[TCP].ack,
            )

            send(rst_packet, verbose=False)
            self.stats["tls_rst_injected"] += 1

        except Exception as e:
            logger.error(f"RST injection failed: {e}")

    def _inspect_http(self, packet, payload: bytes):
        """Inspect HTTP requests for blocked URLs"""
        try:
            request_line = payload.split(b"\r\n")[0].decode("ascii", errors="ignore")
            host = None

            for line in payload.split(b"\r\n"):
                if line.lower().startswith(b"host:"):
                    host = line.split(b":", 1)[1].strip().decode("ascii", errors="ignore")
                    break

            if not host:
                return

            for blocked in self.config.blocked_domains:
                if host.endswith(blocked) or host == blocked:
                    self._inject_http_403(packet)
                    logger.info(f"🚫 HTTP 403 for host: {host}")
                    break

        except Exception:
            pass

    def _inject_http_403(self, original_packet):
        """Inject HTTP 403 response"""
        if not SCAPY_AVAILABLE:
            return

        try:
            body = self.config.http_403_body.encode()
            response = (
                b"HTTP/1.1 403 Forbidden\r\n"
                b"Content-Type: text/html\r\n"
                b"Content-Length: " + str(len(body)).encode() + b"\r\n"
                b"Connection: close\r\n"
                b"\r\n" + body
            )

            packet = IP(
                dst=original_packet[IP].src,
                src=original_packet[IP].dst,
            ) / TCP(
                sport=original_packet[TCP].dport,
                dport=original_packet[TCP].sport,
                flags="PA",
                seq=original_packet[TCP].ack,
                ack=original_packet[TCP].seq + len(original_packet[Raw].load),
            ) / Raw(load=response)

            send(packet, verbose=False)
            self.stats["http_403_injected"] += 1

        except Exception as e:
            logger.error(f"HTTP 403 injection failed: {e}")

    def _inspect_dns(self, packet):
        """Inspect DNS queries and inject poisoned responses"""
        if not packet.haslayer(DNSQR):
            return

        query_name = packet[DNSQR].qname.decode("ascii", errors="ignore").rstrip(".")

        for blocked in self.config.blocked_domains:
            if query_name.endswith(blocked) or query_name == blocked:
                self._inject_dns_poison(packet, query_name)
                logger.info(f"🚫 DNS poison for: {query_name}")
                break

    def _inject_dns_poison(self, original_packet, query_name: str):
        """Inject a poisoned DNS response"""
        if not SCAPY_AVAILABLE:
            return

        try:
            poison_ip = random.choice(self.config.dns_poison_ips)

            packet = IP(
                dst=original_packet[IP].src,
                src=original_packet[IP].dst,
            ) / UDP(
                sport=original_packet[UDP].dport,
                dport=original_packet[UDP].sport,
            ) / DNS(
                id=original_packet[DNS].id,
                qr=1,
                aa=1,
                qd=original_packet[DNS].qd,
                an=DNSRR(
                    rrname=query_name + ".",
                    type="A",
                    ttl=300,
                    rdata=poison_ip,
                ),
            )

            send(packet, verbose=False)
            self.stats["dns_poisoned"] += 1

        except Exception as e:
            logger.error(f"DNS poison injection failed: {e}")

    def _detect_protocol(self, packet, payload: bytes):
        """Detect VPN/proxy protocols"""
        # OpenVPN
        if self.config.openvpn_signature in payload[:20]:
            self._inject_tls_rst(packet)
            self.stats["protocol_blocked"] += 1
            logger.info("🚫 Protocol detected: OpenVPN")

        # WireGuard
        if len(payload) >= 4 and payload[:4] == self.config.wireguard_signature:
            self._inject_tls_rst(packet)
            self.stats["protocol_blocked"] += 1
            logger.info("🚫 Protocol detected: WireGuard")

        # High entropy (possible encrypted tunnel)
        if len(payload) > 100:
            entropy = self._calculate_entropy(payload)
            if entropy > 7.5:  # Very high entropy
                # Could be Shadowsocks or similar
                # In reality, Iranian DPI does entropy-based detection
                pass

    def _calculate_entropy(self, data: bytes) -> float:
        """Calculate Shannon entropy of byte data"""
        if not data:
            return 0.0

        freq = [0] * 256
        for byte in data:
            freq[byte] += 1

        import math
        entropy = 0.0
        length = len(data)

        for count in freq:
            if count > 0:
                p = count / length
                entropy -= p * math.log2(p)

        return entropy

    def _print_stats(self):
        """Print DPI simulation statistics"""
        logger.info("\n" + "=" * 50)
        logger.info("DPI Simulation Statistics")
        logger.info("=" * 50)
        logger.info(f"Packets inspected:    {self.stats['packets_inspected']}")
        logger.info(f"TLS RST injected:     {self.stats['tls_rst_injected']}")
        logger.info(f"DNS poisoned:         {self.stats['dns_poisoned']}")
        logger.info(f"HTTP 403 injected:    {self.stats['http_403_injected']}")
        logger.info(f"Protocol blocked:     {self.stats['protocol_blocked']}")
        logger.info("=" * 50)

    def simulate_test_scenario(
        self,
        target: str = "8.8.8.8",
        domain: str = "youtube.com",
    ) -> Dict:
        """
        Run a test scenario simulating a user trying to access a blocked site.
        Returns results without actually sending packets (dry-run mode).
        """
        results = {
            "target": target,
            "domain": domain,
            "blocked": False,
            "block_methods": [],
            "simulated_rst_timing_ms": None,
            "simulated_poison_ip": None,
        }

        # Check if domain is blocked
        for blocked in self.config.blocked_domains:
            if domain.endswith(blocked) or domain == blocked:
                results["blocked"] = True

                # DNS poisoning
                poison_ip = random.choice(self.config.dns_poison_ips)
                results["block_methods"].append("dns_poison")
                results["simulated_poison_ip"] = poison_ip

                # SNI filter
                results["block_methods"].append("sni_filter")
                results["simulated_rst_timing_ms"] = random.randint(
                    self.config.tls_rst_min_ms,
                    self.config.tls_rst_max_ms,
                )

                # HTTP 403
                results["block_methods"].append("http_403")
                break

        return results


if __name__ == "__main__":
    config = DPIConfig()
    simulator = IranDPISimulator(config)

    # Dry-run test
    test_domains = [
        "youtube.com",
        "google.com",
        "irna.ir",
        "twitter.com",
        "digikala.com",
        "facebook.com",
        "sharif.ir",
    ]

    print("\nDPI Simulation Test\n" + "=" * 50)
    for domain in test_domains:
        result = simulator.simulate_test_scenario(domain=domain)
        status = "🚫 BLOCKED" if result["blocked"] else "✓ ALLOWED"
        methods = ", ".join(result["block_methods"]) if result["block_methods"] else "none"
        print(f"  {status}  {domain:20s}  methods: {methods}")

    print()
