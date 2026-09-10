import 'dart:async';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_foreground_task/flutter_foreground_task.dart';

/// Base class for platform-specific code
abstract class ShieldPlatform {
  /// Check if the platform supports the required features
  bool get isSupported;

  /// Get platform name
  String get platformName;
}

/// Android platform-specific code
///
/// Handles:
/// - MethodChannel for VpnService binding
/// - Foreground service management
/// - Battery optimization intent
/// - WiFi Aware availability check
/// - SMS permission request
/// - Quick Settings tile state
class AndroidPlatform implements ShieldPlatform {
  static const MethodChannel _channel = MethodChannel('com.shield/android');

  // VPN service method channel
  static const MethodChannel _vpnChannel =
      MethodChannel('com.shield/vpn_service');

  // Quick Settings tile channel
  static const MethodChannel _tileChannel =
      MethodChannel('com.shield/quick_tile');

  @override
  bool get isSupported => Platform.isAndroid;

  @override
  String get platformName => 'android';

  /// Prepare the VPN service (must be called before connecting)
  /// Returns true if user granted VPN permission
  Future<bool> prepareVpnService() async {
    try {
      final result = await _vpnChannel.invokeMethod<bool>('prepare');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Start the VPN service
  Future<bool> startVpnService() async {
    try {
      final result = await _vpnChannel.invokeMethod<bool>('start');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Stop the VPN service
  Future<bool> stopVpnService() async {
    try {
      final result = await _vpnChannel.invokeMethod<bool>('stop');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Check if VPN service is running
  Future<bool> isVpnServiceRunning() async {
    try {
      final result = await _vpnChannel.invokeMethod<bool>('isRunning');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Start Android foreground service to keep connection alive
  Future<void> startForegroundService() async {
    // Use flutter_foreground_task for foreground service
    await FlutterForegroundTask.saveData(key: 'isConnected', value: true);

    FlutterForegroundTask.startService(
      serviceId: 256,
      notificationTitle: 'Shield',
      notificationText: 'Active',
      notificationIcon: const NotificationIconData(
        resType: ResourceType.mipmap,
        resPrefix: ResourcePrefix.ic,
        name: 'launcher',
      ),
      notificationButtons: [
        const NotificationButton(id: 'disconnect', text: 'Stop'),
      ],
      callback: startCallback,
    );
  }

  /// Stop foreground service
  Future<void> stopForegroundService() async {
    await FlutterForegroundTask.saveData(key: 'isConnected', value: false);
    FlutterForegroundTask.stopService();
  }

  /// Check if battery optimization exemption is granted
  Future<bool> isBatteryOptimizationExempt() async {
    try {
      final result = await _channel.invokeMethod<bool>(
        'isBatteryOptimizationExempt',
      );
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Request battery optimization exemption
  Future<bool> requestBatteryOptimizationExemption() async {
    try {
      final result = await _channel.invokeMethod<bool>(
        'requestBatteryOptimization',
      );
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Open battery optimization settings
  Future<void> openBatteryOptimizationSettings() async {
    try {
      await _channel.invokeMethod<void>('openBatteryOptimizationSettings');
    } on PlatformException {
      // Fallback: open general settings
    }
  }

  /// Check if WiFi Aware (Wi-Fi Direct / NAN) is available
  Future<bool> isWifiAwareAvailable() async {
    try {
      final result = await _channel.invokeMethod<bool>(
        'isWifiAwareAvailable',
      );
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Request SMS permissions for auto-receiving config codes
  Future<bool> requestSmsPermission() async {
    try {
      final result = await _channel.invokeMethod<bool>('requestSmsPermission');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Check if SMS permission is granted
  Future<bool> hasSmsPermission() async {
    try {
      final result = await _channel.invokeMethod<bool>('hasSmsPermission');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Update Quick Settings tile state
  Future<void> updateQuickTileState(bool isConnected) async {
    try {
      await _tileChannel.invokeMethod<void>('updateState', {
        'connected': isConnected,
      });
    } on PlatformException {
      // Quick Settings tile not available
    }
  }

  /// Switch launcher icon alias (steganographic disguise)
  Future<void> switchLauncherIcon(String iconName) async {
    try {
      await _channel.invokeMethod<void>('switchLauncherIcon', {
        'iconName': iconName,
      });
    } on PlatformException {
      // Icon switching not supported
    }
  }

  /// Request all necessary permissions
  Future<Map<String, bool>> requestAllPermissions() async {
    final results = <String, bool>{};

    results['vpn'] = await prepareVpnService();
    results['battery'] = await requestBatteryOptimizationExemption();
    results['sms'] = await requestSmsPermission();

    return results;
  }

  /// Check all permission states
  Future<Map<String, bool>> checkAllPermissions() async {
    final results = <String, bool>{};

    results['vpn'] = await isVpnServiceRunning();
    results['battery'] = await isBatteryOptimizationExempt();
    results['sms'] = await hasSmsPermission();
    results['wifi_aware'] = await isWifiAwareAvailable();

    return results;
  }
}

/// Reference to foreground task callback (defined in main.dart)
/// This must be a top-level function for Android foreground service
void startCallback() {
  FlutterForegroundTask.setTaskHandler(
    _AndroidForegroundTaskHandler(),
  );
}

class _AndroidForegroundTaskHandler extends TaskHandler {
  @override
  Future<void> onStart(DateTime timestamp, TaskStarter starter) async {}

  @override
  Future<void> onEvent(DateTime timestamp) async {}

  @override
  Future<void> onDestroy(DateTime timestamp, bool isTimeout) async {}

  @override
  Future<void> onNotificationButtonPress(String id) async {}

  @override
  Future<void> onNotificationPressed() async {}
}
