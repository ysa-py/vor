import 'package:flutter/material.dart';

import '../services/daemon_bridge.dart';

/// Animated connection status indicator widget
///
/// Green = connected
/// Red = disconnected
/// Yellow = connecting (with pulse animation)
/// Orange = NAIN mode (alternative channels active)
class ConnectionIndicator extends StatefulWidget {
  final ConnectionStatus status;
  final bool isNainMode;

  const ConnectionIndicator({
    super.key,
    required this.status,
    this.isNainMode = false,
  });

  @override
  State<ConnectionIndicator> createState() => _ConnectionIndicatorState();
}

class _ConnectionIndicatorState extends State<ConnectionIndicator>
    with TickerProviderStateMixin {
  late AnimationController _pulseController;
  late AnimationController _scaleController;
  late Animation<double> _pulseAnimation;
  late Animation<double> _scaleAnimation;

  @override
  void initState() {
    super.initState();

    // Pulse animation for connecting state
    _pulseController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1500),
    );
    _pulseAnimation = Tween<double>(begin: 1.0, end: 1.4).animate(
      CurvedAnimation(
        parent: _pulseController,
        curve: Curves.easeInOut,
      ),
    );

    // Scale animation for state transitions
    _scaleController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 300),
    );
    _scaleAnimation = Tween<double>(begin: 0.8, end: 1.0).animate(
      CurvedAnimation(
        parent: _scaleController,
        curve: Curves.elasticOut,
      ),
    );

    _scaleController.forward();

    if (widget.status == ConnectionStatus.connecting) {
      _pulseController.repeat(reverse: true);
    }
  }

  @override
  void didUpdateWidget(ConnectionIndicator oldWidget) {
    super.didUpdateWidget(oldWidget);

    if (oldWidget.status != widget.status) {
      // Trigger scale animation on state change
      _scaleController.reset();
      _scaleController.forward();

      // Handle pulse animation
      if (widget.status == ConnectionStatus.connecting) {
        _pulseController.repeat(reverse: true);
      } else {
        _pulseController.stop();
        _pulseController.reset();
      }
    }
  }

  @override
  void dispose() {
    _pulseController.dispose();
    _scaleController.dispose();
    super.dispose();
  }

  Color _getStatusColor() {
    if (widget.isNainMode) {
      return const Color(0xFFE65100); // Orange for NAIN mode
    }

    switch (widget.status) {
      case ConnectionStatus.connected:
        return const Color(0xFF2E7D32); // Green
      case ConnectionStatus.disconnected:
        return const Color(0xFFC62828); // Red
      case ConnectionStatus.connecting:
        return const Color(0xFFF57F17); // Yellow/amber
    }
  }

  IconData _getStatusIcon() {
    switch (widget.status) {
      case ConnectionStatus.connected:
        return Icons.check_circle;
      case ConnectionStatus.disconnected:
        return Icons.cancel;
      case ConnectionStatus.connecting:
        return Icons.hourglass_top;
    }
  }

  @override
  Widget build(BuildContext context) {
    final statusColor = _getStatusColor();

    return AnimatedBuilder(
      animation: Listenable.merge([_pulseAnimation, _scaleAnimation]),
      builder: (context, child) {
        final scale = _scaleAnimation.value *
            (widget.status == ConnectionStatus.connecting
                ? _pulseAnimation.value
                : 1.0);

        return Transform.scale(
          scale: scale,
          child: SizedBox(
            width: 120,
            height: 120,
            child: Stack(
              alignment: Alignment.center,
              children: [
                // Outer glow ring
                if (widget.status == ConnectionStatus.connecting)
                  Container(
                    width: 120,
                    height: 120,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      color: statusColor.withOpacity(0.1 * _pulseAnimation.value),
                    ),
                  ),

                // Pulse ring (connecting state)
                if (widget.status == ConnectionStatus.connecting)
                  Container(
                    width: 100 * _pulseAnimation.value,
                    height: 100 * _pulseAnimation.value,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      border: Border.all(
                        color: statusColor.withOpacity(0.3),
                        width: 2,
                      ),
                    ),
                  ),

                // Main circle
                Container(
                  width: 80,
                  height: 80,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    gradient: RadialGradient(
                      colors: [
                        statusColor,
                        statusColor.withOpacity(0.8),
                      ],
                      center: Alignment.center,
                      radius: 0.8,
                    ),
                    boxShadow: [
                      BoxShadow(
                        color: statusColor.withOpacity(0.4),
                        blurRadius: 20,
                        spreadRadius: 2,
                      ),
                    ],
                  ),
                  child: Icon(
                    _getStatusIcon(),
                    color: Colors.white,
                    size: 40,
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}
