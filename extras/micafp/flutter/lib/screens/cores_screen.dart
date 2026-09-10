import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_animate/flutter_animate.dart';
import '../l10n/app_localizations.dart';
import '../models/core_state.dart';

/// Core management screen with UCB1 algorithm visualization.
///
/// Shows all 9 cores with their status, latency, bandwidth,
/// and UCB1 scores. Allows manual core switching or auto-selection.
class CoresScreen extends ConsumerStatefulWidget {
  const CoresScreen({super.key});

  @override
  ConsumerState<CoresScreen> createState() => _CoresScreenState();
}

class _CoresScreenState extends ConsumerState<CoresScreen> {
  bool _autoSelect = true;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final cores = ref.watch(coresProvider);
    final bestCore = ref.read(coresProvider.notifier).getBestCore();

    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.coresTitle),
        actions: [
          // Auto-select toggle
          Switch.adaptive(
            value: _autoSelect,
            onChanged: (val) => setState(() => _autoSelect = val),
            activeColor: const Color(0xFF00E5FF),
          ),
          const SizedBox(width: 8),
        ],
      ),
      body: CustomScrollView(
        slivers: [
          // UCB1 Recommendation Card
          SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Card(
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          const Icon(Icons.auto_awesome, color: Color(0xFF00E5FF)),
                          const SizedBox(width: 8),
                          Text(
                            l10n.ucb1Recommendation,
                            style: Theme.of(context).textTheme.titleMedium,
                          ),
                        ],
                      ),
                      const SizedBox(height: 12),
                      Text(
                        '${bestCore.iconEmoji} ${bestCore.name}',
                        style: const TextStyle(
                          fontSize: 20,
                          fontWeight: FontWeight.bold,
                          color: Color(0xFF00E5FF),
                        ),
                      ),
                      Text(
                        bestCore.description,
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                      const SizedBox(height: 8),
                      Row(
                        children: [
                          _buildStatChip(
                            '${l10n.score}: ${bestCore.ucb1Score.toStringAsFixed(2)}',
                            const Color(0xFF00E5FF),
                          ),
                          const SizedBox(width: 8),
                          _buildStatChip(
                            '${l10n.successRate}: ${(bestCore.successRate * 100).toStringAsFixed(0)}%',
                            const Color(0xFF00E676),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),

          // Core Cards
          SliverList(
            delegate: SliverChildBuilderDelegate(
              (context, index) {
                final core = cores[index];
                return _CoreCard(
                  core: core,
                  isBest: core.id == bestCore.id,
                  autoSelect: _autoSelect,
                  onActivate: () => _activateCore(core),
                  onTest: () => _testCore(core),
                );
              },
              childCount: cores.length,
            ),
          ),
        ],
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: _testAllCores,
        icon: const Icon(Icons.speed),
        label: Text(l10n.testAllCores),
        backgroundColor: const Color(0xFF00E5FF),
      ),
    );
  }

  Widget _buildStatChip(String label, Color color) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: color.withOpacity(0.1),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withOpacity(0.3)),
      ),
      child: Text(
        label,
        style: TextStyle(color: color, fontSize: 12, fontWeight: FontWeight.w600),
      ),
    );
  }

  Future<void> _activateCore(CoreState core) async {
    ref.read(coresProvider.notifier).setCoreStatus(core.id, CoreStatus.connecting);
    // Simulate connection attempt
    await Future.delayed(const Duration(seconds: 2));
    ref.read(coresProvider.notifier).recordSuccess(core.id, latency: 45, bandwidth: 5000);
  }

  Future<void> _testCore(CoreState core) async {
    ref.read(coresProvider.notifier).setCoreStatus(core.id, CoreStatus.testing);
    await Future.delayed(const Duration(seconds: 3));
    final success = DateTime.now().millisecond % 3 != 0; // 66% success rate
    if (success) {
      ref.read(coresProvider.notifier).recordSuccess(core.id, latency: 30 + (DateTime.now().millisecond % 200).toDouble());
    } else {
      ref.read(coresProvider.notifier).recordFailure(core.id);
    }
  }

  Future<void> _testAllCores() async {
    for (final core in ref.read(coresProvider)) {
      _testCore(core);
      await Future.delayed(const Duration(milliseconds: 500));
    }
  }
}

class _CoreCard extends ConsumerWidget {
  final CoreState core;
  final bool isBest;
  final bool autoSelect;
  final VoidCallback onActivate;
  final VoidCallback onTest;

  const _CoreCard({
    required this.core,
    required this.isBest,
    required this.autoSelect,
    required this.onActivate,
    required this.onTest,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = AppLocalizations.of(context)!;
    final statusColor = _getStatusColor(core.status);

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
      child: Card(
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: isBest
              ? const BorderSide(color: Color(0xFF00E5FF), width: 2)
              : BorderSide.none,
        ),
        child: InkWell(
          onTap: onActivate,
          borderRadius: BorderRadius.circular(16),
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                Row(
                  children: [
                    // Emoji icon
                    Text(core.iconEmoji, style: const TextStyle(fontSize: 32)),
                    const SizedBox(width: 16),

                    // Core info
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Text(
                                core.name,
                                style: const TextStyle(
                                  fontSize: 16,
                                  fontWeight: FontWeight.bold,
                                ),
                              ),
                              if (isBest) ...[
                                const SizedBox(width: 8),
                                Container(
                                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                                  decoration: BoxDecoration(
                                    color: const Color(0xFF00E5FF).withOpacity(0.2),
                                    borderRadius: BorderRadius.circular(8),
                                  ),
                                  child: const Text(
                                    'BEST',
                                    style: TextStyle(
                                      color: Color(0xFF00E5FF),
                                      fontSize: 10,
                                      fontWeight: FontWeight.bold,
                                    ),
                                  ),
                                ),
                              ],
                            ],
                          ),
                          const SizedBox(height: 2),
                          Text(
                            core.description,
                            style: Theme.of(context).textTheme.bodySmall,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ],
                      ),
                    ),

                    // Status indicator
                    Container(
                      width: 12,
                      height: 12,
                      decoration: BoxDecoration(
                        color: statusColor,
                        shape: BoxShape.circle,
                      ),
                    ),
                  ],
                ),

                const SizedBox(height: 12),

                // Stats row
                Row(
                  children: [
                    _StatItem(label: l10n.latency, value: '${core.latency.toStringAsFixed(0)} ms'),
                    _StatItem(label: l10n.bandwidth, value: '${(core.bandwidth / 1024).toStringAsFixed(1)} MB/s'),
                    _StatItem(label: l10n.success, value: '${core.successCount}'),
                    _StatItem(label: l10n.fail, value: '${core.failureCount}'),
                  ],
                ),

                // UCB1 progress bar
                if (core.successCount + core.failureCount > 0) ...[
                  const SizedBox(height: 8),
                  LinearProgressIndicator(
                    value: core.successRate,
                    backgroundColor: const Color(0xFF21262D),
                    color: const Color(0xFF00E676),
                    borderRadius: BorderRadius.circular(4),
                  ),
                ],

                const SizedBox(height: 8),

                // Action buttons
                Row(
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: [
                    TextButton.icon(
                      onPressed: onTest,
                      icon: const Icon(Icons.speed, size: 16),
                      label: Text(l10n.test),
                      style: TextButton.styleFrom(
                        foregroundColor: const Color(0xFF00E5FF),
                      ),
                    ),
                    const SizedBox(width: 8),
                    FilledButton.icon(
                      onPressed: core.status == CoreStatus.connected ? null : onActivate,
                      icon: const Icon(Icons.play_arrow, size: 16),
                      label: Text(l10n.activate),
                      style: FilledButton.styleFrom(
                        backgroundColor: const Color(0xFF00E5FF),
                        foregroundColor: Colors.black,
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ).animate().fadeIn(duration: 300.ms, delay: (50).ms),
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

class _StatItem extends StatelessWidget {
  final String label;
  final String value;

  const _StatItem({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    return Expanded(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label, style: const TextStyle(color: Color(0xFF8B949E), fontSize: 10)),
          Text(value, style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 12)),
        ],
      ),
    );
  }
}
