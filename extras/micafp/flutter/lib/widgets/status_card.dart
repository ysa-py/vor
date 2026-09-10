import 'package:flutter/material.dart';
import 'package:flutter_animate/flutter_animate.dart';
import '../models/connection_stats.dart';

/// Animated status card showing connection details.
class StatusCard extends StatelessWidget {
  final ConnectionStats stats;
  final bool isConnected;
  final String activeCore;
  final String connectedServer;

  const StatusCard({
    super.key,
    required this.stats,
    required this.isConnected,
    required this.activeCore,
    required this.connectedServer,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  'Connection Status',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.bold,
                      ),
                ),
                _StatusBadge(isConnected: isConnected),
              ],
            ),
            const SizedBox(height: 20),

            // Connection details grid
            GridView.count(
              crossAxisCount: 2,
              shrinkWrap: true,
              physics: const NeverScrollableScrollPhysics(),
              childAspectRatio: 2.5,
              mainAxisSpacing: 12,
              crossAxisSpacing: 12,
              children: [
                _StatTile(
                  icon: Icons.hub,
                  label: 'Core',
                  value: activeCore.isNotEmpty ? activeCore : '--',
                  color: const Color(0xFF00E5FF),
                ),
                _StatTile(
                  icon: Icons.dns,
                  label: 'Server',
                  value: connectedServer.isNotEmpty ? connectedServer : '--',
                  color: const Color(0xFF7C4DFF),
                ),
                _StatTile(
                  icon: Icons.speed,
                  label: 'Latency',
                  value: isConnected ? '${stats.latency.toStringAsFixed(0)} ms' : '--',
                  color: const Color(0xFF00E676),
                ),
                _StatTile(
                  icon: Icons.schedule,
                  label: 'Uptime',
                  value: isConnected ? stats.uptimeFormatted : '--',
                  color: const Color(0xFFFFB74D),
                ),
                _StatTile(
                  icon: Icons.download,
                  label: 'Downloaded',
                  value: isConnected ? stats.totalDownFormatted : '--',
                  color: const Color(0xFF00E5FF),
                ),
                _StatTile(
                  icon: Icons.upload,
                  label: 'Uploaded',
                  value: isConnected ? stats.totalUpFormatted : '--',
                  color: const Color(0xFF7C4DFF),
                ),
              ],
            ),
          ],
        ),
      ),
    )
        .animate(target: isConnected ? 1 : 0)
        .shimmer(duration: 1500.ms, color: const Color(0xFF00E5FF).withOpacity(0.1));
  }
}

class _StatusBadge extends StatelessWidget {
  final bool isConnected;
  const _StatusBadge({required this.isConnected});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      decoration: BoxDecoration(
        color: isConnected
            ? const Color(0xFF00E676).withOpacity(0.2)
            : const Color(0xFF8B949E).withOpacity(0.2),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: isConnected ? const Color(0xFF00E676) : const Color(0xFF8B949E),
        ),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 8,
            height: 8,
            decoration: BoxDecoration(
              color: isConnected ? const Color(0xFF00E676) : const Color(0xFF8B949E),
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: 6),
          Text(
            isConnected ? 'Connected' : 'Disconnected',
            style: TextStyle(
              color: isConnected ? const Color(0xFF00E676) : const Color(0xFF8B949E),
              fontSize: 12,
              fontWeight: FontWeight.w600,
            ),
          ),
        ],
      ),
    );
  }
}

class _StatTile extends StatelessWidget {
  final IconData icon;
  final String label;
  final String value;
  final Color color;

  const _StatTile({
    required this.icon,
    required this.label,
    required this.value,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: color.withOpacity(0.05),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withOpacity(0.1)),
      ),
      child: Row(
        children: [
          Icon(icon, size: 20, color: color),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Text(
                  label,
                  style: TextStyle(
                    color: color.withOpacity(0.7),
                    fontSize: 10,
                  ),
                ),
                Text(
                  value,
                  style: const TextStyle(
                    fontWeight: FontWeight.w600,
                    fontSize: 13,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
