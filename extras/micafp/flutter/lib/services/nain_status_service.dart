import 'dart:async';

import 'package:flutter/foundation.dart';

import 'daemon_bridge.dart';
import 'battery_service.dart';

/// NAIN (National Internet) status monitoring service
///
/// Listens to NAIN status updates from the Rust daemon.
/// When a CompleteBlackout is detected:
///   - Shows warning to user
///   - Automatically activates mesh channels
///   - Requests all battery exemptions
///   - Temporarily boosts to Performance power mode
///
/// When FullInternet is restored:
///   - Deactivates mesh channels to save battery
///   - Returns to normal power mode
class NainStatusService extends ChangeNotifier {
  final DaemonBridge _daemonBridge;
  final BatteryService _batteryService;

  NainStatus _currentStatus = NainStatus.fullInternet;
  bool _meshChannelsActive = false;
  bool _hasShownBlackoutWarning = false;
  StreamSubscription? _statusSubscription;

  NainStatusService(this._daemonBridge, this._batteryService) {
    _init();
  }

  NainStatus get currentStatus => _currentStatus;
  bool get meshChannelsActive => _meshChannelsActive;

  void _init() {
    // Listen for status updates from daemon
    _statusSubscription = _daemonBridge.statusStream.listen(_handleStatusUpdate);

    // Get initial status
    _currentStatus = _daemonBridge.nainStatus;
  }

  void _handleStatusUpdate(StatusResponse status) {
    final newStatus = status.nainStatus;
    if (newStatus == _currentStatus) return;

    final previousStatus = _currentStatus;
    _currentStatus = newStatus;

    debugPrint('NAIN status changed: ${previousStatus.name} -> ${newStatus.name}');

    switch (newStatus) {
      case NainStatus.completeBlackout:
        _handleBlackout();
        break;
      case NainStatus.nationalIntranet:
        _handleNationalIntranet();
        break;
      case NainStatus.fullInternet:
        _handleFullInternet();
        break;
    }

    notifyListeners();
  }

  /// Handle CompleteBlackout — emergency mode
  void _handleBlackout() {
    // 1. Show warning to user
    if (!_hasShownBlackoutWarning) {
      _hasShownBlackoutWarning = true;
      _showBlackoutWarning();
    }

    // 2. Activate mesh channels (WiFi Aware, Bluetooth, acoustic)
    _activateMeshChannels();

    // 3. Request battery optimization exemptions
    _requestBatteryExemptions();

    // 4. Boost to Performance power mode
    _batteryService.setPowerMode(PowerMode.performance);

    // 5. Notify daemon to activate mesh relay mode
    _daemonBridge.sendConfigUpdate(
      'mesh_mode',
      'active',
    );
  }

  /// Handle National Intranet — limited connectivity
  void _handleNationalIntranet() {
    _hasShownBlackoutWarning = false;

    // Activate mesh channels preemptively
    _activateMeshChannels();

    // Set to normal power mode
    _batteryService.setPowerMode(PowerMode.normal);

    // Notify daemon
    _daemonBridge.sendConfigUpdate(
      'mesh_mode',
      'standby',
    );
  }

  /// Handle Full Internet — normal operation
  void _handleFullInternet() {
    _hasShownBlackoutWarning = false;

    // Deactivate mesh channels to save battery
    _deactivateMeshChannels();

    // Return to normal power mode (will auto-adjust based on battery)
    _batteryService.setPowerMode(PowerMode.normal);

    // Notify daemon
    _daemonBridge.sendConfigUpdate(
      'mesh_mode',
      'inactive',
    );
  }

  /// Activate all mesh communication channels
  void _activateMeshChannels() {
    if (_meshChannelsActive) return;

    _meshChannelsActive = true;
    debugPrint('Activating mesh channels');

    // Notify daemon to start WiFi Aware, Bluetooth, and acoustic channels
    _daemonBridge.sendConfigUpdate('wifi_aware', 'active');
    _daemonBridge.sendConfigUpdate('bluetooth_mesh', 'active');
    _daemonBridge.sendConfigUpdate('acoustic_channel', 'standby');
  }

  /// Deactivate mesh channels to conserve battery
  void _deactivateMeshChannels() {
    if (!_meshChannelsActive) return;

    _meshChannelsActive = false;
    debugPrint('Deactivating mesh channels');

    // Notify daemon to stop mesh channels
    _daemonBridge.sendConfigUpdate('wifi_aware', 'inactive');
    _daemonBridge.sendConfigUpdate('bluetooth_mesh', 'inactive');
    _daemonBridge.sendConfigUpdate('acoustic_channel', 'inactive');
  }

  /// Request all battery optimization exemptions
  void _requestBatteryExemptions() {
    // Request foreground service exemption
    _batteryService.requestOptimizationExemption();

    // Request wake lock exemption (for mesh relay)
    _daemonBridge.sendConfigUpdate('wake_lock', 'requested');

    // Request background data exemption
    _daemonBridge.sendConfigUpdate('background_data', 'unrestricted');
  }

  /// Show blackout warning to user
  void _showBlackoutWarning() {
    // This is surfaced through the UI via the NAIN status indicator
    // The HomeScreen shows the warning state based on currentStatus
    // A more prominent dialog can be triggered here if needed
    debugPrint('BLACKOUT WARNING: Internet connectivity severely disrupted');
  }

  /// Update NAIN status (called by external listeners)
  void updateStatus(NainStatus status) {
    if (status == _currentStatus) return;

    _currentStatus = status;

    switch (status) {
      case NainStatus.completeBlackout:
        _handleBlackout();
        break;
      case NainStatus.nationalIntranet:
        _handleNationalIntranet();
        break;
      case NainStatus.fullInternet:
        _handleFullInternet();
        break;
    }

    notifyListeners();
  }

  @override
  void dispose() {
    _statusSubscription?.cancel();
    super.dispose();
  }
}
