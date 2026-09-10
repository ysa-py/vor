import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:crypto/crypto.dart' as crypto;

class OtaUpdater {
  static const String _githubRepo = 'unifiedshield/unifiedshield-nextgen';
  static const String _releasesUrl = 'https://api.github.com/repos/$_githubRepo/releases/latest';
  static const Duration _pollInterval = Duration(hours: 6);

  final Dio _dio = Dio(BaseOptions(
    connectTimeout: const Duration(seconds: 30),
    receiveTimeout: const Duration(minutes: 10),
    headers: {
      'Accept': 'application/vnd.github.v3+json',
      'User-Agent': 'UnifiedShield-OTA',
    },
  ));

  Future<Map<String, dynamic>?> checkForUpdate() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final currentVersion = prefs.getString('app_version') ?? '1.0.0';
      final otaEnabled = prefs.getBool('ota_enabled') ?? true;

      if (!otaEnabled) return null;

      final response = await _dio.get(_releasesUrl);
      if (response.statusCode != 200) return null;

      final data = response.data as Map<String, dynamic>;
      final latestVersion = (data['tag_name'] as String?)?.replaceFirst('v', '') ?? '';
      if (_compareVersions(latestVersion, currentVersion) <= 0) return null;

      final assets = data['assets'] as List<dynamic>? ?? [];
      String? downloadUrl;
      String? sha256Url;

      final platformSuffix = defaultTargetPlatform == TargetPlatform.android
          ? 'arm64.apk'
          : 'ios.ipa';

      for (final asset in assets) {
        final name = (asset['name'] as String?).toLowerCase();
        final url = asset['browser_download_url'] as String?;
        if (name != null && name.contains(platformSuffix) && url != null) {
          downloadUrl = url;
        }
        if (name != null && name.contains('sha256') && url != null) {
          sha256Url = url;
        }
      }

      if (downloadUrl == null) {
        for (final asset in assets) {
          final name = asset['name'] as String?;
          final url = asset['browser_download_url'] as String?;
          if (name != null && name.endsWith('.apk') && url != null && downloadUrl == null) {
            downloadUrl = url;
          }
        }
      }

      return {
        'version': latestVersion,
        'download_url': downloadUrl,
        'sha256_url': sha256Url,
        'release_notes': data['body'] ?? '',
        'published_at': data['published_at'] ?? '',
        'size': data['size'],
      };
    } on DioException catch (e) {
      debugPrint('OTA check failed: ${e.message}');
      return null;
    }
  }

  Future<String> downloadUpdate(String downloadUrl, void Function(double)? onProgress) async {
    try {
      final dir = await getTemporaryDirectory();
      final filePath = '${dir.path}/unifiedshield_update.apk';

      await _dio.download(
        downloadUrl,
        filePath,
        onReceiveProgress: (received, total) {
          if (total > 0 && onProgress != null) {
            onProgress(received / total);
          }
        },
      );

      return filePath;
    } on DioException catch (e) {
      throw OtaException('Download failed: ${e.message}');
    }
  }

  Future<bool> verifySha256(String filePath, String? sha256Url) async {
    try {
      final file = File(filePath);
      if (!await file.exists()) return false;

      final bytes = await file.readAsBytes();
      final actualHash = crypto.sha256.convert(bytes).toString();

      if (sha256Url != null) {
        try {
          final response = await _dio.get(sha256Url);
          final expectedHash = (response.data as String).trim().split(' ').first.toLowerCase();
          return actualHash.toLowerCase() == expectedHash;
        } catch (_) {
          debugPrint('Could not fetch SHA256 file, skipping verification');
          return true;
        }
      }

      final prefs = await SharedPreferences.getInstance();
      final expectedHash = prefs.getString('pending_update_sha256');
      if (expectedHash != null) {
        return actualHash.toLowerCase() == expectedHash.toLowerCase();
      }

      return true;
    } catch (e) {
      debugPrint('SHA256 verification error: $e');
      return false;
    }
  }

  Future<bool> installUpdate(String filePath) async {
    try {
      if (defaultTargetPlatform == TargetPlatform.android) {
        const channel = MethodChannel('com.unifiedshield/ota');
        final result = await channel.invokeMethod<bool>('installUpdate', {
          'file_path': filePath,
        });
        return result ?? false;
      }
      return false;
    } on PlatformException catch (e) {
      throw OtaException('Install failed: ${e.message}');
    }
  }

  int _compareVersions(String v1, String v2) {
    final parts1 = v1.split('.').map((p) => int.tryParse(p) ?? 0).toList();
    final parts2 = v2.split('.').map((p) => int.tryParse(p) ?? 0).toList();

    for (var i = 0; i < parts1.length || i < parts2.length; i++) {
      final p1 = i < parts1.length ? parts1[i] : 0;
      final p2 = i < parts2.length ? parts2[i] : 0;
      if (p1 != p2) return p1.compareTo(p2);
    }
    return 0;
  }
}

class OtaException implements Exception {
  final String message;
  OtaException(this.message);

  @override
  String toString() => 'OtaException: $message';
}

// Stub for MethodChannel used in installUpdate
class MethodChannel {
  static const MethodChannel _channel = MethodChannel('com.unifiedshield/ota');
  final String name;
  const MethodChannel(this.name);

  Future<T?> invokeMethod<T>(String method, [dynamic arguments]) async {
    return null;
  }
}

class PlatformException implements Exception {
  final String? message;
  final String? code;
  PlatformException({this.message, this.code});
}
