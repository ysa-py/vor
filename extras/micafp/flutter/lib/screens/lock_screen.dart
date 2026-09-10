import 'dart:async';

import 'package:flutter/material.dart';
import 'package:local_auth/local_auth.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:vibration/vibration.dart';

import '../services/daemon_bridge.dart';
import 'home_screen.dart';

class LockScreen extends StatefulWidget {
  const LockScreen({super.key});

  @override
  State<LockScreen> createState() => _LockScreenState();
}

class _LockScreenState extends State<LockScreen> {
  final LocalAuthentication _localAuth = LocalAuthentication();
  final FlutterSecureStorage _secureStorage = const FlutterSecureStorage();

  String _enteredPin = '';
  int _failedAttempts = 0;
  bool _isWipeTriggered = false;
  String _displayText = 'Enter PIN';

  // Steganographic: in wipe state, show calculator UI
  bool _showCalculator = false;
  String _calcDisplay = '0';
  String _calcOperand = '';
  double? _calcPrevValue;
  String _calcOperator = '';

  @override
  void initState() {
    super.initState();
    _checkBiometrics();
  }

  Future<void> _checkBiometrics() async {
    final canAuthenticate = await _localAuth.canCheckBiometrics;
    final isDeviceSupported = await _localAuth.isDeviceSupported();

    if (canAuthenticate && isDeviceSupported) {
      // Auto-prompt biometric on launch
      _authenticateBiometric();
    }
  }

  Future<void> _authenticateBiometric() async {
    try {
      final authenticated = await _localAuth.authenticate(
        localizedReason: 'Verify your identity',
        options: const AuthenticationOptions(
          stickyAuth: true,
          biometricOnly: false,
        ),
      );

      if (authenticated && mounted) {
        _navigateToHome();
      }
    } catch (_) {
      // Biometric failed, fall back to PIN
    }
  }

  void _onDigitPress(String digit) {
    if (_isWipeTriggered || _showCalculator) {
      // In calculator mode, handle calc input
      _handleCalcDigit(digit);
      return;
    }

    setState(() {
      _enteredPin += digit;
    });

    if (_enteredPin.length == 4) {
      _validatePin();
    }
  }

  Future<void> _validatePin() async {
    final storedPin = await _secureStorage.read(key: 'user_pin');
    if (storedPin == null) {
      // No PIN set, proceed
      _navigateToHome();
      return;
    }

    if (_enteredPin == storedPin) {
      _failedAttempts = 0;
      _navigateToHome();
    } else {
      _failedAttempts++;

      // Vibrate on wrong PIN
      final canVibrate = await Vibration.hasVibrator() ?? false;
      if (canVibrate) {
        await Vibration.vibrate(duration: 200);
      }

      // TRIGGER_C: 3 wrong attempts triggers anti-forensics wipe
      if (_failedAttempts >= 3) {
        await _triggerWipe();
        return;
      }

      setState(() {
        _enteredPin = '';
        _displayText =
            'Incorrect PIN (${3 - _failedAttempts} attempts remaining)';
      });
    }
  }

  Future<void> _triggerWipe() async {
    setState(() {
      _isWipeTriggered = true;
      _showCalculator = true;
      _calcDisplay = '0';
    });

    // Notify daemon to perform wipe
    try {
      final bridge = DaemonBridge();
      await bridge.sendWipeTrigger('TRIGGER_C');
    } catch (_) {
      // Daemon may already be wiped
    }

    // Clear local secure storage
    await _secureStorage.deleteAll();

    // Clear shared preferences
    final prefs = await SharedPreferences.getInstance();
    await prefs.clear();

    // The app now looks like a calculator
  }

  void _navigateToHome() {
    Navigator.of(context).pushReplacement(
      MaterialPageRoute(
        builder: (_) => const HomeScreen(),
      ),
    );
  }

  void _onDeletePress() {
    if (_showCalculator) {
      _handleCalcDelete();
      return;
    }

    setState(() {
      if (_enteredPin.isNotEmpty) {
        _enteredPin = _enteredPin.substring(0, _enteredPin.length - 1);
      }
    });
  }

  // Calculator mode (steganographic)
  void _handleCalcDigit(String digit) {
    setState(() {
      if (_calcDisplay == '0' || _calcDisplay == 'Error') {
        _calcDisplay = digit;
      } else {
        _calcDisplay += digit;
      }
    });
  }

  void _handleCalcDelete() {
    setState(() {
      if (_calcDisplay.length > 1) {
        _calcDisplay = _calcDisplay.substring(0, _calcDisplay.length - 1);
      } else {
        _calcDisplay = '0';
      }
    });
  }

  void _handleCalcOperator(String op) {
    setState(() {
      _calcPrevValue = double.tryParse(_calcDisplay);
      _calcOperator = op;
      _calcOperand = _calcDisplay;
      _calcDisplay = '0';
    });
  }

  void _handleCalcEquals() {
    if (_calcPrevValue == null) return;
    final current = double.tryParse(_calcDisplay) ?? 0;
    double result;

    switch (_calcOperator) {
      case '+':
        result = _calcPrevValue! + current;
        break;
      case '-':
        result = _calcPrevValue! - current;
        break;
      case '×':
        result = _calcPrevValue! * current;
        break;
      case '÷':
        if (current == 0) {
          setState(() => _calcDisplay = 'Error');
          return;
        }
        result = _calcPrevValue! / current;
        break;
      default:
        return;
    }

    setState(() {
      _calcDisplay = result == result.truncateToDouble()
          ? result.toInt().toString()
          : result.toStringAsFixed(6).replaceAll(RegExp(r'0+$'), '').replaceAll(RegExp(r'\.$'), '');
      _calcPrevValue = null;
      _calcOperator = '';
    });
  }

  void _handleCalcClear() {
    setState(() {
      _calcDisplay = '0';
      _calcPrevValue = null;
      _calcOperator = '';
      _calcOperand = '';
    });
  }

  void _handleCalcPercent() {
    final val = double.tryParse(_calcDisplay) ?? 0;
    setState(() {
      _calcDisplay = (val / 100).toString();
    });
  }

  void _handleCalcNegate() {
    if (_calcDisplay.startsWith('-')) {
      setState(() {
        _calcDisplay = _calcDisplay.substring(1);
      });
    } else if (_calcDisplay != '0') {
      setState(() {
        _calcDisplay = '-$_calcDisplay';
      });
    }
  }

  void _handleCalcDot() {
    if (!_calcDisplay.contains('.')) {
      setState(() {
        _calcDisplay += '.';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_showCalculator) {
      return _buildCalculatorUI();
    }
    return _buildPinUI();
  }

  Widget _buildPinUI() {
    return Scaffold(
      backgroundColor: const Color(0xFF0F0F1A),
      body: SafeArea(
        child: Column(
          children: [
            const Spacer(flex: 2),

            // App icon (calculator appearance)
            Container(
              width: 64,
              height: 64,
              decoration: BoxDecoration(
                color: const Color(0xFF1A1A2E),
                borderRadius: BorderRadius.circular(16),
              ),
              child: const Icon(
                Icons.calculate_outlined,
                size: 32,
                color: Color(0xFFE0E0E0),
              ),
            ),
            const SizedBox(height: 16),

            // Status text
            Text(
              _displayText,
              style: const TextStyle(
                color: Color(0xFFB0BEC5),
                fontSize: 16,
              ),
            ),
            const SizedBox(height: 24),

            // PIN dots
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: List.generate(4, (index) {
                final isFilled = index < _enteredPin.length;
                return Container(
                  width: 16,
                  height: 16,
                  margin: const EdgeInsets.symmetric(horizontal: 12),
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: isFilled
                        ? const Color(0xFF2E7D32)
                        : const Color(0xFF37474F),
                  ),
                );
              }),
            ),

            const Spacer(flex: 1),

            // Biometric button
            FutureBuilder<bool>(
              future: _localAuth.canCheckBiometrics,
              builder: (context, snapshot) {
                if (snapshot.data == true) {
                  return IconButton(
                    onPressed: _authenticateBiometric,
                    icon: const Icon(Icons.fingerprint, size: 40),
                    color: const Color(0xFF2E7D32),
                    tooltip: 'Use biometrics',
                  );
                }
                return const SizedBox.shrink();
              },
            ),
            const SizedBox(height: 8),

            // Number pad
            _buildNumpad(),

            const Spacer(flex: 1),
          ],
        ),
      ),
    );
  }

  Widget _buildNumpad() {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 48),
      child: Column(
        children: [
          _buildNumpadRow(['1', '2', '3']),
          const SizedBox(height: 16),
          _buildNumpadRow(['4', '5', '6']),
          const SizedBox(height: 16),
          _buildNumpadRow(['7', '8', '9']),
          const SizedBox(height: 16),
          _buildNumpadRowWithBiometric(),
        ],
      ),
    );
  }

  Widget _buildNumpadRow(List<String> digits) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceEvenly,
      children: digits.map((d) => _buildDigitButton(d)).toList(),
    );
  }

  Widget _buildNumpadRowWithBiometric() {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceEvenly,
      children: [
        // Empty placeholder or custom action
        const SizedBox(width: 72, height: 72),
        _buildDigitButton('0'),
        _buildDeleteButton(),
      ],
    );
  }

  Widget _buildDigitButton(String digit) {
    return SizedBox(
      width: 72,
      height: 72,
      child: Material(
        color: const Color(0xFF1A1A2E),
        shape: const CircleBorder(),
        child: InkWell(
          customBorder: const CircleBorder(),
          onTap: () => _onDigitPress(digit),
          child: Center(
            child: Text(
              digit,
              style: const TextStyle(
                fontSize: 28,
                color: Color(0xFFE0E0E0),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildDeleteButton() {
    return SizedBox(
      width: 72,
      height: 72,
      child: Material(
        color: Colors.transparent,
        shape: const CircleBorder(),
        child: InkWell(
          customBorder: const CircleBorder(),
          onTap: _onDeletePress,
          child: const Center(
            child: Icon(
              Icons.backspace_outlined,
              color: Color(0xFFB0BEC5),
              size: 24,
            ),
          ),
        ),
      ),
    );
  }

  /// Calculator UI (shown after wipe for steganographic disguise)
  Widget _buildCalculatorUI() {
    return Scaffold(
      backgroundColor: const Color(0xFF0F0F1A),
      body: SafeArea(
        child: Column(
          children: [
            // Display
            Expanded(
              child: Container(
                alignment: Alignment.bottomRight,
                padding: const EdgeInsets.all(24),
                child: Text(
                  _calcDisplay,
                  style: const TextStyle(
                    fontSize: 48,
                    fontWeight: FontWeight.w300,
                    color: Color(0xFFE0E0E0),
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ),

            // Keypad
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              child: Column(
                children: [
                  _buildCalcRow([
                    _CalcBtn('AC', _handleCalcClear, const Color(0xFF616161)),
                    _CalcBtn('+/-', _handleCalcNegate, const Color(0xFF616161)),
                    _CalcBtn('%', _handleCalcPercent, const Color(0xFF616161)),
                    _CalcBtn('÷', () => _handleCalcOperator('÷'), const Color(0xFFE65100)),
                  ]),
                  const SizedBox(height: 12),
                  _buildCalcRow([
                    _CalcBtn('7', () => _handleCalcDigit('7'), const Color(0xFF37474F)),
                    _CalcBtn('8', () => _handleCalcDigit('8'), const Color(0xFF37474F)),
                    _CalcBtn('9', () => _handleCalcDigit('9'), const Color(0xFF37474F)),
                    _CalcBtn('×', () => _handleCalcOperator('×'), const Color(0xFFE65100)),
                  ]),
                  const SizedBox(height: 12),
                  _buildCalcRow([
                    _CalcBtn('4', () => _handleCalcDigit('4'), const Color(0xFF37474F)),
                    _CalcBtn('5', () => _handleCalcDigit('5'), const Color(0xFF37474F)),
                    _CalcBtn('6', () => _handleCalcDigit('6'), const Color(0xFF37474F)),
                    _CalcBtn('-', () => _handleCalcOperator('-'), const Color(0xFFE65100)),
                  ]),
                  const SizedBox(height: 12),
                  _buildCalcRow([
                    _CalcBtn('1', () => _handleCalcDigit('1'), const Color(0xFF37474F)),
                    _CalcBtn('2', () => _handleCalcDigit('2'), const Color(0xFF37474F)),
                    _CalcBtn('3', () => _handleCalcDigit('3'), const Color(0xFF37474F)),
                    _CalcBtn('+', () => _handleCalcOperator('+'), const Color(0xFFE65100)),
                  ]),
                  const SizedBox(height: 12),
                  _buildCalcRow([
                    _CalcBtn('0', () => _handleCalcDigit('0'), const Color(0xFF37474F), flex: 2),
                    _CalcBtn('.', _handleCalcDot, const Color(0xFF37474F)),
                    _CalcBtn('=', _handleCalcEquals, const Color(0xFFE65100)),
                  ]),
                  const SizedBox(height: 16),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildCalcRow(List<_CalcBtn> buttons) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceEvenly,
      children: buttons.map((btn) {
        return Expanded(
          flex: btn.flex,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 4),
            child: SizedBox(
              height: 56,
              child: Material(
                color: btn.color,
                borderRadius: BorderRadius.circular(btn.flex > 1 ? 28 : 28),
                child: InkWell(
                  borderRadius: BorderRadius.circular(28),
                  onTap: btn.onTap,
                  child: Center(
                    child: Text(
                      btn.label,
                      style: const TextStyle(
                        fontSize: 24,
                        color: Colors.white,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
      }).toList(),
    );
  }
}

class _CalcBtn {
  final String label;
  final VoidCallback onTap;
  final Color color;
  final int flex;

  const _CalcBtn(this.label, this.onTap, this.color, {this.flex = 1});
}
