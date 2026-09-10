import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/foundation.dart';

/// IPC Bridge to the Rust daemon
///
/// Communicates via:
/// - Unix domain socket on Android/Linux/macOS
/// - Named pipe on Windows
///
/// JSON message protocol matching Rust IPC schema.
class DaemonBridge extends ChangeNotifier {
  Socket? _socket;
  StreamSubscription<Uint8List>? _subscription;
  Timer? _reconnectTimer;
  Timer? _heartbeatTimer;
  bool _isConnecting = false;
  String _buffer = '';

  // Connection state
  ConnectionStatus _connectionStatus = ConnectionStatus.disconnected;
  int _bytesUploaded = 0;
  int _bytesDownloaded = 0;
  NainStatus _nainStatus = NainStatus.fullInternet;
  String _currentTransport = 'none';
  int _batteryLevel = 100;
  bool _isCharging = false;

  // Stream controllers for status updates
  final _statusController = StreamController<StatusResponse>.broadcast();

  ConnectionStatus get connectionStatus => _connectionStatus;
  int get bytesUploaded => _bytesUploaded;
  int get bytesDownloaded => _bytesDownloaded;
  NainStatus get nainStatus => _nainStatus;
  String get currentTransport => _currentTransport;
  int get batteryLevel => _batteryLevel;
  bool get isCharging => _isCharging;

  Stream<StatusResponse> get statusStream => _statusController.stream;

  /// Initialize the IPC connection
  Future<void> initialize() async {
    await _connect();
    _startHeartbeat();
  }

  /// Get the socket path based on platform
  String _getSocketPath() {
    if (Platform.isWindows) {
      return r'\\.\pipe\shield-daemon';
    } else if (Platform.isAndroid) {
      // Android: use abstract namespace
      return '\0shield-daemon';
    } else if (Platform.isIOS) {
      // iOS: use app group container
      return '/var/mobile/Containers/Data/Application/shield-daemon.sock';
    } else if (Platform.isLinux) {
      return '/run/shield-daemon.sock';
    } else if (Platform.isMacOS) {
      return '/tmp/shield-daemon.sock';
    }
    return '/tmp/shield-daemon.sock';
  }

  /// Connect to the Rust daemon
  Future<void> _connect() async {
    if (_isConnecting || _socket != null) return;
    _isConnecting = true;

    try {
      final socketPath = _getSocketPath();

      if (Platform.isWindows) {
        // Windows named pipe - use separate implementation
        // For now, fall through to TCP loopback
        _socket = await Socket.connect('127.0.0.1', 9527);
      } else if (Platform.isAndroid && socketPath.startsWith('\0')) {
        // Android abstract namespace - connect via TCP loopback
        // The Rust daemon listens on a local TCP port
        _socket = await Socket.connect('127.0.0.1', 9527);
      } else {
        // Unix domain socket
        _socket = await Socket.connect(
          InternetAddress(socketPath, type: InternetAddressType.unix),
          0,
        );
      }

      _isConnecting = false;

      // Listen for incoming messages
      _subscription = _socket!.listen(
        _handleData,
        onError: _handleError,
        onDone: _handleDone,
      );

      // Query initial status
      await queryStatus();
    } catch (e) {
      _isConnecting = false;
      _scheduleReconnect();
    }
  }

  /// Handle incoming data from daemon
  void _handleData(Uint8List data) {
    _buffer += utf8.decode(data, allowMalformed: true);

    // Messages are newline-delimited JSON
    while (_buffer.contains('\n')) {
      final index = _buffer.indexOf('\n');
      final message = _buffer.substring(0, index).trim();
      _buffer = _buffer.substring(index + 1);

      if (message.isNotEmpty) {
        _processMessage(message);
      }
    }
  }

  /// Process a single JSON message from daemon
  void _processMessage(String message) {
    try {
      final json = jsonDecode(message) as Map<String, dynamic>;
      final type = json['type'] as String?;

      switch (type) {
        case 'StatusResponse':
          _handleStatusResponse(json);
          break;
        case 'ErrorResponse':
          _handleErrorResponse(json);
          break;
        case 'ConnectionStateChanged':
          _handleConnectionStateChanged(json);
          break;
        case 'NainStatusChanged':
          _handleNainStatusChanged(json);
          break;
        case 'DataUsageUpdate':
          _handleDataUsageUpdate(json);
          break;
        case 'AcousticConfig':
          // Forward to caller via stream
          _statusController.add(StatusResponse.fromJson(json));
          break;
      }
    } catch (e) {
      // Malformed message — ignore
    }
  }

  void _handleStatusResponse(Map<String, dynamic> json) {
    final status = StatusResponse.fromJson(json);
    _statusController.add(status);
    _updateFromStatus(status);
  }

  void _handleErrorResponse(Map<String, dynamic> json) {
    final error = json['error'] as String? ?? 'Unknown error';
    debugPrint('Daemon error: $error');
  }

  void _handleConnectionStateChanged(Map<String, dynamic> json) {
    final state = json['state'] as String? ?? 'disconnected';
    switch (state) {
      case 'connected':
        _connectionStatus = ConnectionStatus.connected;
        break;
      case 'connecting':
        _connectionStatus = ConnectionStatus.connecting;
        break;
      default:
        _connectionStatus = ConnectionStatus.disconnected;
    }
    notifyListeners();
  }

  void _handleNainStatusChanged(Map<String, dynamic> json) {
    final status = json['status'] as String? ?? 'full_internet';
    switch (status) {
      case 'full_internet':
        _nainStatus = NainStatus.fullInternet;
        break;
      case 'national_intranet':
        _nainStatus = NainStatus.nationalIntranet;
        break;
      case 'complete_blackout':
        _nainStatus = NainStatus.completeBlackout;
        break;
    }
    notifyListeners();
  }

  void _handleDataUsageUpdate(Map<String, dynamic> json) {
    _bytesUploaded = json['uploaded'] as int? ?? _bytesUploaded;
    _bytesDownloaded = json['downloaded'] as int? ?? _bytesDownloaded;
    notifyListeners();
  }

  void _updateFromStatus(StatusResponse status) {
    _connectionStatus = status.isConnected
        ? ConnectionStatus.connected
        : ConnectionStatus.disconnected;
    _bytesUploaded = status.bytesUploaded;
    _bytesDownloaded = status.bytesDownloaded;
    _nainStatus = status.nainStatus;
    _currentTransport = status.transport;
    _batteryLevel = status.batteryLevel;
    _isCharging = status.isCharging;
    notifyListeners();
  }

  void _handleError(dynamic error) {
    debugPrint('Daemon socket error: $error');
    _cleanup();
    _scheduleReconnect();
  }

  void _handleDone() {
    debugPrint('Daemon socket closed');
    _cleanup();
    _scheduleReconnect();
  }

  void _cleanup() {
    _subscription?.cancel();
    _subscription = null;
    _socket?.destroy();
    _socket = null;
    _connectionStatus = ConnectionStatus.disconnected;
    notifyListeners();
  }

  /// Schedule reconnection attempt
  void _scheduleReconnect() {
    _reconnectTimer?.cancel();
    _reconnectTimer = Timer(const Duration(seconds: 3), () {
      _connect();
    });
  }

  /// Send heartbeat to daemon every 30 seconds
  void _startHeartbeat() {
    _heartbeatTimer?.cancel();
    _heartbeatTimer = Timer.periodic(const Duration(seconds: 30), (_) {
      if (_socket != null) {
        _sendMessage({'type': 'Heartbeat'});
      }
    });
  }

  /// Send a JSON message to the daemon
  void _sendMessage(Map<String, dynamic> message) {
    if (_socket == null) return;
    try {
      final data = utf8.encode('${jsonEncode(message)}\n');
      _socket!.add(data);
    } catch (e) {
      debugPrint('Failed to send message: $e');
    }
  }

  // ========== Public API ==========

  /// Request connection to transport
  Future<void> sendConnect() async {
    _sendMessage({
      'type': 'Connect',
      'timestamp': DateTime.now().millisecondsSinceEpoch,
    });
    _connectionStatus = ConnectionStatus.connecting;
    notifyListeners();
  }

  /// Request disconnection
  Future<void> sendDisconnect() async {
    _sendMessage({
      'type': 'Disconnect',
      'timestamp': DateTime.now().millisecondsSinceEpoch,
    });
  }

  /// Query current status from daemon
  Future<StatusResponse> queryStatus() async {
    _sendMessage({'type': 'StatusQuery'});

    // Wait for response with timeout
    final completer = Completer<StatusResponse>();
    final subscription = statusStream.listen((status) {
      if (!completer.isCompleted) {
        completer.complete(status);
      }
    });

    return completer.future.timeout(
      const Duration(seconds: 5),
      onTimeout: () {
        subscription.cancel();
        return StatusResponse(
          isConnected: false,
          bytesUploaded: _bytesUploaded,
          bytesDownloaded: _bytesDownloaded,
          nainStatus: _nainStatus,
          transport: _currentTransport,
          batteryLevel: _batteryLevel,
          isCharging: _isCharging,
          powerMode: 'normal',
        );
      },
    );
  }

  /// Send configuration update
  Future<void> sendConfigUpdate(String key, String value) async {
    _sendMessage({
      'type': 'ConfigUpdate',
      'key': key,
      'value': value,
      'timestamp': DateTime.now().millisecondsSinceEpoch,
    });
  }

  /// Send wipe trigger
  Future<void> sendWipeTrigger(String triggerType) async {
    _sendMessage({
      'type': 'WipeTrigger',
      'trigger': triggerType,
      'timestamp': DateTime.now().millisecondsSinceEpoch,
    });
  }

  /// Request acoustic config for sharing
  Future<String> requestAcousticConfig() async {
    _sendMessage({'type': 'AcousticConfigRequest'});

    final completer = Completer<String>();
    final subscription = statusStream.listen((status) {
      if (!completer.isCompleted && status.acousticConfig != null) {
        completer.complete(status.acousticConfig!);
      }
    });

    return completer.future.timeout(
      const Duration(seconds: 10),
      onTimeout: () {
        subscription.cancel();
        throw TimeoutException('Acoustic config request timed out');
      },
    );
  }

  /// Send acoustic chirp to nearby device
  Future<void> sendAcousticChirp(String config) async {
    _sendMessage({
      'type': 'AcousticChirp',
      'config': config,
      'timestamp': DateTime.now().millisecondsSinceEpoch,
    });
  }

  /// Report battery state to daemon
  void reportBatteryState(int level, bool isCharging, String powerMode) {
    _sendMessage({
      'type': 'BatteryStateUpdate',
      'level': level,
      'is_charging': isCharging,
      'power_mode': powerMode,
      'timestamp': DateTime.now().millisecondsSinceEpoch,
    });
  }

  /// Report screen state to daemon
  void reportScreenState(bool isScreenOn) {
    _sendMessage({
      'type': 'ScreenStateUpdate',
      'is_screen_on': isScreenOn,
      'timestamp': DateTime.now().millisecondsSinceEpoch,
    });
  }

  @override
  void dispose() {
    _reconnectTimer?.cancel();
    _heartbeatTimer?.cancel();
    _subscription?.cancel();
    _socket?.destroy();
    _statusController.close();
    super.dispose();
  }
}

/// Connection status enum
enum ConnectionStatus {
  connected,
  disconnected,
  connecting,
}

/// NAIN (National Internet) status
enum NainStatus {
  fullInternet,
  nationalIntranet,
  completeBlackout,
}

/// Status response from daemon
class StatusResponse {
  final bool isConnected;
  final int bytesUploaded;
  final int bytesDownloaded;
  final NainStatus nainStatus;
  final String transport;
  final int batteryLevel;
  final bool isCharging;
  final String powerMode;
  final String? acousticConfig;

  StatusResponse({
    required this.isConnected,
    required this.bytesUploaded,
    required this.bytesDownloaded,
    required this.nainStatus,
    required this.transport,
    required this.batteryLevel,
    required this.isCharging,
    required this.powerMode,
    this.acousticConfig,
  });

  factory StatusResponse.fromJson(Map<String, dynamic> json) {
    return StatusResponse(
      isConnected: json['is_connected'] as bool? ?? false,
      bytesUploaded: json['bytes_uploaded'] as int? ?? 0,
      bytesDownloaded: json['bytes_downloaded'] as int? ?? 0,
      nainStatus: _parseNainStatus(json['nain_status'] as String?),
      transport: json['transport'] as String? ?? 'none',
      batteryLevel: json['battery_level'] as int? ?? 100,
      isCharging: json['is_charging'] as bool? ?? false,
      powerMode: json['power_mode'] as String? ?? 'normal',
      acousticConfig: json['acoustic_config'] as String?,
    );
  }

  static NainStatus _parseNainStatus(String? status) {
    switch (status) {
      case 'full_internet':
        return NainStatus.fullInternet;
      case 'national_intranet':
        return NainStatus.nationalIntranet;
      case 'complete_blackout':
        return NainStatus.completeBlackout;
      default:
        return NainStatus.fullInternet;
    }
  }
}
