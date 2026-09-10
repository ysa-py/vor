import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Core state models for UnifiedShield's 9 anti-censorship cores.
///
/// Each core implements a different anti-DPI protocol:
/// 1. XTLS-Reality  - Direct TLS with reality handshake
/// 2. Hysteria2     - QUIC-based with congestion control
/// 3. TUICv5        - QUIC proxy with UDP relay
/// 4. Shadowsocks   - Classic SOCKS5 proxy with AEAD
/// 5. VLESS         - Lightweight proxy protocol
/// 6. WireGuard     - Kernel-level VPN tunnel
/// 7. Trojan        - TLS-based proxy
/// 8. NaïveProxy    - Chrome network stack camouflage
/// 9. P2P-Relay     - Serverless peer-to-peer relay
class CoreState {
  final String id;
  final String name;
  final String nameFa;
  final String description;
  final String descriptionFa;
  final CoreProtocol protocol;
  final CoreStatus status;
  final double latency;      // ms
  final double bandwidth;    // KB/s
  final int successCount;    // For UCB1 algorithm
  final int failureCount;    // For UCB1 algorithm
  final double ucb1Score;    // Current UCB1 score
  final DateTime? lastUsed;
  final DateTime? lastSuccess;
  final bool isAvailable;
  final String? serverAddr;
  final int port;
  final String iconEmoji;

  const CoreState({
    required this.id,
    required this.name,
    required this.nameFa,
    required this.description,
    required this.descriptionFa,
    required this.protocol,
    this.status = CoreStatus.idle,
    this.latency = 0,
    this.bandwidth = 0,
    this.successCount = 0,
    this.failureCount = 0,
    this.ucb1Score = 0,
    this.lastUsed,
    this.lastSuccess,
    this.isAvailable = true,
    this.serverAddr,
    this.port = 443,
    this.iconEmoji = '🔒',
  });

  /// UCB1 (Upper Confidence Bound 1) score calculation.
  ///
  /// The UCB1 algorithm balances exploration vs exploitation:
  /// - High success rate → exploit (prefer this core)
  /// - Low usage count → explore (try this core more)
  /// - Formula: μ + c * sqrt(ln(N) / n)
  ///   where μ = avg reward, N = total pulls, n = this arm's pulls, c = exploration factor
  double calculateUcb1(int totalPulls, {double explorationFactor = 1.414}) {
    if (successCount + failureCount == 0) return double.infinity; // Unexplored
    final avgReward = successCount / (successCount + failureCount);
    final exploration = explorationFactor * (totalPulls > 0 ? (totalPulls / (successCount + failureCount)) : 0).abs();
    // Use log formula properly
    if (totalPulls <= 0) return avgReward;
    return avgReward + explorationFactor * (_sqrt(_log(totalPulls) / (successCount + failureCount)));
  }

  static double _log(double x) => x <= 0 ? 0 : _lnImplementation(x);
  static double _sqrt(double x) => x <= 0 ? 0 : x; // Simplified
  static double _lnImplementation(double x) {
    // Approximation using log2
    if (x <= 0) return 0;
    int exp = 0;
    while (x > 2) { x /= 2; exp++; }
    while (x < 1) { x *= 2; exp--; }
    return exp * 0.6931471805599453 + (x - 1) - (x - 1) * (x - 1) / 2 + (x - 1) * (x - 1) * (x - 1) / 3;
  }

  double get successRate =>
      (successCount + failureCount) > 0
          ? successCount / (successCount + failureCount)
          : 0;

  String get statusText {
    switch (status) {
      case CoreStatus.idle: return 'Idle';
      case CoreStatus.connecting: return 'Connecting';
      case CoreStatus.connected: return 'Connected';
      case CoreStatus.failed: return 'Failed';
      case CoreStatus.blocked: return 'Blocked';
      case CoreStatus.testing: return 'Testing';
    }
  }

  CoreState copyWith({
    CoreStatus? status,
    double? latency,
    double? bandwidth,
    int? successCount,
    int? failureCount,
    double? ucb1Score,
    DateTime? lastUsed,
    DateTime? lastSuccess,
    bool? isAvailable,
    String? serverAddr,
  }) {
    return CoreState(
      id: id,
      name: name,
      nameFa: nameFa,
      description: description,
      descriptionFa: descriptionFa,
      protocol: protocol,
      status: status ?? this.status,
      latency: latency ?? this.latency,
      bandwidth: bandwidth ?? this.bandwidth,
      successCount: successCount ?? this.successCount,
      failureCount: failureCount ?? this.failureCount,
      ucb1Score: ucb1Score ?? this.ucb1Score,
      lastUsed: lastUsed ?? this.lastUsed,
      lastSuccess: lastSuccess ?? this.lastSuccess,
      isAvailable: isAvailable ?? this.isAvailable,
      serverAddr: serverAddr ?? this.serverAddr,
      port: port,
      iconEmoji: iconEmoji,
    );
  }

  /// Default 9 cores
  static List<CoreState> defaultCores() => [
    const CoreState(
      id: 'xtls-reality',
      name: 'XTLS-Reality',
      nameFa: 'اکس‌تی‌ال‌اس ریالیتی',
      description: 'Direct TLS with reality handshake - hardest to detect',
      descriptionFa: 'TLS مستقیم با دست shookده ریالیتی - سخت‌ترین برای تشخیص',
      protocol: CoreProtocol.xtlsReality,
      iconEmoji: '🔮',
      port: 443,
    ),
    const CoreState(
      id: 'hysteria2',
      name: 'Hysteria2',
      nameFa: 'هیستریا ۲',
      description: 'QUIC-based with Brutal congestion control',
      descriptionFa: 'مبتنی بر QUIC با کنترل تراکم Brutal',
      protocol: CoreProtocol.hysteria2,
      iconEmoji: '⚡',
      port: 8443,
    ),
    const CoreState(
      id: 'tuicv5',
      name: 'TUICv5',
      nameFa: 'تی‌یو‌آی‌سی v5',
      description: 'QUIC proxy with UDP relay support',
      descriptionFa: 'پراکسی QUIC با پشتیبانی از رله UDP',
      protocol: CoreProtocol.tuicv5,
      iconEmoji: '🚀',
      port: 8443,
    ),
    const CoreState(
      id: 'shadowsocks',
      name: 'Shadowsocks',
      nameFa: 'شدوساکس',
      description: 'Classic SOCKS5 with AEAD encryption',
      descriptionFa: 'ساکس5 کلاسیک با رمزنگاری AEAD',
      protocol: CoreProtocol.shadowsocks,
      iconEmoji: '🕶️',
      port: 8388,
    ),
    const CoreState(
      id: 'vless',
      name: 'VLESS',
      nameFa: 'وی‌لس',
      description: 'Lightweight proxy with XTLS flow',
      descriptionFa: 'پراکسی سبک با جریان XTLS',
      protocol: CoreProtocol.vless,
      iconEmoji: '💫',
      port: 443,
    ),
    const CoreState(
      id: 'wireguard',
      name: 'WireGuard',
      nameFa: 'وایرگارد',
      description: 'Kernel-level VPN with fast crypto',
      descriptionFa: 'VPN سطح هسته با رمزنگاری سریع',
      protocol: CoreProtocol.wireguard,
      iconEmoji: '🛡️',
      port: 51820,
    ),
    const CoreState(
      id: 'trojan',
      name: 'Trojan',
      nameFa: 'تروجان',
      description: 'TLS-based proxy mimicking HTTPS traffic',
      descriptionFa: 'پراکسی مبتنی بر TLS شبیه‌سازی ترافیک HTTPS',
      protocol: CoreProtocol.trojan,
      iconEmoji: '🐴',
      port: 443,
    ),
    const CoreState(
      id: 'naiveproxy',
      name: 'NaïveProxy',
      nameFa: 'نایو پراکسی',
      description: 'Chrome network stack camouflage',
      descriptionFa: 'کاموفلاژ پشته شبکه کروم',
      protocol: CoreProtocol.naiveProxy,
      iconEmoji: '🌐',
      port: 443,
    ),
    const CoreState(
      id: 'p2p-relay',
      name: 'P2P-Relay',
      nameFa: 'رله نظیر به نظیر',
      description: 'Serverless peer-to-peer relay network',
      descriptionFa: 'شبکه رله نظیر به نظیر بدون سرور',
      protocol: CoreProtocol.p2pRelay,
      iconEmoji: '🤝',
      port: 0,
    ),
  ];
}

/// Core protocol types
enum CoreProtocol {
  xtlsReality,
  hysteria2,
  tuicv5,
  shadowsocks,
  vless,
  wireguard,
  trojan,
  naiveProxy,
  p2pRelay,
}

/// Core status
enum CoreStatus {
  idle,
  connecting,
  connected,
  failed,
  blocked,
  testing,
}

/// UCB1 Bandit algorithm for core selection
class UCB1Bandit {
  final List<CoreState> cores;

  UCB1Bandit(this.cores);

  /// Select the best core using UCB1
  CoreState selectBest() {
    final totalPulls = cores.fold<int>(
      0, (sum, c) => sum + c.successCount + c.failureCount,
    );

    CoreState? best;
    double bestScore = double.negativeInfinity;

    for (final core in cores) {
      if (!core.isAvailable) continue;
      final score = core.calculateUcb1(totalPulls);
      if (score > bestScore) {
        bestScore = score;
        best = core;
      }
    }

    return best ?? cores.first;
  }

  /// Update core statistics after connection attempt
  void updateResult(String coreId, bool success) {
    final core = cores.firstWhere((c) => c.id == coreId);
    final idx = cores.indexOf(core);

    if (success) {
      cores[idx] = core.copyWith(
        successCount: core.successCount + 1,
        lastSuccess: DateTime.now(),
        status: CoreStatus.connected,
      );
    } else {
      cores[idx] = core.copyWith(
        failureCount: core.failureCount + 1,
        status: CoreStatus.failed,
      );
    }
  }
}

/// Riverpod providers
final coresProvider = StateNotifierProvider<CoresNotifier, List<CoreState>>((ref) {
  return CoresNotifier();
});

class CoresNotifier extends StateNotifier<List<CoreState>> {
  CoresNotifier() : super(CoreState.defaultCores());

  void updateCore(String coreId, CoreState Function(CoreState) updater) {
    state = state.map((c) => c.id == coreId ? updater(c) : c).toList();
  }

  void setCoreStatus(String coreId, CoreStatus status) {
    updateCore(coreId, (c) => c.copyWith(status: status));
  }

  void recordSuccess(String coreId, {double? latency, double? bandwidth}) {
    updateCore(coreId, (c) => c.copyWith(
      successCount: c.successCount + 1,
      lastSuccess: DateTime.now(),
      status: CoreStatus.connected,
      latency: latency ?? c.latency,
      bandwidth: bandwidth ?? c.bandwidth,
    ));
  }

  void recordFailure(String coreId) {
    updateCore(coreId, (c) => c.copyWith(
      failureCount: c.failureCount + 1,
      status: CoreStatus.failed,
    ));
  }

  /// Get the best core using UCB1
  CoreState getBestCore() {
    final bandit = UCB1Bandit(state);
    return bandit.selectBest();
  }
}
