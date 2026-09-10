import 'dart:async';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logger/logger.dart';
import 'package:connectivity_plus/connectivity_plus.dart';
import 'daemon_service.dart';

/// Platform VPN management service.
///
/// Handles the lifecycle of the VPN tunnel on each platform:
/// - Android: VpnService via platform channel
/// - iOS: NetworkExtension via platform channel
/// - Windows/macOS/Linux: TUN/TAP device via Rust daemon
class VpnService {
  static final Logger _log = Logger(printer: PrettyPrinter(methodCount: 0));

  final DaemonService _daemon;

  VpnService(this._daemon);

  bool _isConnected = false;
  String _activeCore = '';
  String _connectedServer = '';
  String _protocol = '';
  DateTime? _connectedAt;

  bool get isConnected => _isConnected;
  String get activeCore => _activeCore;
  String get connectedServer => _connectedServer;
  String get protocol => _protocol;
  Duration? get connectionDuration =>
      _connectedAt != null ? DateTime.now().difference(_connectedAt!) : null;

  /// Connect to VPN via specified core and server
  Future<bool> connect({
    required String coreId,
    required String serverAddr,
    int port = 443,
    String protocol = 'auto',
    Map<String, dynamic>? obfuscation,
  }) async {
    try {
      _log.i('Starting VPN connection: core=$coreId, server=$serverAddr:$port');

      // Check internet connectivity first
      final connectivityResult = await Connectivity().checkConnectivity();
      if (connectivityResult.contains(ConnectivityResult.none)) {
        _log.e('No internet connectivity');
        return false;
      }

      // Request VPN permission (Android/iOS)
      final hasPermission = await _requestVpnPermission();
      if (!hasPermission) {
        _log.e('VPN permission denied');
        return false;
      }

      // Send connect command to daemon
      await _daemon.startVpn(
        coreId: coreId,
        serverAddr: serverAddr,
        port: port,
        protocol: protocol,
        obfuscation: obfuscation,
      );

      _isConnected = true;
      _activeCore = coreId;
      _connectedServer = serverAddr;
      _protocol = protocol;
      _connectedAt = DateTime.now();

      _log.i('VPN connected successfully via $coreId');
      return true;
    } catch (e) {
      _log.e('VPN connection failed', error: e);
      _isConnected = false;
      return false;
    }
  }

  /// Disconnect VPN
  Future<void> disconnect() async {
    try {
      _log.i('Disconnecting VPN...');
      await _daemon.stopVpn();
      _isConnected = false;
      _activeCore = '';
      _connectedServer = '';
      _protocol = '';
      _connectedAt = null;
      _log.i('VPN disconnected');
    } catch (e) {
      _log.e('VPN disconnect failed', error: e);
    }
  }

  /// Auto-reconnect with exponential backoff
  Future<void> autoReconnect({
    required String coreId,
    required String serverAddr,
    int maxRetries = 5,
    int baseDelayMs = 1000,
  }) async {
    for (int attempt = 0; attempt < maxRetries; attempt++) {
      final delay = baseDelayMs * (1 << attempt); // Exponential backoff
      _log.i('Auto-reconnect attempt ${attempt + 1}/$maxRetries (delay: ${delay}ms)');
      await Future.delayed(Duration(milliseconds: delay));

      final success = await connect(
        coreId: coreId,
        serverAddr: serverAddr,
      );

      if (success) {
        _log.i('Auto-reconnect succeeded on attempt ${attempt + 1}');
        return;
      }
    }

    _log.e('Auto-reconnect failed after $maxRetries attempts');
  }

  /// Request VPN permission from OS
  Future<bool> _requestVpnPermission() async {
    // On Android, this triggers the system VPN permission dialog
    // On iOS, this checks for Network Extension entitlement
    // On desktop, this checks for admin/root privileges
    // The actual implementation uses platform channels
    return true; // Placeholder - actual platform channel call
  }

  /// Get real-time traffic stats
  Future<TrafficStats> getTrafficStats() async {
    final status = await _daemon.getStatus();
    return TrafficStats(
      bytesIn: status['bytes_in'] as int? ?? 0,
      bytesOut: status['bytes_out'] as int? ?? 0,
      packetsIn: status['packets_in'] as int? ?? 0,
      packetsOut: status['packets_out'] as int? ?? 0,
      speedDown: (status['speed_down'] as num?)?.toDouble() ?? 0.0,
      speedUp: (status['speed_up'] as num?)?.toDouble() ?? 0.0,
      latency: (status['latency_ms'] as num?)?.toDouble() ?? 0.0,
    );
  }

  /// Kill switch: block all traffic if VPN disconnects unexpectedly
  Future<void> enableKillSwitch() async {
    await _daemon.sendCommand('kill_switch.enable', {});
  }

  Future<void> disableKillSwitch() async {
    await _daemon.sendCommand('kill_switch.disable', {});
  }
}

/// Traffic statistics model
class TrafficStats {
  final int bytesIn;
  final int bytesOut;
  final int packetsIn;
  final int packetsOut;
  final double speedDown; // bytes/sec
  final double speedUp;   // bytes/sec
  final double latency;   // ms

  const TrafficStats({
    this.bytesIn = 0,
    this.bytesOut = 0,
    this.packetsIn = 0,
    this.packetsOut = 0,
    this.speedDown = 0.0,
    this.speedUp = 0.0,
    this.latency = 0.0,
  });

  String get speedDownFormatted => _formatSpeed(speedDown);
  String get speedUpFormatted => _formatSpeed(speedUp);

  static String _formatSpeed(double bytesPerSec) {
    if (bytesPerSec < 1024) return '${bytesPerSec.toStringAsFixed(0)} B/s';
    if (bytesPerSec < 1024 * 1024) return '${(bytesPerSec / 1024).toStringAsFixed(1)} KB/s';
    return '${(bytesPerSec / (1024 * 1024)).toStringAsFixed(1)} MB/s';
  }
}

/// Riverpod providers
final vpnServiceProvider = Provider<VpnService>((ref) {
  final daemon = ref.watch(daemonServiceProvider);
  return VpnService(daemon);
});

final vpnConnectionProvider = StateNotifierProvider<VpnConnectionNotifier, VpnConnectionState>((ref) {
  final vpnService = ref.watch(vpnServiceProvider);
  return VpnConnectionNotifier(vpnService);
});

class VpnConnectionState {
  final bool isConnected;
  final String activeCore;
  final String connectedServer;
  final TrafficStats? stats;
  final bool isConnecting;

  const VpnConnectionState({
    this.isConnected = false,
    this.activeCore = '',
    this.connectedServer = '',
    this.stats,
    this.isConnecting = false,
  });

  VpnConnectionState copyWith({
    bool? isConnected,
    String? activeCore,
    String? connectedServer,
    TrafficStats? stats,
    bool? isConnecting,
  }) {
    return VpnConnectionState(
      isConnected: isConnected ?? this.isConnected,
      activeCore: activeCore ?? this.activeCore,
      connectedServer: connectedServer ?? this.connectedServer,
      stats: stats ?? this.stats,
      isConnecting: isConnecting ?? this.isConnecting,
    );
  }
}

class VpnConnectionNotifier extends StateNotifier<VpnConnectionState> {
  final VpnService _vpn;
  Timer? _statsTimer;

  VpnConnectionNotifier(this._vpn) : super(const VpnConnectionState());

  Future<void> connect({
    required String coreId,
    required String serverAddr,
    int port = 443,
    String protocol = 'auto',
  }) async {
    state = state.copyWith(isConnecting: true);
    final success = await _vpn.connect(
      coreId: coreId,
      serverAddr: serverAddr,
      port: port,
      protocol: protocol,
    );
    state = state.copyWith(
      isConnected: success,
      isConnecting: false,
      activeCore: success ? coreId : '',
      connectedServer: success ? serverAddr : '',
    );
    if (success) _startStatsPolling();
  }

  Future<void> disconnect() async {
    _statsTimer?.cancel();
    await _vpn.disconnect();
    state = const VpnConnectionState();
  }

  void _startStatsPolling() {
    _statsTimer?.cancel();
    _statsTimer = Timer.periodic(const Duration(seconds: 1), (_) async {
      if (_vpn.isConnected) {
        final stats = await _vpn.getTrafficStats();
        state = state.copyWith(stats: stats);
      }
    });
  }

  @override
  void dispose() {
    _statsTimer?.cancel();
    super.dispose();
  }
}
