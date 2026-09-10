import 'dart:async';

import 'package:battery_plus/battery_plus.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

/// Power state enum matching the native side definitions.
enum PowerState {
  screenOn(0, 'screen_on'),
  screenOffLight(1, 'screen_off_light'),
  screenOffDeep(2, 'screen_off_deep'),
  charging(3, 'charging');

  final int code;
  final String label;
  const PowerState(this.code, this.label);

  static PowerState fromCode(int code) {
    return PowerState.values.firstWhere(
      (s) => s.code == code,
      orElse: () => PowerState.screenOffLight,
    );
  }
}

/// BatteryService — Battery monitoring service.
///
/// Monitors battery level and charging state, communicates the current
/// power state to the Rust daemon, provides adaptive UI hints based on
/// battery state, warns when battery is critically low, and tracks
/// battery history for drain rate calculations.
class BatteryService extends ChangeNotifier {
  static const MethodChannel _channel = MethodChannel('com.shield.daemon');

  final Battery _battery = Battery();

  // Current state
  int _batteryLevel = 100;
  bool _isCharging = false;
  PowerState _powerState = PowerState.screenOn;
  bool _isLowPowerMode = false;
  bool _isScreenOn = true;

  // Battery history for drain rate calculation
  final List<_BatteryHistoryEntry> _history = [];
  static const int _maxHistoryEntries = 100;

  // Monitoring
  Timer? _monitorTimer;
  StreamSubscription<BatteryState>? _batteryStateSubscription;

  // Thresholds
  static const int _ultraLowThreshold = 15;
  static const int _lowThreshold = 25;

  // Getters
  int get batteryLevel => _batteryLevel;
  bool get isCharging => _isCharging;
  PowerState get powerState => _powerState;
  bool get isLowPowerMode => _isLowPowerMode;
  bool get isScreenOn => _isScreenOn;

  bool get isUltraLowPower => _batteryLevel <= _ultraLowThreshold && !_isCharging;
  bool get isLowPower => _batteryLevel <= _lowThreshold;
  bool get shouldReduceAnimations =>
      _powerState == PowerState.screenOffDeep ||
      _batteryLevel <= _ultraLowThreshold ||
      _isLowPowerMode;

  /// Start monitoring battery state.
  Future<void> startMonitoring() async {
    try {
      // Get initial battery state
      _batteryLevel = await _battery.batteryLevel;
      _isCharging = await _isCurrentlyCharging();

      // Listen to battery state changes
      _batteryStateSubscription = _battery.onBatteryStateChanged.listen(
        _handleBatteryStateChanged,
        onError: (error) {
          debugPrint('BatteryService: Battery state stream error: $error');
        },
      );

      // Start periodic monitoring (fallback for states that don't trigger events)
      _monitorTimer = Timer.periodic(
        const Duration(seconds: 30),
        (_) => refreshBatteryState(),
      );

      _recordHistory();
      _evaluatePowerState();

      debugPrint('BatteryService: Started monitoring (level: $_batteryLevel%, charging: $_isCharging)');
    } catch (e) {
      debugPrint('BatteryService: Failed to start monitoring: $e');
    }
  }

  /// Stop monitoring battery state.
  void stopMonitoring() {
    _batteryStateSubscription?.cancel();
    _batteryStateSubscription = null;
    _monitorTimer?.cancel();
    _monitorTimer = null;
    debugPrint('BatteryService: Stopped monitoring');
  }

  /// Manually refresh the battery state.
  Future<void> refreshBatteryState() async {
    try {
      final newLevel = await _battery.batteryLevel;
      final newCharging = await _isCurrentlyCharging();

      bool changed = false;

      if (newLevel != _batteryLevel) {
        _batteryLevel = newLevel;
        changed = true;
      }

      if (newCharging != _isCharging) {
        _isCharging = newCharging;
        changed = true;
      }

      if (changed) {
        _recordHistory();
        _evaluatePowerState();
        _notifyDaemon();
        notifyListeners();
      }
    } catch (e) {
      debugPrint('BatteryService: Failed to refresh state: $e');
    }
  }

  /// Set the screen on/off state (called from main app lifecycle observer).
  void setScreenOn(bool isOn) {
    if (_isScreenOn != isOn) {
      _isScreenOn = isOn;
      _evaluatePowerState();
      _notifyDaemon();
      notifyListeners();
    }
  }

  /// Set the low power mode state.
  void setLowPowerMode(bool enabled) {
    if (_isLowPowerMode != enabled) {
      _isLowPowerMode = enabled;
      _evaluatePowerState();
      _notifyDaemon();
      notifyListeners();
    }
  }

  // ---------------------------------------------------------------
  // Battery state change handler
  // ---------------------------------------------------------------

  void _handleBatteryStateChanged(BatteryState state) {
    final wasCharging = _isCharging;
    _isCharging = state == BatteryState.charging || state == BatteryState.full;

    if (wasCharging != _isCharging) {
      debugPrint('BatteryService: Charging state changed: $_isCharging');
      _evaluatePowerState();
      _notifyDaemon();
      notifyListeners();
    }
  }

  Future<bool> _isCurrentlyCharging() async {
    try {
      final state = await _battery.batteryState;
      return state == BatteryState.charging || state == BatteryState.full;
    } catch (_) {
      return false;
    }
  }

  // ---------------------------------------------------------------
  // Power state evaluation
  // ---------------------------------------------------------------

  void _evaluatePowerState() {
    final previousState = _powerState;

    if (_isCharging) {
      _powerState = PowerState.charging;
    } else if (_isScreenOn && !_isLowPowerMode) {
      if (_batteryLevel <= _ultraLowThreshold) {
        _powerState = PowerState.screenOffDeep;
      } else {
        _powerState = PowerState.screenOn;
      }
    } else if (_isLowPowerMode || _batteryLevel <= _lowThreshold) {
      if (_batteryLevel <= _ultraLowThreshold) {
        _powerState = PowerState.screenOffDeep;
      } else {
        _powerState = PowerState.screenOffLight;
      }
    } else {
      _powerState = PowerState.screenOffLight;
    }

    if (_powerState != previousState) {
      debugPrint('BatteryService: Power state changed: ${previousState.label} → ${_powerState.label} ' +
          '(battery: $_batteryLevel%, charging: $_isCharging)');

      // Check threshold crossings
      if (_batteryLevel <= _ultraLowThreshold) {
        debugPrint('BatteryService: ⚠️ Ultra-low battery threshold crossed!');
      } else if (_batteryLevel <= _lowThreshold) {
        debugPrint('BatteryService: ⚠️ Low battery threshold crossed');
      }
    }
  }

  // ---------------------------------------------------------------
  // Rust daemon communication
  // ---------------------------------------------------------------

  void _notifyDaemon() {
    try {
      _channel.invokeMethod<void>('setPowerState', {
        'state': _powerState.code,
        'batteryLevel': _batteryLevel,
      });
    } on PlatformException catch (e) {
      debugPrint('BatteryService: Failed to notify daemon: ${e.message}');
    }
  }

  // ---------------------------------------------------------------
  // Battery history and drain rate
  // ---------------------------------------------------------------

  void _recordHistory() {
    _history.add(_BatteryHistoryEntry(
      timestamp: DateTime.now(),
      level: _batteryLevel,
      isCharging: _isCharging,
      powerState: _powerState,
    ));

    // Trim history to max size
    while (_history.length > _maxHistoryEntries) {
      _history.removeAt(0);
    }
  }

  /// Calculate the battery drain rate in percentage per hour.
  /// Returns null if there isn't enough history data.
  double? get drainRatePerHour {
    if (_history.length < 2) return null;

    final recent = _history.sublist(
      _history.length - _history.length.clamp(0, 20),
    );

    if (recent.isEmpty) return null;

    final first = recent.first;
    final last = recent.last;

    final timeDiff = last.timestamp.difference(first.timestamp).inSeconds;
    if (timeDiff <= 0) return null;

    final levelDiff = first.level - last.level;
    if (levelDiff <= 0) return null; // Not draining

    final hoursDiff = timeDiff / 3600.0;
    return levelDiff / hoursDiff;
  }

  /// Estimate time remaining until battery reaches ultra-low threshold.
  /// Returns null if unable to estimate.
  Duration? get estimatedTimeUntilUltraLow {
    final rate = drainRatePerHour;
    if (rate == null || rate <= 0) return null;

    final remainingLevel = _batteryLevel - _ultraLowThreshold;
    if (remainingLevel <= 0) return Duration.zero;

    final hoursRemaining = remainingLevel / rate;
    return Duration(milliseconds: (hoursRemaining * 3600000).round());
  }

  /// Get the battery history entries for charting or analysis.
  List<BatteryHistoryPoint> get historyPoints {
    return _history
        .map((e) => BatteryHistoryPoint(
              timestamp: e.timestamp,
              level: e.level,
              isCharging: e.isCharging,
            ))
        .toList();
  }

  /// Get a summary string of the current battery state.
  String get stateSummary {
    return 'BatteryService: level=$_batteryLevel%, charging=$_isCharging, ' +
        'state=${_powerState.label}, lowPower=$_isLowPowerMode, ' +
        'drainRate=${drainRatePerHour?.toStringAsFixed(1) ?? "unknown"}%/hr';
  }

  // ---------------------------------------------------------------
  // Adaptive feature recommendations
  // ---------------------------------------------------------------

  /// Whether the acoustic channel should be enabled.
  bool get shouldEnableAcousticChannel {
    switch (_powerState) {
      case PowerState.charging:
        return true;
      case PowerState.screenOn:
        return _batteryLevel > _ultraLowThreshold;
      case PowerState.screenOffLight:
      case PowerState.screenOffDeep:
        return false;
    }
  }

  /// Whether NAN (WiFi Aware) should be enabled.
  bool get shouldEnableNan {
    switch (_powerState) {
      case PowerState.charging:
      case PowerState.screenOn:
        return _batteryLevel > _ultraLowThreshold;
      case PowerState.screenOffLight:
        return _batteryLevel > _lowThreshold;
      case PowerState.screenOffDeep:
        return false;
    }
  }

  /// Recommended probe interval in seconds based on current power state.
  int get recommendedProbeIntervalSeconds {
    switch (_powerState) {
      case PowerState.charging:
        return 15;
      case PowerState.screenOn:
        return 30;
      case PowerState.screenOffLight:
        return 120;
      case PowerState.screenOffDeep:
        return 600;
    }
  }

  @override
  void dispose() {
    stopMonitoring();
    super.dispose();
  }
}

/// Internal battery history entry.
class _BatteryHistoryEntry {
  final DateTime timestamp;
  final int level;
  final bool isCharging;
  final PowerState powerState;

  _BatteryHistoryEntry({
    required this.timestamp,
    required this.level,
    required this.isCharging,
    required this.powerState,
  });
}

/// Public battery history data point for UI charting.
class BatteryHistoryPoint {
  final DateTime timestamp;
  final int level;
  final bool isCharging;

  BatteryHistoryPoint({
    required this.timestamp,
    required this.level,
    required this.isCharging,
  });
}
