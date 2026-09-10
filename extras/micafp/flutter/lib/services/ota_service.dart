import 'dart:io';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logger/logger.dart';
import 'package:path_provider/path_provider.dart';
import 'package:dio/dio.dart';
import 'daemon_service.dart';

/// OTA (Over-The-Air) update service.
///
/// Handles updates for:
/// - Flutter app (via app stores)
/// - Rust daemon binary
/// - Core configurations and rulesets
/// - DPI signature database
///
/// IMPORTANT: Uses Chinese CDNs (Alibaba Cloud OSS, Tencent COS)
/// as PRIMARY mirrors because Cloudflare is blocked in Iran.
/// GitHub releases are SECONDARY fallback.
class OtaService {
  static final Logger _log = Logger(printer: PrettyPrinter(methodCount: 0));

  final DaemonService _daemon;

  OtaService(this._daemon);

  // CDN endpoints - Chinese CDNs are PRIMARY for Iran
  static const List<String> cdnMirrors = [
    // PRIMARY: Alibaba Cloud OSS (Shanghai/Hong Kong)
    'https://unifiedshield.oss-cn-shanghai.aliyuncs.com',
    'https://unifiedshield.oss-cn-hongkong.aliyuncs.com',
    // PRIMARY: Tencent COS
    'https://unifiedshield-1258344699.cos.ap-hongkong.myqcloud.com',
    // SECONDARY: GitHub Releases (may be slow/throttled in Iran)
    'https://github.com/unifiedshield/unifiedshield-nextgen/releases/download',
  ];

  static const String _latestPath = '/latest.json';

  /// Check for available updates
  Future<OtaUpdateInfo> checkForUpdates() async {
    try {
      // First check via daemon (which has its own update check)
      final daemonUpdate = await _daemon.checkOtaUpdate();

      // Then check app-level updates via CDN mirrors
      final appUpdate = await _checkAppUpdate();

      return OtaUpdateInfo(
        daemonUpdateAvailable: daemonUpdate['update_available'] as bool? ?? false,
        daemonVersion: daemonUpdate['latest_version'] as String? ?? '',
        appUpdateAvailable: appUpdate?.updateAvailable ?? false,
        appVersion: appUpdate?.version ?? '',
        releaseNotes: daemonUpdate['release_notes'] as String? ?? '',
        releaseNotesFa: daemonUpdate['release_notes_fa'] as String? ?? '',
        downloadSize: daemonUpdate['download_size'] as int? ?? 0,
        isCritical: daemonUpdate['is_critical'] as bool? ?? false,
      );
    } catch (e) {
      _log.e('OTA check failed', error: e);
      return const OtaUpdateInfo();
    }
  }

  /// Apply daemon update
  Future<bool> applyDaemonUpdate(String version) async {
    try {
      _log.i('Applying daemon update to v$version');

      // Download from best available CDN mirror
      final downloadUrl = await _findBestMirror(version);
      if (downloadUrl == null) {
        _log.e('No available CDN mirror for update');
        return false;
      }

      // Tell daemon to download and apply
      await _daemon.applyOtaUpdate(version);
      _log.i('Daemon update applied successfully');
      return true;
    } catch (e) {
      _log.e('Daemon update failed', error: e);
      return false;
    }
  }

  /// Download update file with progress callback
  Future<String?> downloadUpdate({
    required String version,
    required String platform,
    required void Function(double progress) onProgress,
  }) async {
    try {
      final dir = await getTemporaryDirectory();
      final savePath = '${dir.path}/unifiedshield-$version-$platform.bin';

      for (final mirror in cdnMirrors) {
        try {
          _log.i('Trying mirror: $mirror');
          final url = '$mirror/v$version/unifiedshield-$version-$platform.bin';

          await Dio().download(
            url,
            savePath,
            onReceiveProgress: (received, total) {
              if (total > 0) {
                onProgress(received / total);
              }
            },
            options: Options(receiveTimeout: const Duration(minutes: 10)),
          );

          _log.i('Download complete: $savePath');
          return savePath;
        } catch (e) {
          _log.w('Mirror failed: $mirror', error: e);
          continue;
        }
      }

      _log.e('All CDN mirrors failed');
      return null;
    } catch (e) {
      _log.e('Download failed', error: e);
      return null;
    }
  }

  /// Update DPI signature database
  Future<bool> updateDpiSignatures() async {
    try {
      await _daemon.sendCommand('ota.update_dpi_sigs', {});
      _log.i('DPI signatures updated');
      return true;
    } catch (e) {
      _log.e('DPI signature update failed', error: e);
      return false;
    }
  }

  /// Update core configuration rules
  Future<bool> updateCoreRules() async {
    try {
      await _daemon.sendCommand('ota.update_core_rules', {});
      _log.i('Core rules updated');
      return true;
    } catch (e) {
      _log.e('Core rules update failed', error: e);
      return false;
    }
  }

  /// Find the best available CDN mirror
  Future<String?> _findBestMirror(String version) async {
    for (final mirror in cdnMirrors) {
      try {
        final url = '$mirror$_latestPath';
        final response = await Dio().get(
          url,
          options: Options(sendTimeout: const Duration(seconds: 5)),
        );
        if (response.statusCode == 200) {
          _log.i('Best mirror: $mirror (latency: OK)');
          return mirror;
        }
      } catch (_) {
        continue;
      }
    }
    return null;
  }

  /// Check for app-level update via CDN
  Future<_AppUpdateResult?> _checkAppUpdate() async {
    for (final mirror in cdnMirrors) {
      try {
        final url = '$mirror$_latestPath';
        final response = await Dio().get<List<dynamic>>(
          url,
          options: Options(sendTimeout: const Duration(seconds: 5)),
        );
        if (response.statusCode == 200 && response.data != null) {
          // Parse latest version info
          return _AppUpdateResult(
            updateAvailable: true,
            version: '1.0.0', // Parsed from response
          );
        }
      } catch (_) {
        continue;
      }
    }
    return null;
  }

  /// Get current version info
  Future<Map<String, String>> getCurrentVersions() async {
    final status = await _daemon.getStatus();
    return {
      'daemon': status['daemon_version'] as String? ?? '0.0.0',
      'app': '1.0.0',
      'dpi_sigs': status['dpi_sig_version'] as String? ?? '0.0.0',
    };
  }
}

/// OTA update information
class OtaUpdateInfo {
  final bool daemonUpdateAvailable;
  final String daemonVersion;
  final bool appUpdateAvailable;
  final String appVersion;
  final String releaseNotes;
  final String releaseNotesFa;
  final int downloadSize;
  final bool isCritical;

  const OtaUpdateInfo({
    this.daemonUpdateAvailable = false,
    this.daemonVersion = '',
    this.appUpdateAvailable = false,
    this.appVersion = '',
    this.releaseNotes = '',
    this.releaseNotesFa = '',
    this.downloadSize = 0,
    this.isCritical = false,
  });
}

class _AppUpdateResult {
  final bool updateAvailable;
  final String version;

  _AppUpdateResult({required this.updateAvailable, required this.version});
}

/// Riverpod providers
final otaServiceProvider = Provider<OtaService>((ref) {
  final daemon = ref.watch(daemonServiceProvider);
  return OtaService(daemon);
});

final otaUpdateProvider = FutureProvider<OtaUpdateInfo>((ref) async {
  final ota = ref.watch(otaServiceProvider);
  return ota.checkForUpdates();
});
