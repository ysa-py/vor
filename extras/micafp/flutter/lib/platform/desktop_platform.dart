import 'dart:async';
import 'dart:io';

import 'package:flutter/services.dart';

import 'android_platform.dart';

/// Desktop platform code (Windows/Linux)
///
/// Handles:
/// - System tray integration
/// - Autostart configuration
/// - TUN device management
/// - Battery monitoring via platform APIs
class DesktopPlatform implements ShieldPlatform {
  static const MethodChannel _channel = MethodChannel('com.shield/desktop');

  @override
  bool get isSupported => Platform.isWindows || Platform.isLinux;

  @override
  String get platformName => Platform.isWindows ? 'windows' : 'linux';

  // ========== System Tray ==========

  /// Initialize system tray icon
  Future<bool> initSystemTray({
    required String iconPath,
    required String tooltip,
  }) async {
    try {
      final result = await _channel.invokeMethod<bool>('initSystemTray', {
        'iconPath': iconPath,
        'tooltip': tooltip,
      });
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Update system tray icon
  Future<bool> updateSystemTrayIcon(String iconPath) async {
    try {
      final result =
          await _channel.invokeMethod<bool>('updateSystemTrayIcon', {
        'iconPath': iconPath,
      });
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Update system tray tooltip
  Future<bool> updateSystemTrayTooltip(String tooltip) async {
    try {
      final result =
          await _channel.invokeMethod<bool>('updateSystemTrayTooltip', {
        'tooltip': tooltip,
      });
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Set system tray context menu
  Future<bool> setSystemTrayMenu(List<TrayMenuItem> items) async {
    try {
      final result = await _channel.invokeMethod<bool>('setSystemTrayMenu', {
        'items': items.map((item) => item.toJson()).toList(),
      });
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Show system tray notification
  Future<bool> showTrayNotification({
    required String title,
    required String body,
  }) async {
    try {
      final result =
          await _channel.invokeMethod<bool>('showTrayNotification', {
        'title': title,
        'body': body,
      });
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  // ========== Autostart ==========

  /// Configure application to start on boot
  Future<bool> enableAutostart() async {
    try {
      final result = await _channel.invokeMethod<bool>('enableAutostart');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Remove application from autostart
  Future<bool> disableAutostart() async {
    try {
      final result = await _channel.invokeMethod<bool>('disableAutostart');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Check if autostart is enabled
  Future<bool> isAutostartEnabled() async {
    try {
      final result = await _channel.invokeMethod<bool>('isAutostartEnabled');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  // ========== TUN Device ==========

  /// Create TUN interface
  /// On Linux: creates /dev/net/tun interface
  /// On Windows: creates Wintun adapter
  Future<bool> createTunDevice({
    required String name,
    required String address,
    required int mtu,
  }) async {
    try {
      final result = await _channel.invokeMethod<bool>('createTunDevice', {
        'name': name,
        'address': address,
        'mtu': mtu,
      });
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Configure TUN device routes
  Future<bool> configureTunRoutes({
    required List<String> routes,
    required String gateway,
  }) async {
    try {
      final result = await _channel.invokeMethod<bool>('configureTunRoutes', {
        'routes': routes,
        'gateway': gateway,
      });
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Set DNS servers for TUN device
  Future<bool> setTunDns({
    required List<String> servers,
  }) async {
    try {
      final result = await _channel.invokeMethod<bool>('setTunDns', {
        'servers': servers,
      });
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Destroy TUN device
  Future<bool> destroyTunDevice() async {
    try {
      final result = await _channel.invokeMethod<bool>('destroyTunDevice');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Check if TUN device exists
  Future<bool> isTunDeviceActive() async {
    try {
      final result = await _channel.invokeMethod<bool>('isTunDeviceActive');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  // ========== Battery Monitoring ==========

  /// Get battery level (desktop)
  /// Linux: reads from /sys/class/power_supply or UPower
  /// Windows: uses Win32 GetSystemPowerStatus
  Future<int> getBatteryLevel() async {
    try {
      final result = await _channel.invokeMethod<int>('getBatteryLevel');
      return result ?? 100;
    } on PlatformException {
      return 100;
    }
  }

  /// Check if device is charging (desktop)
  Future<bool> isCharging() async {
    try {
      final result = await _channel.invokeMethod<bool>('isCharging');
      return result ?? true;
    } on PlatformException {
      return true;
    }
  }

  /// Check if device has battery (desktops may not)
  Future<bool> hasBattery() async {
    try {
      final result = await _channel.invokeMethod<bool>('hasBattery');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  // ========== Kill Switch (Desktop Firewall) ==========

  /// Enable kill switch via platform firewall
  /// Linux: iptables / nftables rules
  /// Windows: Windows Firewall API
  Future<bool> enableKillSwitch({
    required String tunInterface,
    required String localInterface,
  }) async {
    try {
      final result = await _channel.invokeMethod<bool>('enableKillSwitch', {
        'tunInterface': tunInterface,
        'localInterface': localInterface,
      });
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Disable kill switch
  Future<bool> disableKillSwitch() async {
    try {
      final result = await _channel.invokeMethod<bool>('disableKillSwitch');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }
}

/// System tray menu item
class TrayMenuItem {
  final String id;
  final String label;
  final bool enabled;
  final bool separator;

  const TrayMenuItem({
    required this.id,
    required this.label,
    this.enabled = true,
    this.separator = false,
  });

  Map<String, dynamic> toJson() => {
        'id': id,
        'label': label,
        'enabled': enabled,
        'separator': separator,
      };
}
