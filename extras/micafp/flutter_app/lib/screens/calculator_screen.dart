import 'package:flutter/material.dart';

/// CalculatorScreen — Steganographic calculator screen.
///
/// A fully functional calculator UI that serves as the default launcher screen.
/// This disguises the app as a normal calculator to casual observers.
/// Access to VPN controls is hidden behind a special gesture or code:
/// typing the specific number sequence "7373" (S-H-I-E-L-D on T9 keypad)
/// unlocks the shield interface. The calculator never shows any VPN-related
/// UI unless unlocked.
class CalculatorScreen extends StatefulWidget {
  final VoidCallback onUnlock;

  const CalculatorScreen({
    super.key,
    required this.onUnlock,
  });

  @override
  State<CalculatorScreen> createState() => _CalculatorScreenState();
}

class _CalculatorScreenState extends State<CalculatorScreen> {
  String _display = '0';
  String _expression = '';
  double? _previousValue;
  String? _operator;
  bool _shouldResetDisplay = false;

  // Unlock sequence: typing "7373" unlocks the shield
  static const String _unlockCode = '7373';
  String _inputSequence = '';

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Theme.of(context).colorScheme.surface,
      body: SafeArea(
        child: Column(
          children: [
            // Display area
            Expanded(
              flex: 2,
              child: _buildDisplay(context),
            ),

            // Button grid
            Expanded(
              flex: 5,
              child: _buildButtonGrid(context),
            ),
          ],
        ),
      ),
    );
  }

  /// Build the calculator display area.
  Widget _buildDisplay(BuildContext context) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
      alignment: Alignment.bottomRight,
      child: Column(
        mainAxisAlignment: MainAxisAlignment.end,
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          // Expression line (smaller, muted)
          if (_expression.isNotEmpty)
            Text(
              _expression,
              style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant.withOpacity(0.6),
                  ),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          const SizedBox(height: 8),

          // Main display value
          FittedBox(
            fit: BoxFit.scaleDown,
            alignment: Alignment.centerRight,
            child: Text(
              _display,
              style: Theme.of(context).textTheme.displayLarge?.copyWith(
                    fontWeight: FontWeight.w300,
                    fontSize: 56,
                  ),
              maxLines: 1,
            ),
          ),
        ],
      ),
    );
  }

  /// Build the calculator button grid.
  Widget _buildButtonGrid(BuildContext context) {
    final buttons = [
      ['C', '±', '%', '÷'],
      ['7', '8', '9', '×'],
      ['4', '5', '6', '−'],
      ['1', '2', '3', '+'],
      ['0', '0', '.', '='],
    ];

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      child: Column(
        children: buttons.map((row) {
          return Expanded(
            child: Row(
              children: row.map((label) {
                return Expanded(
                  child: _buildButton(context, label),
                );
              }).toList(),
            ),
          );
        }).toList(),
      ),
    );
  }

  /// Build a single calculator button.
  Widget _buildButton(BuildContext context, String label) {
    final isOperator = ['÷', '×', '−', '+', '='].contains(label);
    final isFunction = ['C', '±', '%'].contains(label);
    final isZero = label == '0';

    Color backgroundColor;
    Color textColor;

    if (isOperator) {
      backgroundColor = Theme.of(context).colorScheme.primary;
      textColor = Colors.white;
    } else if (isFunction) {
      backgroundColor = Theme.of(context).colorScheme.surfaceContainerHighest;
      textColor = Theme.of(context).colorScheme.onSurface;
    } else {
      backgroundColor = Theme.of(context).colorScheme.surfaceContainer;
      textColor = Theme.of(context).colorScheme.onSurface;
    }

    return Padding(
      padding: const EdgeInsets.all(4),
      child: Material(
        color: backgroundColor,
        borderRadius: BorderRadius.circular(24),
        child: InkWell(
          borderRadius: BorderRadius.circular(24),
          onTap: () => _onButtonPressed(label),
          child: Container(
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(24),
            ),
            child: Center(
              child: Text(
                label,
                style: TextStyle(
                  fontSize: 28,
                  fontWeight: isOperator ? FontWeight.w500 : FontWeight.w400,
                  color: textColor,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  /// Handle calculator button press.
  void _onButtonPressed(String label) {
    // Track input for unlock code detection
    if (RegExp(r'^[0-9]$').hasMatch(label)) {
      _inputSequence += label;

      // Keep only the last 4 digits for comparison
      if (_inputSequence.length > _unlockCode.length) {
        _inputSequence = _inputSequence.substring(
          _inputSequence.length - _unlockCode.length,
        );
      }

      // Check for unlock code
      if (_inputSequence == _unlockCode) {
        _inputSequence = '';
        _resetCalculator();
        widget.onUnlock();
        return;
      }
    }

    // Standard calculator operations
    switch (label) {
      case 'C':
        _resetCalculator();
        break;
      case '±':
        _toggleSign();
        break;
      case '%':
        _applyPercentage();
        break;
      case '÷':
      case '×':
      case '−':
      case '+':
        _setOperator(label);
        break;
      case '=':
        _calculate();
        break;
      case '.':
        _appendDecimal();
        break;
      default:
        // Digit
        _appendDigit(label);
        break;
    }
  }

  /// Reset the calculator to initial state.
  void _resetCalculator() {
    setState(() {
      _display = '0';
      _expression = '';
      _previousValue = null;
      _operator = null;
      _shouldResetDisplay = false;
    });
  }

  /// Toggle the sign of the current display value.
  void _toggleSign() {
    setState(() {
      if (_display != '0' && _display.isNotEmpty) {
        if (_display.startsWith('-')) {
          _display = _display.substring(1);
        } else {
          _display = '-$_display';
        }
      }
    });
  }

  /// Apply percentage to the current display value.
  void _applyPercentage() {
    setState(() {
      final current = double.tryParse(_display) ?? 0;
      final result = current / 100;
      _display = _formatResult(result);
    });
  }

  /// Set the operator for the next calculation.
  void _setOperator(String op) {
    setState(() {
      if (_previousValue != null && _operator != null && !_shouldResetDisplay) {
        // Chain calculations
        _calculate();
      }

      _previousValue = double.tryParse(_display) ?? 0;
      _operator = op;
      _expression = '${_formatResult(_previousValue!)} $op';
      _shouldResetDisplay = true;
    });
  }

  /// Calculate the result of the current expression.
  void _calculate() {
    if (_previousValue == null || _operator == null) return;

    final currentValue = double.tryParse(_display) ?? 0;
    double result;

    switch (_operator) {
      case '+':
        result = _previousValue! + currentValue;
        break;
      case '−':
        result = _previousValue! - currentValue;
        break;
      case '×':
        result = _previousValue! * currentValue;
        break;
      case '÷':
        if (currentValue == 0) {
          setState(() {
            _display = 'Error';
            _expression = '';
            _previousValue = null;
            _operator = null;
            _shouldResetDisplay = true;
          });
          return;
        }
        result = _previousValue! / currentValue;
        break;
      default:
        return;
    }

    setState(() {
      _expression = '${_formatResult(_previousValue!)} $_operator ${_formatResult(currentValue)} =';
      _display = _formatResult(result);
      _previousValue = null;
      _operator = null;
      _shouldResetDisplay = true;
    });
  }

  /// Append a digit to the display.
  void _appendDigit(String digit) {
    setState(() {
      if (_shouldResetDisplay) {
        _display = digit;
        _shouldResetDisplay = false;
      } else {
        if (_display == '0' || _display == 'Error') {
          _display = digit;
        } else {
          if (_display.replaceFirst('-', '').replaceFirst('.', '').length < 12) {
            _display += digit;
          }
        }
      }
    });
  }

  /// Append a decimal point to the display.
  void _appendDecimal() {
    setState(() {
      if (_shouldResetDisplay) {
        _display = '0.';
        _shouldResetDisplay = false;
      } else if (!_display.contains('.')) {
        _display += '.';
      }
    });
  }

  /// Format a double result for display.
  String _formatResult(double value) {
    if (value == value.truncateToDouble()) {
      return value.toInt().toString();
    }

    // Limit decimal places
    String result = value.toStringAsFixed(8);

    // Remove trailing zeros
    result = result.replaceAll(RegExp(r'0+$'), '');
    result = result.replaceAll(RegExp(r'\.$'), '');

    return result;
  }
}
