import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logger/logger.dart';
import 'daemon_service.dart';

/// National Intranet Mode service.
///
/// When Iran's internet is under severe restrictions or total shutdown,
/// this mode allows access to approved national services while
/// maintaining security and privacy.
///
/// Features:
/// - Whitelist-based domain access (Iranian banks, government, education)
/// - DNS-over-HTTPS for domestic resolvers
/// - Automatic detection of national intranet mode
/// - Fallback to P2P relay for critical external services
/// - Emergency contacts and information access
class IntranetService {
  static final Logger _log = Logger(printer: PrettyPrinter(methodCount: 0));

  final DaemonService _daemon;

  IntranetService(this._daemon);

  bool _intranetModeActive = false;
  IntranetMode _mode = IntranetMode.disabled;
  List<String> _accessibleDomains = [];

  bool get isIntranetModeActive => _intranetModeActive;
  IntranetMode get mode => _mode;
  List<String> get accessibleDomains => _accessibleDomains;

  /// Pre-defined categories of national services
  static const Map<String, List<String>> nationalCategories = {
    'banking': [
      'bmi.ir', 'bankmellat.ir', 'sb24.ir', 'pec.ir',
      'shaparak.ir', 'sep.ir', 'parsian-bank.ir',
    ],
    'government': [
      'dolat.ir', 'irancell.ir', 'mci.ir', 'post.ir',
      'ssaa.ir', 'dastyar.ir',
    ],
    'education': [
      'ac.ir', 'edu.ir', 'sut.ac.ir', 'ut.ac.ir',
      'sharif.edu', 'iust.ac.ir', 'tehran.ir',
    ],
    'health': [
      'tamin.ir', 'fda.ir', 'behdasht.gov.ir',
    ],
    'news': [
      'isna.ir', 'irna.ir', 'mehrnews.com', 'tasnimnews.com',
    ],
    'essential': [
      'digikala.com', 'snapp.ir', 'esam.ir', 'divar.ir',
    ],
  };

  /// Enable national intranet mode
  Future<void> enable({
    IntranetMode mode = IntranetMode.smart,
    List<String>? customDomains,
    bool blockAllExternal = false,
    bool enableP2PFallback = true,
  }) async {
    try {
      _log.i('Enabling national intranet mode: $mode');

      final domains = customDomains ?? _getDefaultDomains(mode);

      await _daemon.enableIntranetMode(
        allowedDomains: domains,
        blockAllExternal: blockAllExternal,
      );

      // Enable P2P fallback for critical services
      if (enableP2FFallback) {
        await _daemon.sendCommand('p2p.fallback_enable', {
          'critical_services': ['signal.org', 'telegram.org', 'whatsapp.com'],
        });
      }

      _intranetModeActive = true;
      _mode = mode;
      _accessibleDomains = domains;

      _log.i('National intranet mode enabled with ${domains.length} domains');
    } catch (e) {
      _log.e('Failed to enable intranet mode', error: e);
      rethrow;
    }
  }

  /// Disable national intranet mode
  Future<void> disable() async {
    try {
      _log.i('Disabling national intranet mode');
      await _daemon.disableIntranetMode();
      _intranetModeActive = false;
      _mode = IntranetMode.disabled;
      _accessibleDomains = [];
    } catch (e) {
      _log.e('Failed to disable intranet mode', error: e);
      rethrow;
    }
  }

  /// Auto-detect if national intranet mode should be activated
  Future<bool> autoDetect() async {
    try {
      final response = await _daemon.sendCommand('intranet.detect', {});

      final shouldActivate = response['should_activate'] as bool? ?? false;
      final detectedMode = response['detected_mode'] as String? ?? 'disabled';

      if (shouldActivate) {
        _log.w('National intranet conditions detected! Activating...');
        await enable(mode: IntranetMode.values.firstWhere(
          (m) => m.name == detectedMode,
          orElse: () => IntranetMode.smart,
        ));
      }

      return shouldActivate;
    } catch (e) {
      _log.e('Auto-detection failed', error: e);
      return false;
    }
  }

  /// Get current intranet mode status
  Future<IntranetStatus> getStatus() async {
    try {
      final response = await _daemon.sendCommand('intranet.status', {});
      return IntranetStatus.fromJson(response);
    } catch (e) {
      _log.e('Failed to get intranet status', error: e);
      return const IntranetStatus(mode: IntranetMode.disabled);
    }
  }

  /// Add a domain to the whitelist
  Future<void> addDomain(String domain) async {
    try {
      await _daemon.sendCommand('intranet.add_domain', {'domain': domain});
      _accessibleDomains.add(domain);
    } catch (e) {
      _log.e('Failed to add domain $domain', error: e);
      rethrow;
    }
  }

  /// Remove a domain from the whitelist
  Future<void> removeDomain(String domain) async {
    try {
      await _daemon.sendCommand('intranet.remove_domain', {'domain': domain});
      _accessibleDomains.remove(domain);
    } catch (e) {
      _log.e('Failed to remove domain $domain', error: e);
      rethrow;
    }
  }

  /// Get the emergency information page
  Future<Map<String, dynamic>> getEmergencyInfo() async {
    try {
      return await _daemon.sendCommand('intranet.emergency_info', {});
    } catch (e) {
      return {
        'emergency_numbers': ['110', '115', '125', '112'],
        'information_urls': [],
      };
    }
  }

  List<String> _getDefaultDomains(IntranetMode mode) {
    switch (mode) {
      case IntranetMode.disabled:
        return [];
      case IntranetMode.essential:
        return [
          ...nationalCategories['banking']!,
          ...nationalCategories['government']!,
          ...nationalCategories['health']!,
        ];
      case IntranetMode.smart:
        return nationalCategories.values.expand((e) => e).toList();
      case IntranetMode.full:
        return ['*.ir']; // All .ir domains
    }
  }
}

/// Intranet mode enum
enum IntranetMode {
  disabled,
  essential,  // Only banking, government, health
  smart,      // All national services + P2P fallback
  full,       // All .ir domains
}

/// Intranet status model
class IntranetStatus {
  final IntranetMode mode;
  final List<String> accessibleDomains;
  final bool p2pFallbackActive;
  final DateTime? activatedAt;
  final int blockedConnections;
  final int allowedConnections;

  const IntranetStatus({
    this.mode = IntranetMode.disabled,
    this.accessibleDomains = const [],
    this.p2pFallbackActive = false,
    this.activatedAt,
    this.blockedConnections = 0,
    this.allowedConnections = 0,
  });

  factory IntranetStatus.fromJson(Map<String, dynamic> json) {
    return IntranetStatus(
      mode: IntranetMode.values.firstWhere(
        (m) => m.name == json['mode'],
        orElse: () => IntranetMode.disabled,
      ),
      accessibleDomains: (json['accessible_domains'] as List<dynamic>?)
              ?.map((e) => e as String)
              .toList() ??
          [],
      p2pFallbackActive: json['p2p_fallback_active'] as bool? ?? false,
      activatedAt: json['activated_at'] != null
          ? DateTime.fromMillisecondsSinceEpoch(json['activated_at'] as int)
          : null,
      blockedConnections: json['blocked_connections'] as int? ?? 0,
      allowedConnections: json['allowed_connections'] as int? ?? 0,
    );
  }
}

/// Riverpod providers
final intranetServiceProvider = Provider<IntranetService>((ref) {
  final daemon = ref.watch(daemonServiceProvider);
  return IntranetService(daemon);
});

final intranetStatusProvider = FutureProvider<IntranetStatus>((ref) async {
  final service = ref.watch(intranetServiceProvider);
  return service.getStatus();
});
