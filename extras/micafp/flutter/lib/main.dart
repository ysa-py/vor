import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_foreground_task/flutter_foreground_task.dart';
import 'package:provider/provider.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:workmanager/workmanager.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import 'screens/home_screen.dart';
import 'screens/lock_screen.dart';
import 'services/daemon_bridge.dart';
import 'services/battery_service.dart';
import 'services/nain_status_service.dart';
import 'platform/android_platform.dart';
import 'platform/ios_platform.dart';
import 'platform/desktop_platform.dart';

/// Background task callback for WorkManager
@pragma('vm:entry-point')
void callbackDispatcher() {
  Workmanager().executeTask((task, inputData) async {
    switch (task) {
      case 'connectivityCheck':
        // Attempt to verify daemon is alive and connected
        final bridge = DaemonBridge();
        try {
          final status = await bridge.queryStatus();
          if (!status.isConnected) {
            // Daemon reports disconnected — attempt reconnection
            await bridge.sendConnect();
          }
        } catch (_) {
          // Daemon not reachable, try to start it
          await bridge.sendConnect();
        }
        break;
      case 'batteryOptimization':
        // Verify battery optimization exemption
        if (Platform.isAndroid) {
          final android = AndroidPlatform();
          final isExempt = await android.isBatteryOptimizationExempt();
          if (!isExempt) {
            // Schedule a notification to remind the user
            // (handled via foreground task notification)
          }
        }
        break;
    }
    return true;
  });
}

/// Foreground task callback for Android foreground service
@pragma('vm:entry-point')
void startCallback() {
  // The foreground task keeps the service alive while connected
  FlutterForegroundTask.setTaskHandler(
    ShieldForegroundTaskHandler(),
  );
}

class ShieldForegroundTaskHandler extends TaskHandler {
  @override
  Future<void> onStart(DateTime timestamp, TaskStarter starter) async {
    // Service started — no additional action needed
    // Daemon manages the actual connection
  }

  @override
  Future<void> onEvent(DateTime timestamp) async {
    // Periodic heartbeat — update notification content
    final bridge = DaemonBridge();
    try {
      final status = await bridge.queryStatus();
      FlutterForegroundTask.updateNotification(
        text: status.isConnected ? 'Active' : 'Disconnected',
      );
    } catch (_) {
      FlutterForegroundTask.updateNotification(
        text: 'Reconnecting...',
      );
    }
  }

  @override
  Future<void> onDestroy(DateTime timestamp, bool isTimeout) async {
    // Service stopped — clean up
  }

  @override
  Future<void> onNotificationButtonPress(String id) async {
    if (id == 'disconnect') {
      final bridge = DaemonBridge();
      await bridge.sendDisconnect();
    }
  }

  @override
  Future<void> onNotificationPressed() async {
    // User tapped notification — open app
  }
}

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Initialize secure storage
  const secureStorage = FlutterSecureStorage();

  // Check if first launch — set up PIN
  final prefs = await SharedPreferences.getInstance();
  final hasPin = prefs.getBool('has_pin') ?? false;

  // Initialize WorkManager for background tasks
  if (Platform.isAndroid || Platform.isIOS) {
    await Workmanager().initialize(callbackDispatcher, isInDebugMode: false);
    // Periodic connectivity check every 15 minutes (minimum allowed)
    await Workmanager().registerPeriodicTask(
      'connectivityCheck',
      'connectivityCheck',
      frequency: const Duration(minutes: 15),
      constraints: Constraints(
        networkType: NetworkType.connected,
      ),
      existingWorkPolicy: ExistingWorkPolicy.keep,
    );
  }

  // Initialize daemon bridge
  final daemonBridge = DaemonBridge();
  await daemonBridge.initialize();

  // Initialize services
  final batteryService = BatteryService(daemonBridge);
  final nainStatusService = NainStatusService(daemonBridge, batteryService);

  // Detect platform
  final platform = _detectPlatform();

  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider<DaemonBridge>.value(value: daemonBridge),
        ChangeNotifierProvider<BatteryService>.value(value: batteryService),
        ChangeNotifierProvider<NainStatusService>.value(value: nainStatusService),
        Provider<ShieldPlatform>.value(value: platform),
      ],
      child: ShieldApp(hasPin: hasPin),
    ),
  );
}

/// Detect current platform and return appropriate platform handler
ShieldPlatform _detectPlatform() {
  if (Platform.isAndroid) {
    return AndroidPlatform();
  } else if (Platform.isIOS) {
    return IOSPlatform();
  } else {
    return DesktopPlatform();
  }
}

class ShieldApp extends StatelessWidget {
  final bool hasPin;

  const ShieldApp({super.key, required this.hasPin});

  @override
  Widget build(BuildContext context) {
    return WithForegroundTask(
      child: MaterialApp(
        title: 'Shield',
        debugShowCheckedModeBanner: false,
        theme: _buildDarkTheme(),
        home: hasPin ? const LockScreen() : const HomeScreen(),
      ),
    );
  }

  /// Privacy-friendly dark theme
  /// No identifying colors — uses neutral dark tones
  ThemeData _buildDarkTheme() {
    return ThemeData(
      brightness: Brightness.dark,
      useMaterial3: true,
      colorScheme: const ColorScheme.dark(
        primary: Color(0xFF2E7D32),         // Muted green for status
        onPrimary: Color(0xFFFFFFFF),
        secondary: Color(0xFF37474F),        // Blue-grey for secondary
        onSecondary: Color(0xFFFFFFFF),
        error: Color(0xFFC62828),            // Subtle red
        onError: Color(0xFFFFFFFF),
        surface: Color(0xFF1A1A2E),          // Deep navy-black
        onSurface: Color(0xFFE0E0E0),
        surfaceContainerHighest: Color(0xFF16213E),
      ),
      scaffoldBackgroundColor: const Color(0xFF0F0F1A),
      appBarTheme: const AppBarTheme(
        backgroundColor: Color(0xFF1A1A2E),
        elevation: 0,
        centerTitle: true,
        titleTextStyle: TextStyle(
          color: Color(0xFFE0E0E0),
          fontSize: 18,
          fontWeight: FontWeight.w600,
        ),
        iconTheme: IconThemeData(
          color: Color(0xFFE0E0E0),
        ),
      ),
      cardTheme: CardThemeData(
        color: const Color(0xFF1A1A2E),
        elevation: 2,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
        ),
      ),
      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          backgroundColor: const Color(0xFF2E7D32),
          foregroundColor: Colors.white,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
          ),
          padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 16),
        ),
      ),
      iconTheme: const IconThemeData(
        color: Color(0xFFE0E0E0),
      ),
      textTheme: const TextTheme(
        bodyLarge: TextStyle(color: Color(0xFFE0E0E0)),
        bodyMedium: TextStyle(color: Color(0xFFB0BEC5)),
        titleLarge: TextStyle(
          color: Color(0xFFE0E0E0),
          fontWeight: FontWeight.bold,
        ),
      ),
      // System UI overlay
      systemOverlayStyle: SystemUiOverlayStyle.light,
    );
  }
}
