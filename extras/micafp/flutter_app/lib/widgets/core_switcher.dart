import 'package:flutter/material.dart';

import '../models/core_model.dart';

class QuickCoreSwitcher extends StatefulWidget {
  final String currentCore;
  final ValueChanged<String> onCoreSelected;

  const QuickCoreSwitcher({
    super.key,
    required this.currentCore,
    required this.onCoreSelected,
  });

  @override
  State<QuickCoreSwitcher> createState() => _QuickCoreSwitcherState();
}

class _QuickCoreSwitcherState extends State<QuickCoreSwitcher>
    with TickerProviderStateMixin {
  late AnimationController _switchController;
  late Animation<double> _fadeAnimation;
  late Animation<Offset> _slideAnimation;
  String? _previousCore;

  static const Map<String, IconData> _coreIcons = {
    'warp': Icons.cloud,
    'xray': Icons.bolt,
    'hysteria': Icons.speed,
    'naive': Icons.visibility_off,
    'tuic': Icons.swap_horiz,
    'psiphon': Icons.shield,
    'outline': Icons.outlined_flag,
    'meek': Icons.cloud_queue,
    'snowflake': Icons.ac_unit,
  };

  static const Map<String, Color> _coreColors = {
    'warp': Colors.orange,
    'xray': Colors.purple,
    'hysteria': Colors.red,
    'naive': Colors.teal,
    'tuic': Colors.indigo,
    'psiphon': Colors.green,
    'outline': Colors.blue,
    'meek': Colors.cyan,
    'snowflake': Colors.lightBlue,
  };

  @override
  void initState() {
    super.initState();
    _switchController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 400),
    );
    _fadeAnimation = Tween<double>(begin: 0.0, end: 1.0).animate(
      CurvedAnimation(parent: _switchController, curve: Curves.easeIn),
    );
    _slideAnimation = Tween<Offset>(
      begin: const Offset(0.3, 0.0),
      end: Offset.zero,
    ).animate(
      CurvedAnimation(parent: _switchController, curve: Curves.easeOutCubic),
    );
    _switchController.forward();
    _previousCore = widget.currentCore;
  }

  @override
  void didUpdateWidget(QuickCoreSwitcher oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.currentCore != widget.currentCore) {
      _previousCore = oldWidget.currentCore;
      _switchController.reset();
      _switchController.forward();
    }
  }

  @override
  void dispose() {
    _switchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final currentCore = defaultCores.firstWhere(
      (c) => c.id == widget.currentCore,
      orElse: () => defaultCores.first,
    );

    return Column(
      children: [
        Text(
          'Quick Switch',
          style: Theme.of(context).textTheme.titleSmall?.copyWith(
                color: Colors.grey[400],
              ),
        ),
        const SizedBox(height: 12),
        // Horizontal scrollable core buttons
        SizedBox(
          height: 60,
          child: ListView.separated(
            scrollDirection: Axis.horizontal,
            itemCount: defaultCores.length,
            separatorBuilder: (_, __) => const SizedBox(width: 8),
            itemBuilder: (context, index) {
              final core = defaultCores[index];
              final isActive = core.id == widget.currentCore;
              final color = _coreColors[core.id] ?? Colors.grey;

              return AnimatedContainer(
                duration: const Duration(milliseconds: 300),
                curve: Curves.easeInOut,
                child: Material(
                  color: isActive ? color.withOpacity(0.2) : Colors.transparent,
                  borderRadius: BorderRadius.circular(12),
                  child: InkWell(
                    borderRadius: BorderRadius.circular(12),
                    onTap: isActive ? null : () => widget.onCoreSelected(core.id),
                    child: AnimatedContainer(
                      duration: const Duration(milliseconds: 300),
                      padding: const EdgeInsets.symmetric(
                        horizontal: 14,
                        vertical: 8,
                      ),
                      decoration: BoxDecoration(
                        borderRadius: BorderRadius.circular(12),
                        border: Border.all(
                          color: isActive ? color : Colors.grey.withOpacity(0.3),
                          width: isActive ? 2 : 1,
                        ),
                      ),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(
                            _coreIcons[core.id] ?? Icons.circle,
                            size: 18,
                            color: isActive ? color : Colors.grey[400],
                          ),
                          const SizedBox(width: 6),
                          Text(
                            core.name,
                            style: TextStyle(
                              fontSize: 12,
                              color: isActive ? color : Colors.grey[400],
                              fontWeight: isActive ? FontWeight.bold : FontWeight.normal,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              );
            },
          ),
        ),
        const SizedBox(height: 16),
        // Current core info with animated transition
        SlideTransition(
          position: _slideAnimation,
          child: FadeTransition(
            opacity: _fadeAnimation,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
              decoration: BoxDecoration(
                color: (_coreColors[currentCore.id] ?? Colors.indigo).withOpacity(0.1),
                borderRadius: BorderRadius.circular(12),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(
                    _coreIcons[currentCore.id] ?? Icons.circle,
                    color: _coreColors[currentCore.id] ?? Colors.indigo,
                    size: 20,
                  ),
                  const SizedBox(width: 8),
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        currentCore.name,
                        style: const TextStyle(
                          fontWeight: FontWeight.bold,
                          fontSize: 13,
                        ),
                      ),
                      Text(
                        currentCore.protocols.join(', '),
                        style: TextStyle(
                          fontSize: 10,
                          color: Colors.grey[400],
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }
}
