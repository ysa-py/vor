import 'package:flutter/material.dart';

import '../services/battery_service.dart';

/// Battery optimization indicator widget
///
/// Shows current power mode icon, battery percentage,
/// and tap to open battery optimization settings.
class BatteryIndicator extends StatelessWidget {
  final BatteryService batteryService;
  final VoidCallback? onTap;

  const BatteryIndicator({
    super.key,
    required this.batteryService,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(16),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              // Power mode icon
              _buildPowerModeIcon(),
              const SizedBox(width: 12),

              // Battery info
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Text(
                          '${batteryService.batteryLevel}%',
                          style: const TextStyle(
                            fontSize: 18,
                            fontWeight: FontWeight.bold,
                            color: Color(0xFFE0E0E0),
                          ),
                        ),
                        const SizedBox(width: 8),
                        if (batteryService.isCharging)
                          const Icon(
                            Icons.bolt,
                            color: Color(0xFF2E7D32),
                            size: 18,
                          ),
                      ],
                    ),
                    const SizedBox(height: 2),
                    Text(
                      _getPowerModeLabel(),
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  ],
                ),
              ),

              // Battery level bar
              SizedBox(
                width: 48,
                height: 24,
                child: CustomPaint(
                  painter: _BatteryBarPainter(
                    level: batteryService.batteryLevel / 100.0,
                    color: _getBatteryColor(),
                  ),
                ),
              ),

              const SizedBox(width: 8),

              // Tap indicator
              Icon(
                Icons.chevron_right,
                color: Colors.grey[500],
                size: 20,
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildPowerModeIcon() {
    IconData icon;
    Color color;

    switch (batteryService.powerMode) {
      case PowerMode.performance:
        icon = Icons.bolt;
        color = const Color(0xFF2E7D32);
        break;
      case PowerMode.normal:
        icon = Icons.battery_std;
        color = const Color(0xFF546E7A);
        break;
      case PowerMode.save:
        icon = Icons.battery_saver;
        color = const Color(0xFFF57F17);
        break;
      case PowerMode.critical:
        icon = Icons.battery_alert;
        color = const Color(0xFFC62828);
        break;
    }

    return Container(
      width: 44,
      height: 44,
      decoration: BoxDecoration(
        color: color.withOpacity(0.15),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Icon(icon, color: color, size: 24),
    );
  }

  Color _getBatteryColor() {
    if (batteryService.isCharging) {
      return const Color(0xFF2E7D32);
    }

    switch (batteryService.powerMode) {
      case PowerMode.performance:
        return const Color(0xFF2E7D32);
      case PowerMode.normal:
        return const Color(0xFF546E7A);
      case PowerMode.save:
        return const Color(0xFFF57F17);
      case PowerMode.critical:
        return const Color(0xFFC62828);
    }
  }

  String _getPowerModeLabel() {
    switch (batteryService.powerMode) {
      case PowerMode.performance:
        return 'Performance Mode';
      case PowerMode.normal:
        return 'Normal Mode';
      case PowerMode.save:
        return 'Battery Saver';
      case PowerMode.critical:
        return 'Critical — Minimal refresh';
    }
  }
}

/// Custom painter for battery level bar
class _BatteryBarPainter extends CustomPainter {
  final double level; // 0.0 to 1.0
  final Color color;

  _BatteryBarPainter({required this.level, required this.color});

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = const Color(0xFF37474F)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.5;

    final fillPaint = Paint()
      ..color = color
      ..style = PaintingStyle.fill;

    const borderRadius = 3.0;

    // Draw battery outline
    final rect = RRect.fromRectAndRadius(
      Rect.fromLTWH(0, 2, size.width - 4, size.height - 4),
      const Radius.circular(borderRadius),
    );
    canvas.drawRRect(rect, paint);

    // Draw battery tip (right side nub)
    canvas.drawRect(
      Rect.fromLTWH(size.width - 3, size.height / 2 - 4, 3, 8),
      paint,
    );

    // Draw fill level
    final fillWidth = (size.width - 8) * level.clamp(0.0, 1.0);
    if (fillWidth > 0) {
      final fillRect = RRect.fromRectAndRadius(
        Rect.fromLTWH(2, 4, fillWidth, size.height - 8),
        const Radius.circular(2),
      );
      canvas.drawRRect(fillRect, fillPaint);
    }
  }

  @override
  bool shouldRepaint(_BatteryBarPainter oldDelegate) {
    return oldDelegate.level != level || oldDelegate.color != color;
  }
}
