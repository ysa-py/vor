import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../main.dart';
import '../models/core_model.dart';
import '../models/vpn_state.dart';
import '../widgets/status_card.dart';

final coresListProvider = StateProvider<List<CoreAdapter>>((ref) => defaultCores);

class CoresScreen extends ConsumerStatefulWidget {
  const CoresScreen({super.key});

  @override
  ConsumerState<CoresScreen> createState() => _CoresScreenState();
}

class _CoresScreenState extends ConsumerState<CoresScreen> {
  @override
  Widget build(BuildContext context) {
    final vpnState = ref.watch(vpnStateProvider);
    final cores = ref.watch(coresListProvider);
    final locale = ref.watch(localeProvider);
    final isFa = locale.languageCode == 'fa';

    return Scaffold(
      appBar: AppBar(
        title: Text(isFa ? 'مدیریت هسته‌ها' : 'Core Management'),
        centerTitle: true,
      ),
      body: RefreshIndicator(
        onRefresh: _refreshCores,
        child: GridView.builder(
          padding: const EdgeInsets.all(16),
          gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
            crossAxisCount: 2,
            childAspectRatio: 0.72,
            crossAxisSpacing: 12,
            mainAxisSpacing: 12,
          ),
          itemCount: cores.length,
          itemBuilder: (context, index) {
            final core = cores[index];
            final isActive = core.id == vpnState.activeCore;
            return _CoreCard(
              core: core,
              isActive: isActive,
              isFa: isFa,
              onSwitch: () => _switchCore(core.id),
            );
          },
        ),
      ),
    );
  }

  Future<void> _refreshCores() async {
    try {
      final bridge = ref.read(daemonBridgeProvider);
      final available = await bridge.getAvailableCores();
      if (available.isNotEmpty) {
        ref.read(coresListProvider.notifier).state = available.map((e) {
          return CoreAdapter(
            id: e['id'] as String? ?? '',
            name: e['name'] as String? ?? '',
            nameFa: e['name_fa'] as String? ?? '',
            version: e['version'] as String? ?? '1.0.0',
            status: _parseCoreStatus(e['status'] as String?),
            health: HealthStatus(
              latency: e['latency'] as int? ?? 0,
              packetLoss: (e['packet_loss'] as num?)?.toDouble() ?? 0.0,
              blocked: e['blocked'] as bool? ?? false,
              dnsLeak: e['dns_leak'] as bool? ?? false,
              dpiExposure: (e['dpi_exposure'] as num?)?.toDouble() ?? 0.0,
              bandwidth: e['bandwidth'] as int? ?? 0,
            ),
            protocols: (e['protocols'] as List<dynamic>?)
                    ?.map((p) => p.toString())
                    .toList() ??
                [],
            capabilities: (e['capabilities'] as List<dynamic>?)
                    ?.map((c) => c.toString())
                    .toList() ??
                [],
          );
        }).toList();
      }
    } catch (_) {}
  }

  CoreStatus _parseCoreStatus(String? status) {
    switch (status) {
      case 'connected':
        return CoreStatus.connected;
      case 'disconnected':
        return CoreStatus.disconnected;
      case 'connecting':
        return CoreStatus.connecting;
      case 'error':
        return CoreStatus.error;
      default:
        return CoreStatus.standby;
    }
  }

  void _switchCore(String coreId) async {
    try {
      final bridge = ref.read(daemonBridgeProvider);
      await bridge.switchCore(coreId);
      ref.read(vpnStateProvider.notifier).connect();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Core switch failed: $e')),
        );
      }
    }
  }
}

class _CoreCard extends StatelessWidget {
  final CoreAdapter core;
  final bool isActive;
  final bool isFa;
  final VoidCallback onSwitch;

  const _CoreCard({
    required this.core,
    required this.isActive,
    required this.isFa,
    required this.onSwitch,
  });

  @override
  Widget build(BuildContext context) {
    final score = core.score;
    final scoreColor = score >= 70
        ? Colors.green
        : score >= 40
            ? Colors.orange
            : Colors.red;

    return StatusCard(
      borderColor: isActive ? Colors.green.withOpacity(0.5) : null,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header
          Row(
            children: [
              Expanded(
                child: Text(
                  isFa ? core.nameFa : core.name,
                  style: Theme.of(context).textTheme.titleSmall?.copyWith(
                        fontWeight: FontWeight.bold,
                      ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              if (isActive)
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(
                    color: Colors.green,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: const Text(
                    'ACTIVE',
                    style: TextStyle(fontSize: 9, color: Colors.white, fontWeight: FontWeight.bold),
                  ),
                ),
            ],
          ),
          const SizedBox(height: 4),
          Text(
            'v${core.version}',
            style: TextStyle(fontSize: 10, color: Colors.grey[400]),
          ),

          const Divider(height: 16),

          // Health stats
          _HealthRow(
            label: isFa ? 'تأخیر' : 'Latency',
            value: '${core.health.latency}ms',
            icon: Icons.speed,
            color: core.health.latency < 200
                ? Colors.green
                : core.health.latency < 500
                    ? Colors.orange
                    : Colors.red,
          ),
          const SizedBox(height: 4),
          _HealthRow(
            label: isFa ? 'افت بسته' : 'Loss',
            value: '${(core.health.packetLoss * 100).toStringAsFixed(1)}%',
            icon: Icons.packet_loss,
            color: core.health.packetLoss < 0.05
                ? Colors.green
                : core.health.packetLoss < 0.15
                    ? Colors.orange
                    : Colors.red,
          ),
          const SizedBox(height: 4),
          _HealthRow(
            label: isFa ? 'دپی' : 'DPI',
            value: '${(core.health.dpiExposure * 100).toStringAsFixed(0)}%',
            icon: Icons.visibility_off,
            color: core.health.dpiExposure < 0.3
                ? Colors.green
                : core.health.dpiExposure < 0.6
                    ? Colors.orange
                    : Colors.red,
          ),

          // Blocked warning
          if (core.health.blocked) ...[
            const SizedBox(height: 6),
            Container(
              width: double.infinity,
              padding: const EdgeInsets.all(6),
              decoration: BoxDecoration(
                color: Colors.red.withOpacity(0.1),
                borderRadius: BorderRadius.circular(6),
              ),
              child: Row(
                children: [
                  const Icon(Icons.block, color: Colors.red, size: 14),
                  const SizedBox(width: 4),
                  Text(
                    isFa ? 'مسدود شده' : 'Blocked',
                    style: const TextStyle(color: Colors.red, fontSize: 11),
                  ),
                ],
              ),
            ),
          ],

          const Spacer(),

          // Score + Switch
          Row(
            children: [
              Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    isFa ? 'امتیاز' : 'Score',
                    style: TextStyle(fontSize: 10, color: Colors.grey[400]),
                  ),
                  Text(
                    score.toStringAsFixed(0),
                    style: TextStyle(
                      fontSize: 20,
                      fontWeight: FontWeight.bold,
                      color: scoreColor,
                    ),
                  ),
                ],
              ),
              const Spacer(),
              SizedBox(
                height: 36,
                child: ElevatedButton(
                  onPressed: core.health.blocked ? null : onSwitch,
                  style: ElevatedButton.styleFrom(
                    backgroundColor: isActive ? Colors.green : Colors.indigo,
                    foregroundColor: Colors.white,
                    padding: const EdgeInsets.symmetric(horizontal: 12),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                  child: Text(
                    isActive
                        ? (isFa ? 'فعال' : 'Active')
                        : (isFa ? 'انتخاب' : 'Switch'),
                    style: const TextStyle(fontSize: 12),
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _HealthRow extends StatelessWidget {
  final String label;
  final String value;
  final IconData icon;
  final Color color;

  const _HealthRow({
    required this.label,
    required this.value,
    required this.icon,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Icon(icon, size: 14, color: color),
        const SizedBox(width: 4),
        Text(label, style: TextStyle(fontSize: 11, color: Colors.grey[400])),
        const Spacer(),
        Text(value, style: TextStyle(fontSize: 11, color: color, fontWeight: FontWeight.w500)),
      ],
    );
  }
}
