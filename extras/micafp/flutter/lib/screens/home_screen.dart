import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:vibration/vibration.dart';

import '../services/daemon_bridge.dart';
import '../services/battery_service.dart';
import '../services/nain_status_service.dart';
import '../widgets/connection_indicator.dart';
import '../widgets/battery_indicator.dart';
import 'settings_screen.dart';
import 'paste_config_screen.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> with WidgetsBindingObserver {
  int _tapCount = 0;
  DateTime? _lastTapTime;
  Timer? _tapResetTimer;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _tapResetTimer?.cancel();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    // When app goes to background, reset tap counter
    if (state == AppLifecycleState.paused) {
      _tapCount = 0;
    }
  }

  /// Anti-forensics: 5 rapid taps triggers wipe (TRIGGER_A)
  void _handleAntiForensicsTap() {
    final now = DateTime.now();
    if (_lastTapTime != null &&
        now.difference(_lastTapTime!).inSeconds < 3) {
      _tapCount++;
    } else {
      _tapCount = 1;
    }
    _lastTapTime = now;

    _tapResetTimer?.cancel();
    _tapResetTimer = Timer(const Duration(seconds: 3), () {
      _tapCount = 0;
    });

    if (_tapCount >= 5) {
      _triggerWipe();
      _tapCount = 0;
    }
  }

  void _triggerWipe() async {
    final canVibrate = await Vibration.hasVibrator() ?? false;
    if (canVibrate) {
      await Vibration.vibrate(duration: 500);
    }

    if (!mounted) return;
    final bridge = context.read<DaemonBridge>();
    await bridge.sendWipeTrigger('TRIGGER_A');

    // After wipe, app returns to calculator-like state
    // The actual transformation happens via the platform code
  }

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: _handleAntiForensicsTap,
      child: Scaffold(
        appBar: AppBar(
          title: const Text('Shield'),
          actions: [
            IconButton(
              icon: const Icon(Icons.settings_outlined),
              onPressed: () {
                Navigator.of(context).push(
                  MaterialPageRoute(
                    builder: (_) => const SettingsScreen(),
                  ),
                );
              },
              tooltip: 'Settings',
            ),
          ],
        ),
        body: Consumer3<DaemonBridge, BatteryService, NainStatusService>(
          builder: (context, daemon, battery, nain, _) {
            return RefreshIndicator(
              onRefresh: () => daemon.queryStatus(),
              child: ListView(
                padding: const EdgeInsets.all(16),
                children: [
                  // Connection status indicator
                  _buildConnectionSection(daemon, nain),
                  const SizedBox(height: 24),

                  // Connect/Disconnect button
                  _buildConnectButton(daemon),
                  const SizedBox(height: 24),

                  // NAIN status indicator
                  _buildNainStatusCard(nain),
                  const SizedBox(height: 16),

                  // Battery optimization indicator
                  BatteryIndicator(
                    batteryService: battery,
                    onTap: () => battery.openBatterySettings(),
                  ),
                  const SizedBox(height: 16),

                  // Data usage
                  _buildDataUsageCard(daemon),
                  const SizedBox(height: 16),

                  // Share config via acoustic chirp
                  _buildShareConfigButton(daemon),

                  // Paste config (iOS fallback)
                  if (Platform.isIOS) ...[
                    const SizedBox(height: 12),
                    _buildPasteConfigButton(),
                  ],
                ],
              ),
            );
          },
        ),
      ),
    );
  }

  Widget _buildConnectionSection(DaemonBridge daemon, NainStatusService nain) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          children: [
            ConnectionIndicator(
              status: daemon.connectionStatus,
              isNainMode: nain.currentStatus != NainStatus.fullInternet,
            ),
            const SizedBox(height: 16),
            Text(
              _getStatusText(daemon.connectionStatus),
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const SizedBox(height: 4),
            Text(
              _getStatusSubtext(daemon.connectionStatus, nain.currentStatus),
              style: Theme.of(context).textTheme.bodyMedium,
            ),
          ],
        ),
      ),
    );
  }

  String _getStatusText(ConnectionStatus status) {
    switch (status) {
      case ConnectionStatus.connected:
        return 'Active';
      case ConnectionStatus.disconnected:
        return 'Inactive';
      case ConnectionStatus.connecting:
        return 'Connecting';
    }
  }

  String _getStatusSubtext(ConnectionStatus status, NainStatus nain) {
    if (status == ConnectionStatus.connected) {
      if (nain != NainStatus.fullInternet) {
        return 'Alternative channel active';
      }
      return 'All services running';
    } else if (status == ConnectionStatus.connecting) {
      return 'Establishing connection...';
    }
    return 'Tap to connect';
  }

  Widget _buildConnectButton(DaemonBridge daemon) {
    final isConnected = daemon.connectionStatus == ConnectionStatus.connected;
    final isConnecting = daemon.connectionStatus == ConnectionStatus.connecting;

    return SizedBox(
      width: double.infinity,
      height: 56,
      child: ElevatedButton(
        onPressed: isConnecting
            ? null
            : () async {
                if (isConnected) {
                  await daemon.sendDisconnect();
                } else {
                  await daemon.sendConnect();
                }
              },
        style: ElevatedButton.styleFrom(
          backgroundColor: isConnected
              ? const Color(0xFFC62828)
              : const Color(0xFF2E7D32),
          foregroundColor: Colors.white,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(16),
          ),
          elevation: 4,
        ),
        child: isConnecting
            ? const SizedBox(
                width: 24,
                height: 24,
                child: CircularProgressIndicator(
                  strokeWidth: 2,
                  valueColor: AlwaysStoppedAnimation(Colors.white),
                ),
              )
            : Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(isConnected ? Icons.stop : Icons.play_arrow, size: 24),
                  const SizedBox(width: 8),
                  Text(
                    isConnected ? 'Stop' : 'Start',
                    style: const TextStyle(
                      fontSize: 18,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ],
              ),
      ),
    );
  }

  Widget _buildNainStatusCard(NainStatusService nain) {
    final status = nain.currentStatus;
    final color = _getNainColor(status);
    final icon = _getNainIcon(status);
    final label = _getNainLabel(status);

    return Card(
      child: ListTile(
        leading: Icon(icon, color: color, size: 28),
        title: Text(
          label,
          style: TextStyle(color: color, fontWeight: FontWeight.w600),
        ),
        subtitle: Text(
          _getNainDescription(status),
          style: Theme.of(context).textTheme.bodySmall,
        ),
        trailing: status == NainStatus.completeBlackout
            ? const Icon(Icons.warning_amber, color: Colors.orange)
            : null,
      ),
    );
  }

  Color _getNainColor(NainStatus status) {
    switch (status) {
      case NainStatus.fullInternet:
        return const Color(0xFF2E7D32);
      case NainStatus.nationalIntranet:
        return const Color(0xFFF57F17);
      case NainStatus.completeBlackout:
        return const Color(0xFFE65100);
    }
  }

  IconData _getNainIcon(NainStatus status) {
    switch (status) {
      case NainStatus.fullInternet:
        return Icons.language;
      case NainStatus.nationalIntranet:
        return Icons.domain;
      case NainStatus.completeBlackout:
        return Icons.cloud_off;
    }
  }

  String _getNainLabel(NainStatus status) {
    switch (status) {
      case NainStatus.fullInternet:
        return 'Full Access';
      case NainStatus.nationalIntranet:
        return 'Limited Access';
      case NainStatus.completeBlackout:
        return 'Emergency Mode';
    }
  }

  String _getNainDescription(NainStatus status) {
    switch (status) {
      case NainStatus.fullInternet:
        return 'All channels operational';
      case NainStatus.nationalIntranet:
        return 'Alternative channels available';
      case NainStatus.completeBlackout:
        return 'Mesh channels activated';
    }
  }

  Widget _buildDataUsageCard(DaemonBridge daemon) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            Expanded(
              child: _buildDataItem(
                icon: Icons.arrow_upward,
                label: 'Uploaded',
                value: _formatBytes(daemon.bytesUploaded),
                color: const Color(0xFF5C6BC0),
              ),
            ),
            const SizedBox(
              height: 40,
              child: VerticalDivider(color: Colors.grey),
            ),
            Expanded(
              child: _buildDataItem(
                icon: Icons.arrow_downward,
                label: 'Downloaded',
                value: _formatBytes(daemon.bytesDownloaded),
                color: const Color(0xFF26A69A),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildDataItem({
    required IconData icon,
    required String label,
    required String value,
    required Color color,
  }) {
    return Column(
      children: [
        Icon(icon, color: color, size: 20),
        const SizedBox(height: 4),
        Text(
          value,
          style: const TextStyle(
            fontSize: 16,
            fontWeight: FontWeight.bold,
            color: Color(0xFFE0E0E0),
          ),
        ),
        Text(
          label,
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }

  String _formatBytes(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1048576) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    if (bytes < 1073741824) {
      return '${(bytes / 1048576).toStringAsFixed(1)} MB';
    }
    return '${(bytes / 1073741824).toStringAsFixed(2)} GB';
  }

  Widget _buildShareConfigButton(DaemonBridge daemon) {
    return OutlinedButton.icon(
      onPressed: daemon.connectionStatus == ConnectionStatus.connected
          ? () => _shareConfigViaAcoustic(daemon)
          : null,
      icon: const Icon(Icons.share_outlined),
      label: const Text('Share Configuration'),
      style: OutlinedButton.styleFrom(
        minimumSize: const Size(double.infinity, 48),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
        ),
      ),
    );
  }

  void _shareConfigViaAcoustic(DaemonBridge daemon) async {
    // Request acoustic config from daemon, then encode as chirp
    try {
      final config = await daemon.requestAcousticConfig();
      if (!mounted) return;

      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Hold devices close together to share...'),
          duration: Duration(seconds: 3),
        ),
      );

      // The actual acoustic transmission is handled by the Rust daemon
      // via the flutter_sound service for audio I/O
      await daemon.sendAcousticChirp(config);
    } catch (e) {
      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Sharing failed: $e')),
      );
    }
  }

  Widget _buildPasteConfigButton() {
    return OutlinedButton.icon(
      onPressed: () {
        Navigator.of(context).push(
          MaterialPageRoute(
            builder: (_) => const PasteConfigScreen(),
          ),
        );
      },
      icon: const Icon(Icons.content_paste),
      label: const Text('Import Configuration'),
      style: OutlinedButton.styleFrom(
        minimumSize: const Size(double.infinity, 48),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
        ),
      ),
    );
  }
}
