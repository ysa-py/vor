// ──────────────────────────────────────────────
// MICAFP-UnifiedShield — Geographic Routing Engine
// Server country detection, latency mapping, auto-select,
// geographic load balancing, health checking, and
// country-specific bypass rules — all with Persian labels
// ──────────────────────────────────────────────

import type {
  ServerCountry,
  GeoLatencyMap,
  GeoRouterState,
  LoadBalancingState,
} from './unified-shield-types';

// ──────────────────────────────────────────────
// Server Country Definitions (enhanced from COUNTRY_SERVERS)
// ──────────────────────────────────────────────
const SERVER_COUNTRY_DEFINITIONS: Omit<ServerCountry, 'activeServers' | 'avgLatencyMs' | 'loadPercent' | 'isHealthy' | 'lastPingMs' | 'lastChecked' | 'bandwidthCapacity' | 'currentLoad'>[] = [
  {
    code: 'DE',
    name: 'Germany',
    nameFa: 'آلمان',
    servers: 24,
    supportsIranBypass: true,
    features: ['VLESS Reality', 'Hysteria2', 'AmneziaWG', 'VMess WS'],
    featuresFa: ['VLESS Reality', 'هیستریا۲', 'آمنزیاوی‌جی', 'VMess WS'],
  },
  {
    code: 'NL',
    name: 'Netherlands',
    nameFa: 'هلند',
    servers: 18,
    supportsIranBypass: true,
    features: ['VLESS Reality', 'Trojan gRPC', 'ShadowTLS', 'TUIC v5'],
    featuresFa: ['VLESS Reality', 'تروجان gRPC', 'شدوتی‌ال‌اس', 'TUIC نسخه ۵'],
  },
  {
    code: 'FI',
    name: 'Finland',
    nameFa: 'فنلاند',
    servers: 8,
    supportsIranBypass: true,
    features: ['VLESS Reality', 'WireGuard+Noise', 'Hysteria2'],
    featuresFa: ['VLESS Reality', 'وایرگارد+نویز', 'هیستریا۲'],
  },
  {
    code: 'SE',
    name: 'Sweden',
    nameFa: 'سوئد',
    servers: 6,
    supportsIranBypass: true,
    features: ['AmneziaWG', 'VLESS Fragment', 'NaiveProxy'],
    featuresFa: ['آمنزیاوی‌جی', 'VLESS Fragment', 'نایوپروکسی'],
  },
  {
    code: 'FR',
    name: 'France',
    nameFa: 'فرانسه',
    servers: 12,
    supportsIranBypass: true,
    features: ['VLESS Reality', 'VMess WS', 'Trojan', 'Hysteria2'],
    featuresFa: ['VLESS Reality', 'VMess WS', 'تروجان', 'هیستریا۲'],
  },
  {
    code: 'US',
    name: 'USA',
    nameFa: 'آمریکا',
    servers: 30,
    supportsIranBypass: true,
    features: ['VLESS Reality', 'AmneziaWG', 'Trojan', 'Hysteria2', 'VMess'],
    featuresFa: ['VLESS Reality', 'آمنزیاوی‌جی', 'تروجان', 'هیستریا۲', 'VMess'],
  },
  {
    code: 'CA',
    name: 'Canada',
    nameFa: 'کانادا',
    servers: 10,
    supportsIranBypass: true,
    features: ['VLESS Reality', 'AmneziaWG', 'Hysteria2'],
    featuresFa: ['VLESS Reality', 'آمنزیاوی‌جی', 'هیستریا۲'],
  },
  {
    code: 'GB',
    name: 'UK',
    nameFa: 'انگلستان',
    servers: 14,
    supportsIranBypass: true,
    features: ['VLESS Reality', 'VMess WS', 'Trojan gRPC'],
    featuresFa: ['VLESS Reality', 'VMess WS', 'تروجان gRPC'],
  },
  {
    code: 'JP',
    name: 'Japan',
    nameFa: 'ژاپن',
    servers: 8,
    supportsIranBypass: true,
    features: ['VLESS Reality', 'Hysteria2', 'WireGuard+Noise'],
    featuresFa: ['VLESS Reality', 'هیستریا۲', 'وایرگارد+نویز'],
  },
  {
    code: 'KR',
    name: 'South Korea',
    nameFa: 'کره جنوبی',
    servers: 6,
    supportsIranBypass: true,
    features: ['VLESS Reality', 'ShadowTLS', 'VMess'],
    featuresFa: ['VLESS Reality', 'شدوتی‌ال‌اس', 'VMess'],
  },
  {
    code: 'SG',
    name: 'Singapore',
    nameFa: 'سنگاپور',
    servers: 10,
    supportsIranBypass: true,
    features: ['VLESS Reality', 'Hysteria2', 'Trojan'],
    featuresFa: ['VLESS Reality', 'هیستریا۲', 'تروجان'],
  },
  {
    code: 'AU',
    name: 'Australia',
    nameFa: 'استرالیا',
    servers: 5,
    supportsIranBypass: false,
    features: ['VLESS Reality', 'VMess'],
    featuresFa: ['VLESS Reality', 'VMess'],
  },
  {
    code: 'BR',
    name: 'Brazil',
    nameFa: 'برزیل',
    servers: 4,
    supportsIranBypass: false,
    features: ['VLESS Reality'],
    featuresFa: ['VLESS Reality'],
  },
  {
    code: 'IN',
    name: 'India',
    nameFa: 'هند',
    servers: 8,
    supportsIranBypass: false,
    features: ['VMess WS', 'Trojan'],
    featuresFa: ['VMess WS', 'تروجان'],
  },
  {
    code: 'TR',
    name: 'Turkey',
    nameFa: 'ترکیه',
    servers: 16,
    supportsIranBypass: true,
    features: ['VLESS Reality', 'Hysteria2', 'AmneziaWG', 'MoaV Tunnel'],
    featuresFa: ['VLESS Reality', 'هیستریا۲', 'آمنزیاوی‌جی', 'تونل موآوی'],
  },
  {
    code: 'AE',
    name: 'UAE',
    nameFa: 'امارات',
    servers: 6,
    supportsIranBypass: true,
    features: ['VLESS Reality', 'ShadowTLS', 'Trojan'],
    featuresFa: ['VLESS Reality', 'شدوتی‌ال‌اس', 'تروجان'],
  },
  {
    code: 'CH',
    name: 'Switzerland',
    nameFa: 'سوئیس',
    servers: 8,
    supportsIranBypass: true,
    features: ['VLESS Reality', 'AmneziaWG', 'WireGuard+Noise'],
    featuresFa: ['VLESS Reality', 'آمنزیاوی‌جی', 'وایرگارد+نویز'],
  },
  {
    code: 'NO',
    name: 'Norway',
    nameFa: 'نروژ',
    servers: 4,
    supportsIranBypass: true,
    features: ['VLESS Reality', 'Hysteria2'],
    featuresFa: ['VLESS Reality', 'هیستریا۲'],
  },
  {
    code: 'PL',
    name: 'Poland',
    nameFa: 'لهستان',
    servers: 6,
    supportsIranBypass: true,
    features: ['VMess WS', 'Trojan', 'VLESS Fragment'],
    featuresFa: ['VMess WS', 'تروجان', 'VLESS Fragment'],
  },
  {
    code: 'ES',
    name: 'Spain',
    nameFa: 'اسپانیا',
    servers: 5,
    supportsIranBypass: true,
    features: ['VLESS Reality', 'Hysteria2'],
    featuresFa: ['VLESS Reality', 'هیستریا۲'],
  },
];

// ──────────────────────────────────────────────
// Latency Mapping from Iran to each country
// Based on realistic geographic distance and peering
// ──────────────────────────────────────────────
const BASE_LATENCY_FROM_IRAN: Record<string, number> = {
  DE: 72, NL: 78, FI: 85, SE: 88, FR: 80, US: 155, CA: 162,
  GB: 90, JP: 195, KR: 205, SG: 175, AU: 260, BR: 310,
  IN: 55, TR: 32, AE: 48, CH: 76, NO: 90, PL: 68, ES: 84,
};

// ──────────────────────────────────────────────
// ISP-based auto-select recommendation
// ──────────────────────────────────────────────
const ISP_COUNTRY_RECOMMENDATIONS: Record<string, { country: string; countryFa: string; reason: string; reasonFa: string }> = {
  mci: {
    country: 'DE',
    countryFa: 'آلمان',
    reason: 'Best VLESS Reality performance on MCI network',
    reasonFa: 'بهترین عملکرد VLESS Reality روی شبکه همراه اول',
  },
  irancell: {
    country: 'NL',
    countryFa: 'هلند',
    reason: 'Optimal Hysteria2 and VLESS routing through Irancell',
    reasonFa: 'مسیریابی بهینه هیستریا۲ و VLESS از طریق ایرانسل',
  },
  shatel: {
    country: 'DE',
    countryFa: 'آلمان',
    reason: 'AmneziaWG performs best on Shatel with German servers',
    reasonFa: 'آمنزیاوی‌جی بهترین عملکرد را روی شتل با سرورهای آلمانی دارد',
  },
  asiatech: {
    country: 'TR',
    countryFa: 'ترکیه',
    reason: 'Lowest latency to Turkey from Asiatech network',
    reasonFa: 'کمترین تأخیر تا ترکیه از شبکه آسیاتک',
  },
  rightel: {
    country: 'NL',
    countryFa: 'هلند',
    reason: 'VLESS Reality stable on Rightel via Netherlands',
    reasonFa: 'VLESS Reality پایدار روی رایتل از طریق هلند',
  },
  default: {
    country: 'DE',
    countryFa: 'آلمان',
    reason: 'Best overall bypass performance and server availability',
    reasonFa: 'بهترین عملکرد عبور و دسترسی به سرور به‌طور کلی',
  },
};

// ──────────────────────────────────────────────
// Simulate server health ping
// ──────────────────────────────────────────────
export function simulatePing(countryCode: string): number {
  const baseLatency = BASE_LATENCY_FROM_IRAN[countryCode] ?? 150;
  const jitter = Math.random() * 20 - 10;
  return Math.max(10, Math.round(baseLatency + jitter));
}

// ──────────────────────────────────────────────
// Build full server list with live stats
// ──────────────────────────────────────────────
export function buildServerList(): ServerCountry[] {
  return SERVER_COUNTRY_DEFINITIONS.map((def) => {
    const lastPingMs = simulatePing(def.code);
    const activeServers = Math.max(
      1,
      Math.floor(def.servers * (0.7 + Math.random() * 0.25)),
    );
    const currentLoad = Math.round((20 + Math.random() * 60) * 10) / 10;
    const bandwidthCapacity = def.servers * 500; // 500 Mbps per server
    const isHealthy = activeServers > 0 && lastPingMs < 300;

    return {
      ...def,
      activeServers,
      avgLatencyMs: lastPingMs,
      loadPercent: Math.round((currentLoad / bandwidthCapacity) * 1000) / 10,
      isHealthy,
      lastPingMs,
      lastChecked: Date.now(),
      bandwidthCapacity,
      currentLoad,
    };
  });
}

// ──────────────────────────────────────────────
// Build latency map for all countries
// ──────────────────────────────────────────────
export function buildLatencyMap(): GeoLatencyMap {
  const map: GeoLatencyMap = {};
  for (const def of SERVER_COUNTRY_DEFINITIONS) {
    const baseLatency = BASE_LATENCY_FROM_IRAN[def.code] ?? 150;
    map[def.code] = {
      latencyMs: Math.round(baseLatency + Math.random() * 15 - 7),
      jitterMs: Math.round((2 + Math.random() * 8) * 10) / 10,
      packetLoss: Math.round(Math.random() * 1.5 * 100) / 100,
      lastMeasured: Date.now(),
    };
  }
  return map;
}

// ──────────────────────────────────────────────
// Auto-select best server country for current ISP
// ──────────────────────────────────────────────
export function autoSelectBestCountry(
  detectedISP: string,
  serverList: ServerCountry[],
): {
  countryCode: string;
  countryNameFa: string;
  reason: string;
  reasonFa: string;
} {
  const recommendation = ISP_COUNTRY_RECOMMENDATIONS[detectedISP] ?? ISP_COUNTRY_RECOMMENDATIONS['default'];

  // Verify the recommended country has healthy servers
  const recommended = serverList.find((s) => s.code === recommendation.country);
  if (recommended && recommended.isHealthy) {
    return {
      countryCode: recommendation.country,
      countryNameFa: recommendation.countryFa,
      reason: recommendation.reason,
      reasonFa: recommendation.reasonFa,
    };
  }

  // Fallback: find the healthiest server with lowest latency
  const healthyServers = serverList
    .filter((s) => s.isHealthy && s.supportsIranBypass)
    .sort((a, b) => a.avgLatencyMs - b.avgLatencyMs);

  if (healthyServers.length > 0) {
    const best = healthyServers[0];
    return {
      countryCode: best.code,
      countryNameFa: best.nameFa,
      reason: `Lowest latency healthy server (${best.avgLatencyMs}ms)`,
      reasonFa: `کمترین تأخیر سرور سالم (${best.avgLatencyMs} میلی‌ثانیه)`,
    };
  }

  return {
    countryCode: 'DE',
    countryNameFa: 'آلمان',
    reason: 'Default fallback — most reliable servers',
    reasonFa: 'بکاپ پیش‌فرض — قابل‌اعتمادترین سرورها',
  };
}

// ──────────────────────────────────────────────
// Geographic Load Balancing
// ──────────────────────────────────────────────
const STRATEGY_LABELS: Record<LoadBalancingState['strategy'], string> = {
  'round-robin': 'نوبت‌چرخشی',
  'least-connections': 'کمترین اتصال',
  'lowest-latency': 'کمترین تأخیر',
  'weighted': 'وزنی',
};

export function calculateLoadDistribution(
  serverList: ServerCountry[],
  strategy: LoadBalancingState['strategy'],
): Record<string, number> {
  const healthyServers = serverList.filter((s) => s.isHealthy);
  if (healthyServers.length === 0) return {};

  const distribution: Record<string, number> = {};

  switch (strategy) {
    case 'round-robin': {
      const share = Math.round((100 / healthyServers.length) * 10) / 10;
      for (const s of healthyServers) {
        distribution[s.code] = share;
      }
      break;
    }
    case 'least-connections': {
      // Distribute more to servers with lower current load
      const totalInverseLoad = healthyServers.reduce(
        (sum, s) => sum + (100 - s.loadPercent),
        0,
      );
      for (const s of healthyServers) {
        const weight = (100 - s.loadPercent) / totalInverseLoad;
        distribution[s.code] = Math.round(weight * 1000) / 10;
      }
      break;
    }
    case 'lowest-latency': {
      // Distribute more to servers with lower latency
      const totalInverseLatency = healthyServers.reduce(
        (sum, s) => sum + (1 / s.avgLatencyMs),
        0,
      );
      for (const s of healthyServers) {
        const weight = 1 / s.avgLatencyMs / totalInverseLatency;
        distribution[s.code] = Math.round(weight * 1000) / 10;
      }
      break;
    }
    case 'weighted': {
      // Weight by: 40% latency, 30% load, 20% server count, 10% iran bypass support
      const weights: { code: string; weight: number }[] = healthyServers.map((s) => {
        const latencyW = Math.max(0, (300 - s.avgLatencyMs) / 300) * 0.4;
        const loadW = Math.max(0, (100 - s.loadPercent) / 100) * 0.3;
        const countW = (s.servers / 30) * 0.2;
        const bypassW = (s.supportsIranBypass ? 1 : 0.3) * 0.1;
        return { code: s.code, weight: latencyW + loadW + countW + bypassW };
      });
      const totalWeight = weights.reduce((sum, w) => sum + w.weight, 0);
      for (const w of weights) {
        distribution[w.code] = Math.round((w.weight / totalWeight) * 1000) / 10;
      }
      break;
    }
  }

  return distribution;
}

// ──────────────────────────────────────────────
// Country-specific bypass rules
// Iran internal traffic should go direct, external via VPN
// ──────────────────────────────────────────────
export interface BypassRule {
  target: string;
  targetFa: string;
  action: 'vpn' | 'direct';
  actionFa: string;
  reason: string;
  reasonFa: string;
}

export function getIranBypassRules(): BypassRule[] {
  return [
    {
      target: 'Iranian websites (.ir)',
      targetFa: 'وبسایت‌های ایرانی (.ir)',
      action: 'direct',
      actionFa: 'مستقیم',
      reason: 'No need for VPN — Iranian sites are accessible directly',
      reasonFa: 'نیازی به VPN نیست — سایت‌های ایرانی مستقیماً قابل دسترسی هستند',
    },
    {
      target: 'Iranian banking apps',
      targetFa: 'اپلیکیشن‌های بانکی ایرانی',
      action: 'direct',
      actionFa: 'مستقیم',
      reason: 'Banks block foreign IPs for security',
      reasonFa: 'بانک‌ها برای امنیت IP‌های خارجی را مسدود می‌کنند',
    },
    {
      target: 'Iranian government services',
      targetFa: 'سرویس‌های دولتی ایران',
      action: 'direct',
      actionFa: 'مستقیم',
      reason: 'Government services require Iranian IP',
      reasonFa: 'سرویس‌های دولتی نیازمند IP ایرانی هستند',
    },
    {
      target: 'Social media (Instagram, Twitter)',
      targetFa: 'شبکه‌های اجتماعی (اینستاگرام، توییتر)',
      action: 'vpn',
      actionFa: 'VPN',
      reason: 'Blocked by Iran national filtering',
      reasonFa: 'مسدودشده توسط فیلترینگ ملی ایران',
    },
    {
      target: 'Messaging (Telegram, WhatsApp, Signal)',
      targetFa: 'پیام‌رسان‌ها (تلگرام، واتساپ، سیگنال)',
      action: 'vpn',
      actionFa: 'VPN',
      reason: 'Partially or fully blocked in Iran',
      reasonFa: 'جزئیاً یا کاملاً مسدود در ایران',
    },
    {
      target: 'Streaming (YouTube, Netflix)',
      targetFa: 'استریمینگ (یوتیوب، نتفلیکس)',
      action: 'vpn',
      actionFa: 'VPN',
      reason: 'Blocked and requires foreign IP',
      reasonFa: 'مسدود و نیازمند IP خارجی',
    },
    {
      target: 'Developer tools (GitHub, Stack Overflow)',
      targetFa: 'ابزارهای توسعه (گیت‌هاب، استک‌اورفلو)',
      action: 'vpn',
      actionFa: 'VPN',
      reason: 'Intermittently throttled or blocked',
      reasonFa: 'گاهی کند یا مسدود می‌شود',
    },
  ];
}

// ──────────────────────────────────────────────
// Full GeoRouter State Builder
// ──────────────────────────────────────────────
export function buildInitialGeoRouterState(
  detectedISP: string = 'irancell',
): GeoRouterState {
  const serverList = buildServerList();
  const latencyMap = buildLatencyMap();
  const strategy: LoadBalancingState['strategy'] = 'weighted';
  const loadBalancing: LoadBalancingState = {
    strategy,
    strategyFa: STRATEGY_LABELS[strategy],
    enabled: true,
    currentDistribution: calculateLoadDistribution(serverList, strategy),
  };

  const autoSelect = autoSelectBestCountry(detectedISP, serverList);

  return {
    selectedCountry: autoSelect.countryCode,
    selectedCountryFa: autoSelect.countryNameFa,
    serverList,
    latencyMap,
    loadBalancing,
    autoSelectEnabled: true,
    iranInternalBypass: true,
    healthCheckInterval: 30000,
    lastHealthCheck: Date.now(),
    recommendedCountry: autoSelect.countryCode,
    recommendedCountryFa: autoSelect.countryNameFa,
    recommendationReason: autoSelect.reason,
    recommendationReasonFa: autoSelect.reasonFa,
  };
}

// ──────────────────────────────────────────────
// Select a specific server country
// ──────────────────────────────────────────────
export function selectServerCountry(
  state: GeoRouterState,
  countryCode: string,
): GeoRouterState {
  const server = state.serverList.find((s) => s.code === countryCode);
  if (!server) return state;

  // Recalculate load distribution when changing country
  const newDistribution = { ...state.loadBalancing.currentDistribution };
  // Give selected country a larger share
  const totalCurrentShare = newDistribution[countryCode] ?? 0;
  const boost = 40;
  newDistribution[countryCode] = Math.round((totalCurrentShare + boost) * 10) / 10;

  // Reduce other countries proportionally
  const otherKeys = Object.keys(newDistribution).filter((k) => k !== countryCode);
  const reduction = boost / otherKeys.length;
  for (const key of otherKeys) {
    newDistribution[key] = Math.max(0, Math.round((newDistribution[key] - reduction) * 10) / 10);
  }

  return {
    ...state,
    selectedCountry: countryCode,
    selectedCountryFa: server.nameFa,
    loadBalancing: {
      ...state.loadBalancing,
      currentDistribution: newDistribution,
    },
  };
}

// ──────────────────────────────────────────────
// Run health check on all servers
// ──────────────────────────────────────────────
export function runServerHealthCheck(
  state: GeoRouterState,
): GeoRouterState {
  const updatedServerList = state.serverList.map((server) => {
    const lastPingMs = simulatePing(server.code);
    const activeServers = Math.max(
      1,
      Math.floor(server.servers * (0.7 + Math.random() * 0.25)),
    );
    const currentLoad = Math.round((20 + Math.random() * 60) * 10) / 10;
    const isHealthy = activeServers > 0 && lastPingMs < 300;

    return {
      ...server,
      activeServers,
      avgLatencyMs: lastPingMs,
      loadPercent: Math.round((currentLoad / server.bandwidthCapacity) * 1000) / 10,
      isHealthy,
      lastPingMs,
      lastChecked: Date.now(),
      currentLoad,
    };
  });

  // Update latency map
  const updatedLatencyMap: GeoLatencyMap = {};
  for (const server of updatedServerList) {
    updatedLatencyMap[server.code] = {
      latencyMs: server.avgLatencyMs,
      jitterMs: Math.round((2 + Math.random() * 8) * 10) / 10,
      packetLoss: Math.round(Math.random() * 1.5 * 100) / 100,
      lastMeasured: Date.now(),
    };
  }

  // Recalculate load distribution
  const newDistribution = calculateLoadDistribution(
    updatedServerList,
    state.loadBalancing.strategy,
  );

  // Re-evaluate recommendation
  const autoSelect = autoSelectBestCountry(
    state.recommendationReason.includes('MCI') ? 'mci'
      : state.recommendationReason.includes('Irancell') ? 'irancell'
      : 'default',
    updatedServerList,
  );

  return {
    ...state,
    serverList: updatedServerList,
    latencyMap: updatedLatencyMap,
    loadBalancing: {
      ...state.loadBalancing,
      currentDistribution: newDistribution,
    },
    lastHealthCheck: Date.now(),
    recommendedCountry: autoSelect.countryCode,
    recommendedCountryFa: autoSelect.countryNameFa,
    recommendationReason: autoSelect.reason,
    recommendationReasonFa: autoSelect.reasonFa,
  };
}
