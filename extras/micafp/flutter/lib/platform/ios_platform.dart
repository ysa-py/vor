import 'dart:async';
import 'dart:io';

import 'package:flutter/services.dart';

import 'android_platform.dart';

/// iOS platform-specific code
///
/// Handles:
/// - NetworkExtension VPN management
/// - Keychain access for device secret
/// - Background task scheduling
/// - AVAudioSession management for acoustic channel
class IOSPlatform implements ShieldPlatform {
  static const MethodChannel _channel = MethodChannel('com.shield/ios');

  // NetworkExtension channel
  static const MethodChannel _neChannel =
      MethodChannel('com.shield/network_extension');

  @override
  bool get isSupported => Platform.isIOS;

  @override
  String get platformName => 'ios';

  /// Check if NetworkExtension VPN configuration exists
  Future<bool> hasVpnConfiguration() async {
    try {
      final result = await _neChannel.invokeMethod<bool>('hasConfiguration');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Create or update the VPN configuration
  /// On iOS, this requires a NetworkExtension app extension
  /// with packet tunnel provider
  Future<bool> createVpnConfiguration({
    required String serverAddress,
    required int port,
    required String transport,
    required String secret,
  }) async {
    try {
      final result = await _neChannel.invokeMethod<bool>('createConfiguration', {
        'serverAddress': serverAddress,
        'port': port,
        'transport': transport,
        'secret': secret,
      });
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Start the VPN tunnel
  Future<bool> startVpnTunnel() async {
    try {
      final result = await _neChannel.invokeMethod<bool>('startTunnel');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Stop the VPN tunnel
  Future<bool> stopVpnTunnel() async {
    try {
      final result = await _neChannel.invokeMethod<bool>('stopTunnel');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Get VPN connection status
  Future<String> getVpnStatus() async {
    try {
      final result = await _neChannel.invokeMethod<String>('getStatus');
      return result ?? 'disconnected';
    } on PlatformException {
      return 'disconnected';
    }
  }

  /// Remove VPN configuration
  Future<bool> removeVpnConfiguration() async {
    try {
      final result = await _neChannel.invokeMethod<bool>('removeConfiguration');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  // ========== Keychain ==========

  /// Store device secret in iOS Keychain
  Future<bool> storeDeviceSecret(String secret) async {
    try {
      final result = await _channel.invokeMethod<bool>('storeDeviceSecret', {
        'secret': secret,
      });
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Retrieve device secret from iOS Keychain
  Future<String?> getDeviceSecret() async {
    try {
      final result = await _channel.invokeMethod<String>('getDeviceSecret');
      return result;
    } on PlatformException {
      return null;
    }
  }

  /// Delete device secret from Keychain (for wipe)
  Future<bool> deleteDeviceSecret() async {
    try {
      final result = await _channel.invokeMethod<bool>('deleteDeviceSecret');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  // ========== Background Tasks ==========

  /// Schedule a background task for connectivity check
  /// iOS BGTaskScheduler with minimum 15-minute intervals
  Future<bool> scheduleBackgroundTask({
    required String taskId,
    required Duration interval,
  }) async {
    try {
      final result = await _channel.invokeMethod<bool>('scheduleBackgroundTask', {
        'taskId': taskId,
        'intervalSeconds': interval.inSeconds,
      });
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Cancel a scheduled background task
  Future<bool> cancelBackgroundTask(String taskId) async {
    try {
      final result = await _channel.invokeMethod<bool>('cancelBackgroundTask', {
        'taskId': taskId,
      });
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  // ========== Audio Session ==========

  /// Configure AVAudioSession for acoustic channel
  /// Sets category to .playAndRecord with .defaultToSpeaker
  Future<bool> configureAudioSession() async {
    try {
      final result =
          await _channel.invokeMethod<bool>('configureAudioSession');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Activate audio session for acoustic transmission
  Future<bool> activateAudioSession() async {
    try {
      final result =
          await _channel.invokeMethod<bool>('activateAudioSession');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Deactivate audio session
  Future<bool> deactivateAudioSession() async {
    try {
      final result =
          await _channel.invokeMethod<bool>('deactivateAudioSession');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Check if acoustic channel is available
  /// (microphone permission + speaker available)
  Future<bool> isAcousticChannelAvailable() async {
    try {
      final result =
          await _channel.invokeMethod<bool>('isAcousticChannelAvailable');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  // ========== App Lifecycle ==========

  /// Request notification authorization
  Future<bool> requestNotificationAuthorization() async {
    try {
      final result = await _channel
          .invokeMethod<bool>('requestNotificationAuthorization');
      return result ?? false;
    } on PlatformException {
      return false;
    }
  }

  /// Perform anti-forensics wipe specific to iOS
  /// Deletes Keychain items, app group containers, and VPN config
  Future<void> performWipe() async {
    await deleteDeviceSecret();
    await removeVpnConfiguration();

    try {
      await _channel.invokeMethod<void>('performWipe');
    } on PlatformException {
      // Partial wipe
    }
  }
}
