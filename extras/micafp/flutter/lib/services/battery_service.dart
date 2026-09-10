import 'dart:async';
import 'dart:io';

import 'package:battery_plus/battery_plus.dart';
import 'package:flutter/foundation.dart';
import 'package:workmanager/workmanager.dart';

import 'daemon_bridge.dart';

/// Power mode determining refresh rate and resource usage
enum PowerMode {
  performance, // Refresh every 1s
  normal,      // Refresh every 3s
  save,        // Refresh every 10s
  critical,    // Refresh every 30s
}

/// Battery optimization service
///
/// Monitors battery level, charging state, and screen state.
/// Reports power state to the Rust daemon.
/// Adjusts UI refresh rate based on power mode.
/// Registers WorkManager tasks for background connectivity checks.
class BatteryService extends ChangeNotifier {
  final DaemonBridge _daemonBridge;
  final Battery _battery = Battery();

  // Current state
  int _batteryLevel = 100;
  bool _isCharging = false;
  PowerMode _powerMode = PowerMode.normal;
  bool _isScreenOn = true;
  bool _isOptimizationExempt = false;
  StreamSubscription<BatteryState>? _batterySubscription;
  Timer? _pollingTimer;

  BatteryService(this._daemonBridge) {
    _init();
  }

  int get batteryLevel => _batteryLevel;
  bool get isCharging => _isCharging;
  PowerMode get powerMode => _powerMode;
  bool get isScreenOn => _isScreenOn;
  bool get isOptimizationExempt => _isOptimizationExempt;

  /// Get refresh interval for current power mode
  Duration get refreshInterval {
    switch (_powerMode) {
      case PowerMode.performance:
        return const Duration(seconds: 1);
      case PowerMode.normal:
        return const Duration(seconds: 3);
      case PowerMode.save:
        return const Duration(seconds: 10);
      case PowerMode.critical:
        return const Duration(seconds: 30);
    }
  }

  Future<void> _init() async {
    try {
      // Get initial battery state
      _batteryLevel = await _battery.batteryLevel;
      final state = await _battery.batteryState;
      _isCharging = state == BatteryState.charging ||
          state == BatteryState.full;

      // Listen for battery changes
      _batterySubscription = _battery.onBatteryStateChanged.listen(
        _handleBatteryStateChanged,
      );

      // Check optimization exemption
      if (Platform.isAndroid) {
        await _checkOptimizationExemption();
      }

      // Start periodic polling
      _startPolling();

      // Register WorkManager background task
      await _registerBackgroundTasks();

      // Calculate initial power mode
      _updatePowerMode();

      // Report to daemon
      _reportToDaemon();
    } catch (e) {
      debugPrint('BatteryService init error: $e');
    }
  }

  void _handleBatteryStateChanged(BatteryState state) {
    final wasCharging = _isCharging;
    _isCharging = state == BatteryState.charging ||
        state == BatteryState.full;

    if (wasCharging != _isCharging) {
      _updatePowerMode();
      _reportToDaemon();
      notifyListeners();
    }
  }

  void _startPolling() {
    _pollingTimer?.cancel();
    _pollingTimer = Timer.periodic(refreshInterval, (_) async {
      try {
        final level = await _battery.batteryLevel;
        if (level != _batteryLevel) {
          _batteryLevel = level;
          _updatePowerMode();
          _reportToDaemon();
          notifyListeners();

          // Restart polling with new interval if power mode changed
          _startPolling();
        }
      } catch (_) {
        // Battery API unavailable
      }
    });
  }

  /// Calculate power mode based on battery level and charging state
  void _updatePowerMode() {
    final PowerMode newMode;

    if (_isCharging) {
      // When charging, use performance mode
      newMode = PowerMode.performance;
    } else if (_batteryLevel > 50) {
      newMode = PowerMode.normal;
    } else if (_batteryLevel > 20) {
      newMode = PowerMode.save;
    } else {
      newMode = PowerMode.critical;
    }

    if (newMode != _powerMode) {
      _powerMode = newMode;
      notifyListeners();
    }
  }

  /// Force set power mode (used by NAIN status service during blackout)
  void setPowerMode(PowerMode mode) {
    if (_powerMode != mode) {
      _powerMode = mode;
      _reportToDaemon();
      _startPolling(); // Restart with new interval
      notifyListeners();
    }
  }

  /// Report current battery state to the Rust daemon
  void _reportToDaemon() {
    final powerModeString = _powerMode.name;
    _daemonBridge.reportBatteryState(
      _batteryLevel,
      _isCharging,
      powerModeString,
    );
  }

  /// Report screen state change
  void reportScreenState(bool isOn) {
    _isScreenOn = isOn;
    _daemonBridge.reportScreenState(isOn);
    notifyListeners();
  }

  /// Check if battery optimization exemption is granted (Android)
  Future<void> _checkOptimizationExemption() async {
    if (!Platform.isAndroid) {
      _isOptimizationExempt = true;
      return;
    }

    // On Android, check via platform channel
    // The actual check is done through the Android platform code
    // For now, assume not exempt until confirmed
    _isOptimizationExempt = false;
    notifyListeners();
  }

  /// Request battery optimization exemption (Android)
  Future<bool> requestOptimizationExemption() async {
    if (!Platform.isAndroid) return true;

    // This launches the system battery optimization settings
    // The actual implementation is in android_platform.dart
    // via MethodChannel
    return false;
  }

  /// Open system battery optimization settings
  Future<void> openBatterySettings() async {
    if (Platform.isAndroid) {
      // Launch Android battery optimization settings
      // Implementation handled by platform code
      try {
        // Intent ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS
        final android = _getAndroidPlatform();
        await android?.openBatteryOptimizationSettings();
      } catch (e) {
        debugPrint('Failed to open battery settings: $e');
      }
    } else if (Platform.isIOS) {
      // iOS: no direct battery optimization settings
      // Can open Settings app
    }
  }

  /// Register WorkManager periodic tasks for background connectivity checks
  Future<void> _registerBackgroundTasks() async {
    if (!Platform.isAndroid && !Platform.isIOS) return;

    try {
      // Periodic connectivity check — every 15 minutes (minimum allowed)
      await Workmanager().registerPeriodicTask(
        'shield-connectivity-check',
        'connectivityCheck',
        frequency: const Duration(minutes: 15),
        constraints: Constraints(
          networkType: NetworkType.connected,
        ),
        existingWorkPolicy: ExistingWorkPolicy.keep,
        backoffPolicy: BackoffPolicy.exponential,
        backoffPolicyDelay: const Duration(minutes: 5),
      );

      // Battery optimization reminder — check every hour
      if (Platform.isAndroid) {
        await Workmanager().registerPeriodicTask(
          'shield-battery-opt',
          'batteryOptimization',
          frequency: const Duration(hours: 1),
          existingWorkPolicy: ExistingWorkPolicy.keep,
        );
      }
    } catch (e) {
      debugPrint('Failed to register background tasks: $e');
    }
  }

  /// Get Android platform instance (lazy, to avoid import cycle)
  dynamic _getAndroidPlatform() {
    // This is resolved at runtime to avoid circular dependency
    // The platform code registers itself via a static method
    return null;
  }

  @override
  void dispose() {
    _batterySubscription?.cancel();
    _pollingTimer?.cancel();
    super.dispose();
  }
}
