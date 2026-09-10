import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../services/daemon_service.dart';
import '../services/battery_service.dart';
import '../widgets/connection_indicator.dart';
import '../main.dart';

/// HomeScreen — Main VPN control screen.
///
/// Shows a large connection status indicator, one-tap connect/disconnect button,
/// current transport name (without revealing details), battery status, data usage
/// counter, and quick settings integration. NEVER shows endpoint URLs or transport
/// details. The connection status animates smoothly between states.
class HomeScreen extends StatefulWidget {
  final VoidCallback onLock;
  final VoidCallback onToggleLocale;
  final VoidCallback onToggleTheme;

  const HomeScreen({
    super.key,
    required this.onLock,
    required this.onToggleLocale,
    required this.onToggleTheme,
  });

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> with TickerProviderStateMixin {
  late AnimationController _pulseController;
  late Animation<double> _pulseAnimation;
  bool _isConnecting = false;

  @override
  void initState() {
    super.initState();

    _pulseController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1500),
    );

    _pulseAnimation = Tween<double>(begin: 1.0, end: 1.15).animate(
      CurvedAnimation(parent: _pulseController, curve: Curves.easeInOut),
    );
  }

  @override
  void dispose() {
    _pulseController.dispose();
    super.dispose();
  }

  /// Handle connect/disconnect button press.
  Future<void> _toggleConnection() async {
    final daemonService = context.read<DaemonService>();

    if (daemonService.isConnected) {
      await daemonService.disconnect();
    } else {
      setState(() => _isConnecting = true);
      try {
        await daemonService.connect();
      } finally {
        if (mounted) {
          setState(() => _isConnecting = false);
        }
      }
    }
  }

  /// Format bytes to human-readable string.
  String _formatBytes(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    if (bytes < 1024 * 1024 * 1024) return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }

  /// Format duration to human-readable string.
  String _formatDuration(Duration? duration) {
    if (duration == null) return '--:--';
    final hours = duration.inHours;
    final minutes = duration.inMinutes.remainder(60);
    final seconds = duration.inSeconds.remainder(60);
    if (hours > 0) {
      return '${hours}h ${minutes}m';
    }
    return '${minutes}m ${seconds}s';
  }

  @override
  Widget build(BuildContext context) {
    final daemonService = context.watch<DaemonService>();
    final batteryService = context.watch<BatteryService>();
    final l10n = context.l10n;

    // Update pulse animation based on connection state
    if (daemonService.isConnecting || _isConnecting) {
      _pulseController.repeat(reverse: true);
    } else {
      _pulseController.stop();
      _pulseController.value = 0;
    }

    // Battery-aware: reduce animations on low battery
    final reduceAnimations = batteryService.shouldReduceAnimations;

    return Scaffold(
      backgroundColor: Theme.of(context).colorScheme.surface,
      body: SafeArea(
        child: Column(
          children: [
            // Top bar with lock and settings
            _buildTopBar(context, l10n),

            // Main content area
            Expanded(
              child: SingleChildScrollView(
                padding: const EdgeInsets.symmetric(horizontal: 24),
                child: Column(
                  children: [
                    const SizedBox(height: 32),

                    // Connection status indicator
                    ConnectionIndicator(
                      isConnected: daemonService.isConnected,
                      isConnecting: daemonService.isConnecting || _isConnecting,
                      pulseAnimation: reduceAnimations ? null : _pulseAnimation,
                    ),

                    const SizedBox(height: 16),

                    // Status text
                    _buildStatusText(context, daemonService, l10n),

                    const SizedBox(height: 32),

                    // Connect/disconnect button
                    _buildConnectButton(context, daemonService, l10n),

                    const SizedBox(height: 32),

                    // Info cards
                    _buildInfoCards(context, daemonService, batteryService, l10n),

                    const SizedBox(height: 16),

                    // Battery warning
                    if (batteryService.isUltraLowPower)
                      _buildBatteryWarning(context, l10n),

                    // Paste config code button (iOS fallback)
                    if (Theme.of(context).platform == TargetPlatform.iOS)
                      _buildPasteConfigButton(context, l10n),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  /// Build the top bar with lock button and settings.
  Widget _buildTopBar(BuildContext context, String Function(String) l10n) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          // Lock button
          IconButton(
            onPressed: widget.onLock,
            icon: const Icon(Icons.lock_outline),
            tooltip: l10n('lock'),
          ),

          // App title — always "Shield"
          Text(
            l10n('app_title'),
            style: Theme.of(context).textTheme.titleLarge?.copyWith(
                  fontWeight: FontWeight.bold,
                ),
          ),

          // Settings row
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              IconButton(
                onPressed: widget.onToggleTheme,
                icon: Icon(
                  Theme.of(context).brightness == Brightness.dark
                      ? Icons.light_mode_outlined
                      : Icons.dark_mode_outlined,
                ),
                tooltip: l10n('dark_mode'),
              ),
              IconButton(
                onPressed: widget.onToggleLocale,
                icon: const Icon(Icons.translate),
                tooltip: l10n('language'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  /// Build the status text below the connection indicator.
  Widget _buildStatusText(
    BuildContext context,
    DaemonService daemonService,
    String Function(String) l10n,
  ) {
    String statusText;
    Color statusColor;

    if (daemonService.isConnected) {
      statusText = l10n('status_connected');
      statusColor = Colors.green;
    } else if (daemonService.isConnecting || _isConnecting) {
      statusText = l10n('status_connecting');
      statusColor = Colors.amber;
    } else {
      statusText = l10n('status_disconnected');
      statusColor = Colors.red;
    }

    return AnimatedSwitcher(
      duration: const Duration(milliseconds: 300),
      child: Text(
        statusText,
        key: ValueKey(statusText),
        style: Theme.of(context).textTheme.titleMedium?.copyWith(
              color: statusColor,
              fontWeight: FontWeight.w600,
            ),
      ),
    );
  }

  /// Build the main connect/disconnect button.
  Widget _buildConnectButton(
    BuildContext context,
    DaemonService daemonService,
    String Function(String) l10n,
  ) {
    final isConnected = daemonService.isConnected;
    final isConnecting = daemonService.isConnecting || _isConnecting;

    return SizedBox(
      width: double.infinity,
      height: 56,
      child: FilledButton(
        onPressed: isConnecting ? null : _toggleConnection,
        style: FilledButton.styleFrom(
          backgroundColor: isConnected
              ? Colors.red.shade700
              : Theme.of(context).colorScheme.primary,
          foregroundColor: Colors.white,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(16),
          ),
          elevation: 0,
        ),
        child: isConnecting
            ? const SizedBox(
                width: 24,
                height: 24,
                child: CircularProgressIndicator(
                  strokeWidth: 2.5,
                  color: Colors.white,
                ),
              )
            : Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(
                    isConnected ? Icons.stop : Icons.play_arrow,
                    size: 24,
                  ),
                  const SizedBox(width: 8),
                  Text(
                    isConnected ? l10n('disconnect') : l10n('connect'),
                    style: const TextStyle(
                      fontSize: 18,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                ],
              ),
      ),
    );
  }

  /// Build the info cards showing connection details.
  Widget _buildInfoCards(
    BuildContext context,
    DaemonService daemonService,
    BatteryService batteryService,
    String Function(String) l10n,
  ) {
    return Column(
      children: [
        // Transport name (hidden details)
        _buildInfoCard(
          context,
          icon: Icons.swap_horiz,
          title: l10n('transport'),
          value: daemonService.isConnected ? 'Shield' : '--',
        ),

        const SizedBox(height: 12),

        // Connection duration
        _buildInfoCard(
          context,
          icon: Icons.timer_outlined,
          title: l10n('status_connected'),
          value: daemonService.isConnected
              ? _formatDuration(daemonService.connectionDuration)
              : '--:--',
        ),

        const SizedBox(height: 12),

        // Data usage
        _buildInfoCard(
          context,
          icon: Icons.data_usage,
          title: l10n('data_used'),
          value: daemonService.isConnected
              ? '${_formatBytes(daemonService.bytesIn)} ↓ / ${_formatBytes(daemonService.bytesOut)} ↑'
              : '0 B',
        ),

        const SizedBox(height: 12),

        // Battery status
        _buildInfoCard(
          context,
          icon: batteryService.isCharging
              ? Icons.battery_charging_full
              : Icons.battery_std,
          title: l10n('battery_low'),
          value: '${batteryService.batteryLevel}%${batteryService.isCharging ? ' ⚡' : ''}',
          trailing: batteryService.isUltraLowPower
              ? const Icon(Icons.warning, color: Colors.red, size: 20)
              : null,
        ),
      ],
    );
  }

  /// Build a single info card.
  Widget _buildInfoCard(
    BuildContext context, {
    required IconData icon,
    required String title,
    required String value,
    Widget? trailing,
  }) {
    return Card(
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(
          color: Theme.of(context).colorScheme.outlineVariant.withOpacity(0.3),
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
        child: Row(
          children: [
            Icon(icon, size: 22, color: Theme.of(context).colorScheme.primary),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                title,
                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
              ),
            ),
            Text(
              value,
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
            ),
            if (trailing != null) ...[
              const SizedBox(width: 8),
              trailing,
            ],
          ],
        ),
      ),
    );
  }

  /// Build the battery warning banner.
  Widget _buildBatteryWarning(BuildContext context, String Function(String) l10n) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: Colors.red.shade50,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: Colors.red.shade200),
      ),
      child: Row(
        children: [
          Icon(Icons.battery_alert, color: Colors.red.shade700, size: 24),
          const SizedBox(width: 12),
          Expanded(
            child: Text(
              l10n('battery_warning'),
              style: TextStyle(
                color: Colors.red.shade900,
                fontSize: 14,
                fontWeight: FontWeight.w500,
              ),
            ),
          ),
        ],
      ),
    );
  }

  /// Build the paste config code button (iOS SMS fallback).
  Widget _buildPasteConfigButton(BuildContext context, String Function(String) l10n) {
    return Padding(
      padding: const EdgeInsets.only(top: 16),
      child: OutlinedButton.icon(
        onPressed: () => _showPasteConfigDialog(context),
        icon: const Icon(Icons.paste, size: 18),
        label: Text(l10n('paste_config')),
        style: OutlinedButton.styleFrom(
          minimumSize: const Size(double.infinity, 48),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
          ),
        ),
      ),
    );
  }

  /// Show the paste config code dialog.
  void _showPasteConfigDialog(BuildContext context) {
    final controller = TextEditingController();

    showDialog(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: Text(context.l10n('paste_config')),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              'Paste the configuration code you received via SMS or another channel.',
              style: Theme.of(context).textTheme.bodySmall,
            ),
            const SizedBox(height: 12),
            TextField(
              controller: controller,
              maxLines: 3,
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                hintText: 'Enter config code...',
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () {
              final code = controller.text.trim();
              if (code.isNotEmpty) {
                context.read<DaemonService>().pasteConfigCode(code);
              }
              Navigator.pop(dialogContext);
            },
            child: const Text('Apply'),
          ),
        ],
      ),
    );
  }
}
