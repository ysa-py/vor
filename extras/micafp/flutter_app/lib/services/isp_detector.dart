import 'dart:io';
import 'package:network_info_plus/network_info_plus.dart';
import 'package:connectivity_plus/connectivity_plus.dart';
import 'daemon_bridge.dart';

class IspInfo {
  final String name;
  final String? asn;
  final String? nameFa;
  final IspBehavior behavior;

  const IspInfo({
    required this.name,
    this.asn,
    this.nameFa,
    this.behavior = IspBehavior.normal,
  });
}

enum IspBehavior {
  normal,
  aggressiveDpi,
  throttling,
  partialBlocking,
  fullBlocking,
  nationalIntranet,
}

class IspDetector {
  final NetworkInfo _networkInfo = NetworkInfo();
  final DaemonBridge _daemonBridge = DaemonBridge();

  static const Map<String, IspInfo> _iranianIsps = {
    'AS12880': IspInfo(
      name: 'Iran Telecom (TCI)',
      asn: 'AS12880',
      nameFa: 'مخابرات ایران',
      behavior: IspBehavior.aggressiveDpi,
    ),
    'AS16322': IspInfo(
      name: 'Pars Online',
      asn: 'AS16322',
      nameFa: 'پارس آنلاین',
      behavior: IspBehavior.normal,
    ),
    'AS24631': IspInfo(
      name: 'Asre Danesh Azma',
      asn: 'AS24631',
      nameFa: 'عصر دانش آزما',
      behavior: IspBehavior.normal,
    ),
    'AS25184': IspInfo(
      name: 'Afranet',
      asn: 'AS25184',
      nameFa: 'آفرانت',
      behavior: IspBehavior.normal,
    ),
    'AS31549': IspInfo(
      name: 'Aria Shatel',
      asn: 'AS31549',
      nameFa: 'شاتل',
      behavior: IspBehavior.throttling,
    ),
    'AS34918': IspInfo(
      name: 'Iran Cell',
      asn: 'AS34918',
      nameFa: 'ایرانسل',
      behavior: IspBehavior.aggressiveDpi,
    ),
    'AS39074': IspInfo(
      name: 'Mokhaberat-e-Eslami',
      asn: 'AS39074',
      nameFa: 'مخابرات اسلامی',
      behavior: IspBehavior.aggressiveDpi,
    ),
    'AS41689': IspInfo(
      name: 'Mobin Net',
      asn: 'AS41689',
      nameFa: 'مبین نت',
      behavior: IspBehavior.partialBlocking,
    ),
    'AS44244': IspInfo(
      name: 'Iran Post',
      asn: 'AS44244',
      nameFa: 'پست ایران',
      behavior: IspBehavior.normal,
    ),
    'AS48434': IspInfo(
      name: 'Rasanet',
      asn: 'AS48434',
      nameFa: 'رسانت',
      behavior: IspBehavior.normal,
    ),
    'AS56402': IspInfo(
      name: 'Rightel',
      asn: 'AS56402',
      nameFa: 'رایتل',
      behavior: IspBehavior.throttling,
    ),
    'AS197207': IspInfo(
      name: 'Saman System',
      asn: 'AS197207',
      nameFa: 'سامان سیستم',
      behavior: IspBehavior.normal,
    ),
    'AS43754': IspInfo(
      name: 'Asiatech',
      asn: 'AS43754',
      nameFa: 'آسیاتک',
      behavior: IspBehavior.throttling,
    ),
    'AS49666': IspInfo(
      name: 'MCI',
      asn: 'AS49666',
      nameFa: 'همراه اول',
      behavior: IspBehavior.aggressiveDpi,
    ),
    'AS50810': IspInfo(
      name: 'Mobin Net Communication',
      asn: 'AS50810',
      nameFa: 'مبین نت ارتباط',
      behavior: IspBehavior.partialBlocking,
    ),
  };

  Future<IspInfo?> detectIsp() async {
    try {
      final connectivityResults = await Connectivity().checkConnectivity();
      if (connectivityResults.contains(ConnectivityResult.none)) {
        return const IspInfo(
          name: 'No Connection',
          behavior: IspBehavior.nationalIntranet,
        );
      }

      final wifiIp = await _networkInfo.getWifiIP();
      final wifiGateway = await _networkInfo.getWifiGatewayIP();

      if (wifiIp == null || wifiIp.isEmpty) {
        return _detectFromMobileNetwork();
      }

      final ipOctets = wifiIp.split('.');
      if (ipOctets.length == 4) {
        final firstOctet = int.tryParse(ipOctets[0]) ?? 0;
        final secondOctet = int.tryParse(ipOctets[1]) ?? 0;

        if (_isNationalIntranetIp(firstOctet, secondOctet, wifiGateway)) {
          return const IspInfo(
            name: 'National Intranet',
            nameFa: 'اینترنت ملی',
            behavior: IspBehavior.nationalIntranet,
          );
        }
      }

      return _detectFromMobileNetwork();
    } catch (e) {
      return null;
    }
  }

  IspInfo? _detectFromMobileNetwork() {
    return const IspInfo(
      name: 'Unknown Iranian ISP',
      nameFa: 'ارائه‌دهنده ناشناخته',
      behavior: IspBehavior.aggressiveDpi,
    );
  }

  bool _isNationalIntranetIp(int first, int second, String? gateway) {
    if (first == 10) return true;
    if (first == 172 && second >= 16 && second <= 31) return true;
    if (first == 192 && second == 168) {
      if (gateway != null && gateway.startsWith('10.')) return true;
    }
    return false;
  }

  Future<void> applyIspRules(IspInfo isp) async {
    try {
      await _daemonBridge.reportIsp(isp.name, isp.asn);

      switch (isp.behavior) {
        case IspBehavior.aggressiveDpi:
          await _daemonBridge.triggerObfuscationMode('domain_fronting');
          break;
        case IspBehavior.throttling:
          await _daemonBridge.triggerObfuscationMode('stealth');
          break;
        case IspBehavior.partialBlocking:
          await _daemonBridge.triggerObfuscationMode('full_tunnel');
          break;
        case IspBehavior.fullBlocking:
        case IspBehavior.nationalIntranet:
          await _daemonBridge.triggerObfuscationMode('shutdown_mode');
          break;
        case IspBehavior.normal:
          await _daemonBridge.triggerObfuscationMode('default');
          break;
      }
    } catch (_) {}
  }

  IspInfo? lookupByAsn(String asn) {
    return _iranianIsps[asn];
  }

  List<IspInfo> getAllKnownIsps() {
    return _iranianIsps.values.toList();
  }
}
