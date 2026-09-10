import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../main.dart';
import '../models/vpn_state.dart';

class SettingsScreen extends ConsumerStatefulWidget {
  const SettingsScreen({super.key});

  @override
  ConsumerState<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends ConsumerState<SettingsScreen> {
  bool _killSwitch = true;
  bool _splitTunnelIran = true;
  bool _autoConnect = false;
  bool _otaEnabled = true;
  String _dnsProvider = 'alidns';
  String _language = 'en';
  ThemeMode _themeMode = ThemeMode.dark;
  bool _nationalIntranetMode = false;

  @override
  void initState() {
    super.initState();
    _loadSettings();
  }

  Future<void> _loadSettings() async {
    final prefs = await SharedPreferences.getInstance();
    setState(() {
      _killSwitch = prefs.getBool('kill_switch') ?? true;
      _splitTunnelIran = prefs.getBool('split_tunnel_iran') ?? true;
      _autoConnect = prefs.getBool('auto_connect') ?? false;
      _otaEnabled = prefs.getBool('ota_enabled') ?? true;
      _dnsProvider = prefs.getString('dns_provider') ?? 'alidns';
      _language = prefs.getString('locale') ?? 'en';
      final themeStr = prefs.getString('theme') ?? 'dark';
      _themeMode = themeStr == 'dark' ? ThemeMode.dark : ThemeMode.light;
      _nationalIntranetMode = prefs.getBool('national_intranet_mode') ?? false;
    });
  }

  Future<void> _saveSetting(String key, dynamic value) async {
    final prefs = await SharedPreferences.getInstance();
    if (value is bool) {
      await prefs.setBool(key, value);
    } else if (value is String) {
      await prefs.setString(key, value);
    }
  }

  @override
  Widget build(BuildContext context) {
    final isFa = ref.watch(localeProvider).languageCode == 'fa';

    return Scaffold(
      appBar: AppBar(
        title: Text(isFa ? 'تنظیمات' : 'Settings'),
        centerTitle: true,
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // Connection section
          _SectionHeader(title: isFa ? 'اتصال' : 'Connection'),
          _SettingsTile(
            icon: Icons.security,
            title: isFa ? 'کلید توقف (Kill Switch)' : 'Kill Switch',
            subtitle: isFa
                ? 'قطع تمام ترافیک هنگام قطع VPN'
                : 'Block all traffic when VPN disconnects',
            trailing: Switch(
              value: _killSwitch,
              onChanged: (val) async {
                setState(() => _killSwitch = val);
                await _saveSetting('kill_switch', val);
                ref.read(vpnStateProvider.notifier).updateKillSwitch(val);
                try {
                  await ref.read(daemonBridgeProvider).setKillSwitch(val);
                } catch (_) {}
              },
            ),
          ),
          _SettingsTile(
            icon: Icons.call_split,
            title: isFa ? 'تونل اسپلیت برای IP ایران' : 'Split Tunnel Iran IPs',
            subtitle: isFa
                ? 'دسترسی مستقیم به سایت‌های داخلی ایران'
                : 'Direct access to Iranian domestic sites',
            trailing: Switch(
              value: _splitTunnelIran,
              onChanged: (val) async {
                setState(() => _splitTunnelIran = val);
                await _saveSetting('split_tunnel_iran', val);
                try {
                  await ref.read(daemonBridgeProvider).configureSplitTunneling(
                        excludedApps: [],
                        excludeIranianIps: val,
                      );
                } catch (_) {}
              },
            ),
          ),
          _SettingsTile(
            icon: Icons.power_settings_new,
            title: isFa ? 'اتصال خودکار' : 'Auto-Connect on Boot',
            subtitle: isFa
                ? 'اتصال خودکار هنگام روشن شدن دستگاه'
                : 'Automatically connect on device startup',
            trailing: Switch(
              value: _autoConnect,
              onChanged: (val) async {
                setState(() => _autoConnect = val);
                await _saveSetting('auto_connect', val);
              },
            ),
          ),

          const SizedBox(height: 16),

          // DNS section
          _SectionHeader(title: isFa ? 'DNS' : 'DNS Provider'),
          _SettingsTile(
            icon: Icons.dns,
            title: isFa ? 'ارائه‌دهنده DNS' : 'DNS Provider',
            subtitle: isFa
                ? 'انتخاب سرور DNS (CDN چینی در ایران کار می‌کند)'
                : 'DNS server (Chinese CDNs work in Iran)',
            trailing: DropdownButton<String>(
              value: _dnsProvider,
              underline: const SizedBox.shrink(),
              items: const [
                DropdownMenuItem(value: 'alidns', child: Text('AliDNS')),
                DropdownMenuItem(value: 'dnspod', child: Text('Tencent DNSPod')),
                DropdownMenuItem(value: 'cloudflare', child: Text('Cloudflare')),
                DropdownMenuItem(value: 'google', child: Text('Google')),
              ],
              onChanged: (val) async {
                if (val == null) return;
                setState(() => _dnsProvider = val);
                await _saveSetting('dns_provider', val);
              },
            ),
          ),

          const SizedBox(height: 16),

          // National Intranet section
          _SectionHeader(title: isFa ? 'اینترنت ملی' : 'National Intranet'),
          _SettingsTile(
            icon: Icons.warning_amber,
            title: isFa ? 'حالت اینترنت ملی' : 'National Intranet Mode',
            subtitle: isFa
                ? 'فعال‌سازی پروتکل‌های ضد خاموشی'
                : 'Enable anti-shutdown protocols',
            trailing: Switch(
              value: _nationalIntranetMode,
              onChanged: (val) async {
                setState(() => _nationalIntranetMode = val);
                await _saveSetting('national_intranet_mode', val);
                ref.read(vpnStateProvider.notifier).setNationalIntranetMode(val);
                try {
                  await ref.read(daemonBridgeProvider).triggerObfuscationMode(
                        val ? 'shutdown_mode' : 'default',
                      );
                } catch (_) {}
              },
            ),
          ),

          const SizedBox(height: 16),

          // Appearance section
          _SectionHeader(title: isFa ? 'ظاهر' : 'Appearance'),
          _SettingsTile(
            icon: Icons.language,
            title: isFa ? 'زبان' : 'Language',
            subtitle: isFa ? 'فارسی / English' : 'Persian / English',
            trailing: DropdownButton<String>(
              value: _language,
              underline: const SizedBox.shrink(),
              items: const [
                DropdownMenuItem(value: 'en', child: Text('English')),
                DropdownMenuItem(value: 'fa', child: Text('فارسی')),
              ],
              onChanged: (val) async {
                if (val == null) return;
                setState(() => _language = val);
                await _saveSetting('locale', val);
                ref.read(localeProvider.notifier).state = Locale(val);
              },
            ),
          ),
          _SettingsTile(
            icon: Icons.dark_mode,
            title: isFa ? 'پوسته' : 'Theme',
            subtitle: isFa ? 'تاریک / روشن' : 'Dark / Light',
            trailing: DropdownButton<ThemeMode>(
              value: _themeMode,
              underline: const SizedBox.shrink(),
              items: const [
                DropdownMenuItem(value: ThemeMode.dark, child: Text('Dark')),
                DropdownMenuItem(value: ThemeMode.light, child: Text('Light')),
              ],
              onChanged: (val) async {
                if (val == null) return;
                setState(() => _themeMode = val);
                await _saveSetting('theme', val == ThemeMode.dark ? 'dark' : 'light');
                ref.read(themeProvider.notifier).state = val;
              },
            ),
          ),

          const SizedBox(height: 16),

          // Updates section
          _SectionHeader(title: isFa ? 'به‌روزرسانی' : 'Updates'),
          _SettingsTile(
            icon: Icons.system_update,
            title: isFa ? 'به‌روزرسانی خودکار OTA' : 'Auto OTA Updates',
            subtitle: isFa
                ? 'بررسی به‌روزرسانی هر ۶ ساعت'
                : 'Check for updates every 6 hours',
            trailing: Switch(
              value: _otaEnabled,
              onChanged: (val) async {
                setState(() => _otaEnabled = val);
                await _saveSetting('ota_enabled', val);
              },
            ),
          ),
          _SettingsTile(
            icon: Icons.update,
            title: isFa ? 'بررسی دستی به‌روزرسانی' : 'Check for Updates Now',
            subtitle: isFa
                ? 'بررسی فوری به‌روزرسانی'
                : 'Immediately check for updates',
            onTap: () async {
              try {
                final updater = ref.read(otaUpdaterProvider);
                final update = await updater.checkForUpdate();
                if (!mounted) return;
                if (update != null) {
                  _showUpdateDialog(update, isFa);
                } else {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(
                      content: Text(isFa ? 'شما آخرین نسخه را دارید' : 'You are on the latest version'),
                    ),
                  );
                }
              } catch (e) {
                if (mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(content: Text('Update check failed: $e')),
                  );
                }
              }
            },
          ),

          const SizedBox(height: 32),

          // About
          Center(
            child: Text(
              'UnifiedShield NextGen v1.0.0',
              style: TextStyle(color: Colors.grey[500], fontSize: 12),
            ),
          ),
          const SizedBox(height: 8),
          Center(
            child: Text(
              isFa ? 'بدون نیاز به VPS • بدون نیاز به روت' : 'No VPS needed • No root needed',
              style: TextStyle(color: Colors.grey[600], fontSize: 11),
            ),
          ),
        ],
      ),
    );
  }

  void _showUpdateDialog(Map<String, dynamic> update, bool isFa) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(isFa ? 'به‌روزرسانی در دسترس' : 'Update Available'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('v${update['version']}'),
            const SizedBox(height: 8),
            Text(update['release_notes'] ?? ''),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: Text(isFa ? 'بعداً' : 'Later'),
          ),
          ElevatedButton(
            onPressed: () async {
              Navigator.pop(ctx);
              try {
                final updater = ref.read(otaUpdaterProvider);
                final filePath = await updater.downloadUpdate(
                  update['download_url'],
                  (progress) {
                    // Could show progress dialog
                  },
                );
                final verified = await updater.verifySha256(filePath, update['sha256_url']);
                if (verified) {
                  await updater.installUpdate(filePath);
                } else {
                  if (mounted) {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('SHA256 verification failed')),
                    );
                  }
                }
              } catch (e) {
                if (mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(content: Text('Update failed: $e')),
                  );
                }
              }
            },
            child: Text(isFa ? 'نصب' : 'Install'),
          ),
        ],
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  final String title;
  const _SectionHeader({required this.title});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8, top: 4),
      child: Text(
        title,
        style: Theme.of(context).textTheme.titleSmall?.copyWith(
              color: Colors.indigo[300],
              fontWeight: FontWeight.w600,
            ),
      ),
    );
  }
}

class _SettingsTile extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final Widget? trailing;
  final VoidCallback? onTap;

  const _SettingsTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    this.trailing,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.only(bottom: 4),
      child: ListTile(
        leading: Icon(icon, color: Colors.indigo[300]),
        title: Text(title, style: const TextStyle(fontSize: 14)),
        subtitle: Text(subtitle, style: TextStyle(fontSize: 11, color: Colors.grey[400])),
        trailing: trailing ?? const Icon(Icons.chevron_right, size: 20),
        onTap: onTap,
      ),
    );
  }
}
