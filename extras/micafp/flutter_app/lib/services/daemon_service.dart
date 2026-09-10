import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

/// Connection state of the daemon.
enum ConnectionState {
  disconnected,
  connecting,
  connected,
  error,
}

/// DaemonService — Rust daemon communication service.
///
/// Provides IPC with the native Rust daemon via MethodChannel.
/// Handles connection lifecycle, status queries, battery state
/// communication, anti-forensics triggers, acoustic config sharing,
/// iOS SMS fallback, and event monitoring via EventChannel.
class DaemonService extends ChangeNotifier {
  // MethodChannel for IPC with native Rust daemon
  static const MethodChannel _channel = MethodChannel('com.shield.daemon');

  // EventChannel for streaming events from the daemon
  static const EventChannel _eventChannel = EventChannel('com.shield.daemon/events');

  // State
  ConnectionState _state = ConnectionState.disconnected;
  String _errorMessage = '';
  int _bytesIn = 0;
  int _bytesOut = 0;
  String _currentTransport = '';
  DateTime? _connectedAt;
  StreamSubscription? _eventSubscription;
  Timer? _statusPollTimer;

  // Getters
  bool get isConnected => _state == ConnectionState.connected;
  bool get isConnecting =>
      _state == ConnectionState.connecting;
  ConnectionState get state => _state;
  String get errorMessage => _errorMessage;
  int get bytesIn => _bytesIn;
  int get bytesOut => _bytesOut;
  String get currentTransport => _currentTransport;
  Duration? get connectionDuration => _connectedAt != null
      ? DateTime.now().difference(_connectedAt!)
      : null;

  /// Initialize the daemon service.
  /// Sets up method call handlers and starts event monitoring.
  Future<void> initialize() async {
    // Set up method call handler for calls FROM native
    _channel.setMethodCallHandler(_handleMethodCall);

    // Start listening to daemon events
    _startEventMonitoring();

    // Get initial status
    try {
      final status = await _channel.invokeMethod<Map>('getStatus');
      if (status != null) {
        _updateStateFromStatus(status);
      }
    } on PlatformException catch (e) {
      debugPrint('DaemonService: Failed to get initial status: ${e.message}');
    }

    // Start periodic status polling
    _statusPollTimer = Timer.periodic(
      const Duration(seconds: 5),
      (_) => _pollStatus(),
    );

    debugPrint('DaemonService: Initialized');
  }

  /// Connect to the shield tunnel.
  Future<void> connect() async {
    if (_state == ConnectionState.connecting || _state == ConnectionState.connected) {
      return;
    }

    _setState(ConnectionState.connecting);

    try {
      await _channel.invokeMethod<void>('connect');
      _connectedAt = DateTime.now();
      _setState(ConnectionState.connected);
      debugPrint('DaemonService: Connected');
    } on PlatformException catch (e) {
      _errorMessage = e.message ?? 'Connection failed';
      _setState(ConnectionState.error);
      debugPrint('DaemonService: Connect failed: $_errorMessage');

      // Auto-retry after delay
      Future.delayed(const Duration(seconds: 3), () {
        if (_state == ConnectionState.error) {
          _setState(ConnectionState.disconnected);
        }
      });
    }
  }

  /// Disconnect from the shield tunnel.
  Future<void> disconnect() async {
    try {
      await _channel.invokeMethod<void>('disconnect');
      _connectedAt = null;
      _bytesIn = 0;
      _bytesOut = 0;
      _currentTransport = '';
      _setState(ConnectionState.disconnected);
      debugPrint('DaemonService: Disconnected');
    } on PlatformException catch (e) {
      _errorMessage = e.message ?? 'Disconnect failed';
      _setState(ConnectionState.error);
      debugPrint('DaemonService: Disconnect failed: $_errorMessage');
    }
  }

  /// Get current connection status from the daemon.
  Future<Map<String, dynamic>?> getStatus() async {
    try {
      final status = await _channel.invokeMethod<Map>('getStatus');
      return status?.cast<String, dynamic>();
    } on PlatformException catch (e) {
      debugPrint('DaemonService: getStatus failed: ${e.message}');
      return null;
    }
  }

  /// Get battery status info from the daemon.
  Future<Map<String, dynamic>?> getBatteryStatus() async {
    try {
      final status = await _channel.invokeMethod<Map>('getBatteryStatus');
      return status?.cast<String, dynamic>();
    } on PlatformException catch (e) {
      debugPrint('DaemonService: getBatteryStatus failed: ${e.message}');
      return null;
    }
  }

  /// Trigger anti-forensics emergency wipe.
  Future<void> triggerAntiForensics() async {
    try {
      await _channel.invokeMethod<void>('triggerAntiForensics');
      _connectedAt = null;
      _bytesIn = 0;
      _bytesOut = 0;
      _currentTransport = '';
      _setState(ConnectionState.disconnected);
      debugPrint('DaemonService: Anti-forensics triggered');
    } on PlatformException catch (e) {
      debugPrint('DaemonService: Anti-forensics failed: ${e.message}');
    }
  }

  /// Share configuration via acoustic channel.
  Future<bool> shareConfig() async {
    try {
      final result = await _channel.invokeMethod<bool>('shareConfig');
      return result ?? false;
    } on PlatformException catch (e) {
      debugPrint('DaemonService: shareConfig failed: ${e.message}');
      return false;
    }
  }

  /// Paste a config code (iOS SMS fallback).
  Future<bool> pasteConfigCode(String code) async {
    try {
      final result = await _channel.invokeMethod<bool>('pasteConfigCode', {
        'code': code,
      });
      return result ?? false;
    } on PlatformException catch (e) {
      debugPrint('DaemonService: pasteConfigCode failed: ${e.message}');
      return false;
    }
  }

  /// Get the event stream from the daemon.
  Stream<List<int>> get eventStream {
    return _eventChannel.receiveBroadcastStream().map((event) {
      if (event is List) {
        return event.cast<int>();
      }
      return <int>[];
    });
  }

  // ---------------------------------------------------------------
  // Private Methods
  // ---------------------------------------------------------------

  /// Handle method calls FROM the native side.
  Future<dynamic> _handleMethodCall(MethodCall call) async {
    switch (call.method) {
      case 'onConnectionStateChanged':
        final state = call.arguments as String?;
        _handleConnectionStateChange(state);
        return null;

      case 'onTrafficUpdate':
        final args = call.arguments as Map?;
        if (args != null) {
          _bytesIn = (args['bytesIn'] as int?) ?? _bytesIn;
          _bytesOut = (args['bytesOut'] as int?) ?? _bytesOut;
          notifyListeners();
        }
        return null;

      case 'onTransportChanged':
        _currentTransport = (call.arguments as String?) ?? '';
        notifyListeners();
        return null;

      case 'onError':
        _errorMessage = (call.arguments as String?) ?? 'Unknown error';
        _setState(ConnectionState.error);
        return null;

      default:
        debugPrint('DaemonService: Unknown method call: ${call.method}');
        return null;
    }
  }

  /// Start monitoring daemon events via EventChannel.
  void _startEventMonitoring() {
    _eventSubscription = _eventChannel
        .receiveBroadcastStream()
        .listen(
      (event) {
        if (event is Map) {
          _handleEvent(event.cast<String, dynamic>());
        }
      },
      onError: (error) {
        debugPrint('DaemonService: Event stream error: $error');
        // Reconnect event stream after delay
        Future.delayed(const Duration(seconds: 5), () {
          _startEventMonitoring();
        });
      },
      cancelOnError: true,
    );
  }

  /// Handle a daemon event.
  void _handleEvent(Map<String, dynamic> event) {
    final type = event['type'] as String? ?? '';

    switch (type) {
      case 'connection_state':
        final state = event['state'] as String?;
        _handleConnectionStateChange(state);
        break;

      case 'traffic':
        _bytesIn = (event['bytesIn'] as int?) ?? _bytesIn;
        _bytesOut = (event['bytesOut'] as int?) ?? _bytesOut;
        notifyListeners();
        break;

      case 'transport':
        _currentTransport = (event['name'] as String?) ?? '';
        notifyListeners();
        break;

      case 'error':
        _errorMessage = (event['message'] as String?) ?? 'Unknown error';
        _setState(ConnectionState.error);
        break;

      case 'peer_discovered':
        debugPrint('DaemonService: Peer discovered via NAN');
        break;

      case 'endpoint_update':
        debugPrint('DaemonService: Endpoints updated');
        break;
    }
  }

  /// Handle connection state change from native.
  void _handleConnectionStateChange(String? state) {
    switch (state) {
      case 'connected':
        _connectedAt ??= DateTime.now();
        _setState(ConnectionState.connected);
        break;
      case 'connecting':
        _setState(ConnectionState.connecting);
        break;
      case 'disconnected':
        _connectedAt = null;
        _setState(ConnectionState.disconnected);
        break;
      case 'error':
        _setState(ConnectionState.error);
        break;
      default:
        debugPrint('DaemonService: Unknown connection state: $state');
    }
  }

  /// Update state from a status map.
  void _updateStateFromStatus(Map status) {
    final connected = status['connected'] as bool? ?? false;
    final connecting = status['connecting'] as bool? ?? false;
    _currentTransport = status['transport'] as String? ?? '';
    _bytesIn = (status['bytesIn'] as int?) ?? 0;
    _bytesOut = (status['bytesOut'] as int?) ?? 0;

    if (connected) {
      _connectedAt ??= DateTime.now();
      _setState(ConnectionState.connected);
    } else if (connecting) {
      _setState(ConnectionState.connecting);
    } else {
      _connectedAt = null;
      _setState(ConnectionState.disconnected);
    }
  }

  /// Poll the daemon for current status.
  Future<void> _pollStatus() async {
    try {
      final status = await getStatus();
      if (status != null) {
        _updateStateFromStatus(status);
      }
    } catch (e) {
      // Silently handle poll failures
      debugPrint('DaemonService: Status poll failed: $e');
    }
  }

  /// Set the connection state and notify listeners.
  void _setState(ConnectionState newState) {
    if (_state != newState) {
      _state = newState;
      notifyListeners();
    }
  }

  @override
  void dispose() {
    _eventSubscription?.cancel();
    _statusPollTimer?.cancel();
    super.dispose();
  }
}
