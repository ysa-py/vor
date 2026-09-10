// MICAFP UnifiedShield VIP-ULTRA — Advanced Security Screen
// Displays post-quantum KEX status, anti-forensics controls,
// mesh network status, and resilience chain.

import 'package:flutter/material.dart';

class AdvancedSecurityScreen extends StatefulWidget {
  const AdvancedSecurityScreen({super.key});

  @override
  State<AdvancedSecurityScreen> createState() => _AdvancedSecurityScreenState();
}

class _AdvancedSecurityScreenState extends State<AdvancedSecurityScreen> {
  bool _antiForensicsEnabled = false;
  bool _postQuantumEnabled = true;
  bool _meshNetworkEnabled = false;
  bool _ephemeralIdentityEnabled = true;
  String _currentFallback = 'PrimaryTransport';
  int _pqKexCount = 0;
  int _meshPeers = 0;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('🛡️ Advanced Security'),
        backgroundColor: Colors.black87,
        foregroundColor: Colors.white,
      ),
      backgroundColor: Colors.black,
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          _sectionHeader('🔐 Post-Quantum Cryptography'),
          _settingTile(
            title: 'ML-KEM-768 Hybrid KEX',
            subtitle: 'X25519 + Kyber post-quantum key exchange',
            value: _postQuantumEnabled,
            onChanged: (v) => setState(() => _postQuantumEnabled = v),
            trailing: Text('$_pqKexCount KEX', style: const TextStyle(color: Colors.greenAccent, fontSize: 12)),
          ),
          _infoTile('Algorithm', 'X25519 + ML-KEM-768 (NIST PQC Standard)'),
          _infoTile('Session Key', '256-bit HKDF-SHA256 hybrid derived'),
          _infoTile('Protection', 'Harvest-now-decrypt-later attacks'),
          const SizedBox(height: 16),

          _sectionHeader('🕵️ Anti-Forensics'),
          _settingTile(
            title: 'Emergency Wipe',
            subtitle: 'Triggers secure erase on panic gesture',
            value: _antiForensicsEnabled,
            onChanged: (v) => setState(() => _antiForensicsEnabled = v),
          ),
          _settingTile(
            title: 'Ephemeral Identity',
            subtitle: 'Rotating peer IDs — never reuse P2P identity',
            value: _ephemeralIdentityEnabled,
            onChanged: (v) => setState(() => _ephemeralIdentityEnabled = v),
          ),
          _infoTile('Wipe Scope', 'Config, logs, keys, daemon socket'),
          _infoTile('Identity Rotation', 'Every 24 hours (HKDF derived)'),
          const SizedBox(height: 16),

          _sectionHeader('🕸️ Mesh Network'),
          _settingTile(
            title: 'Mesh Network (NAIN Fallback)',
            subtitle: 'BLE + WiFi Aware + Yggdrasil overlay',
            value: _meshNetworkEnabled,
            onChanged: (v) => setState(() => _meshNetworkEnabled = v),
            trailing: Text('$_meshPeers peers', style: const TextStyle(color: Colors.blueAccent, fontSize: 12)),
          ),
          _infoTile('BLE Mesh', '~30m range, 2mA battery'),
          _infoTile('WiFi Aware', '~150m range, 30mA battery'),
          _infoTile('Yggdrasil', 'Global overlay, routed via internet'),
          _infoTile('Topology', 'Dijkstra shortest-path routing'),
          const SizedBox(height: 16),

          _sectionHeader('🔄 Resilience Fallback Chain'),
          ...[
            ('1', 'PrimaryTransport', _currentFallback == 'PrimaryTransport'),
            ('2', 'ChineseCdnWorker', false),
            ('3', 'P2pLibp2pRelay', false),
            ('4', 'DohTunnel', false),
            ('5', 'IcmpTunnel', false),
            ('6', 'MeshNetwork', false),
            ('7', 'TorBridgeSnowflake', false),
            ('8', 'TorBridgeMeek', false),
          ].map((e) => _fallbackTile(e.$1, e.$2, e.$3)),
          const SizedBox(height: 16),

          _sectionHeader('📊 Monitoring'),
          _infoTile('Prometheus Metrics', 'http://127.0.0.1:9090/metrics'),
          _infoTile('Health Check Interval', '30 seconds'),
          _infoTile('Alert Thresholds', 'Packet loss >5%, RTT >300ms'),
          _infoTile('Circuit Breakers', '12 transports monitored'),
        ],
      ),
    );
  }

  Widget _sectionHeader(String title) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Text(
        title,
        style: const TextStyle(
          color: Colors.white, fontSize: 15, fontWeight: FontWeight.bold,
        ),
      ),
    );
  }

  Widget _settingTile({
    required String title,
    required String subtitle,
    required bool value,
    required ValueChanged<bool> onChanged,
    Widget? trailing,
  }) {
    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      decoration: BoxDecoration(
        color: Colors.grey[900],
        borderRadius: BorderRadius.circular(8),
      ),
      child: ListTile(
        title: Text(title, style: const TextStyle(color: Colors.white, fontSize: 14)),
        subtitle: Text(subtitle, style: const TextStyle(color: Colors.grey, fontSize: 12)),
        trailing: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (trailing != null) ...[trailing, const SizedBox(width: 8)],
            Switch(value: value, onChanged: onChanged, activeColor: Colors.greenAccent),
          ],
        ),
      ),
    );
  }

  Widget _infoTile(String label, String value) {
    return Container(
      margin: const EdgeInsets.only(bottom: 4),
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: Colors.grey[900],
        borderRadius: BorderRadius.circular(6),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label, style: const TextStyle(color: Colors.grey, fontSize: 12)),
          Text(value, style: const TextStyle(color: Colors.white70, fontSize: 12, fontFamily: 'monospace')),
        ],
      ),
    );
  }

  Widget _fallbackTile(String num, String name, bool active) {
    return Container(
      margin: const EdgeInsets.only(bottom: 4),
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: active ? Colors.green[900] : Colors.grey[900],
        borderRadius: BorderRadius.circular(6),
        border: active ? Border.all(color: Colors.greenAccent, width: 1) : null,
      ),
      child: Row(
        children: [
          Text('#$num ', style: TextStyle(color: Colors.grey[600], fontSize: 12)),
          Text(name, style: TextStyle(
            color: active ? Colors.greenAccent : Colors.white70,
            fontSize: 12, fontFamily: 'monospace',
            fontWeight: active ? FontWeight.bold : FontWeight.normal,
          )),
          if (active) ...[
            const Spacer(),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
              decoration: BoxDecoration(
                color: Colors.greenAccent, borderRadius: BorderRadius.circular(4),
              ),
              child: const Text('ACTIVE', style: TextStyle(color: Colors.black, fontSize: 10, fontWeight: FontWeight.bold)),
            ),
          ],
        ],
      ),
    );
  }
}
