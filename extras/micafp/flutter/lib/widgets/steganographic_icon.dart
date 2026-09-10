import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// Steganographic app icon manager
///
/// Default: calculator icon (does not attract attention)
/// Configurable: can change to other common utility icons
/// NEVER shows VPN/shield/privacy-related icon
class SteganographicIcon {
  static const String _prefsKey = 'steg_icon_type';

  /// Available icon disguises — all are common utility apps
  static const Map<String, StegIconConfig> availableIcons = {
    'calculator': StegIconConfig(
      id: 'calculator',
      label: 'Calculator',
      icon: Icons.calculate_outlined,
      packageName: 'com.android.calculator2',
    ),
    'weather': StegIconConfig(
      id: 'weather',
      label: 'Weather',
      icon: Icons.wb_sunny_outlined,
      packageName: 'com.android.weather',
    ),
    'notes': StegIconConfig(
      id: 'notes',
      label: 'Notes',
      icon: Icons.note_outlined,
      packageName: 'com.android.notes',
    ),
    'clock': StegIconConfig(
      id: 'clock',
      label: 'Clock',
      icon: Icons.access_time,
      packageName: 'com.android.clock',
    ),
    'files': StegIconConfig(
      id: 'files',
      label: 'Files',
      icon: Icons.folder_outlined,
      packageName: 'com.android.files',
    ),
    'settings': StegIconConfig(
      id: 'settings',
      label: 'Settings',
      icon: Icons.settings_outlined,
      packageName: 'com.android.settings',
    ),
  };

  /// Get current icon type
  static Future<String> getCurrentIconType() async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getString(_prefsKey) ?? 'calculator';
  }

  /// Set icon type
  static Future<void> setIconType(String iconType) async {
    if (!availableIcons.containsKey(iconType)) {
      throw ArgumentError('Unknown icon type: $iconType');
    }

    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_prefsKey, iconType);

    // On Android, we would update the launcher icon via platform channel
    // This requires the activity-alias approach in AndroidManifest.xml
    // The actual icon switch is handled by the Android platform code
  }

  /// Get current icon config
  static Future<StegIconConfig> getCurrentConfig() async {
    final type = await getCurrentIconType();
    return availableIcons[type]!;
  }

  /// Get the icon widget for the current disguise
  static Widget buildIcon({
    double size = 48,
    Color? color,
  }) {
    return FutureBuilder<StegIconConfig>(
      future: getCurrentConfig(),
      builder: (context, snapshot) {
        final config = snapshot.data ?? availableIcons['calculator']!;
        return Container(
          width: size,
          height: size,
          decoration: BoxDecoration(
            color: const Color(0xFF1A1A2E),
            borderRadius: BorderRadius.circular(size * 0.3),
          ),
          child: Icon(
            config.icon,
            size: size * 0.6,
            color: color ?? const Color(0xFFE0E0E0),
          ),
        );
      },
    );
  }

  /// Show icon selection dialog
  static Future<void> showIconPicker(BuildContext context) async {
    final selected = await showDialog<String>(
      context: context,
      builder: (context) => const _IconPickerDialog(),
    );

    if (selected != null) {
      await setIconType(selected);

      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              'Icon changed to ${availableIcons[selected]!.label}. '
              'Restart app to apply.',
            ),
          ),
        );
      }
    }
  }
}

/// Configuration for a steganographic icon
class StegIconConfig {
  final String id;
  final String label;
  final IconData icon;
  final String packageName;

  const StegIconConfig({
    required this.id,
    required this.label,
    required this.icon,
    required this.packageName,
  });
}

/// Icon picker dialog
class _IconPickerDialog extends StatelessWidget {
  const _IconPickerDialog();

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('App Appearance'),
      content: SizedBox(
        width: 300,
        child: GridView.count(
          crossAxisCount: 3,
          shrinkWrap: true,
          mainAxisSpacing: 12,
          crossAxisSpacing: 12,
          children: SteganographicIcon.availableIcons.entries.map((entry) {
            return InkWell(
              onTap: () => Navigator.of(context).pop(entry.key),
              borderRadius: BorderRadius.circular(16),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Container(
                    width: 48,
                    height: 48,
                    decoration: BoxDecoration(
                      color: const Color(0xFF1A1A2E),
                      borderRadius: BorderRadius.circular(14),
                    ),
                    child: Icon(
                      entry.value.icon,
                      color: const Color(0xFFE0E0E0),
                      size: 28,
                    ),
                  ),
                  const SizedBox(height: 6),
                  Text(
                    entry.value.label,
                    style: const TextStyle(fontSize: 11),
                    textAlign: TextAlign.center,
                  ),
                ],
              ),
            );
          }).toList(),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
      ],
    );
  }
}
