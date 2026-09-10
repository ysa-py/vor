import 'dart:io';

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:url_launcher/url_launcher.dart';

import '../services/daemon_bridge.dart';
import '../services/battery_service.dart';
import '../platform/android_platform.dart';

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key});

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  bool _autoConnect = false;
  bool _killSwitch = true;
  bool _encryptedDns = true;
  bool _splitTunneling = false;
  String _preferredTransport = 'auto';
  String _wipeTrigger = 'none';
  bool _isLoading = true;

  @override
  void initState() {
    super.initState();
    _loadSettings();
  }

  Future<void> _loadSettings() async {
    final prefs = await SharedPreferences.getInstance();
    setState(() {
      _autoConnect = prefs.getBool('auto_connect') ?? false;
      _killSwitch = prefs.getBool('kill_switch') ?? true;
      _encryptedDns = prefs.getBool('encrypted_dns') ?? true;
      _splitTunneling = prefs.getBool('split_tunneling') ?? false;
      _preferredTransport = prefs.getString('preferred_transport') ?? 'auto';
      _wipeTrigger = prefs.getString('wipe_trigger') ?? 'none';
      _isLoading = false;
    });
  }

  Future<void> _saveSetting(String key, dynamic value) async {
    final prefs = await SharedPreferences.getInstance();
    if (value is bool) {
      await prefs.setBool(key, value);
    } else if (value is String) {
      await prefs.setString(key, value);
    }

    // Notify daemon of config change
    if (mounted) {
      final bridge = context.read<DaemonBridge>();
      await bridge.sendConfigUpdate(key, value.toString());
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_isLoading) {
      return const Scaffold(
        body: Center(child: CircularProgressIndicator()),
      );
    }

    return Scaffold(
      appBar: AppBar(
        title: const Text('Settings'),
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // Connection section
          _buildSectionHeader('Connection'),
          SwitchListTile(
            title: const Text('Auto-connect on Start'),
            subtitle: const Text('Connect automatically when app opens'),
            value: _autoConnect,
            onChanged: (val) {
              setState(() => _autoConnect = val);
              _saveSetting('auto_connect', val);
            },
          ),
          SwitchListTile(
            title: const Text('Kill Switch'),
            subtitle: const Text('Block all traffic when disconnected'),
            value: _killSwitch,
            onChanged: (val) {
              setState(() => _killSwitch = val);
              _saveSetting('kill_switch', val);
            },
          ),
          ListTile(
            title: const Text('Transport'),
            subtitle: Text(
              _preferredTransport == 'auto'
                  ? 'Automatic (recommended)'
                  : _preferredTransport,
            ),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => _showTransportPicker(),
          ),

          const SizedBox(height: 24),

          // DNS section
          _buildSectionHeader('DNS'),
          SwitchListTile(
            title: const Text('Encrypted DNS'),
            subtitle: const Text('Use DNS over HTTPS'),
            value: _encryptedDns,
            onChanged: (val) {
              setState(() => _encryptedDns = val);
              _saveSetting('encrypted_dns', val);
            },
          ),

          // Split tunneling (Android only)
          if (Platform.isAndroid) ...[
            const SizedBox(height: 24),
            _buildSectionHeader('Advanced'),
            SwitchListTile(
              title: const Text('Per-App Tunneling'),
              subtitle: const Text('Select which apps use the tunnel'),
              value: _splitTunneling,
              onChanged: (val) {
                setState(() => _splitTunneling = val);
                _saveSetting('split_tunneling', val);
              },
            ),
          ],

          const SizedBox(height: 24),

          // Battery section
          _buildSectionHeader('Battery'),
          Consumer<BatteryService>(
            builder: (context, battery, _) {
              return Column(
                children: [
                  ListTile(
                    title: const Text('Power Mode'),
                    subtitle: Text(_getPowerModeLabel(battery.powerMode)),
                    trailing: _getPowerModeIcon(battery.powerMode),
                  ),
                  if (Platform.isAndroid)
                    ListTile(
                      title: const Text('Battery Optimization'),
                      subtitle: Text(
                        battery.isOptimizationExempt
                            ? 'Exempted (recommended)'
                            : 'Not exempted — may cause disconnections',
                        style: TextStyle(
                          color: battery.isOptimizationExempt
                              ? const Color(0xFF2E7D32)
                              : const Color(0xFFC62828),
                        ),
                      ),
                      trailing: const Icon(Icons.battery_alert),
                      onTap: () => battery.openBatterySettings(),
                    ),
                ],
              );
            },
          ),

          const SizedBox(height: 24),

          // Anti-forensics section
          _buildSectionHeader('Security'),
          ListTile(
            title: const Text('Emergency Wipe'),
            subtitle: Text(
              _wipeTrigger == 'none'
                  ? 'Disabled'
                  : 'Enabled: $_wipeTrigger',
            ),
            trailing: const Icon(Icons.chevron_right),
            onTap: () => _showWipeTriggerPicker(),
          ),

          const SizedBox(height: 24),

          // About section
          _buildSectionHeader('About'),
          const ListTile(
            title: Text('Version'),
            subtitle: Text('6.0.0'),
          ),
          ListTile(
            title: const Text('Open Source Licenses'),
            trailing: const Icon(Icons.chevron_right),
            onTap: () {
              showLicensePage(
                context: context,
                applicationName: 'Shield',
                applicationVersion: '6.0.0',
              );
            },
          ),
        ],
      ),
    );
  }

  Widget _buildSectionHeader(String title) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Text(
        title.toUpperCase(),
        style: TextStyle(
          color: Colors.grey[500],
          fontSize: 12,
          fontWeight: FontWeight.w600,
          letterSpacing: 1.2,
        ),
      ),
    );
  }

  String _getPowerModeLabel(PowerMode mode) {
    switch (mode) {
      case PowerMode.performance:
        return 'Performance';
      case PowerMode.normal:
        return 'Normal';
      case PowerMode.save:
        return 'Battery Saver';
      case PowerMode.critical:
        return 'Critical';
    }
  }

  Widget _getPowerModeIcon(PowerMode mode) {
    switch (mode) {
      case PowerMode.performance:
        return const Icon(Icons.bolt, color: Colors.green);
      case PowerMode.normal:
        return const Icon(Icons.battery_std, color: Colors.blueGrey);
      case PowerMode.save:
        return const Icon(Icons.battery_saver, color: Colors.orange);
      case PowerMode.critical:
        return const Icon(Icons.battery_alert, color: Colors.red);
    }
  }

  void _showTransportPicker() {
    showDialog(
      context: context,
      builder: (context) => SimpleDialog(
        title: const Text('Transport'),
        children: [
          SimpleDialogOption(
            onPressed: () {
              Navigator.pop(context);
              setState(() => _preferredTransport = 'auto');
              _saveSetting('preferred_transport', 'auto');
            },
            child: const ListTile(
              leading: Icon(Icons.auto_mode),
              title: Text('Automatic (recommended)'),
              contentPadding: EdgeInsets.zero,
            ),
          ),
          SimpleDialogOption(
            onPressed: () {
              Navigator.pop(context);
              setState(() => _preferredTransport = 'ws');
              _saveSetting('preferred_transport', 'ws');
            },
            child: const ListTile(
              leading: Icon(Icons.http),
              title: Text('WebSocket'),
              contentPadding: EdgeInsets.zero,
            ),
          ),
          SimpleDialogOption(
            onPressed: () {
              Navigator.pop(context);
              setState(() => _preferredTransport = 'quic');
              _saveSetting('preferred_transport', 'quic');
            },
            child: const ListTile(
              leading: Icon(Icons.speed),
              title: Text('QUIC'),
              contentPadding: EdgeInsets.zero,
            ),
          ),
        ],
      ),
    );
  }

  void _showWipeTriggerPicker() {
    showDialog(
      context: context,
      builder: (context) => SimpleDialog(
        title: const Text('Emergency Wipe Trigger'),
        children: [
          SimpleDialogOption(
            onPressed: () {
              Navigator.pop(context);
              setState(() => _wipeTrigger = 'none');
              _saveSetting('wipe_trigger', 'none');
            },
            child: const ListTile(
              leading: Icon(Icons.block),
              title: Text('Disabled'),
              contentPadding: EdgeInsets.zero,
            ),
          ),
          SimpleDialogOption(
            onPressed: () {
              Navigator.pop(context);
              setState(() => _wipeTrigger = '5tap');
              _saveSetting('wipe_trigger', '5tap');
            },
            child: const ListTile(
              leading: Icon(Icons.touch_app),
              title: Text('5 Rapid Taps'),
              contentPadding: EdgeInsets.zero,
            ),
          ),
          SimpleDialogOption(
            onPressed: () {
              Navigator.pop(context);
              setState(() => _wipeTrigger = '3pin');
              _saveSetting('wipe_trigger', '3pin');
            },
            child: const ListTile(
              leading: Icon(Icons.password),
              title: Text('3 Wrong PIN Attempts'),
              contentPadding: EdgeInsets.zero,
            ),
          ),
          SimpleDialogOption(
            onPressed: () {
              Navigator.pop(context);
              setState(() => _wipeTrigger = 'both');
              _saveSetting('wipe_trigger', 'both');
            },
            child: const ListTile(
              leading: Icon(Icons.shield),
              title: Text('Both'),
              contentPadding: EdgeInsets.zero,
            ),
          ),
        ],
      ),
    );
  }
}
