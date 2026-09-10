import 'package:freezed_annotation/freezed_annotation.dart';

part 'core_model.g.dart';

enum CoreStatus {
  @JsonValue('connected')
  connected,
  @JsonValue('disconnected')
  disconnected,
  @JsonValue('connecting')
  connecting,
  @JsonValue('error')
  error,
  @JsonValue('standby')
  standby,
}

@freezed
class HealthStatus with _$HealthStatus {
  const factory HealthStatus({
    @Default(0) int latency,
    @Default(0.0) double packetLoss,
    @Default(false) bool blocked,
    @Default(false) bool dnsLeak,
    @Default(0.0) double dpiExposure,
    @Default(0) int bandwidth,
  }) = _HealthStatus;

  factory HealthStatus.fromJson(Map<String, dynamic> json) =>
      _$HealthStatusFromJson(json);
}

@freezed
class CoreAdapter with _$CoreAdapter {
  const factory CoreAdapter({
    required String id,
    required String name,
    @Default('') String nameFa,
    @Default('1.0.0') String version,
    @Default(CoreStatus.standby) CoreStatus status,
    @Default(HealthStatus()) HealthStatus health,
    @Default([]) List<String> protocols,
    @Default([]) List<String> capabilities,
  }) = _CoreAdapter;

  factory CoreAdapter.fromJson(Map<String, dynamic> json) =>
      _$CoreAdapterFromJson(json);
}

extension CoreAdapterX on CoreAdapter {
  double get score {
    if (health.blocked) return 0.0;
    if (status == CoreStatus.error) return 0.0;

    double latencyScore = 0;
    if (health.latency > 0) {
      latencyScore = (1.0 - (health.latency / 1000).clamp(0.0, 1.0)) * 30;
    }

    double packetLossScore = (1.0 - health.packetLoss) * 25;

    double dpiScore = (1.0 - health.dpiExposure) * 25;

    double dnsScore = health.dnsLeak ? 0.0 : 20.0;

    return (latencyScore + packetLossScore + dpiScore + dnsScore).clamp(0.0, 100.0);
  }

  String get statusText {
    switch (status) {
      case CoreStatus.connected:
        return 'Connected';
      case CoreStatus.disconnected:
        return 'Disconnected';
      case CoreStatus.connecting:
        return 'Connecting';
      case CoreStatus.error:
        return 'Error';
      case CoreStatus.standby:
        return 'Standby';
    }
  }

  String get statusTextFa {
    switch (status) {
      case CoreStatus.connected:
        return 'متصل';
      case CoreStatus.disconnected:
        return 'قطع';
      case CoreStatus.connecting:
        return 'در حال اتصال';
      case CoreStatus.error:
        return 'خطا';
      case CoreStatus.standby:
        return 'آماده';
    }
  }
}

const List<CoreAdapter> defaultCores = [
  CoreAdapter(
    id: 'warp',
    name: 'Cloudflare WARP',
    nameFa: 'کلودفلر وارپ',
    version: '1.0.0',
    protocols: ['wireguard', 'warp'],
    capabilities: ['ipv4', 'ipv6', 'split_tunnel'],
  ),
  CoreAdapter(
    id: 'xray',
    name: 'Xray-core',
    nameFa: 'ایکس‌ری',
    version: '1.8.0',
    protocols: ['vless', 'vmess', 'trojan', 'shadowsocks'],
    capabilities: ['xhttp', 'splithttp', 'ws', 'grpc', 'tcp', 'reality'],
  ),
  CoreAdapter(
    id: 'hysteria',
    name: 'Hysteria 2',
    nameFa: 'هیستریا ۲',
    version: '2.0.0',
    protocols: ['hysteria2', 'quic'],
    capabilities: ['udp_relay', 'bandwidth_control'],
  ),
  CoreAdapter(
    id: 'naive',
    name: 'NaïveProxy',
    nameFa: 'نایو پروکسی',
    version: '1.0.0',
    protocols: ['http_proxy', 'https_proxy'],
    capabilities: ['domain_fronting', 'chrome_fingerprint'],
  ),
  CoreAdapter(
    id: 'tuic',
    name: 'TUIC',
    nameFa: 'توئیک',
    version: '1.0.0',
    protocols: ['tuic', 'quic'],
    capabilities: ['udp_relay', 'congestion_control'],
  ),
  CoreAdapter(
    id: 'psiphon',
    name: 'Psiphon',
    nameFa: 'سایفون',
    version: '1.0.0',
    protocols: ['ssh', 'obfs4', 'meek'],
    capabilities: ['domain_fronting', 'obfuscation', 'shutdown_resistant'],
  ),
  CoreAdapter(
    id: 'outline',
    name: 'Outline Shadowsocks',
    nameFa: 'اوتلاین',
    version: '1.0.0',
    protocols: ['shadowsocks'],
    capabilities: ['transport_encryption'],
  ),
  CoreAdapter(
    id: 'meek',
    name: 'Meek Lite',
    nameFa: 'میک لایت',
    version: '1.0.0',
    protocols: ['meek', 'domain_fronting'],
    capabilities: ['domain_fronting', 'cdn_relay'],
  ),
  CoreAdapter(
    id: 'snowflake',
    name: 'Snowflake',
    nameFa: 'اسنوفلیک',
    version: '1.0.0',
    protocols: ['webrtc', 'kcp'],
    capabilities: ['webrtc_relay', 'p2p', 'shutdown_resistant'],
  ),
];
