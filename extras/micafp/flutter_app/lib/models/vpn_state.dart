import 'package:freezed_annotation/freezed_annotation.dart';

part 'vpn_state.g.dart';

enum ConnectionState {
  @JsonValue('connected')
  connected,
  @JsonValue('disconnected')
  disconnected,
  @JsonValue('connecting')
  connecting,
  @JsonValue('error')
  error,
}

@freezed
class VpnState with _$VpnState {
  const factory VpnState({
    @Default(ConnectionState.disconnected) ConnectionState connectionState,
    @Default('warp') String activeCore,
    @Default([]) List<String> shadowConnections,
    @Default(0) int uploadSpeed,
    @Default(0) int downloadSpeed,
    @Default(false) bool nationalIntranetMode,
    @Default(true) bool killSwitchEnabled,
    @Default('') String errorMessage,
    @Default('') String currentIsp,
    @Default('') String obfuscationMode,
  }) = _VpnState;

  factory VpnState.initial() => const VpnState();

  factory VpnState.fromJson(Map<String, dynamic> json) =>
      _$VpnStateFromJson(json);
}

extension VpnStateX on VpnState {
  String get connectionText {
    switch (connectionState) {
      case ConnectionState.connected:
        return 'Connected';
      case ConnectionState.disconnected:
        return 'Disconnected';
      case ConnectionState.connecting:
        return 'Connecting...';
      case ConnectionState.error:
        return 'Error';
    }
  }

  String get connectionTextFa {
    switch (connectionState) {
      case ConnectionState.connected:
        return 'متصل';
      case ConnectionState.disconnected:
        return 'قطع';
      case ConnectionState.connecting:
        return 'در حال اتصال...';
      case ConnectionState.error:
        return 'خطا';
    }
  }

  String get uploadSpeedText => _formatSpeed(uploadSpeed);
  String get downloadSpeedText => _formatSpeed(downloadSpeed);

  String _formatSpeed(int bytesPerSecond) {
    if (bytesPerSecond < 1024) return '$bytesPerSecond B/s';
    if (bytesPerSecond < 1024 * 1024) {
      return '${(bytesPerSecond / 1024).toStringAsFixed(1)} KB/s';
    }
    return '${(bytesPerSecond / (1024 * 1024)).toStringAsFixed(1)} MB/s';
  }

  bool get isConnected => connectionState == ConnectionState.connected;
  bool get isConnecting => connectionState == ConnectionState.connecting;
  bool get isDisconnected => connectionState == ConnectionState.disconnected;
  bool get hasError => connectionState == ConnectionState.error;
}
