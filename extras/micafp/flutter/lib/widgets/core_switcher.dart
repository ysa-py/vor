import 'package:flutter/material.dart';
import 'package:flutter_animate/flutter_animate.dart';
import '../models/core_state.dart';

/// Core switcher widget for quick core selection.
///
/// Displays a horizontal scrollable list of cores with
/// visual indicators for status and performance.
class CoreSwitcher extends StatelessWidget {
  final List<CoreState> cores;
  final String activeCoreId;
  final void Function(String coreId) onCoreSelected;

  const CoreSwitcher({
    super.key,
    required this.cores,
    required this.activeCoreId,
    required this.onCoreSelected,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Active Core',
          style: Theme.of(context).textTheme.titleMedium?.copyWith(
                fontWeight: FontWeight.bold,
              ),
        ),
        const SizedBox(height: 12),
        SizedBox(
          height: 90,
          child: ListView.separated(
            scrollDirection: Axis.horizontal,
            itemCount: cores.length,
            separatorBuilder: (_, __) => const SizedBox(width: 10),
            itemBuilder: (context, index) {
              final core = cores[index];
              final isActive = core.id == activeCoreId;

              return _CoreChip(
                core: core,
                isActive: isActive,
                onTap: () => onCoreSelected(core.id),
              );
            },
          ),
        ),
      ],
    );
  }
}

class _CoreChip extends StatelessWidget {
  final CoreState core;
  final bool isActive;
  final VoidCallback onTap;

  const _CoreChip({
    required this.core,
    required this.isActive,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final statusColor = _getStatusColor(core.status);

    return GestureDetector(
      onTap: onTap,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 300),
        curve: Curves.easeInOut,
        width: 100,
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: isActive
              ? const Color(0xFF00E5FF).withOpacity(0.15)
              : const Color(0xFF161B22),
          borderRadius: BorderRadius.circular(16),
          border: Border.all(
            color: isActive
                ? const Color(0xFF00E5FF)
                : const Color(0xFF30363D),
            width: isActive ? 2 : 1,
          ),
          boxShadow: isActive
              ? [
                  BoxShadow(
                    color: const Color(0xFF00E5FF).withOpacity(0.2),
                    blurRadius: 12,
                    offset: const Offset(0, 4),
                  ),
                ]
              : null,
        ),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            // Emoji icon
            Text(core.iconEmoji, style: const TextStyle(fontSize: 24)),
            const SizedBox(height: 4),
            // Core name
            Text(
              core.name,
              style: TextStyle(
                fontSize: 10,
                fontWeight: isActive ? FontWeight.bold : FontWeight.w500,
                color: isActive ? const Color(0xFF00E5FF) : const Color(0xFF8B949E),
              ),
              textAlign: TextAlign.center,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            const SizedBox(height: 2),
            // Status dot
            Container(
              width: 6,
              height: 6,
              decoration: BoxDecoration(
                color: statusColor,
                shape: BoxShape.circle,
              ),
            ),
          ],
        ),
      ),
    )
        .animate(target: isActive ? 1 : 0)
        .scale(
          begin: const Offset(0.95, 0.95),
          end: const Offset(1.0, 1.0),
          duration: 300.ms,
        );
  }

  Color _getStatusColor(CoreStatus status) {
    switch (status) {
      case CoreStatus.idle: return const Color(0xFF8B949E);
      case CoreStatus.connecting: return const Color(0xFFFFB74D);
      case CoreStatus.connected: return const Color(0xFF00E676);
      case CoreStatus.failed: return const Color(0xFFFF5252);
      case CoreStatus.blocked: return const Color(0xFFFF1744);
      case CoreStatus.testing: return const Color(0xFF00E5FF);
    }
  }
}
