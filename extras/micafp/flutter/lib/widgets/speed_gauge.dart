import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter_animate/flutter_animate.dart';
import '../models/connection_stats.dart';

/// Circular speed gauge widget for download/upload speed display.
///
/// Uses a custom painter for a sleek, cyberpunk-inspired gauge
/// with animated arc and digital readout.
class SpeedGauge extends StatelessWidget {
  final String label;
  final double speed;        // bytes/sec
  final double maxSpeed;     // bytes/sec
  final Color color;

  const SpeedGauge({
    super.key,
    required this.label,
    required this.speed,
    required this.maxSpeed,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    final progress = (speed / maxSpeed).clamp(0.0, 1.0);
    final displaySpeed = ConnectionStats.formatSpeed(speed);

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          children: [
            // Label
            Text(
              label,
              style: TextStyle(
                color: color.withOpacity(0.7),
                fontSize: 12,
                fontWeight: FontWeight.w600,
                letterSpacing: 1.2,
              ),
            ),
            const SizedBox(height: 8),

            // Gauge
            SizedBox(
              width: 120,
              height: 120,
              child: CustomPaint(
                painter: _SpeedGaugePainter(
                  progress: progress,
                  color: color,
                  backgroundColor: const Color(0xFF21262D),
                ),
                child: Center(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Text(
                        _getSpeedValue(displaySpeed),
                        style: TextStyle(
                          fontSize: 22,
                          fontWeight: FontWeight.bold,
                          color: color,
                        ),
                      ),
                      Text(
                        _getSpeedUnit(displaySpeed),
                        style: TextStyle(
                          fontSize: 10,
                          color: color.withOpacity(0.7),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            )
                .animate(target: progress > 0 ? 1 : 0)
                .fadeIn(duration: 500.ms),
          ],
        ),
      ),
    );
  }

  String _getSpeedValue(String formatted) {
    final parts = formatted.split(' ');
    return parts.isNotEmpty ? parts[0] : '0';
  }

  String _getSpeedUnit(String formatted) {
    final parts = formatted.split(' ');
    return parts.length > 1 ? parts[1] : 'B/s';
  }
}

/// Custom painter for the circular speed gauge
class _SpeedGaugePainter extends CustomPainter {
  final double progress;
  final Color color;
  final Color backgroundColor;

  _SpeedGaugePainter({
    required this.progress,
    required this.color,
    required this.backgroundColor,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final radius = (size.width - 16) / 2;

    // Background arc
    final bgPaint = Paint()
      ..color = backgroundColor
      ..style = PaintingStyle.stroke
      ..strokeWidth = 8
      ..strokeCap = StrokeCap.round;

    const startAngle = -math.pi * 0.75;
    const sweepAngle = math.pi * 1.5;

    canvas.drawArc(
      Rect.fromCircle(center: center, radius: radius),
      startAngle,
      sweepAngle,
      false,
      bgPaint,
    );

    // Progress arc
    if (progress > 0) {
      final progressPaint = Paint()
        ..shader = SweepGradient(
          startAngle: startAngle,
          endAngle: startAngle + sweepAngle * progress,
          colors: [
            color.withOpacity(0.5),
            color,
          ],
          stops: const [0.0, 1.0],
          transform: GradientRotation(startAngle),
        ).createShader(Rect.fromCircle(center: center, radius: radius))
        ..style = PaintingStyle.stroke
        ..strokeWidth = 8
        ..strokeCap = StrokeCap.round;

      canvas.drawArc(
        Rect.fromCircle(center: center, radius: radius),
        startAngle,
        sweepAngle * progress,
        false,
        progressPaint,
      );

      // Glow effect
      final glowPaint = Paint()
        ..color = color.withOpacity(0.3)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 16
        ..strokeCap = StrokeCap.round
        ..maskFilter = const MaskFilter.blur(BlurStyle.normal, 8);

      canvas.drawArc(
        Rect.fromCircle(center: center, radius: radius),
        startAngle + sweepAngle * progress - 0.05,
        0.05,
        false,
        glowPaint,
      );
    }

    // Tick marks
    final tickPaint = Paint()
      ..color = const Color(0xFF30363D)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1;

    for (int i = 0; i <= 10; i++) {
      final angle = startAngle + (sweepAngle * i / 10);
      final innerRadius = radius - 14;
      final outerRadius = radius - 10;
      canvas.drawLine(
        Offset(
          center.dx + innerRadius * math.cos(angle),
          center.dy + innerRadius * math.sin(angle),
        ),
        Offset(
          center.dx + outerRadius * math.cos(angle),
          center.dy + outerRadius * math.sin(angle),
        ),
        tickPaint,
      );
    }
  }

  @override
  bool shouldRepaint(_SpeedGaugePainter oldDelegate) {
    return oldDelegate.progress != progress || oldDelegate.color != color;
  }
}
