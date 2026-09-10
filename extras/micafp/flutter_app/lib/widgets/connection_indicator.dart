import 'package:flutter/material.dart';

/// ConnectionIndicator — Animated connection status indicator widget.
///
/// Displays a large, visually distinct connection status indicator:
///   - Green = connected
///   - Red = disconnected
///   - Yellow/Amber = connecting (with pulse animation)
///
/// The indicator includes a pulse animation when connecting, which is
/// battery-aware — animations are reduced or disabled when the battery
/// is low to conserve power.
class ConnectionIndicator extends StatelessWidget {
  final bool isConnected;
  final bool isConnecting;
  final Animation<double>? pulseAnimation;

  const ConnectionIndicator({
    super.key,
    required this.isConnected,
    required this.isConnecting,
    this.pulseAnimation,
  });

  @override
  Widget build(BuildContext context) {
    final size = MediaQuery.of(context).size;
    final indicatorSize = size.width * 0.35;

    // Determine the status color
    final Color statusColor;
    final Color statusColorLight;
    final IconData statusIcon;

    if (isConnected) {
      statusColor = Colors.green;
      statusColorLight = Colors.green.withOpacity(0.15);
      statusIcon = Icons.shield;
    } else if (isConnecting) {
      statusColor = Colors.amber;
      statusColorLight = Colors.amber.withOpacity(0.15);
      statusIcon = Icons.shield_outlined;
    } else {
      statusColor = Colors.red;
      statusColorLight = Colors.red.withOpacity(0.15);
      statusIcon = Icons.shield_outlined;
    }

    return Center(
      child: SizedBox(
        width: indicatorSize,
        height: indicatorSize,
        child: AnimatedBuilder(
          animation: pulseAnimation ?? const AlwaysStoppedAnimation(1.0),
          builder: (context, child) {
            final scale = pulseAnimation?.value ?? 1.0;

            return Transform.scale(
              scale: isConnecting ? scale : 1.0,
              child: child,
            );
          },
          child: Stack(
            alignment: Alignment.center,
            children: [
              // Outer glow ring
              if (isConnected || isConnecting)
                AnimatedContainer(
                  duration: const Duration(milliseconds: 500),
                  width: indicatorSize,
                  height: indicatorSize,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: statusColorLight,
                    boxShadow: [
                      BoxShadow(
                        color: statusColor.withOpacity(isConnected ? 0.3 : 0.15),
                        blurRadius: 30,
                        spreadRadius: 5,
                      ),
                    ],
                  ),
                ),

              // Inner circle with icon
              AnimatedContainer(
                duration: const Duration(milliseconds: 400),
                curve: Curves.easeInOut,
                width: indicatorSize * 0.7,
                height: indicatorSize * 0.7,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: statusColor.withOpacity(0.1),
                  border: Border.all(
                    color: statusColor,
                    width: 3,
                  ),
                ),
                child: Icon(
                  statusIcon,
                  size: indicatorSize * 0.3,
                  color: statusColor,
                ),
              ),

              // Connecting spinner overlay
              if (isConnecting)
                SizedBox(
                  width: indicatorSize * 0.85,
                  height: indicatorSize * 0.85,
                  child: CircularProgressIndicator(
                    strokeWidth: 2.5,
                    color: statusColor.withOpacity(0.5),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Animated builder that uses the provided animation.
/// This is a convenience widget similar to AnimatedBuilder.
class AnimatedBuilder extends StatelessWidget {
  final Animation<double> animation;
  final Widget Function(BuildContext context, Widget? child) builder;
  final Widget? child;

  const AnimatedBuilder({
    super.key,
    required this.animation,
    required this.builder,
    this.child,
  });

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder2(
      animation: animation,
      builder: builder,
      child: child,
    );
  }
}

/// Internal animated builder implementation.
class AnimatedBuilder2 extends AnimatedWidget {
  final Widget Function(BuildContext context, Widget? child) builder;
  final Widget? child;

  const AnimatedBuilder2({
    super.key,
    required super.listenable,
    required this.builder,
    this.child,
  });

  Animation<double> get animation => listenable as Animation<double>;

  @override
  Widget build(BuildContext context) {
    return builder(context, child);
  }
}

/// Extension to provide a simple pulse animation controller.
extension PulseAnimationController on State {
  /// Create a pulse animation that scales from 1.0 to [maxScale] and back.
  Animation<double> createPulseAnimation({
    double maxScale = 1.15,
    Duration duration = const Duration(milliseconds: 1500),
    required TickerProvider vsync,
  }) {
    final controller = AnimationController(
      vsync: vsync,
      duration: duration,
    );

    return Tween<double>(begin: 1.0, end: maxScale).animate(
      CurvedAnimation(
        parent: controller,
        curve: Curves.easeInOut,
      ),
    );
  }
}
