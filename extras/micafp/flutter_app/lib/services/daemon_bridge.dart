import 'dart:async';
import 'dart:convert';
import 'package:flutter/services.dart';

class DaemonBridge {
  static const MethodChannel _channel = MethodChannel('com.unifiedshield/daemon');
  static const EventChannel _statusChannel = EventChannel('com.unifiedshield/status');

  final StreamController<Map<String, dynamic>> _statusController =
      StreamController<Map<String, dynamic>>.broadcast();
  StreamSubscription? _statusSubscription;
  bool _initialized = false;

  Stream<Map<String, dynamic>> get statusStream => _statusController.stream;

  void init() {
    if (_initialized) return;
    _initialized = true;

    _statusSubscription = _statusChannel.receiveBroadcastStream().listen(
      (dynamic event) {
        if (event is String) {
          try {
            _statusController.add(jsonDecode(event) as Map<String, dynamic>);
          } catch (_) {
            _statusController.add({'raw': event});
          }
        } else if (event is Map) {
          _statusController.add(Map<String, dynamic>.from(event));
        }
      },
      onError: (dynamic error) {
        _statusController.add({
          'error': true,
          'message': error.toString(),
        });
      },
    );
  }

  Future<void> startDaemon({
    required String coreId,
    String? obfuscationMode,
  }) async {
    init();
    try {
      await _channel.invokeMethod<void>('startDaemon', {
        'core_id': coreId,
        'obfuscation_mode': obfuscationMode ?? 'default',
      });
    } on PlatformException catch (e) {
      throw DaemonException(
        message: e.message ?? 'Failed to start daemon',
        code: e.code,
        details: e.details,
      );
    }
  }

  Future<void> stopDaemon() async {
    try {
      await _channel.invokeMethod<void>('stopDaemon');
    } on PlatformException catch (e) {
      throw DaemonException(
        message: e.message ?? 'Failed to stop daemon',
        code: e.code,
        details: e.details,
      );
    }
  }

  Future<Map<String, dynamic>> getStatus() async {
    try {
      final result = await _channel.invokeMethod<Map>('getStatus');
      if (result == null) {
        return {'status': 'unknown'};
      }
      return Map<String, dynamic>.from(result);
    } on PlatformException catch (e) {
      throw DaemonException(
        message: e.message ?? 'Failed to get status',
        code: e.code,
        details: e.details,
      );
    }
  }

  Future<void> switchCore(String coreId) async {
    try {
      await _channel.invokeMethod<void>('switchCore', {
        'core_id': coreId,
      });
    } on PlatformException catch (e) {
      throw DaemonException(
        message: e.message ?? 'Failed to switch core',
        code: e.code,
        details: e.details,
      );
    }
  }

  Future<void> updateReward({
    required String peerId,
    required int bytesRelayed,
  }) async {
    try {
      await _channel.invokeMethod<void>('updateReward', {
        'peer_id': peerId,
        'bytes_relayed': bytesRelayed,
      });
    } on PlatformException catch (e) {
      throw DaemonException(
        message: e.message ?? 'Failed to update reward',
        code: e.code,
        details: e.details,
      );
    }
  }

  Future<void> setKillSwitch(bool enabled) async {
    try {
      await _channel.invokeMethod<void>('setKillSwitch', {
        'enabled': enabled,
      });
    } on PlatformException catch (e) {
      throw DaemonException(
        message: e.message ?? 'Failed to set kill switch',
        code: e.code,
        details: e.details,
      );
    }
  }

  Future<void> triggerObfuscationMode(String mode) async {
    try {
      await _channel.invokeMethod<void>('triggerObfuscationMode', {
        'mode': mode,
      });
    } on PlatformException catch (e) {
      throw DaemonException(
        message: e.message ?? 'Failed to trigger obfuscation mode',
        code: e.code,
        details: e.details,
      );
    }
  }

  Future<void> configureSplitTunneling({
    required List<String> excludedApps,
    required bool excludeIranianIps,
  }) async {
    try {
      await _channel.invokeMethod<void>('configureSplitTunneling', {
        'excluded_apps': excludedApps,
        'exclude_iranian_ips': excludeIranianIps,
      });
    } on PlatformException catch (e) {
      throw DaemonException(
        message: e.message ?? 'Failed to configure split tunneling',
        code: e.code,
        details: e.details,
      );
    }
  }

  Future<List<Map<String, dynamic>>> getAvailableCores() async {
    try {
      final result = await _channel.invokeMethod<List>('getAvailableCores');
      if (result == null) return [];
      return result.map((e) => Map<String, dynamic>.from(e as Map)).toList();
    } on PlatformException catch (e) {
      throw DaemonException(
        message: e.message ?? 'Failed to get available cores',
        code: e.code,
        details: e.details,
      );
    }
  }

  Future<void> reportIsp(String ispName, String? asn) async {
    try {
      await _channel.invokeMethod<void>('reportIsp', {
        'isp_name': ispName,
        'asn': asn ?? '',
      });
    } on PlatformException catch (e) {
      throw DaemonException(
        message: e.message ?? 'Failed to report ISP',
        code: e.code,
        details: e.details,
      );
    }
  }

  void dispose() {
    _statusSubscription?.cancel();
    _statusController.close();
  }
}

class DaemonException implements Exception {
  final String message;
  final String? code;
  final dynamic details;

  DaemonException({
    required this.message,
    this.code,
    this.details,
  });

  @override
  String toString() => 'DaemonException($code): $message';
}
