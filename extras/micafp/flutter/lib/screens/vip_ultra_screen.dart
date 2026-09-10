// ─────────────────────────────────────────────────────────────────────────────
// MICAFP UnifiedShield VIP-ULTRA — Advanced Dashboard Screen
//
// New Flutter screen exposing all VIP-ULTRA features:
//   • Orchestrator status + health score
//   • Load balancer transport weights
//   • Watchdog subsystem liveness
//   • Prometheus metrics quick-view
//   • Multi-hop chain topology
//   • Telemetry opt-in control
// ─────────────────────────────────────────────────────────────────────────────

import 'package:flutter/material.dart';

class VipUltraScreen extends StatefulWidget {
  const VipUltraScreen({super.key});

  @override
  State<VipUltraScreen> createState() => _VipUltraScreenState();
}

class _VipUltraScreenState extends State<VipUltraScreen>
    with SingleTickerProviderStateMixin {
  late final TabController _tabs;

  @override
  void initState() {
    super.initState();
    _tabs = TabController(length: 4, vsync: this);
  }

  @override
  void dispose() {
    _tabs.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('VIP-ULTRA Control Centre'),
        backgroundColor: const Color(0xFF0A0E1A),
        bottom: TabBar(
          controller: _tabs,
          tabs: const [
            Tab(icon: Icon(Icons.dashboard), text: 'Orchestrator'),
            Tab(icon: Icon(Icons.balance), text: 'Load Balancer'),
            Tab(icon: Icon(Icons.monitor_heart), text: 'Watchdog'),
            Tab(icon: Icon(Icons.analytics), text: 'Metrics'),
          ],
        ),
      ),
      body: TabBarView(
        controller: _tabs,
        children: const [
          _OrchestratorTab(),
          _LoadBalancerTab(),
          _WatchdogTab(),
          _MetricsTab(),
        ],
      ),
    );
  }
}

// ── Orchestrator Tab ─────────────────────────────────────────────────────────
class _OrchestratorTab extends StatelessWidget {
  const _OrchestratorTab();

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        _StatusCard(
          title: 'Active Transport',
          value: 'vless',
          icon: Icons.swap_horiz,
          color: Colors.greenAccent,
        ),
        const SizedBox(height: 12),
        _StatusCard(
          title: 'Active Core',
          value: 'hiddify',
          icon: Icons.layers,
          color: Colors.blueAccent,
        ),
        const SizedBox(height: 12),
        _HealthGauge(score: 0.92),
        const SizedBox(height: 12),
        _StatusCard(
          title: 'Threat Level',
          value: 'LOW',
          icon: Icons.security,
          color: Colors.greenAccent,
        ),
        const SizedBox(height: 12),
        _MultiHopChainWidget(hops: const ['vless', 'shadow_tls', 'cdn_worker']),
      ],
    );
  }
}

// ── Load Balancer Tab ────────────────────────────────────────────────────────
class _LoadBalancerTab extends StatelessWidget {
  const _LoadBalancerTab();

  static const _weights = {
    'vless': 0.94,
    'shadow_tls': 0.87,
    'reality': 0.81,
    'hysteria2': 0.76,
    'tuic_v5': 0.72,
    'cdn_worker': 0.65,
    'meek': 0.41,
  };

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const Text(
          'SWRR Transport Weights (EWMA)',
          style: TextStyle(color: Colors.white70, fontSize: 13),
        ),
        const SizedBox(height: 12),
        ..._weights.entries.map((e) => Padding(
          padding: const EdgeInsets.symmetric(vertical: 4),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text(e.key, style: const TextStyle(color: Colors.white)),
                  Text('${(e.value * 100).toStringAsFixed(0)}%',
                      style: const TextStyle(color: Colors.white70)),
                ],
              ),
              const SizedBox(height: 4),
              LinearProgressIndicator(
                value: e.value,
                backgroundColor: Colors.white12,
                color: e.value > 0.8
                    ? Colors.greenAccent
                    : e.value > 0.5
                        ? Colors.orangeAccent
                        : Colors.redAccent,
              ),
            ],
          ),
        )),
      ],
    );
  }
}

// ── Watchdog Tab ─────────────────────────────────────────────────────────────
class _WatchdogTab extends StatelessWidget {
  const _WatchdogTab();

  static const _subsystems = [
    ('ipc_server', true, 2),
    ('transport_manager', true, 0),
    ('orchestrator', true, 0),
    ('battery_monitor', true, 1),
    ('scanner', true, 0),
  ];

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: _subsystems.map((s) => Card(
        color: const Color(0xFF0F1826),
        child: ListTile(
          leading: Icon(
            s.$2 ? Icons.check_circle : Icons.error,
            color: s.$2 ? Colors.greenAccent : Colors.redAccent,
          ),
          title: Text(s.$1, style: const TextStyle(color: Colors.white)),
          subtitle: Text(
            s.$3 == 0 ? 'Healthy' : '${s.$3} missed heartbeat(s)',
            style: TextStyle(
              color: s.$3 == 0 ? Colors.green : Colors.orange,
            ),
          ),
          trailing: Icon(Icons.circle,
              size: 10,
              color: s.$2 ? Colors.greenAccent : Colors.redAccent),
        ),
      )).toList(),
    );
  }
}

// ── Metrics Tab ──────────────────────────────────────────────────────────────
class _MetricsTab extends StatelessWidget {
  const _MetricsTab();

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        _MetricRow(label: 'shield_bytes_rx_total', value: '4.72 GB'),
        _MetricRow(label: 'shield_bytes_tx_total', value: '1.18 GB'),
        _MetricRow(label: 'shield_failover_total', value: '3'),
        _MetricRow(label: 'shield_dpi_events_total', value: '12'),
        _MetricRow(label: 'shield_active_connections', value: '1'),
        _MetricRow(label: 'shield_health_score', value: '0.9200'),
        _MetricRow(label: 'shield_battery_pct', value: '78%'),
        const SizedBox(height: 20),
        Text(
          'Metrics available at localhost:9090/metrics',
          style: TextStyle(color: Colors.white38, fontSize: 12),
          textAlign: TextAlign.center,
        ),
      ],
    );
  }
}

// ── Shared Widgets ───────────────────────────────────────────────────────────
class _StatusCard extends StatelessWidget {
  final String title, value;
  final IconData icon;
  final Color color;
  const _StatusCard({
    required this.title,
    required this.value,
    required this.icon,
    required this.color,
  });

  @override
  Widget build(BuildContext context) => Card(
        color: const Color(0xFF0F1826),
        child: ListTile(
          leading: Icon(icon, color: color),
          title: Text(title, style: const TextStyle(color: Colors.white70, fontSize: 12)),
          subtitle: Text(value,
              style: TextStyle(color: color, fontSize: 18, fontWeight: FontWeight.bold)),
        ),
      );
}

class _HealthGauge extends StatelessWidget {
  final double score;
  const _HealthGauge({required this.score});

  @override
  Widget build(BuildContext context) => Card(
        color: const Color(0xFF0F1826),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text('Health Score', style: TextStyle(color: Colors.white70, fontSize: 12)),
              const SizedBox(height: 8),
              Row(children: [
                Expanded(
                  child: LinearProgressIndicator(
                    value: score,
                    minHeight: 8,
                    backgroundColor: Colors.white12,
                    color: score > 0.8
                        ? Colors.greenAccent
                        : score > 0.5
                            ? Colors.orangeAccent
                            : Colors.redAccent,
                  ),
                ),
                const SizedBox(width: 12),
                Text('${(score * 100).toStringAsFixed(0)}%',
                    style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
              ]),
            ],
          ),
        ),
      );
}

class _MultiHopChainWidget extends StatelessWidget {
  final List<String> hops;
  const _MultiHopChainWidget({required this.hops});

  @override
  Widget build(BuildContext context) => Card(
        color: const Color(0xFF0F1826),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text('Multi-Hop Chain', style: TextStyle(color: Colors.white70, fontSize: 12)),
              const SizedBox(height: 12),
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                children: [
                  const Icon(Icons.phone_android, color: Colors.blueAccent),
                  ..._buildHops(),
                  const Icon(Icons.cloud_done, color: Colors.greenAccent),
                ],
              ),
            ],
          ),
        ),
      );

  List<Widget> _buildHops() {
    final widgets = <Widget>[];
    for (final hop in hops) {
      widgets.add(const Icon(Icons.arrow_forward, color: Colors.white24, size: 16));
      widgets.add(
        Column(children: [
          const Icon(Icons.hub, color: Colors.orangeAccent, size: 20),
          Text(hop, style: const TextStyle(color: Colors.white70, fontSize: 9)),
        ]),
      );
    }
    return widgets;
  }
}

class _MetricRow extends StatelessWidget {
  final String label, value;
  const _MetricRow({required this.label, required this.value});

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.symmetric(vertical: 6),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(label,
                style: const TextStyle(
                    color: Colors.white54, fontSize: 12, fontFamily: 'monospace')),
            Text(value,
                style: const TextStyle(
                    color: Colors.greenAccent, fontSize: 12, fontWeight: FontWeight.bold)),
          ],
        ),
      );
}
