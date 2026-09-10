import 'dart:async';
import 'package:flutter/foundation.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:connectivity_plus/connectivity_plus.dart';

import 'daemon_bridge.dart';
import '../models/vpn_state.dart';

class VpnService {
  final DaemonBridge _daemonBridge;
  final VpnStateNotifier _stateNotifier;

  static const int _maxReconnectAttempts = 10;
  static const Duration _baseReconnectDelay = Duration(seconds: 2);
  static const Duration _maxReconnectDelay = Duration(minutes: 5);

  int _reconnectAttempts = 0;
  Timer? _reconnectTimer;
  StreamSubscription? _connectivitySubscription;
  bool _isManualDisconnect = false;
  String? _currentCoreId;

  VpnService(this._daemonBridge, this._stateNotifier);

  Future<bool> startVpn() async {
    final hasPermission = await _requestVpnPermission();
    if (!hasPermission) {
      throw VpnServiceException('VPN permission denied');
    }

    try {
      final prefs = await SharedPreferences.getInstance();
      _currentCoreId = prefs.getString('active_core') ?? 'warp';

      await _daemonBridge.startDaemon(coreId: _currentCoreId!);

      final killSwitchEnabled = prefs.getBool('kill_switch') ?? true;
      if (killSwitchEnabled) {
        await _daemonBridge.setKillSwitch(true);
      }

      final excludeIranIps = prefs.getBool('split_tunnel_iran') ?? true;
      await _daemonBridge.configureSplitTunneling(
        excludedApps: [],
        excludeIranianIps: excludeIranIps,
      );

      _monitorConnectivity();
      _reconnectAttempts = 0;

      return true;
    } catch (e) {
      throw VpnServiceException('Failed to start VPN: $e');
    }
  }

  Future<void> stopVpn() async {
    _isManualDisconnect = true;
    _reconnectTimer?.cancel();
    _connectivitySubscription?.cancel();
    _reconnectAttempts = 0;

    try {
      await _daemonBridge.stopDaemon();
    } catch (e) {
      throw VpnServiceException('Failed to stop VPN: $e');
    } finally {
      _isManualDisconnect = false;
    }
  }

  Future<bool> _requestVpnPermission() async {
    if (defaultTargetPlatform == TargetPlatform.android) {
      final status = await Permission.notification.request();
      if (!status.isGranted) {
        return false;
      }
      try {
        final result = await _daemonBridge._channel.invokeMethod<bool>('requestVpnPermission');
        return result ?? false;
      } catch (_) {
        return false;
      }
    } else if (defaultTargetPlatform == TargetPlatform.iOS) {
      return true;
    }
    return false;
  }

  void _monitorConnectivity() {
    _connectivitySubscription?.cancel();
    _connectivitySubscription = Connectivity().onConnectivityChanged.listen(
      (List<ConnectivityResult> results) {
        final hasConnection = results.any((r) => r != ConnectivityResult.none);
        if (!hasConnection) {
          _stateNotifier.state = _stateNotifier.state.copyWith(
            connectionState: ConnectionState.disconnected,
          );
          if (!_isManualDisconnect) {
            _scheduleReconnect();
          }
        }
      },
    );
  }

  void _scheduleReconnect() {
    if (_isManualDisconnect) return;
    if (_reconnectAttempts >= _maxReconnectAttempts) {
      _stateNotifier.state = _stateNotifier.state.copyWith(
        connectionState: ConnectionState.error,
        errorMessage: 'Max reconnection attempts reached. Please connect manually.',
      );
      return;
    }

    _reconnectTimer?.cancel();

    final delay = Duration(
      milliseconds: (_baseReconnectDelay.inMilliseconds *
          (1 << _reconnectAttempts)).clamp(
        _baseReconnectDelay.inMilliseconds,
        _maxReconnectDelay.inMilliseconds,
      ),
    );

    _reconnectAttempts++;

    _reconnectTimer = Timer(delay, () async {
      try {
        _stateNotifier.state = _stateNotifier.state.copyWith(
          connectionState: ConnectionState.connecting,
        );
        await startVpn();
      } catch (_) {
        _scheduleReconnect();
      }
    });
  }

  Future<void> switchCore(String coreId) async {
    final wasConnected = _stateNotifier.state.connectionState == ConnectionState.connected;

    try {
      await _daemonBridge.switchCore(coreId);
      _currentCoreId = coreId;

      final prefs = await SharedPreferences.getInstance();
      await prefs.setString('active_core', coreId);

      _stateNotifier.state = _stateNotifier.state.copyWith(
        activeCore: coreId,
      );
    } catch (e) {
      if (wasConnected) {
        _stateNotifier.state = _stateNotifier.state.copyWith(
          connectionState: ConnectionState.error,
          errorMessage: 'Core switch failed: $e',
        );
        _scheduleReconnect();
      }
      rethrow;
    }
  }

  Future<void> enableKillSwitch() async {
    try {
      await _daemonBridge.setKillSwitch(true);
      _stateNotifier.updateKillSwitch(true);
    } catch (e) {
      throw VpnServiceException('Failed to enable kill switch: $e');
    }
  }

  Future<void> disableKillSwitch() async {
    try {
      await _daemonBridge.setKillSwitch(false);
      _stateNotifier.updateKillSwitch(false);
    } catch (e) {
      throw VpnServiceException('Failed to disable kill switch: $e');
    }
  }

  Future<void> configureSplitTunneling({
    List<String>? excludedApps,
    bool? excludeIranianIps,
  }) async {
    try {
      await _daemonBridge.configureSplitTunneling(
        excludedApps: excludedApps ?? [],
        excludeIranianIps: excludeIranianIps ?? true,
      );
    } catch (e) {
      throw VpnServiceException('Failed to configure split tunneling: $e');
    }
  }

  Future<void> setObfuscationMode(String mode) async {
    try {
      await _daemonBridge.triggerObfuscationMode(mode);
    } catch (e) {
      throw VpnServiceException('Failed to set obfuscation mode: $e');
    }
  }

  void dispose() {
    _reconnectTimer?.cancel();
    _connectivitySubscription?.cancel();
  }
}

class VpnServiceException implements Exception {
  final String message;
  VpnServiceException(this.message);

  @override
  String toString() => 'VpnServiceException: $message';
}
