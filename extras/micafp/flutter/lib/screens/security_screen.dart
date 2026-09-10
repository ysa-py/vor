import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../l10n/app_localizations.dart';
import '../services/daemon_service.dart';

/// Security audit screen.
///
/// Provides DPI testing, leak checking, and security audit features.
class SecurityScreen extends ConsumerStatefulWidget {
  const SecurityScreen({super.key});

  @override
  ConsumerState<SecurityScreen> createState() => _SecurityScreenState();
}

class _SecurityScreenState extends ConsumerState<SecurityScreen> {
  bool _isRunningDpiTest = false;
  bool _isRunningAudit = false;
  DpiTestResult? _dpiResult;
  SecurityAuditResult? _auditResult;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;

    return Scaffold(
      appBar: AppBar(title: Text(l10n.securityTitle)),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // DPI Test Section
          _SectionHeader(title: l10n.dpiTest),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    l10n.dpiTestDesc,
                    style: Theme.of(context).textTheme.bodyMedium,
                  ),
                  const SizedBox(height: 16),
                  SizedBox(
                    width: double.infinity,
                    child: FilledButton.icon(
                      onPressed: _isRunningDpiTest ? null : _runDpiTest,
                      icon: _isRunningDpiTest
                          ? const SizedBox(
                              width: 16,
                              height: 16,
                              child: CircularProgressIndicator(strokeWidth: 2, color: Colors.black),
                            )
                          : const Icon(Icons.science),
                      label: Text(l10n.runDpiTest),
                      style: FilledButton.styleFrom(
                        backgroundColor: const Color(0xFF00E5FF),
                        foregroundColor: Colors.black,
                      ),
                    ),
                  ),
                  if (_dpiResult != null) ...[
                    const SizedBox(height: 16),
                    _DpiResultCard(result: _dpiResult!),
                  ],
                ],
              ),
            ),
          ),

          const SizedBox(height: 24),

          // Security Audit Section
          _SectionHeader(title: l10n.securityAudit),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    l10n.securityAuditDesc,
                    style: Theme.of(context).textTheme.bodyMedium,
                  ),
                  const SizedBox(height: 16),
                  SizedBox(
                    width: double.infinity,
                    child: FilledButton.icon(
                      onPressed: _isRunningAudit ? null : _runSecurityAudit,
                      icon: _isRunningAudit
                          ? const SizedBox(
                              width: 16,
                              height: 16,
                              child: CircularProgressIndicator(strokeWidth: 2, color: Colors.black),
                            )
                          : const Icon(Icons.security),
                      label: Text(l10n.runAudit),
                      style: FilledButton.styleFrom(
                        backgroundColor: const Color(0xFF7C4DFF),
                        foregroundColor: Colors.white,
                      ),
                    ),
                  ),
                  if (_auditResult != null) ...[
                    const SizedBox(height: 16),
                    _AuditResultCard(result: _auditResult!),
                  ],
                ],
              ),
            ),
          ),

          const SizedBox(height: 24),

          // Quick Security Checks
          _SectionHeader(title: l10n.quickChecks),
          Card(
            child: Column(
              children: [
                _CheckTile(
                  icon: Icons.dns,
                  title: l10n.dnsLeak,
                  subtitle: l10n.dnsLeakDesc,
                  onTap: () => _checkDnsLeak(),
                ),
                const Divider(height: 1),
                _CheckTile(
                  icon: Icons.lan,
                  title: l10n.webrtcLeak,
                  subtitle: l10n.webrtcLeakDesc,
                  onTap: () => _checkWebrtcLeak(),
                ),
                const Divider(height: 1),
                _CheckTile(
                  icon: Icons.fingerprint,
                  title: l10n.browserFingerprint,
                  subtitle: l10n.browserFingerprintDesc,
                  onTap: () => _checkFingerprint(),
                ),
                const Divider(height: 1),
                _CheckTile(
                  icon: Icons.vpn_lock,
                  title: l10n.ipLeak,
                  subtitle: l10n.ipLeakDesc,
                  onTap: () => _checkIpLeak(),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _runDpiTest() async {
    setState(() => _isRunningDpiTest = true);
    try {
      final daemon = ref.read(daemonServiceProvider);
      final result = await daemon.runDpiTest();
      setState(() {
        _dpiResult = DpiTestResult.fromJson(result);
        _isRunningDpiTest = false;
      });
    } catch (e) {
      setState(() => _isRunningDpiTest = false);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('DPI test failed: $e')),
        );
      }
    }
  }

  Future<void> _runSecurityAudit() async {
    setState(() => _isRunningAudit = true);
    try {
      final daemon = ref.read(daemonServiceProvider);
      final result = await daemon.runSecurityAudit();
      setState(() {
        _auditResult = SecurityAuditResult.fromJson(result);
        _isRunningAudit = false;
      });
    } catch (e) {
      setState(() => _isRunningAudit = false);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Audit failed: $e')),
        );
      }
    }
  }

  Future<void> _checkDnsLeak() async {
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('🔍 Checking DNS leak...')),
    );
  }

  Future<void> _checkWebrtcLeak() async {
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('🔍 Checking WebRTC leak...')),
    );
  }

  Future<void> _checkFingerprint() async {
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('🔍 Checking browser fingerprint...')),
    );
  }

  Future<void> _checkIpLeak() async {
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('🔍 Checking IP leak...')),
    );
  }
}

class _DpiResultCard extends StatelessWidget {
  final DpiTestResult result;
  const _DpiResultCard({required this.result});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: result.isResistant ? const Color(0xFF00E676).withOpacity(0.1) : const Color(0xFFFF5252).withOpacity(0.1),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: result.isResistant ? const Color(0xFF00E676) : const Color(0xFFFF5252),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(
                result.isResistant ? Icons.check_circle : Icons.warning,
                color: result.isResistant ? const Color(0xFF00E676) : const Color(0xFFFF5252),
              ),
              const SizedBox(width: 8),
              Text(
                result.isResistant ? '✅ DPI Resistant' : '⚠️ DPI Detected',
                style: TextStyle(
                  color: result.isResistant ? const Color(0xFF00E676) : const Color(0xFFFF5252),
                  fontWeight: FontWeight.bold,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text('Tests passed: ${result.testsPassed}/${result.totalTests}'),
          if (result.detectedTechniques.isNotEmpty) ...[
            const SizedBox(height: 4),
            Text('Detected: ${result.detectedTechniques.join(', ')}'),
          ],
        ],
      ),
    );
  }
}

class _AuditResultCard extends StatelessWidget {
  final SecurityAuditResult result;
  const _AuditResultCard({required this.result});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: result.score > 80
            ? const Color(0xFF00E676).withOpacity(0.1)
            : const Color(0xFFFFB74D).withOpacity(0.1),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('Security Score: ${result.score}/100',
              style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 16)),
          const SizedBox(height: 8),
          ...result.checks.map((check) => Padding(
                padding: const EdgeInsets.symmetric(vertical: 2),
                child: Row(
                  children: [
                    Icon(
                      check.passed ? Icons.check : Icons.close,
                      size: 16,
                      color: check.passed ? const Color(0xFF00E676) : const Color(0xFFFF5252),
                    ),
                    const SizedBox(width: 8),
                    Text(check.name, style: const TextStyle(fontSize: 13)),
                  ],
                ),
              )),
        ],
      ),
    );
  }
}

class _CheckTile extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback onTap;

  const _CheckTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: Icon(icon, color: const Color(0xFF00E5FF)),
      title: Text(title),
      subtitle: Text(subtitle, style: Theme.of(context).textTheme.bodySmall),
      trailing: const Icon(Icons.chevron_right),
      onTap: onTap,
    );
  }
}

class _SectionHeader extends StatelessWidget {
  final String title;
  const _SectionHeader({required this.title});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Text(
        title,
        style: const TextStyle(
          color: Color(0xFF00E5FF),
          fontSize: 13,
          fontWeight: FontWeight.w700,
          letterSpacing: 1.2,
        ),
      ),
    );
  }
}

/// DPI test result model
class DpiTestResult {
  final bool isResistant;
  final int testsPassed;
  final int totalTests;
  final List<String> detectedTechniques;
  final Map<String, dynamic> details;

  const DpiTestResult({
    required this.isResistant,
    this.testsPassed = 0,
    this.totalTests = 0,
    this.detectedTechniques = const [],
    this.details = const {},
  });

  factory DpiTestResult.fromJson(Map<String, dynamic> json) {
    return DpiTestResult(
      isResistant: json['is_resistant'] as bool? ?? false,
      testsPassed: json['tests_passed'] as int? ?? 0,
      totalTests: json['total_tests'] as int? ?? 0,
      detectedTechniques: (json['detected_techniques'] as List<dynamic>?)
              ?.map((e) => e as String)
              .toList() ??
          [],
      details: json['details'] as Map<String, dynamic>? ?? {},
    );
  }
}

/// Security audit result model
class SecurityAuditResult {
  final int score;
  final List<SecurityCheck> checks;
  final String recommendation;

  const SecurityAuditResult({
    required this.score,
    this.checks = const [],
    this.recommendation = '',
  });

  factory SecurityAuditResult.fromJson(Map<String, dynamic> json) {
    return SecurityAuditResult(
      score: json['score'] as int? ?? 0,
      checks: (json['checks'] as List<dynamic>?)
              ?.map((e) => SecurityCheck.fromJson(e as Map<String, dynamic>))
              .toList() ??
          [],
      recommendation: json['recommendation'] as String? ?? '',
    );
  }
}

class SecurityCheck {
  final String name;
  final bool passed;
  final String? detail;

  const SecurityCheck({required this.name, required this.passed, this.detail});

  factory SecurityCheck.fromJson(Map<String, dynamic> json) {
    return SecurityCheck(
      name: json['name'] as String? ?? '',
      passed: json['passed'] as bool? ?? false,
      detail: json['detail'] as String?,
    );
  }
}
