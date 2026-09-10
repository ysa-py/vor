/// Connection statistics model for real-time display.
class ConnectionStats {
  final double speedDown;        // bytes/sec
  final double speedUp;          // bytes/sec
  final double latency;          // ms
  final double jitter;           // ms
  final double packetLoss;       // percentage 0-100
  final int totalBytesDown;
  final int totalBytesUp;
  final int uptime;              // seconds
  final String activeCore;
  final String connectedServer;
  final String connectedCountry;
  final String protocol;
  final List<double> speedHistory; // Last 60 speed samples
  final List<double> latencyHistory; // Last 60 latency samples
  final DateTime timestamp;

  const ConnectionStats({
    this.speedDown = 0,
    this.speedUp = 0,
    this.latency = 0,
    this.jitter = 0,
    this.packetLoss = 0,
    this.totalBytesDown = 0,
    this.totalBytesUp = 0,
    this.uptime = 0,
    this.activeCore = '',
    this.connectedServer = '',
    this.connectedCountry = '',
    this.protocol = '',
    this.speedHistory = const [],
    this.latencyHistory = const [],
    DateTime? timestamp,
  }) : timestamp = timestamp ?? _now;

  static DateTime get _now => DateTime.now();

  /// Formatted download speed
  String get speedDownFormatted => formatSpeed(speedDown);

  /// Formatted upload speed
  String get speedUpFormatted => formatSpeed(speedUp);

  /// Formatted total download
  String get totalDownFormatted => formatBytes(totalBytesDown);

  /// Formatted total upload
  String get totalUpFormatted => formatBytes(totalBytesUp);

  /// Formatted uptime
  String get uptimeFormatted {
    final hours = uptime ~/ 3600;
    final minutes = (uptime % 3600) ~/ 60;
    final seconds = uptime % 60;
    if (hours > 0) return '${hours}h ${minutes}m ${seconds}s';
    if (minutes > 0) return '${minutes}m ${seconds}s';
    return '${seconds}s';
  }

  /// Connection quality rating (0-5 stars)
  int get qualityRating {
    double score = 5.0;
    if (latency > 100) score -= 1;
    if (latency > 300) score -= 1;
    if (packetLoss > 1) score -= 1;
    if (packetLoss > 5) score -= 1;
    if (speedDown < 1024 * 100) score -= 0.5; // Less than 100 KB/s
    return score.clamp(0, 5).round();
  }

  /// Copy with new speed sample (maintains history)
  ConnectionStats withNewSample({
    required double newSpeedDown,
    required double newSpeedUp,
    required double newLatency,
  }) {
    final newSpeedHistory = [...speedHistory, newSpeedDown];
    final newLatencyHistory = [...latencyHistory, newLatency];
    // Keep only last 60 samples
    if (newSpeedHistory.length > 60) {
      newSpeedHistory.removeAt(0);
    }
    if (newLatencyHistory.length > 60) {
      newLatencyHistory.removeAt(0);
    }

    return ConnectionStats(
      speedDown: newSpeedDown,
      speedUp: newSpeedUp,
      latency: newLatency,
      jitter: _calculateJitter(newLatencyHistory),
      packetLoss: packetLoss,
      totalBytesDown: totalBytesDown + (newSpeedDown ~/ 1),
      totalBytesUp: totalBytesUp + (newSpeedUp ~/ 1),
      uptime: uptime + 1,
      activeCore: activeCore,
      connectedServer: connectedServer,
      connectedCountry: connectedCountry,
      protocol: protocol,
      speedHistory: newSpeedHistory,
      latencyHistory: newLatencyHistory,
    );
  }

  static double _calculateJitter(List<double> latencies) {
    if (latencies.length < 2) return 0;
    double totalDiff = 0;
    for (int i = 1; i < latencies.length; i++) {
      totalDiff += (latencies[i] - latencies[i - 1]).abs();
    }
    return totalDiff / (latencies.length - 1);
  }

  /// Format speed in human-readable form
  static String formatSpeed(double bytesPerSec) {
    if (bytesPerSec <= 0) return '0 B/s';
    if (bytesPerSec < 1024) return '${bytesPerSec.toStringAsFixed(0)} B/s';
    if (bytesPerSec < 1024 * 1024) {
      return '${(bytesPerSec / 1024).toStringAsFixed(1)} KB/s';
    }
    if (bytesPerSec < 1024 * 1024 * 1024) {
      return '${(bytesPerSec / (1024 * 1024)).toStringAsFixed(1)} MB/s';
    }
    return '${(bytesPerSec / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB/s';
  }

  /// Format bytes in human-readable form
  static String formatBytes(int bytes) {
    if (bytes <= 0) return '0 B';
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    if (bytes < 1024 * 1024 * 1024) {
      return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }

  ConnectionStats copyWith({
    double? speedDown,
    double? speedUp,
    double? latency,
    double? jitter,
    double? packetLoss,
    int? totalBytesDown,
    int? totalBytesUp,
    int? uptime,
    String? activeCore,
    String? connectedServer,
    String? connectedCountry,
    String? protocol,
    List<double>? speedHistory,
    List<double>? latencyHistory,
  }) {
    return ConnectionStats(
      speedDown: speedDown ?? this.speedDown,
      speedUp: speedUp ?? this.speedUp,
      latency: latency ?? this.latency,
      jitter: jitter ?? this.jitter,
      packetLoss: packetLoss ?? this.packetLoss,
      totalBytesDown: totalBytesDown ?? this.totalBytesDown,
      totalBytesUp: totalBytesUp ?? this.totalBytesUp,
      uptime: uptime ?? this.uptime,
      activeCore: activeCore ?? this.activeCore,
      connectedServer: connectedServer ?? this.connectedServer,
      connectedCountry: connectedCountry ?? this.connectedCountry,
      protocol: protocol ?? this.protocol,
      speedHistory: speedHistory ?? this.speedHistory,
      latencyHistory: latencyHistory ?? this.latencyHistory,
    );
  }
}
