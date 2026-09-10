import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logger/logger.dart';
import 'package:path_provider/path_provider.dart';

/// IPC communication with the UnifiedShield Rust daemon.
///
/// The Rust daemon runs as a background service and exposes:
/// - Unix domain socket (Linux/Android/macOS)
/// - Named pipe (Windows)
/// - gRPC endpoint (all platforms)
class DaemonService {
  static final Logger _log = Logger(printer: PrettyPrinter(methodCount: 0));

  // Singleton
  static final DaemonService _instance = DaemonService._internal();
  factory DaemonService() => _instance;
  DaemonService._internal();

  Socket? _socket;
  bool _connected = false;
  final StreamController<Map<String, dynamic>> _messageController =
      StreamController<Map<String, dynamic>>.broadcast();

  /// Stream of messages from the daemon
  Stream<Map<String, dynamic>> get messages => _messageController.stream;

  /// Whether we have an active connection to the daemon
  bool get isConnected => _connected;

  /// Connect to the Rust daemon via Unix domain socket
  Future<void> connect() async {
    if (_connected) return;

    try {
      final dir = await getApplicationSupportDirectory();
      final socketPath = '${dir.path}/unifiedshield.sock';

      _log.i('Connecting to daemon at $socketPath');

      _socket = await Socket.connect(
        InternetAddress(socketPath, type: InternetAddressType.unix),
        0,
        timeout: const Duration(seconds: 5),
      );

      _connected = true;
      _log.i('Connected to UnifiedShield daemon');

      // Listen for incoming messages
      _socket!.listen(
        _onData,
        onError: _onError,
        onDone: _onDone,
      );
    } catch (e) {
      _log.e('Failed to connect to daemon', error: e);
      _connected = false;
      rethrow;
    }
  }

  /// Send a command to the daemon
  Future<Map<String, dynamic>> sendCommand(String method, Map<String, dynamic> params) async {
    if (!_connected || _socket == null) {
      await connect();
    }

    final request = {
      'jsonrpc': '2.0',
      'id': DateTime.now().millisecondsSinceEpoch,
      'method': method,
      'params': params,
    };

    final payload = utf8.encode('${jsonEncode(request)}\n');
    _socket!.add(payload);

    // Wait for response with matching ID
    final completer = Completer<Map<String, dynamic>>();
    final sub = messages
        .where((msg) => msg['id'] == request['id'])
        .timeout(const Duration(seconds: 30), onTimeout: (_) => {})
        .listen((msg) {
      if (!completer.isCompleted) completer.complete(msg);
    });

    final response = await completer.future;
    await sub.cancel();
    return response;
  }

  /// Start VPN connection via daemon
  Future<void> startVpn({
    required String coreId,
    required String serverAddr,
    int port = 443,
    String protocol = 'auto',
    Map<String, dynamic>? obfuscation,
  }) async {
    await sendCommand('vpn.start', {
      'core_id': coreId,
      'server_addr': serverAddr,
      'port': port,
      'protocol': protocol,
      'obfuscation': obfuscation ?? {},
    });
  }

  /// Stop VPN connection
  Future<void> stopVpn() async {
    await sendCommand('vpn.stop', {});
  }

  /// Get current connection status
  Future<Map<String, dynamic>> getStatus() async {
    return await sendCommand('vpn.status', {});
  }

  /// Get all 9 core statuses
  Future<Map<String, dynamic>> getCoresStatus() async {
    return await sendCommand('cores.status', {});
  }

  /// Switch active core using UCB1 algorithm
  Future<void> switchCore(String coreId) async {
    await sendCommand('cores.switch', {'core_id': coreId});
  }

  /// Enable national intranet mode
  Future<void> enableIntranetMode({
    required List<String> allowedDomains,
    bool blockAllExternal = false,
  }) async {
    await sendCommand('intranet.enable', {
      'allowed_domains': allowedDomains,
      'block_all_external': blockAllExternal,
    });
  }

  /// Disable national intranet mode
  Future<void> disableIntranetMode() async {
    await sendCommand('intranet.disable', {});
  }

  /// Run DPI test
  Future<Map<String, dynamic>> runDpiTest() async {
    return await sendCommand('security.dpi_test', {});
  }

  /// Run security audit
  Future<Map<String, dynamic>> runSecurityAudit() async {
    return await sendCommand('security.audit', {});
  }

  /// Get P2P peer list
  Future<Map<String, dynamic>> getP2PPeers() async {
    return await sendCommand('p2p.peers', {});
  }

  /// Check for OTA updates
  Future<Map<String, dynamic>> checkOtaUpdate() async {
    return await sendCommand('ota.check', {});
  }

  /// Apply OTA update
  Future<void> applyOtaUpdate(String version) async {
    await sendCommand('ota.apply', {'version': version});
  }

  void _onData(List<int> data) {
    try {
      final message = jsonDecode(utf8.decode(data)) as Map<String, dynamic>;
      _messageController.add(message);
    } catch (e) {
      _log.w('Failed to parse daemon message', error: e);
    }
  }

  void _onError(dynamic error) {
    _log.e('Daemon socket error', error: error);
    _connected = false;
  }

  void _onDone() {
    _log.i('Daemon socket closed');
    _connected = false;
  }

  /// Disconnect from daemon
  Future<void> disconnect() async {
    _socket?.destroy();
    _socket = null;
    _connected = false;
  }

  /// Dispose resources
  void dispose() {
    disconnect();
    _messageController.close();
  }
}

/// Riverpod provider for DaemonService
final daemonServiceProvider = Provider<DaemonService>((ref) {
  final service = DaemonService();
  ref.onDispose(() => service.dispose());
  return service;
});

/// Provider for connection status stream
final daemonStatusProvider = StreamProvider<Map<String, dynamic>>((ref) {
  final service = ref.watch(daemonServiceProvider);
  return service.messages;
});
