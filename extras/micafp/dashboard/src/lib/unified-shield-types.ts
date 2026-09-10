export type CoreStatus = 'connected' | 'disconnected' | 'connecting' | 'error' | 'standby';

export type ProtocolType =
  | 'amneziawg-1.5'
  | 'vless-reality-xtls'
  | 'vless-fragment'
  | 'vmess-ws-tls'
  | 'trojan-grpc'
  | 'hysteria2'
  | 'tuic-v5'
  | 'shadowtls-v3'
  | 'psiphon-ssh-obfs'
  | 'lantern-df-pt'
  | 'mahsang-obfs'
  | 'mvless'
  | 'defyxvpn-layers'
  | 'moav-tunnel'
  | 'wireguard-noise'
  | 'shadowsocks-shadowtls'
  | 'psiphon-cdn-front'
  | 'naiveproxy'
  | 'naira-blackout'
  | 'quazar-quantum'
  | 'stormdns-dnstt'
  | 'meshrelay-p2p'
  | 'stegostream-web'
  | 'rstguard-ttl'
  | 'chronoshield-temporal'
  | 'dualmux-l4'
  | 'cottendns-fec'
  | 'masterdns-arq'
  | 'oobkeys-sms'
  | 'whitedns-utls'
  | 'dohdot-anycast'
  | 'noizdns-noise'
  | 'sshbridge-ws'
  | 'naivechromium-ja4'
  | 'warpguard-noise'
  | 'echfront-ech'
  | 'obfs4bridge-obfs4'
  | 'quicstorm-quic'
  | 'portspoof-hop';

export type PlatformType = 'android' | 'windows' | 'linux' | 'ios' | 'openwrt' | 'macos';

export interface PlatformSupport {
  platform: PlatformType;
  nameFa: string;
  supported: boolean;
  tunnelType: string;
  tunnelTypeFa: string;
  minVersion: string;
  icon: string;
}

export interface HealthStatus {
  latency: number;
  packetLoss: number;
  blocked: boolean;
  dnsLeak: boolean;
  dpiExposure: number;
  uptime: number;
  bandwidth: { up: number; down: number };
}

export interface CoreAdapter {
  id: string;
  name: string;
  nameFa: string;
  version: string;
  latestVersion: string;
  status: CoreStatus;
  priority: number;
  health: HealthStatus;
  capabilities: ProtocolType[];
  lastChecked: number;
  blockEvents24h: number;
  color: string;
  icon: string;
  description: string;
  descriptionFa: string;
  githubUrl: string;
  githubApiUrl: string;
  assetFilter: string;
  role: string;
  roleFa: string;
  specialFeatures: string[];
  specialFeaturesFa: string[];
  platforms: PlatformType[];
  checksumSha256: string;
}

export interface CoreRankingProfile {
  coreId: string;
  coreNameFa: string;
  latencyMs: number;
  packetLoss: number;
  efficiencyRatio: number; // latency * (1 + packetLoss)
  efficiencyScore: number; // normalized 0-100 score
  rank: number;
  idleSampleCount: number;
  lastProfiled: number;
}

export interface AIOrchestratorState {
  activeCoreId: string;
  shadowConnections: string[];
  scoringMatrix: Record<string, number>;
  ucbScores: Record<string, { exploitation: number; exploration: number; total: number }>;
  coreRankings?: CoreRankingProfile[];
  backgroundWorkerActive?: boolean;
  lastWorkerRun?: number;
  predictiveMitigation?: PredictiveMitigationState;
  predictionState: {
    imminentBlockRisk: number;
    predictedBlockCore: string | null;
    proactiveSwitchRecommended: boolean;
  };
  rlWeights: Record<string, number[]>;
  learningRate: number;
  totalSwitches: number;
  successfulSwitches: number;
  averageSwitchTime: number;
  detectedISP: string;
  detectedISPFa: string;
  ispRuleApplied: string;
}

// ──────────────────────────────────────────────
// Predictive Mitigation Planner (Pre-warming high-obfuscation cores)
// ──────────────────────────────────────────────
export interface HistoricalDpiSpikeWindow {
  id: string;
  timeLabel: string;
  timeLabelFa: string;
  targetHour: number;
  targetMinute: number;
  historicalSeverity: 'high' | 'critical';
  historicalSeverityFa: string;
  affectedProtocols: string[];
  recommendedCore: string;
  preWarmMinutesBefore: number;
  descriptionFa: string;
}

export interface PredictiveMitigationState {
  enabled: boolean;
  preWarmedCores: string[];
  activeScheduleWindows: HistoricalDpiSpikeWindow[];
  nextScheduledPreWarm: {
    coreId: string;
    coreNameFa: string;
    scheduledTimestamp: number;
    triggerTime: string;
    dpiSpikeWindow: string;
    preWarmedStatus: 'idle' | 'pre-warming' | 'ready' | 'switched';
  } | null;
  lastPreWarmEvent?: {
    coreId: string;
    timestamp: number;
    reasonFa: string;
  };
}

// ──────────────────────────────────────────────
// Egress Integrity Monitor (Non-proxy leak detection & network reset)
// ──────────────────────────────────────────────
export interface EgressLeakAttempt {
  id: string;
  timestamp: number;
  sourceProcess: string;
  destinationIp: string;
  destinationPort: number;
  detectedProtocol: string;
  bypassType: 'direct_tcp_leak' | 'raw_socket_bypass' | 'unauthorized_dns' | 'udp_leak';
  bypassTypeFa: string;
  severity: 'high' | 'critical';
  blocked: boolean;
  resetTriggered: boolean;
}

export interface EgressIntegrityState {
  monitoringEnabled: boolean;
  autoNetworkResetOnLeak: boolean;
  totalInspectedPackets: number;
  unauthorizedBypassAttempts: number;
  lastResetTimestamp: number | null;
  systemResetStatus: 'idle' | 'resetting' | 'secured';
  recentBypassAttempts: EgressLeakAttempt[];
}

// ──────────────────────────────────────────────
// BGP Health & Proactive Path Switching
// ──────────────────────────────────────────────
export interface BGPRouteHealth {
  routeId: string;
  nodeName: string;
  nodeNameFa: string;
  asNumber: string;
  upstreamIsp: string;
  routeShiftScore: number; // 0 (stable) - 100 (severe shift / route flap)
  asPathHops: number;
  asPathHopsPrevious: number;
  flapCount10m: number;
  packetLossPercent: number;
  healthStatus: 'optimal' | 'route_shift_detected' | 'flapping' | 'blackholed';
  healthStatusFa: string;
  isDomesticRelay: boolean;
  exitPoint: string;
  exitPointFa: string;
  lastEvaluated: number;
}

export interface ProactivePathSwitchingState {
  enabled: boolean;
  evaluationIntervalSec: number;
  autoMigrateOnShift: boolean;
  routes: BGPRouteHealth[];
  activeRouteId: string;
  totalPathMigrations: number;
  lastMigration?: {
    fromRouteId: string;
    toRouteId: string;
    reasonFa: string;
    timestamp: number;
    migrationTimeMs: number;
  };
}

// ──────────────────────────────────────────────
// Obfuscation Entropy Profiler (VLESS / Hysteria dynamic padding)
// ──────────────────────────────────────────────
export interface ObfuscationEntropyProfile {
  targetCoreId: string;
  targetCoreNameFa: string;
  liveEntropyScore: number; // 0.00 - 8.00 (Shannon entropy)
  targetEntropyRange: { min: number; max: number };
  randomPaddingFrequency: number; // percentage (e.g. 85%)
  currentPaddingSizeRange: { minBytes: number; maxBytes: number };
  trafficShapeStatus: 'optimal_mimic' | 'adjusting_padding' | 'high_entropy_alert' | 'low_entropy_alert';
  trafficShapeStatusFa: string;
  timingJitterMs: number;
  chaffPacketsPerSec: number;
  defeatedDpiSignatures: string[];
}

export interface ObfuscationProfilerState {
  realtimeAnalysisActive: boolean;
  targetProfiles: ObfuscationEntropyProfile[];
  averageEntropy: number;
  totalTrafficReshapedMb: number;
  lastAdjustmentTimestamp: number;
}

// ──────────────────────────────────────────────
// Energy Efficiency Optimizer (Battery-Adaptive Polling)
// ──────────────────────────────────────────────
export interface EnergyOptimizerState {
  enabled: boolean;
  batteryLevel: number; // 0 - 100
  isCharging: boolean;
  powerMode: 'auto' | 'performance' | 'eco_saver' | 'ultra_low_power';
  currentPollingIntervalSec: number; // dynamically computed
  normalIntervalSec: number;
  lowBatteryIntervalSec: number;
  ultraLowIntervalSec: number;
  powerSavedPercentage: number;
  backgroundTasksPausedOnBattery: boolean;
}

// ──────────────────────────────────────────────
// Traffic Forecast (24-Hour Predictive Bandwidth)
// ──────────────────────────────────────────────
export interface HourlyTrafficPrediction {
  hour: number;
  hourLabel: string;
  predictedDownMb: number;
  predictedUpMb: number;
  confidenceScore: number; // 0-100
  expectedAppCategory: string;
  isPeakHour: boolean;
}

export interface TrafficForecastState {
  enabled: boolean;
  predictedTotal24hMb: number;
  predictedPeakHour: number;
  predictedPeakSpeedMbps: number;
  hourlyPredictions: HourlyTrafficPrediction[];
  smartSavingsEstimatedMb: number;
  lastForecastGenerated: number;
}

// ──────────────────────────────────────────────
// Adaptive UI Theme (Sun-Time synced palette)
// ──────────────────────────────────────────────
export type DaylightPhase = 'dawn' | 'daylight' | 'golden_hour' | 'twilight' | 'midnight';

export interface AdaptiveThemeState {
  enabled: boolean;
  currentPhase: DaylightPhase;
  phaseLabelFa: string;
  sunPositionAngle: number; // 0-360 deg
  accentColor: string;
  cardGlow: string;
  bgAtmosphere: string;
  localHour: number;
}

// ──────────────────────────────────────────────
// Core Health History (60-Minute Sparkline series)
// ──────────────────────────────────────────────
export interface CoreHealthSparklinePoint {
  timestamp: number;
  latencyMs: number;
  packetLossPercent: number;
  jitterMs: number;
  alive: boolean;
}

export interface CoreHealthHistoryRecord {
  coreId: string;
  coreNameFa: string;
  history60m: CoreHealthSparklinePoint[];
  avgLatency60m: number;
  avgPacketLoss60m: number;
  uptimePercentage60m: number;
}


export interface ISPRule {
  id: string;
  name: string;
  nameFa: string;
  preferredCores: string[];
  blockedProtocols: string[];
  blockedProtocolsFa: string[];
  bestObfuscation: string[];
  bestObfuscationFa: string[];
}

export interface TrafficRoutingState {
  mode: 'full-vpn' | 'split-tunnel' | 'selective';
  iranIpBypass: boolean;
  dnsMode: 'doh' | 'dot' | 'plain';
  dnsProviders: string[];
  activeDnsProvider: string;
  ipv6Enabled: boolean;
  p2pRouting: boolean;
  splitRules: SplitRule[];
}

export interface SplitRule {
  id: string;
  app: string;
  appFa: string;
  route: 'vpn' | 'direct';
  enabled: boolean;
}

export interface OTAUpdateState {
  lastCheck: number;
  nextCheck: number;
  updates: OTAUpdate[];
  autoUpdate: boolean;
  rollbackEnabled: boolean;
  sha256Verification: boolean;
  checkIntervalHours: number;
}

export interface OTAUpdate {
  id: string;
  type: 'core-binary' | 'block-db' | 'ai-weights' | 'node-list';
  target: string;
  version: string;
  currentVersion: string;
  size: number;
  deltaPatch: boolean;
  signature: string;
  sha256: string;
  status: 'available' | 'downloading' | 'installed' | 'failed';
  githubReleaseUrl: string;
}

export interface DPITestResult {
  coreId: string;
  coreName: string;
  coreNameFa: string;
  connected: boolean;
  latency: number;
  protocol: string;
  bypassLevel: 'full' | 'partial' | 'none';
  dpiSignature: string;
  dpiSignatureFa: string;
  timestamp: number;
}

export interface ConnectionStats {
  totalUptime: number;
  totalDataTransferred: { up: number; down: number };
  coresUsed: number;
  switchesPerformed: number;
  blockEventsAvoided: number;
  currentSpeed: { up: number; down: number };
  activePlatform: PlatformType;
}

export const PLATFORMS: PlatformSupport[] = [
  { platform: 'android', nameFa: 'اندروید', supported: true, tunnelType: 'VpnService', tunnelTypeFa: 'سرویس VPN (بدون روت)', minVersion: '5.0+', icon: '🤖' },
  { platform: 'windows', nameFa: 'ویندوز', supported: true, tunnelType: 'Wintun/TAP', tunnelTypeFa: 'درایور Wintun', minVersion: '7+', icon: '🪟' },
  { platform: 'linux', nameFa: 'لینوکس', supported: true, tunnelType: 'tun/tap', tunnelTypeFa: 'رابط tun/tap', minVersion: 'Kernel 4.x+', icon: '🐧' },
  { platform: 'ios', nameFa: 'آی‌اواس', supported: true, tunnelType: 'NEPacketTunnelProvider', tunnelTypeFa: 'اکستنشن شبکه (بدون جیلبریک)', minVersion: '15+', icon: '🍎' },
  { platform: 'openwrt', nameFa: 'اوپن‌دبلیو‌آر‌تی', supported: true, tunnelType: 'netifd/tun', tunnelTypeFa: 'تونل از طریق netifd', minVersion: '21.02+', icon: '📦' },
  { platform: 'macos', nameFa: 'مک‌اواس', supported: true, tunnelType: 'NEPacketTunnelProvider', tunnelTypeFa: 'اکستنشن شبکه', minVersion: '12+', icon: '💻' },
];

export const ISP_RULES: ISPRule[] = [
  {
    id: 'mci',
    name: 'MCI (Hamrahe Avval)',
    nameFa: 'همراه اول',
    preferredCores: ['mahsang', 'amneziavpn'],
    blockedProtocols: ['VMess', 'VLESS_plain'],
    blockedProtocolsFa: ['VMess', 'VLESS ساده'],
    bestObfuscation: ['fragment', 'warp_noise'],
    bestObfuscationFa: ['Fragment', 'نویز Warp'],
  },
  {
    id: 'irancell',
    name: 'Irancell (MTN)',
    nameFa: 'ایرانسل',
    preferredCores: ['hiddify', 'defyxvpn'],
    blockedProtocols: ['Shadowsocks_plain'],
    blockedProtocolsFa: ['شادوساکس ساده'],
    bestObfuscation: ['reality', 'hysteria2'],
    bestObfuscationFa: ['Reality', 'هیستریا۲'],
  },
  {
    id: 'shatel',
    name: 'Shatel',
    nameFa: 'شتل',
    preferredCores: ['amneziavpn', 'psiphon'],
    blockedProtocols: ['WireGuard_plain'],
    blockedProtocolsFa: ['وایرگارد ساده'],
    bestObfuscation: ['amneziawg_junk', 'tls_fragment'],
    bestObfuscationFa: ['آمنزیاوی‌جی جونک', 'Fragment TLS'],
  },
  {
    id: 'asiatech',
    name: 'Asiatech',
    nameFa: 'آسیاتک',
    preferredCores: ['mahsang', 'hiddify'],
    blockedProtocols: ['HTTP_plain', 'SOCKS5'],
    blockedProtocolsFa: ['HTTP ساده', 'SOCKS5'],
    bestObfuscation: ['fake_host', 'doh_fragment'],
    bestObfuscationFa: ['میزبان جعلی', 'Fragment DoH'],
  },
  {
    id: 'rightel',
    name: 'Rightel',
    nameFa: 'رایتل',
    preferredCores: ['defyxvpn', 'hiddify'],
    blockedProtocols: [],
    blockedProtocolsFa: [],
    bestObfuscation: ['reality_vless', 'hysteria2'],
    bestObfuscationFa: ['VLESS Reality', 'هیستریا۲'],
  },
];

export const CORE_DEFINITIONS: Omit<CoreAdapter, 'status' | 'health' | 'priority' | 'lastChecked' | 'blockEvents24h'>[] = [
  {
    id: 'hiddify',
    name: 'hiddify-core',
    nameFa: 'هیدیفای',
    version: 'v4.1.0',
    latestVersion: 'v4.1.0',
    capabilities: ['vless-reality-xtls', 'vmess-ws-tls', 'trojan-grpc', 'hysteria2', 'tuic-v5', 'shadowtls-v3', 'naiveproxy', 'shadowsocks-shadowtls'],
    color: '#10b981',
    icon: '🛡️',
    description: 'Primary orchestration core; handles all sing-box based protocols',
    descriptionFa: 'هسته هماهنگ‌سازی اصلی؛ مدیریت تمام پروتکل‌های مبتنی بر sing-box',
    githubUrl: 'https://github.com/hiddify/hiddify-core',
    githubApiUrl: 'https://api.github.com/repos/hiddify/hiddify-core/releases/latest',
    assetFilter: 'hiddify-core-android-arm64',
    role: 'Primary orchestration core',
    roleFa: 'هسته هماهنگ‌سازی اصلی',
    specialFeatures: ['Auto-protocol selection', 'Built-in sing-box', 'Free node support', 'Multi-protocol fallback'],
    specialFeaturesFa: ['انتخاب خودکار پروتکل', 'sing-box داخلی', 'پشتیبانی از نود رایگان', 'بکاپ چند پروتکلی'],
    platforms: ['android', 'windows', 'linux', 'ios', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'xray-gfw',
    name: 'GFW-knocker/Xray-core',
    nameFa: 'ایکس‌ری GFW',
    version: 'v25.8.3-mahsa-r1',
    latestVersion: 'v25.8.3-mahsa-r1',
    capabilities: ['vless-reality-xtls', 'vless-fragment', 'vmess-ws-tls', 'trojan-grpc', 'wireguard-noise', 'mvless'],
    color: '#6366f1',
    icon: '⚡',
    description: 'Specialized Iran bypass; Fragment+DoH, WireGuard noise, FakeHost, UDP noise',
    descriptionFa: 'عبور تخصصی از فیلترینگ ایران؛ Fragment+DoH، نویز WireGuard، FakeHost، نویز UDP',
    githubUrl: 'https://github.com/GFW-knocker/Xray-core',
    githubApiUrl: 'https://api.github.com/repos/GFW-knocker/Xray-core/releases/latest',
    assetFilter: 'Xray-android-arm64-v8a',
    role: 'Specialized Iran bypass engine',
    roleFa: 'موتور تخصصی عبور از فیلترینگ ایران',
    specialFeatures: ['Custom WireGuard noise', 'TLS fragmentor', 'Fake host injection', 'QUIC manipulation', 'DoH fragment', 'MVLESS protocol'],
    specialFeaturesFa: ['نویز سفارشی WireGuard', 'Fragment TLS', 'تزریق میزبان جعلی', 'دستکاری QUIC', 'Fragment DoH', 'پروتکل MVLESS'],
    platforms: ['android', 'windows', 'linux', 'ios', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'sing-box',
    name: 'sing-box',
    nameFa: 'سینگ‌باکس',
    version: 'v1.14.0-alpha.25',
    latestVersion: 'v1.14.0-alpha.25',
    capabilities: ['hysteria2', 'tuic-v5', 'shadowtls-v3', 'vless-reality-xtls', 'vmess-ws-tls', 'shadowsocks-shadowtls', 'naiveproxy'],
    color: '#f59e0b',
    icon: '📦',
    description: 'Universal proxy platform — bundled as hiddify-core dependency',
    descriptionFa: 'پلتفرم پروکسی جامع — به عنوان وابستگی هیدیفای گنجانده شده',
    githubUrl: 'https://github.com/SagerNet/sing-box',
    githubApiUrl: 'https://api.github.com/repos/SagerNet/sing-box/releases/latest',
    assetFilter: 'sing-box-.*-android-arm64',
    role: 'Protocol handler (embedded in hiddify)',
    roleFa: 'مدیریت پروتکل‌ها (داخلی هیدیفای)',
    specialFeatures: ['ShadowTLS v3', 'Hysteria2 UDP obfuscation', 'TUIC v5 QUIC multiplex', 'NaiveProxy support'],
    specialFeaturesFa: ['شدوتی‌ال‌اس نسخه ۳', 'پنهان‌سازی UDP هیستریا۲', 'مولتی‌پلکس QUIC نسخه ۵ TUIC', 'پشتیبانی NaiveProxy'],
    platforms: ['android', 'windows', 'linux', 'ios', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'amneziavpn',
    name: 'AmneziaVPN (awg-go)',
    nameFa: 'آمنزیاوی‌پی‌ان',
    version: '4.8.15.4',
    latestVersion: '4.8.15.0',
    capabilities: ['amneziawg-1.5'],
    color: '#ec4899',
    icon: '🔐',
    description: 'WireGuard with obfuscation headers — most effective against DPI in Russia/Iran',
    descriptionFa: 'وایرگارد با هدرهای پنهان‌سازی — مؤثرترین در برابر DPI روسیه و ایران',
    githubUrl: 'https://github.com/amnezia-vpn/awg-go',
    githubApiUrl: 'https://api.github.com/repos/amnezia-vpn/awg-go/releases/latest',
    assetFilter: 'awg-go-android',
    role: 'AmneziaWG protocol handler',
    roleFa: 'مدیر پروتکل آمنزیاوی‌جی',
    specialFeatures: ['Junk packet injection', 'Transport header obfuscation', 'AmneziaWG 1.5 custom headers'],
    specialFeaturesFa: ['تزریق بسته‌های جونک', 'پنهان‌سازی هدر ترانسپورت', 'هدرهای سفارشی آمنزیاوی‌جی ۱.۵'],
    platforms: ['android', 'windows', 'linux', 'ios', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'defyxvpn',
    name: 'DefyxVPN',
    nameFa: 'دیفیکسوی‌پی‌ان',
    version: 'v5.2.8',
    latestVersion: 'v5.2.8',
    capabilities: ['defyxvpn-layers', 'vless-reality-xtls', 'amneziawg-1.5'],
    color: '#8b5cf6',
    icon: '🌀',
    description: 'High-speed bypass; P2P support; unlimited bandwidth',
    descriptionFa: 'عبور پرسرعت؛ پشتیبانی P2P؛ پهنای باند نامحدود',
    githubUrl: 'https://github.com/UnboundTechCo/defyxVPN',
    githubApiUrl: 'https://api.github.com/repos/UnboundTechCo/defyxVPN/releases/latest',
    assetFilter: 'defyx-android',
    role: 'High-speed bypass with P2P',
    roleFa: 'عبور پرسرعت با P2P',
    specialFeatures: ['VLESS Reality', 'AmneziaWG 1.5', 'Unlimited bandwidth', 'P2P/torrenting support'],
    specialFeaturesFa: ['VLESS Reality', 'آمنزیاوی‌جی ۱.۵', 'پهنای باند نامحدود', 'پشتیبانی P2P/تورنت'],
    platforms: ['android', 'windows', 'linux', 'ios', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'moav',
    name: 'MoaV',
    nameFa: 'موآوی',
    version: 'v1.7.7',
    latestVersion: 'v1.7.7',
    capabilities: ['moav-tunnel'],
    color: '#14b8a6',
    icon: '🌊',
    description: 'Advanced tunnel protocol with adaptive obfuscation',
    descriptionFa: 'پروتکل تونل پیشرفته با پنهان‌سازی تطبیقی',
    githubUrl: 'https://github.com/GFW-knocker/MahsaNG',
    githubApiUrl: 'https://api.github.com/repos/GFW-knocker/MahsaNG/releases/latest',
    assetFilter: 'moav-android',
    role: 'Adaptive tunnel engine',
    roleFa: 'موتور تونل تطبیقی',
    specialFeatures: ['Adaptive obfuscation', 'Multi-path routing', 'Dynamic key rotation'],
    specialFeaturesFa: ['پنهان‌سازی تطبیقی', 'مسیریابی چندمسیره', 'چرخش کلید پویا'],
    platforms: ['android', 'windows', 'linux', 'ios', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'lantern',
    name: 'Lantern',
    nameFa: 'لنترن',
    version: 'v7.9.0',
    latestVersion: 'v7.9.0',
    capabilities: ['lantern-df-pt', 'psiphon-cdn-front'],
    color: '#f97316',
    icon: '🏮',
    description: 'Domain fronting with pluggable transports for censorship bypass',
    descriptionFa: 'فرانتینگ دامنه با حمل‌های قابل تعویض برای عبور از سانسور',
    githubUrl: 'https://github.com/getlantern/lantern',
    githubApiUrl: 'https://api.github.com/repos/getlantern/lantern/releases/latest',
    assetFilter: 'lantern-android',
    role: 'Domain fronting transport',
    roleFa: 'حمل فرانتینگ دامنه',
    specialFeatures: ['Domain fronting', 'Pluggable transports', 'Peer-to-peer fallback', 'CDN leveraging'],
    specialFeaturesFa: ['فرانتینگ دامنه', 'حمل‌های قابل تعویض', 'بکاپ همتا به همتا', 'استفاده از CDN'],
    platforms: ['android', 'windows', 'linux', 'ios', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'mahsang',
    name: 'MahsaNG core',
    nameFa: 'مهساان‌جی',
    version: 'v26.3.31-mahsa-r1',
    latestVersion: 'v26.3.31-mahsa-r1',
    capabilities: ['mahsang-obfs', 'vless-reality-xtls', 'vless-fragment', 'vmess-ws-tls', 'mvless', 'wireguard-noise'],
    color: '#ef4444',
    icon: '✊',
    description: 'Iranian-specific customizations; MVLESS protocol; rotating configs',
    descriptionFa: 'سفارشی‌سازی‌های ایران؛ پروتکل MVLESS؛ پیکربندی چرخشی',
    githubUrl: 'https://github.com/GFW-knocker/MahsaNG',
    githubApiUrl: 'https://api.github.com/repos/GFW-knocker/MahsaNG/releases/latest',
    assetFilter: 'libv2ray.aar',
    role: 'Iran-optimized bypass engine',
    roleFa: 'موتور عبور بهینه‌شده برای ایران',
    specialFeatures: ['YouTube Direct bypass', 'HTTPS/TLS DoH fragmentor', 'Warp noise', 'MVLESS protocol', 'Rotating configs'],
    specialFeaturesFa: ['عبور مستقیم یوتیوب', 'Fragment DoH HTTPS/TLS', 'نویز Warp', 'پروتکل MVLESS', 'پیکربندی چرخشی'],
    platforms: ['android', 'windows', 'linux', 'ios', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'psiphon',
    name: 'Psiphon Tunnel Core (GFW-knocker)',
    nameFa: 'سایفون',
    version: 'latest',
    latestVersion: 'latest',
    capabilities: ['psiphon-ssh-obfs', 'psiphon-cdn-front'],
    color: '#06b6d4',
    icon: '🔵',
    description: 'Last-resort fallback when all other cores are blocked — connects independently without VPS',
    descriptionFa: 'بکاپ آخرین مرحله وقتی همه هسته‌ها مسدودند — بدون نیاز به سرور متصل می‌شود',
    githubUrl: 'https://github.com/GFW-knocker/psiphon-tunnel-core',
    githubApiUrl: 'https://api.github.com/repos/GFW-knocker/psiphon-tunnel-core/releases/latest',
    assetFilter: 'psiphon-tunnel-core-android',
    role: 'Fallback layer (last resort)',
    roleFa: 'لایه بکاپ (آخرین مرحله)',
    specialFeatures: ['SSH transport + ObfuscatedSSH', 'CDN domain fronting', 'No VPS required', 'Independent connectivity'],
    specialFeaturesFa: ['حمل SSH + SSH مبهم', 'فرانتینگ دامنه CDN', 'بدون نیاز به سرور', 'اتصال مستقل'],
    platforms: ['android', 'windows', 'linux', 'ios', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'naira',
    name: 'NAIRA Core (National AI Relay + Anti-DPI)',
    nameFa: 'نایرا — رله ملی هوشمند ضد DPI',
    version: 'v1.0.0-quantum',
    latestVersion: 'v1.0.0-quantum',
    capabilities: ['naira-blackout', 'mahsang-obfs', 'psiphon-cdn-front', 'psiphon-ssh-obfs', 'lantern-df-pt', 'shadowtls-v3', 'vless-reality-xtls'],
    color: '#22d3ee',
    icon: '🛰️',
    description: 'AI-driven blackout core — national reverse relay + DNS tunneling + anti-DPI evasion; connects even when international internet is fully cut',
    descriptionFa: 'هسته هوشمند قطعی بین‌الملل — رله معکوس ملی + تانلینگ DNS + دور زدن DPI با هوش مصنوعی؛ حتی با قطع کامل اینترنت بین‌الملل متصل می‌شود',
    githubUrl: 'https://github.com/unifiedshield/naira-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/naira-core/releases/latest',
    assetFilter: 'naira-core-android-arm64',
    role: 'National blackout & anti-DPI AI engine',
    roleFa: 'موتور هوش مصنوعی قطعی ملی و ضد DPI',
    specialFeatures: ['National reverse relay (NIN)', 'DNS tunneling (DNSTT/EDNS0)', 'AI DPI evasion', 'Quantum QNS-v4', 'Zero-touch blackout recovery', 'Multi-path aggregation'],
    specialFeaturesFa: ['رله معکوس ملی (NIN)', 'تانلینگ DNS (DNSTT/EDNS0)', 'دور زدن DPI با هوش مصنوعی', 'استگانوگرافی کوانتومی QNS-v4', 'بازیابی خودکار قطعی بدون دخالت کاربر', 'تجمیع چندمسیره'],
    platforms: ['android', 'windows', 'linux', 'ios', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'quazar',
    name: 'QUAZAR Quantum Anti-DPI Core',
    nameFa: 'کوزار — هسته کوانتومی ضد DPI',
    version: 'v2.0.0-quantum',
    latestVersion: 'v2.0.0-quantum',
    capabilities: ['quazar-quantum', 'naira-blackout', 'shadowtls-v3', 'vless-reality-xtls', 'hysteria2'],
    color: '#a855f7',
    icon: '⚛️',
    description: 'Quantum-noise + ML packet morphing engine for extreme multi-layer filtering; multi-path fragmentation',
    descriptionFa: 'موتور نویز کوانتومی + تغییر شکل پکت با یادگیری ماشین برای فیلترینگ چندلایه فوق‌سخت؛ تکه‌سازی چندمسیره',
    githubUrl: 'https://github.com/unifiedshield/quazar-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/quazar-core/releases/latest',
    assetFilter: 'quazar-core-android-arm64',
    role: 'Quantum anti-DPI & multi-path engine',
    roleFa: 'موتور کوانتومی ضد DPI و چندمسیره',
    specialFeatures: ['Quantum noise morphing', 'ML packet reshaping', 'Multi-path fragmentation', 'Entropy matching 7.942', 'Zero-signature'],
    specialFeaturesFa: ['جهش نویز کوانتومی', 'بازسازی پکت با ML', 'تکه‌سازی چندمسیره', 'تطبیق آنتروپی 7.942', 'امضای صفر'],
    platforms: ['android', 'windows', 'linux', 'ios', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'stormdns',
    name: 'StormDNS Blackout Tunnel Core',
    nameFa: 'استورم‌دی‌ان‌اس — هسته تانل قطعی DNS',
    version: 'v1.2.0',
    latestVersion: 'v1.2.0',
    capabilities: ['stormdns-dnstt', 'psiphon-cdn-front', 'lantern-df-pt'],
    color: '#38bdf8',
    icon: '🌩️',
    description: 'DNS tunneling (DNSTT/EDNS0/Iodine) that stays connected when international internet is cut but DNS resolves',
    descriptionFa: 'تانلینگ DNS (DNSTT/EDNS0/Iodine) — وقتی اینترنت بین‌الملل قطع است ولی DNS کار می‌کند وصل می‌ماند',
    githubUrl: 'https://github.com/unifiedshield/stormdns-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/stormdns-core/releases/latest',
    assetFilter: 'stormdns-core-android-arm64',
    role: 'DNS tunnel blackout engine',
    roleFa: 'موتور تانل DNS برای قطعی',
    specialFeatures: ['DNSTT 4096-byte buffer', 'EDNS0 QNAME morphing', 'Iodine fallback', 'TCP stream persistence'],
    specialFeaturesFa: ['بافر ۴۰۹۶ بایتی DNSTT', 'تغییر برچسب QNAME', 'بکاپ Iodine', 'پایداری استریم TCP'],
    platforms: ['android', 'windows', 'linux', 'ios', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'meshrelay',
    name: 'MeshRelay Offline P2P Core',
    nameFa: 'مش‌رله — هسته رله آفلاین همتا‌به‌همتا',
    version: 'v1.0.0-mesh',
    latestVersion: 'v1.0.0-mesh',
    capabilities: ['meshrelay-p2p', 'lantern-df-pt'],
    color: '#34d399',
    icon: '🕸️',
    description: 'Offline P2P mesh rendezvous via WebRTC Snowflake, BLE & Wi-Fi Direct for total network blackout',
    descriptionFa: 'رندو مش آفلاین همتا‌به‌همتا از طریق Snowflake WebRTC، بلوتوث و Wi-Fi Direct برای خاموشی کامل شبکه',
    githubUrl: 'https://github.com/unifiedshield/meshrelay-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/meshrelay-core/releases/latest',
    assetFilter: 'meshrelay-core-android-arm64',
    role: 'Offline P2P mesh engine',
    roleFa: 'موتور مش آفلاین همتا‌به‌همتا',
    specialFeatures: ['WebRTC Snowflake brokers', 'BLE rendezvous', 'Wi-Fi Direct', 'Nearest-hop routing'],
    specialFeaturesFa: ['بروکرهای Snowflake WebRTC', 'رندو بلوتوث کم‌مصرف', 'Wi-Fi Direct', 'مسیریابی نزدیک‌ترین گره'],
    platforms: ['android', 'windows', 'linux', 'ios', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'stegostream',
    name: 'StegoStream Video Masquerade Core',
    nameFa: 'استگواستریم — هسته استتار استریم ویدیویی',
    version: 'v1.0.0-stego',
    latestVersion: 'v1.0.0-stego',
    capabilities: ['stegostream-web', 'psiphon-cdn-front'],
    color: '#f472b6',
    icon: '🎬',
    description: 'Hides tunnel bytes inside real WebRTC/RTMP video-stream frames (Aparat/Telewebion-like) so DPI sees domestic streaming',
    descriptionFa: 'پنهان‌سازی بایت‌های تونل در فریم‌های واقعی استریم ویدیویی WebRTC/RTMP (شبیه آپارات/تلوبیون) تا DPI آن را استریم داخلی ببیند',
    githubUrl: 'https://github.com/unifiedshield/stegostream-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/stegostream-core/releases/latest',
    assetFilter: 'stegostream-core-android-arm64',
    role: 'Steganographic video-stream engine',
    roleFa: 'موتور استتار استریم ویدیویی',
    specialFeatures: ['WebRTC frame embedding', 'RTMP masquerade', 'Aparat-like traffic', 'AI DPI blindness'],
    specialFeaturesFa: ['جاسازی در فریم WebRTC', 'استتار RTMP', 'ترافیک شبیه آپارات', 'کوری DPI هوشمند'],
    platforms: ['android', 'windows', 'linux', 'ios', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'rstguard',
    name: 'RSTGuard Fake-RST/TTL Filter Core',
    nameFa: 'آر‌اس‌تی‌گارد — هسته فیلتر RST جعلی و TTL',
    version: 'v1.0.0',
    latestVersion: 'v1.0.0',
    capabilities: ['rstguard-ttl', 'mahsang-obfs', 'vless-fragment'],
    color: '#f97316',
    icon: '🧱',
    description: 'Drops forged Fake-RST packets and filters TTL-gap anomalies at the local firewall so connections never drop',
    descriptionFa: 'بسته‌های RST جعلی را در فایروال محلی Drop می‌کند و ناهنجاری اختلاف TTL را فیلتر می‌کند تا اتصال هرگز قطع نشود',
    githubUrl: 'https://github.com/unifiedshield/rstguard-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/rstguard-core/releases/latest',
    assetFilter: 'rstguard-core-android-arm64',
    role: 'Fake-RST & TTL-gap firewall engine',
    roleFa: 'موتور فایروال RST جعلی و اختلاف TTL',
    specialFeatures: ['Fake-RST drop', 'TTL gap analysis', 'TCP window guard', 'Injection detection'],
    specialFeaturesFa: ['Drop کردن RST جعلی', 'تحلیل اختلاف TTL', 'گارد پنجره TCP', 'تشخیص تزریق پکت'],
    platforms: ['android', 'windows', 'linux', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'chronoshield',
    name: 'Chronoshield Temporal Throttling Core',
    nameFa: 'کرونوشیلد — پروفایلر زمان‌محور تراتلینگ',
    version: 'v1.0.0-temporal',
    latestVersion: 'v1.0.0-temporal',
    capabilities: ['chronoshield-temporal', 'shadowtls-v3', 'vless-fragment'],
    color: '#eab308',
    icon: '🕒',
    description: 'Predicts Iran peak-hour throttling (19:00–23:00) and pre-switches UDP to TCP TLS Split before the drop',
    descriptionFa: 'تراتلینگ ساعات اوج ایران (۱۹ تا ۲۳) را پیش‌بینی و پیش از افت پهنای‌باند UDP را به TCP TLS Split تغییر می‌دهد',
    githubUrl: 'https://github.com/unifiedshield/chronoshield-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/chronoshield-core/releases/latest',
    assetFilter: 'chronoshield-core-android-arm64',
    role: 'Temporal & diurnal throttling profiler',
    roleFa: 'پروفایلر تراتلینگ زمان‌محور',
    specialFeatures: ['Peak-hour heatmap', 'UDP→TCP pre-switch', 'Per-operator schedule', 'Diurnal bandwidth model'],
    specialFeaturesFa: ['نقشه حرارتی ساعات اوج', 'تعویض پیش‌دستانه UDP→TCP', 'جدول زمانی هر اپراتور', 'مدل پهنای‌باند روزانه'],
    platforms: ['android', 'windows', 'linux', 'ios', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'dualmux',
    name: 'DualMux L4 Multipath Aggregation Core',
    nameFa: 'دوالماکس — تجمیع چندمسیره لایه ۴',
    version: 'v1.0.0-mux',
    latestVersion: 'v1.0.0-mux',
    capabilities: ['dualmux-l4', 'quazar-quantum', 'wireguard-noise'],
    color: '#0ea5e9',
    icon: '🔀',
    description: 'Bonds WiFi (fixed) and cellular (SIM) bandwidth at Layer 4 with seamless per-packet failover',
    descriptionFa: 'پهنای‌باند وای‌فای و دیتای سیم‌کارت را در لایه ۴ ترکیب می‌کند با فیل‌اور بدون قطعی',
    githubUrl: 'https://github.com/unifiedshield/dualmux-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/dualmux-core/releases/latest',
    assetFilter: 'dualmux-core-android-arm64',
    role: 'L4 multipath aggregation engine',
    roleFa: 'موتور تجمیع چندمسیره لایه ۴',
    specialFeatures: ['WiFi + cellular bonding', 'Per-packet failover', 'Load balancing', 'Link health monitor'],
    specialFeaturesFa: ['ترکیب وای‌فای + دیتا', 'فیل‌اور هر پکت', 'توازن بار', 'پایش سلامت لینک'],
    platforms: ['android', 'windows', 'linux', 'ios', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'cottendns',
    name: 'CottenDNS Adaptive FEC Core',
    nameFa: 'کاتن DNS — تصحیح خطای تطبیقی FEC',
    version: 'v1.0.0-fec',
    latestVersion: 'v1.0.0-fec',
    capabilities: ['cottendns-fec', 'stormdns-dnstt', 'lantern-df-pt'],
    color: '#84cc16',
    icon: '🧵',
    description: 'Super-FEC 8:4 forward error correction that keeps DNS tunnels alive through up to 60% packet loss',
    descriptionFa: 'تصحیح خطای رو به جلو Super-FEC 8:4 که تانل DNS را تا ۶۰٪ پکت‌لاس زنده نگه می‌دارد',
    githubUrl: 'https://github.com/unifiedshield/cottendns-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/cottendns-core/releases/latest',
    assetFilter: 'cottendns-core-android-arm64',
    role: 'Adaptive FEC DNS tunnel engine',
    roleFa: 'موتور تصحیح خطای تطبیقی تانل DNS',
    specialFeatures: ['Super-FEC 8:4', '60% loss recovery', 'Adaptive redundancy', 'Latency-optimized coding'],
    specialFeaturesFa: ['Super-FEC 8:4', 'بازیابی ۶۰٪ افت', 'افزونگی تطبیقی', 'کدگذاری بهینه تأخیر'],
    platforms: ['android', 'windows', 'linux', 'ios', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'masterdns',
    name: 'MasterDNS ARQ Multi-Path Core',
    nameFa: 'مستردیانس — ریسینگ چندمسیره ARQ',
    version: 'v1.0.0-arq',
    latestVersion: 'v1.0.0-arq',
    capabilities: ['masterdns-arq', 'stormdns-dnstt'],
    color: '#10b981',
    icon: '🏁',
    description: '8-path racing cluster that neutralizes country-wide DNS cache poisoning (10.10.34.34) with ARQ retransmission',
    descriptionFa: 'کلاستر ریسینگ ۸ مسیره که پویزنینگ کش DNS کشوری (10.10.34.34) را با بازارسال ARQ خنثی می‌کند',
    githubUrl: 'https://github.com/unifiedshield/masterdns-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/masterdns-core/releases/latest',
    assetFilter: 'masterdns-core-android-arm64',
    role: 'ARQ multi-path DNS engine',
    roleFa: 'موتور DNS چندمسیره ARQ',
    specialFeatures: ['8-path racing', 'ARQ retransmission', 'Poisoning neutralization', '10.10.34.34 bypass'],
    specialFeaturesFa: ['ریسینگ ۸ مسیره', 'بازارسال ARQ', 'خنثی‌سازی پویزنینگ', 'عبور از 10.10.34.34'],
    platforms: ['android', 'windows', 'linux', 'ios', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'oobkeys',
    name: 'OOBKeys Staged Key Delivery Core',
    nameFa: 'اوبکیش — توزیع پیش‌دستانه کلید (SMS/Base45)',
    version: 'v1.0.0-oob',
    latestVersion: 'v1.0.0-oob',
    capabilities: ['oobkeys-sms', 'naira-blackout', 'meshrelay-p2p'],
    color: '#f43f5e',
    icon: '📡',
    description: 'Receives clean IPs and Reality keys via Base45/SMS out-of-band when DNS and internet are fully dead',
    descriptionFa: 'آی‌پی‌های تمیز و کلیدهای Reality را از کانال خارج از باند (Base45/SMS) دریافت می‌کند وقتی DNS و اینترنت کاملاً مرده‌اند',
    githubUrl: 'https://github.com/unifiedshield/oobkeys-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/oobkeys-core/releases/latest',
    assetFilter: 'oobkeys-core-android-arm64',
    role: 'Out-of-band staged key delivery engine',
    roleFa: 'موتور توزیع پیش‌دستانه کلید خارج از باند',
    specialFeatures: ['Base45 compression', 'SMS key delivery', 'Reality key staging', 'No-internet bootstrap'],
    specialFeaturesFa: ['فشرده‌سازی Base45', 'ارسال کلید با SMS', 'مرحله‌بندی کلید Reality', 'راه‌اندازی بدون اینترنت'],
    platforms: ['android', 'windows', 'linux', 'ios', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'whitedns',
    name: 'WhiteDNS uTLS Clean DNS Core',
    nameFa: 'وایت‌دی‌ان‌اس — DNS پاکیزه با uTLS',
    version: 'v1.0.0-utls',
    latestVersion: 'v1.0.0-utls',
    capabilities: ['whitedns-utls', 'psiphon-cdn-front'],
    color: '#14b8a6',
    icon: '🦢',
    description: 'Clean DNS resolution with uTLS browser-handshake simulation to evade SNI/DPI on the DNS layer',
    descriptionFa: 'تفکیک DNS پاکیزه با شبیه‌سازی دست‌تکانی uTLS مرورگر برای دور زدن SNI/DPI در لایه DNS',
    githubUrl: 'https://github.com/unifiedshield/whitedns-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/whitedns-core/releases/latest',
    assetFilter: 'whitedns-core-android-arm64',
    role: 'Clean DNS + uTLS handshake engine',
    roleFa: 'موتور DNS پاکیزه + دست‌تکانی uTLS',
    specialFeatures: ['uTLS browser fingerprint', 'Clean resolver pool', 'DoH/DoT fallback', 'Cache poisoning guard'],
    specialFeaturesFa: ['اثر انگشت uTLS مرورگر', 'استخر رزولور پاکیزه', 'بکاپ DoH/DoT', 'گارد پویزنینگ کش'],
    platforms: ['android', 'windows', 'linux', 'ios', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'dohdot',
    name: 'DoH/DoT Anycast Core',
    nameFa: 'دوه‌دات — هسته Anycast (RFC 8484/7858)',
    version: 'v1.0.0-doh',
    latestVersion: 'v1.0.0-doh',
    capabilities: ['dohdot-anycast', 'whitedns-utls', 'psiphon-cdn-front'],
    color: '#2dd4bf',
    icon: '🌐',
    description: 'DoH/DoT (RFC 8484/7858) over international Anycast servers with anti-DNS-poisoning verification',
    descriptionFa: 'DoH/DoT (RFC 8484/7858) روی سرورهای Anycast بین‌المللی با راستی‌آزمایی ضد پویزنینگ DNS',
    githubUrl: 'https://github.com/unifiedshield/dohdot-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/dohdot-core/releases/latest',
    assetFilter: 'dohdot-core-android-arm64',
    role: 'DoH/DoT Anycast engine',
    roleFa: 'موتور Anycast دوه‌دات',
    specialFeatures: ['RFC 8484 DoH', 'RFC 7858 DoT', 'Anti-poisoning verify', 'Anycast low latency'],
    specialFeaturesFa: ['DoH استاندارد 8484', 'DoT استاندارد 7858', 'راستی‌آزمایی ضد پویزنینگ', 'تأخیر کم Anycast'],
    platforms: ['android', 'windows', 'linux', 'ios', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'noizdns',
    name: 'NoizDNS VayDNS Noise Obfuscation Core',
    nameFa: 'نویزدنس — هسته مبهم‌سازی نویز (VayDNS/NoizDNS)',
    version: 'v1.0.0-noise',
    latestVersion: 'v1.0.0-noise',
    capabilities: ['noizdns-noise', 'whitedns-utls', 'stormdns-dnstt'],
    color: '#7c3aed',
    icon: '🌫️',
    description: 'Noise IK handshake + random jitter obfuscation (VayDNS/NoizDNS) to randomize fingerprints against DPI ML',
    descriptionFa: 'دست‌تکانی Noise IK + جیتر تصادفی (VayDNS/NoizDNS) برای تصادفی‌سازی اثر انگشت در برابر یادگیری ماشین DPI',
    githubUrl: 'https://github.com/unifiedshield/noizdns-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/noizdns-core/releases/latest',
    assetFilter: 'noizdns-core-android-arm64',
    role: 'Noise obfuscation engine',
    roleFa: 'موتور مبهم‌سازی نویز',
    specialFeatures: ['Noise IK handshake', 'Random jitter', 'Fingerprint randomization', 'DPI ML evasion'],
    specialFeaturesFa: ['دست‌تکانی Noise IK', 'جیتر تصادفی', 'تصادفی‌سازی اثر انگشت', 'دور زدن ML سیستم DPI'],
    platforms: ['android', 'windows', 'linux', 'ios', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'sshbridge',
    name: 'SSHBridge Secure Wrapper Core',
    nameFa: 'اس‌اس‌اچ‌بریج — هسته پوشش امن SSH',
    version: 'v1.0.0-ssh',
    latestVersion: 'v1.0.0-ssh',
    capabilities: ['sshbridge-ws', 'mahsang-obfs', 'psiphon-cdn-front'],
    color: '#6366f1',
    icon: '🔌',
    description: 'SSH tunnels wrapped in WebSocket / TLS 1.3 / CDN fronting for deep-packet-inspection evasion',
    descriptionFa: 'تانل‌های SSH پوشیده در WebSocket / TLS 1.3 / فرانتینگ CDN برای دور زدن بازرسی عمیق بسته',
    githubUrl: 'https://github.com/unifiedshield/sshbridge-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/sshbridge-core/releases/latest',
    assetFilter: 'sshbridge-core-android-arm64',
    role: 'SSH secure wrapper engine',
    roleFa: 'موتور پوشش امن SSH',
    specialFeatures: ['SSH over WebSocket', 'TLS 1.3 wrapper', 'CDN fronting', 'Anti-DPI handshake'],
    specialFeaturesFa: ['SSH روی WebSocket', 'پوشش TLS 1.3', 'فرانتینگ CDN', 'دست‌تکانی ضد DPI'],
    platforms: ['android', 'windows', 'linux', 'ios', 'openwrt', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'naivechromium',
    name: 'NaiveChromium JA4 Core',
    nameFa: 'نایوکرومیوم — هسته پشته شبکه کروم (JA4)',
    version: 'v1.0.0-ja4',
    latestVersion: 'v1.0.0-ja4',
    capabilities: ['naivechromium-ja4', 'psiphon-cdn-front', 'shadowtls-v3'],
    color: '#f59e0b',
    icon: '🖥️',
    description: 'Proxies over a real Chromium network stack (JA4 fingerprint) with HTTP/3 multiplexing for browser-identical traffic',
    descriptionFa: 'پروکسی روی پشته شبکه واقعی کروم (اثر انگشت JA4) با مالتی‌پلکس HTTP/3 برای ترافیک مشابه مرورگر',
    githubUrl: 'https://github.com/unifiedshield/naivechromium-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/naivechromium-core/releases/latest',
    assetFilter: 'naivechromium-core-android-arm64',
    role: 'Chromium-stack proxy engine',
    roleFa: 'موتور پروکسی پشته کروم',
    specialFeatures: ['Chromium JA4 fingerprint', 'HTTP/3 multiplex', 'Browser-identical TLS', 'Padding control'],
    specialFeaturesFa: ['اثر انگشت JA4 کروم', 'مالتی‌پلکس HTTP/3', 'TLS مشابه مرورگر', 'کنترل پدینگ'],
    platforms: ['android', 'windows', 'linux', 'ios', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'warpguard',
    name: 'WARPGuard Anycast Noise Core',
    nameFa: 'وارپ‌گارد — هسته Anycast وارپ + نویز',
    version: 'v1.0.0-warp',
    latestVersion: 'v1.0.0-warp',
    capabilities: ['warpguard-noise', 'wireguard-noise', 'psiphon-cdn-front'],
    color: '#0ea5e9',
    icon: '🛡️',
    description: 'Cloudflare WARP/WireGuard Anycast endpoints with Noise handshake and reserved headers for clean-IP traffic',
    descriptionFa: 'اندپوینت‌های Anycast وارپ/وایرگارد کلودفلر با دست‌تکانی Noise و هدرهای رزرو برای ترافیک IP پاکیزه',
    githubUrl: 'https://github.com/unifiedshield/warpguard-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/warpguard-core/releases/latest',
    assetFilter: 'warpguard-core-android-arm64',
    role: 'WARP/Anycast Noise engine',
    roleFa: 'موتور Anycast وارپ + نویز',
    specialFeatures: ['Noise handshake', 'Clean Anycast IP', 'Reserved headers', 'WireGuard fallback'],
    specialFeaturesFa: ['دست‌تکانی Noise', 'IP پاکیزه Anycast', 'هدرهای رزرو', 'بکاپ وایرگارد'],
    platforms: ['android', 'windows', 'linux', 'ios', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'echfront',
    name: 'ECHFront ECH Domain Fronting Core',
    nameFa: 'اچ‌فرانت — هسته ECH + فرانتینگ دامنه',
    version: 'v1.0.0-ech',
    latestVersion: 'v1.0.0-ech',
    capabilities: ['echfront-ech', 'psiphon-cdn-front', 'shadowtls-v3'],
    color: '#22c55e',
    icon: '🔏',
    description: 'Encrypted Client Hello (ECH) + domain fronting over whitelisted CDNs to hide the real SNI from DPI',
    descriptionFa: 'Encrypted Client Hello (ECH) + فرانتینگ دامنه روی CDNهای مجاز برای مخفی کردن SNI واقعی از DPI',
    githubUrl: 'https://github.com/unifiedshield/echfront-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/echfront-core/releases/latest',
    assetFilter: 'echfront-core-android-arm64',
    role: 'ECH + domain fronting engine',
    roleFa: 'موتور ECH + فرانتینگ دامنه',
    specialFeatures: ['ECH ClientHello', 'Domain fronting', 'SNI concealment', 'CDN whitelist'],
    specialFeaturesFa: ['ECH در ClientHello', 'فرانتینگ دامنه', 'پنهان‌سازی SNI', 'لیست سفید CDN'],
    platforms: ['android', 'windows', 'linux', 'ios', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'obfs4bridge',
    name: 'Obfs4Bridge Pluggable Transport Core',
    nameFa: 'آبفس‌فور — هسته حمل قابل‌تعویض obfs4',
    version: 'v1.0.0-obfs4',
    latestVersion: 'v1.0.0-obfs4',
    capabilities: ['obfs4bridge-obfs4', 'psiphon-ssh-obfs', 'meshrelay-p2p'],
    color: '#a855f7',
    icon: '🌉',
    description: 'Tor obfs4 / pluggable-transport bridges (incl. Snowflake WebRTC) for bridge-based censorship circumvention',
    descriptionFa: 'پل‌های obfs4 / حمل قابل‌تعویض تور (شامل Snowflake WebRTC) برای عبور مبتنی بر پل از فیلترینگ',
    githubUrl: 'https://github.com/unifiedshield/obfs4bridge-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/obfs4bridge-core/releases/latest',
    assetFilter: 'obfs4bridge-core-android-arm64',
    role: 'Pluggable transport bridge engine',
    roleFa: 'موتور پل حمل قابل‌تعویض',
    specialFeatures: ['obfs4 bridges', 'Snowflake WebRTC', 'Bridge rotation', 'Anti-probing'],
    specialFeaturesFa: ['پل‌های obfs4', 'Snowflake WebRTC', 'چرخش پل', 'ضد کاوش'],
    platforms: ['android', 'windows', 'linux', 'ios', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'quicstorm',
    name: 'QUICStorm HTTP/3 Masquerade Core',
    nameFa: 'کوییک‌استورم — هسته استتار QUIC/HTTP3',
    version: 'v1.0.0-quic',
    latestVersion: 'v1.0.0-quic',
    capabilities: ['quicstorm-quic', 'tuic-v5', 'hysteria2'],
    color: '#06b6d4',
    icon: '⚡',
    description: 'QUIC/HTTP3 traffic masqueraded as benign browser video/stream sessions to defeat ML-based DPI classification',
    descriptionFa: 'ترافیک QUIC/HTTP3 استتارشده در قالب جلسه‌های ویدیویی/استریم عادی مرورگر برای شکست طبقه‌بندی DPI مبتنی بر ML',
    githubUrl: 'https://github.com/unifiedshield/quicstorm-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/quicstorm-core/releases/latest',
    assetFilter: 'quicstorm-core-android-arm64',
    role: 'QUIC/HTTP3 masquerade engine',
    roleFa: 'موتور استتار QUIC/HTTP3',
    specialFeatures: ['HTTP/3 multiplex', 'Stream masquerade', 'Brutal CC', 'UDP fallback'],
    specialFeaturesFa: ['مالتی‌پلکس HTTP/3', 'استتار استریم', 'کنترل ازدحام Brutal', 'بکاپ UDP'],
    platforms: ['android', 'windows', 'linux', 'ios', 'macos'],
    checksumSha256: 'verified',
  },
  {
    id: 'portspoof',
    name: 'PortSpoof Dynamic Hopping Core',
    nameFa: 'پورت‌اسپوف — هسته پرش پویا و جعل پورت',
    version: 'v1.0.0-hop',
    latestVersion: 'v1.0.0-hop',
    capabilities: ['portspoof-hop', 'stormdns-dnstt', 'shadowtls-v3'],
    color: '#f97316',
    icon: '🎯',
    description: 'Dynamic port hopping + spoofing across authorized service ports (443/8443/2053) to evade port-based blocking',
    descriptionFa: 'پرش پویا و جعل پورت روی پورت‌های سرویس مجاز (443/8443/2053) برای دور زدن مسدودی پورت‌محور',
    githubUrl: 'https://github.com/unifiedshield/portspoof-core',
    githubApiUrl: 'https://api.github.com/repos/unifiedshield/portspoof-core/releases/latest',
    assetFilter: 'portspoof-core-android-arm64',
    role: 'Port hopping + spoofing engine',
    roleFa: 'موتور پرش و جعل پورت',
    specialFeatures: ['Dynamic port hop', 'Port spoofing', 'Authorized ports', 'Anti-port-block'],
    specialFeaturesFa: ['پرش پویای پورت', 'جعل پورت', 'پورت‌های مجاز', 'ضد مسدودی پورت'],
    platforms: ['android', 'windows', 'linux', 'ios', 'macos'],
    checksumSha256: 'verified',
  },
];

export const PROTOCOL_LABELS: Record<ProtocolType, { name: string; nameFa: string; color: string; antiDpiMethod: string; antiDpiMethodFa: string; priority: number }> = {
  'vless-reality-xtls': { name: 'VLESS + XTLS-Reality', nameFa: 'VLESS Reality + XTLS', color: '#6366f1', antiDpiMethod: 'Reality handshake mimicry', antiDpiMethodFa: 'تقلید دست‌دهی Reality', priority: 1 },
  'amneziawg-1.5': { name: 'AmneziaWG 1.5', nameFa: 'آمنزیاوی‌جی ۱.۵', color: '#ec4899', antiDpiMethod: 'Junk packet injection', antiDpiMethodFa: 'تزریق بسته‌های جونک', priority: 2 },
  'hysteria2': { name: 'Hysteria2', nameFa: 'هیستریا۲', color: '#8b5cf6', antiDpiMethod: 'UDP obfuscation', antiDpiMethodFa: 'پنهان‌سازی UDP', priority: 3 },
  'wireguard-noise': { name: 'WireGuard + Noise', nameFa: 'وایرگارد + نویز', color: '#14b8a6', antiDpiMethod: 'UDP noise packets', antiDpiMethodFa: 'بسته‌های نویز UDP', priority: 4 },
  'vless-fragment': { name: 'VLESS + Fragment', nameFa: 'VLESS + Fragment', color: '#6366f1', antiDpiMethod: 'TLS fragment split', antiDpiMethodFa: 'تقسیم Fragment TLS', priority: 5 },
  'shadowtls-v3': { name: 'ShadowTLS v3', nameFa: 'شدوتی‌ال‌اس نسخه ۳', color: '#06b6d4', antiDpiMethod: 'TLS handshake masking', antiDpiMethodFa: 'پوشش دست‌دهی TLS', priority: 6 },
  'tuic-v5': { name: 'TUIC v5', nameFa: 'TUIC نسخه ۵', color: '#14b8a6', antiDpiMethod: 'QUIC multiplexing', antiDpiMethodFa: 'مولتی‌پلکس QUIC', priority: 7 },
  'trojan-grpc': { name: 'Trojan + WS + TLS', nameFa: 'تروجان + WS + TLS', color: '#f59e0b', antiDpiMethod: 'CDN fronting', antiDpiMethodFa: 'فرانتینگ CDN', priority: 8 },
  'psiphon-cdn-front': { name: 'Psiphon CDN-Front', nameFa: 'سایفون فرانت CDN', color: '#06b6d4', antiDpiMethod: 'CDN domain fronting', antiDpiMethodFa: 'فرانتینگ دامنه CDN', priority: 9 },
  'shadowsocks-shadowtls': { name: 'SS + ShadowTLS', nameFa: 'شادوساکس + شدوتی‌ال‌اس', color: '#f59e0b', antiDpiMethod: 'Double obfuscation', antiDpiMethodFa: 'پنهان‌سازی دوگانه', priority: 10 },
  'vmess-ws-tls': { name: 'VMess + WS + TLS', nameFa: 'VMess + WS + TLS', color: '#10b981', antiDpiMethod: 'WebSocket tunneling', antiDpiMethodFa: 'تونل WebSocket', priority: 11 },
  'mvless': { name: 'MVLESS', nameFa: 'ام‌وی‌لس (MVLESS)', color: '#ef4444', antiDpiMethod: 'MahsaNG custom obfuscation', antiDpiMethodFa: 'پنهان‌سازی سفارشی مهساان‌جی', priority: 12 },
  'psiphon-ssh-obfs': { name: 'Psiphon SSH+Obfs', nameFa: 'سایفون SSH+Obfs', color: '#f97316', antiDpiMethod: 'Obfuscated SSH transport', antiDpiMethodFa: 'حمل SSH مبهم‌شده', priority: 13 },
  'lantern-df-pt': { name: 'Lantern DF+PT', nameFa: 'لنترن DF+PT', color: '#ef4444', antiDpiMethod: 'Domain fronting + transports', antiDpiMethodFa: 'فرانتینگ دامنه + حمل‌ها', priority: 14 },
  'mahsang-obfs': { name: 'MahsaNG Obfs', nameFa: 'مهساان‌جی Obfs', color: '#ef4444', antiDpiMethod: 'Iran-specific custom obfuscation', antiDpiMethodFa: 'پنهان‌سازی سفارشی ایران', priority: 15 },
  'defyxvpn-layers': { name: 'DefyxVPN Layers', nameFa: 'لایه‌های دیفیکس', color: '#8b5cf6', antiDpiMethod: 'Pluggable obfuscation layers', antiDpiMethodFa: 'لایه‌های پنهان‌سازی قابل تعویض', priority: 16 },
  'moav-tunnel': { name: 'MoaV Tunnel', nameFa: 'تونل موآوی', color: '#14b8a6', antiDpiMethod: 'Adaptive tunnel obfuscation', antiDpiMethodFa: 'پنهان‌سازی تونل تطبیقی', priority: 17 },
  'naiveproxy': { name: 'NaiveProxy', nameFa: 'نایوپروکسی', color: '#10b981', antiDpiMethod: 'Chrome network stack mimicry', antiDpiMethodFa: 'تقلید پشته شبکه کروم', priority: 18 },
  'naira-blackout': { name: 'NAIRA Blackout Relay', nameFa: 'رله قطعی ملی نایرا (NAIRA)', color: '#22d3ee', antiDpiMethod: 'National relay + DNS tunnel + AI DPI evasion', antiDpiMethodFa: 'رله ملی + تانل DNS + دور زدن هوشمند DPI', priority: 19 },
  'quazar-quantum': { name: 'QUAZAR Quantum Morph', nameFa: 'جهش کوانتومی کوزار', color: '#a855f7', antiDpiMethod: 'Quantum noise + ML packet morphing', antiDpiMethodFa: 'نویز کوانتومی + تغییر شکل پکت با یادگیری ماشین', priority: 20 },
  'stormdns-dnstt': { name: 'StormDNS DNSTT', nameFa: 'استورم‌دی‌ان‌اس DNSTT', color: '#38bdf8', antiDpiMethod: 'DNS tunneling (DNSTT/EDNS0)', antiDpiMethodFa: 'تانلینگ DNS (DNSTT/EDNS0)', priority: 21 },
  'meshrelay-p2p': { name: 'MeshRelay Offline P2P', nameFa: 'رله آفلاین مش همتا‌به‌همتا', color: '#34d399', antiDpiMethod: 'WebRTC Snowflake + BLE/WiFi Direct rendezvous', antiDpiMethodFa: 'رندو Snowflake WebRTC + بلوتوث/WiFi Direct', priority: 22 },
  'stegostream-web': { name: 'StegoStream Video Masquerade', nameFa: 'استتار استریم ویدیویی استگو', color: '#f472b6', antiDpiMethod: 'WebRTC/RTMP video-stream masquerade', antiDpiMethodFa: 'استتار در استریم ویدیویی WebRTC/RTMP', priority: 23 },
  'rstguard-ttl': { name: 'RSTGuard TTL Gap Filter', nameFa: 'گارد RST جعلی (اختلاف TTL)', color: '#f97316', antiDpiMethod: 'Fake RST drop + TTL gap filtering', antiDpiMethodFa: 'فیلتر RST جعلی + تحلیل اختلاف TTL', priority: 24 },
  'chronoshield-temporal': { name: 'Chronoshield Temporal Throttling', nameFa: 'پروفایلر زمان‌محور تراتلینگ کرونوشیلد', color: '#eab308', antiDpiMethod: 'Peak-hour throttle prediction + UDP→TCP pre-switch', antiDpiMethodFa: 'پیش‌بینی تراتلینگ ساعت اوج + تعویض پیش‌دستانه UDP→TCP', priority: 25 },
  'dualmux-l4': { name: 'DualMux L4 Multipath Aggregation', nameFa: 'تجمیع چندمسیره لایه ۴ دوالماکس', color: '#0ea5e9', antiDpiMethod: 'WiFi + cellular bonding with seamless failover', antiDpiMethodFa: 'ترکیب وای‌فای + دیتا با فیل‌اور بدون قطعی', priority: 26 },
  'cottendns-fec': { name: 'CottenDNS Adaptive FEC', nameFa: 'تصحیح خطای تطبیقی کاتن DNS', color: '#84cc16', antiDpiMethod: 'Super-FEC 8:4 for up to 60% packet loss', antiDpiMethodFa: 'Super-FEC 8:4 برای پکت‌لاس تا ۶۰٪', priority: 27 },
  'masterdns-arq': { name: 'MasterDNS ARQ Multi-Path', nameFa: 'ریسینگ چندمسیره ARQ مستردیانس', color: '#10b981', antiDpiMethod: '8-path racing vs country DNS cache poisoning', antiDpiMethodFa: 'ریسینگ ۸ مسیره علیه پویزنینگ کش DNS کشوری', priority: 28 },
  'oobkeys-sms': { name: 'OOBKeys Staged Key Delivery', nameFa: 'توزیع پیش‌دستانه کلید اوبکیش (SMS/Base45)', color: '#f43f5e', antiDpiMethod: 'Base45/SMS config delivery when DNS+internet dead', antiDpiMethodFa: 'ارسال کانفیگ Base45/SMS وقتی DNS و اینترنت مرده‌اند', priority: 29 },
  'whitedns-utls': { name: 'WhiteDNS uTLS Clean DNS', nameFa: 'DNS پاکیزه وایت‌دی‌ان‌اس با uTLS', color: '#14b8a6', antiDpiMethod: 'uTLS browser-handshake simulation', antiDpiMethodFa: 'شبیه‌سازی دست‌تکانی uTLS مرورگر', priority: 30 },
  'dohdot-anycast': { name: 'DoH/DoT Anycast', nameFa: 'دوه/دات Anycast (RFC 8484/7858)', color: '#2dd4bf', antiDpiMethod: 'Encrypted DNS over Anycast', antiDpiMethodFa: 'DNS رمزنگاری‌شده روی Anycast', priority: 31 },
  'noizdns-noise': { name: 'NoizDNS Noise Obfuscation', nameFa: 'مبهم‌سازی نویز نویزدنس', color: '#7c3aed', antiDpiMethod: 'Noise IK handshake + random jitter', antiDpiMethodFa: 'دست‌تکانی Noise IK + جیتر تصادفی', priority: 32 },
  'sshbridge-ws': { name: 'SSHBridge Secure Wrapper', nameFa: 'پوشش امن SSH بریج', color: '#6366f1', antiDpiMethod: 'SSH over WebSocket/TLS 1.3 + CDN fronting', antiDpiMethodFa: 'SSH روی WebSocket/TLS 1.3 + فرانتینگ CDN', priority: 33 },
  'naivechromium-ja4': { name: 'NaiveChromium JA4', nameFa: 'نایوکرومیوم JA4', color: '#f59e0b', antiDpiMethod: 'Chromium network stack + HTTP/3 multiplex', antiDpiMethodFa: 'پشته شبکه کروم + مالتی‌پلکس HTTP/3', priority: 34 },
  'warpguard-noise': { name: 'WARPGuard Noise Anycast', nameFa: 'نویز Anycast وارپ‌گارد', color: '#0ea5e9', antiDpiMethod: 'Noise handshake + clean Anycast IP', antiDpiMethodFa: 'دست‌تکانی Noise + IP پاکیزه Anycast', priority: 35 },
  'echfront-ech': { name: 'ECHFront Encrypted ClientHello', nameFa: 'ECH مخفی‌ساز SNI اچ‌فرانت', color: '#22c55e', antiDpiMethod: 'ECH + domain fronting SNI concealment', antiDpiMethodFa: 'پنهان‌سازی SNI با ECH + فرانتینگ دامنه', priority: 36 },
  'obfs4bridge-obfs4': { name: 'Obfs4Bridge Pluggable', nameFa: 'پل قابل‌تعویض obfs4', color: '#a855f7', antiDpiMethod: 'obfs4/Tor bridge transport', antiDpiMethodFa: 'حمل پل obfs4/تور', priority: 37 },
  'quicstorm-quic': { name: 'QUICStorm HTTP/3 Masquerade', nameFa: 'استتار QUIC/HTTP3 کوییک‌استورم', color: '#06b6d4', antiDpiMethod: 'QUIC/HTTP3 stream masquerade', antiDpiMethodFa: 'استتار استریم QUIC/HTTP3', priority: 38 },
  'portspoof-hop': { name: 'PortSpoof Dynamic Hop', nameFa: 'پرش پویای پورت پورت‌اسپوف', color: '#f97316', antiDpiMethod: 'Dynamic port hopping/spoofing', antiDpiMethodFa: 'پرش و جعل پویای پورت', priority: 39 },
};

export const IRAN_IP_RANGES = [
  '5.160.0.0/12', '31.56.0.0/14', '37.32.0.0/14', '46.32.0.0/12',
  '62.60.0.0/14', '77.36.0.0/14', '78.38.0.0/14', '80.191.0.0/14',
  '84.47.0.0/14', '85.9.0.0/14', '86.57.0.0/14', '91.92.0.0/14',
  '92.50.0.0/14', '93.110.0.0/14', '94.101.0.0/14', '95.80.0.0/14',
  '109.72.0.0/14', '151.232.0.0/14', '159.20.0.0/14', '164.215.0.0/14',
  '176.65.0.0/14', '178.22.0.0/14', '185.2.0.0/14', '188.121.0.0/14',
  '194.225.0.0/14', '213.176.0.0/14', '217.11.0.0/14', '217.146.0.0/14',
];

export const DNS_PROVIDERS = [
  { id: 'cloudflare', name: 'Cloudflare', url: 'https://1.1.1.1/dns-query', dotUrl: '1.1.1.1', nameFa: 'کلودفلر' },
  { id: 'google', name: 'Google', url: 'https://8.8.8.8/dns-query', dotUrl: '8.8.8.8', nameFa: 'گوگل' },
  { id: 'quad9', name: 'Quad9', url: 'https://9.9.9.9/dns-query', dotUrl: '9.9.9.9', nameFa: 'کواد۹' },
  { id: 'shecan', name: 'Shecan', url: 'https://178.22.122.100/dns-query', dotUrl: '178.22.122.100', nameFa: 'شکن' },
];

export const IRAN_DPI_SIGNATURES = [
  { signature: 'TLS-ClientHello-Reset', descriptionFa: 'بازنشانی ClientHello TLS — شایع‌ترین روش DPI ایران', hex: '16 03 01' },
  { signature: 'HTTP-403-Block', descriptionFa: 'صفحه ۴۰۳ ایرانی — مسدودسازی HTTP', hex: '48 54 54 50 2F 31 2E 31 20 34 30 33' },
  { signature: 'Null-Route', descriptionFa: 'مسیریابی صفر — قطعی بی‌صدا', hex: '00 00 00 00' },
  { signature: 'SNI-Filter', descriptionFa: 'فیلتر SNI — بررسی نام سرور در TLS', hex: 'SNI-Filter-Detected' },
  { signature: 'DNS-Poison', descriptionFa: 'مسمومیت DNS — پاسخ جعلی DNS', hex: 'DNS-Poison-Response' },
  { signature: 'Protocol-Detect', descriptionFa: 'تشخیص پروتکل — شناسایی الگوی پروتکل', hex: 'Protocol-Pattern-Match' },
];

// ──────────────────────────────────────────────
// Kill Switch Types
// ──────────────────────────────────────────────
export interface KillSwitchState {
  enabled: boolean;
  blockAllOnDisconnect: boolean;
  allowedApps: string[];
  networkLock: boolean;
}

// ──────────────────────────────────────────────
// Auto-Reconnect Types
// ──────────────────────────────────────────────
export interface AutoReconnectState {
  enabled: boolean;
  maxRetries: number;
  retryCount: number;
  retryInterval: number;
  exponentialBackoff: boolean;
  lastReconnectAttempt: number;
  reconnectStatus: 'idle' | 'reconnecting' | 'failed' | 'connected';
}

// ──────────────────────────────────────────────
// Connection Log Types
// ──────────────────────────────────────────────
export interface ConnectionLogEntry {
  id: string;
  timestamp: number;
  type: 'connect' | 'disconnect' | 'switch' | 'block' | 'reconnect' | 'dpi-detect' | 'update' | 'error' | 'routing' | 'warning' | 'info';
  message: string;
  messageFa: string;
  coreId?: string;
  details?: Record<string, string>;
}

// ──────────────────────────────────────────────
// Threat Intelligence Types
// ──────────────────────────────────────────────
export interface ThreatEntry {
  id: string;
  type: string;
  typeFa: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  description: string;
  descriptionFa: string;
  detectedAt: number;
  mitigated: boolean;
  countermeasure: string;
  countermeasureFa: string;
}

export interface ThreatIntelState {
  activeThreats: ThreatEntry[];
  lastScan: number;
  threatLevel: 'low' | 'medium' | 'high' | 'critical';
  dpiPatternsUpdated: string;
  blockedDomainsCount: number;
  activeCountermeasures: string[];
}

// ──────────────────────────────────────────────
// Advanced Settings Types
// ──────────────────────────────────────────────
export interface AdvancedSettings {
  language: 'fa' | 'en';
  theme: 'dark' | 'light' | 'system';
  startOnBoot: boolean;
  autoConnectOnLaunch: boolean;
  notifications: boolean;
  stealthMode: boolean;
  debugMode: boolean;
  connectionTimeout: number;
  mtuSize: number;
}

// ──────────────────────────────────────────────
// Network Analyzer Types
// ──────────────────────────────────────────────
export type NetworkType = 'wifi' | 'mobile-data' | 'ethernet' | 'unknown';

export interface BandwidthHistoryEntry {
  timestamp: number;
  uploadMbps: number;
  downloadMbps: number;
  latencyMs: number;
  packetLoss: number;
}

export interface TrafficBreakdown {
  protocol: string;
  protocolFa: string;
  bytesUp: number;
  bytesDown: number;
  percentage: number;
  color: string;
}

export interface DataUsageEntry {
  date: string;
  uploadMb: number;
  downloadMb: number;
  totalMb: number;
}

export interface PacketStats {
  sent: number;
  received: number;
  retransmitted: number;
  lost: number;
  retransmitRate: number;
  lossRate: number;
}

export interface NetworkAnalyzerState {
  isMonitoring: boolean;
  currentUploadMbps: number;
  currentDownloadMbps: number;
  currentLatencyMs: number;
  currentPacketLoss: number;
  currentJitter: number;
  networkType: NetworkType;
  networkTypeFa: string;
  connectionQualityScore: number;
  connectionQualityLabel: string;
  connectionQualityLabelFa: string;
  stabilityIndex: number;
  stabilityLabel: string;
  stabilityLabelFa: string;
  bandwidthHistory: BandwidthHistoryEntry[];
  trafficBreakdown: TrafficBreakdown[];
  packetStats: PacketStats;
  dataUsageDaily: DataUsageEntry[];
  dataUsageWeekly: DataUsageEntry[];
  dataUsageMonthly: DataUsageEntry[];
  totalDataUsedMb: number;
  monitoringStartTime: number;
}

// ──────────────────────────────────────────────
// Geographic Router Types
// ──────────────────────────────────────────────
export interface ServerCountry {
  code: string;
  name: string;
  nameFa: string;
  servers: number;
  activeServers: number;
  avgLatencyMs: number;
  loadPercent: number;
  isHealthy: boolean;
  lastPingMs: number;
  lastChecked: number;
  bandwidthCapacity: number;
  currentLoad: number;
  supportsIranBypass: boolean;
  features: string[];
  featuresFa: string[];
}

export interface GeoLatencyMap {
  [countryCode: string]: {
    latencyMs: number;
    jitterMs: number;
    packetLoss: number;
    lastMeasured: number;
  };
}

export interface LoadBalancingState {
  strategy: 'round-robin' | 'least-connections' | 'lowest-latency' | 'weighted';
  strategyFa: string;
  enabled: boolean;
  currentDistribution: Record<string, number>;
}

export interface GeoRouterState {
  selectedCountry: string;
  selectedCountryFa: string;
  serverList: ServerCountry[];
  latencyMap: GeoLatencyMap;
  loadBalancing: LoadBalancingState;
  autoSelectEnabled: boolean;
  iranInternalBypass: boolean;
  healthCheckInterval: number;
  lastHealthCheck: number;
  recommendedCountry: string;
  recommendedCountryFa: string;
  recommendationReason: string;
  recommendationReasonFa: string;
}

// ──────────────────────────────────────────────
// Security Audit Types
// ──────────────────────────────────────────────
export interface SecurityRecommendation {
  id: string;
  category: string;
  categoryFa: string;
  title: string;
  titleFa: string;
  description: string;
  descriptionFa: string;
  severity: 'info' | 'warning' | 'critical';
  action: string;
  actionFa: string;
  implemented: boolean;
}

export interface DNSLeakResult {
  isLeaking: boolean;
  detectedServers: string[];
  expectedServer: string;
  leakCount: number;
  totalQueries: number;
  testDurationMs: number;
  details: string;
  detailsFa: string;
}

export interface WebRTCLeakResult {
  isLeaking: boolean;
  detectedIPs: string[];
  localIPs: string[];
  publicIPs: string[];
  details: string;
  detailsFa: string;
}

export interface IPv6LeakResult {
  isLeaking: boolean;
  ipv6Address: string | null;
  expectedIPv6: string | null;
  details: string;
  detailsFa: string;
}

export interface EncryptionAssessment {
  protocol: string;
  protocolFa: string;
  keyExchange: string;
  keyExchangeFa: string;
  cipher: string;
  cipherFa: string;
  strength: 'weak' | 'moderate' | 'strong' | 'excellent';
  strengthFa: string;
  score: number;
}

export interface SecurityAuditState {
  isRunning: boolean;
  lastAuditTime: number;
  privacyScore: number;
  privacyScoreLabel: string;
  privacyScoreLabelFa: string;
  dnsLeak: DNSLeakResult;
  webrtcLeak: WebRTCLeakResult;
  ipv6Leak: IPv6LeakResult;
  killSwitchVerified: boolean;
  killSwitchDetails: string;
  killSwitchDetailsFa: string;
  encryptionAssessment: EncryptionAssessment;
  recommendations: SecurityRecommendation[];
  realTimeMonitoring: boolean;
  lastRealTimeCheck: number;
  overallSecurityStatus: 'secure' | 'warning' | 'vulnerable' | 'critical';
  overallSecurityStatusFa: string;
}

export const COUNTRY_SERVERS = [
  { code: 'DE', name: 'Germany', nameFa: 'آلمان', servers: 24 },
  { code: 'NL', name: 'Netherlands', nameFa: 'هلند', servers: 18 },
  { code: 'FI', name: 'Finland', nameFa: 'فنلاند', servers: 8 },
  { code: 'SE', name: 'Sweden', nameFa: 'سوئد', servers: 6 },
  { code: 'FR', name: 'France', nameFa: 'فرانسه', servers: 12 },
  { code: 'US', name: 'USA', nameFa: 'آمریکا', servers: 30 },
  { code: 'CA', name: 'Canada', nameFa: 'کانادا', servers: 10 },
  { code: 'GB', name: 'UK', nameFa: 'انگلستان', servers: 14 },
  { code: 'JP', name: 'Japan', nameFa: 'ژاپن', servers: 8 },
  { code: 'KR', name: 'South Korea', nameFa: 'کره جنوبی', servers: 6 },
  { code: 'SG', name: 'Singapore', nameFa: 'سنگاپور', servers: 10 },
  { code: 'AU', name: 'Australia', nameFa: 'استرالیا', servers: 5 },
  { code: 'BR', name: 'Brazil', nameFa: 'برزیل', servers: 4 },
  { code: 'IN', name: 'India', nameFa: 'هند', servers: 8 },
  { code: 'TR', name: 'Turkey', nameFa: 'ترکیه', servers: 16 },
  { code: 'AE', name: 'UAE', nameFa: 'امارات', servers: 6 },
  { code: 'CH', name: 'Switzerland', nameFa: 'سوئیس', servers: 8 },
  { code: 'NO', name: 'Norway', nameFa: 'نروژ', servers: 4 },
  { code: 'PL', name: 'Poland', nameFa: 'لهستان', servers: 6 },
  { code: 'ES', name: 'Spain', nameFa: 'اسپانیا', servers: 5 },
];

// ──────────────────────────────────────────────
// Evidence-Driven Circumvention R&D & Iran Resilience Intelligence (Engineering Directive v80)
// ──────────────────────────────────────────────

export type IranNetworkCondition =
  | 'NORMAL_INTERNATIONAL'
  | 'PARTIAL_INTERNATIONAL_DEGRADATION'
  | 'DPI_INTERFERENCE'
  | 'DNS_INTERFERENCE'
  | 'TLS_INTERFERENCE'
  | 'TOR_SPECIFIC_BLOCKING'
  | 'REGIONAL_ROUTING_FAILURE'
  | 'INTERNATIONAL_BLACKOUT'
  | 'NATIONAL_NETWORK_ONLY'
  | 'UNKNOWN';

export type NoveltyClassification =
  | 'ALREADY_EXISTS'
  | 'RELATED_TECHNOLOGY'
  | 'PARTIALLY_NOVEL'
  | 'ARCHITECTURALLY_NOVEL'
  | 'NO_IDENTICAL_IMPLEMENTATION_FOUND'
  | 'UNKNOWN';

export type TransportPromotionStage =
  | 'EXPERIMENTAL'
  | 'LAB_VALIDATED'
  | 'NETWORK_VALIDATED'
  | 'REGIONAL_VALIDATED'
  | 'PRODUCTION_CANDIDATE';

export interface ProtocolResearchEntry {
  id: string;
  protocol: string;
  nameFa: string;
  version: string;
  source: string;
  license: string;
  maturity: 'Concept' | 'Experimental' | 'Lab Tested' | 'Field Tested' | 'Production';
  implementationStatus: 'Prototype' | 'Alpha' | 'Beta' | 'Active Engine';
  testStatus: 'Pending' | 'Passed 8-Stage Ladder' | 'Degraded' | 'Verified';
  noveltyClassification: NoveltyClassification;
  noveltyVerdictSummary: string;
  noveltyVerdictSummaryFa: string;
  knownLimitations: string[];
  knownLimitationsFa: string[];
  performanceScore: number; // 0-100
  reliabilityScore: number; // 0-100
  censorshipResistanceScore: number; // 0-100
  detectabilityRiskScore: number; // 0-100 (lower is stealthier)
  operationalComplexityScore: number; // 0-100
  evidenceQualityScore: number; // 0-100
  evidenceCount: number;
  lastBenchmarkTs: number;
}

export interface ControlProbeResult {
  probeType: 'DomesticControlProbe' | 'InternationalControlProbe' | 'DNSControlProbe' | 'TCPControlProbe' | 'TLSControlProbe' | 'HTTPControlProbe';
  target: string;
  targetFa: string;
  latencyMs: number;
  lossRate: number;
  reachable: boolean;
  statusText: string;
  statusTextFa: string;
}

export interface NetworkSeparationAssessment {
  domesticReachability: number; // 0 - 100%
  internationalReachability: number; // 0 - 100%
  internationalLossProbability: number; // 0 - 100%
  censorshipProbability: number; // 0 - 100%
  routingFailureProbability: number; // 0 - 100%
  currentCondition: IranNetworkCondition;
  conditionDescriptionFa: string;
  systemMode: 'TRANSPORT_OPTIMIZATION' | 'OUTAGE_DETECTION_MODE' | 'EXTREME_CENSORSHIP_MODE';
  circumventionFeasibility: 'FEASIBLE' | 'DEGRADED_BYPASS' | 'CIRCUMVENTION_UNLIKELY_DUE_TO_UPSTREAM_CONNECTIVITY_LOSS';
  circumventionFeasibilityFa: string;
  activeControlProbes: ControlProbeResult[];
}

export interface AISelfTestReport {
  adversarialScenarioId: string;
  scenarioName: string;
  scenarioNameFa: string;
  inputCondition: string;
  aiClassification: string;
  groundTruth: string;
  verdict: 'PASS' | 'FAIL';
  distinguishesObservation: boolean; // OBSERVED vs INFERRED vs PREDICTED
  evidenceRequirementMet: boolean;
  confidenceScore: number;
}

export interface RealValidationStageResult {
  stageNumber: number;
  stageName: string;
  stageNameFa: string;
  status: 'PASSED' | 'TESTING' | 'FAILED' | 'SKIPPED';
  latencyMs: number;
  evidenceSnippet: string;
  failureClass: string | null;
}

export interface SpeedIntelligenceEntry {
  regionKey: 'Iran-Mobile' | 'Iran-FTTH' | 'Iran-ADSL' | 'EU-Transit' | 'Global-Edge';
  regionNameFa: string;
  fastestProtocol: string;
  fastestProtocolFa: string;
  medianLatencyMs: number;
  bandwidthMbps: number;
  sampleSize: number;
  confidencePercent: number;
  timestampIso: string;
}

export interface CircumventionRDState {
  extremeCensorshipMode: boolean;
  networkSeparation: NetworkSeparationAssessment;
  protocolRegistry: ProtocolResearchEntry[];
  speedIntelligence: SpeedIntelligenceEntry[];
  aiSelfTestHistory: AISelfTestReport[];
  realTimeValidationLadder: RealValidationStageResult[];
  lastEvidenceCollectionTs: number;
  evidenceCollectionIntervalMins: number; // 15m, 30m, 60m, etc.
}

export interface NovelEnterpriseProtocol {
  id: string;
  name: string;
  nameFa: string;
  badge: string;
  speedRating: string;
  speedNumericMb: number;
  dpiResistancePercent: number;
  intranetBypassPercent: number;
  pingMs: number;
  enterpriseScore: string;
  layer: string;
  masqueradeTarget: string;
  status: 'active' | 'standby' | 'testing' | 'pre-warmed';
  features: string[];
  mechanismFa: string;
  antiFingerprintFa: string;
  stealthScore: number;
  packetLossRecoveryPercent: number;
}

export interface NationalBlackoutShieldState {
  emergencyModeActive: boolean;
  whitelistedSNICamouflage: boolean;
  activeCamouflageSNI: string;
  asymmetricRoutingEnabled: boolean;
  socketAutoFlushMs: number;
  onDeviceAIInferenceMs: number;
  packetEntropyMatch: number;
  activeRelayHop: string;
  nationalBypassStatus: 'bypassing_active' | 'probing' | 'fully_shielded';
  activeNovelProtocolId: string;
  protocols: NovelEnterpriseProtocol[];
  recentDPIEvasionEvents: {
    id: string;
    timestamp: number;
    protocolId: string;
    protocolNameFa: string;
    evasionTechnique: string;
    evasionTechniqueFa: string;
    targetDPIVendor: string;
    responseTimeMs: number;
    success: boolean;
  }[];
}

// ──────────────────────────────────────────────
// Live Connection Telemetry (v9.1)
// Single source of truth for the stats row + session analytics
// ──────────────────────────────────────────────

export interface SessionRecord {
  id: string;
  startTs: number;
  endTs: number;
  durationSec: number;
  protocolId: string;
  protocolNameFa: string;
  dataDownBytes: number;
  dataUpBytes: number;
  disconnectReason: 'manual' | 'dropped' | 'switched' | 'error';
}

export interface ThroughputSample {
  ts: number;
  downMbps: number;
  upMbps: number;
}

export interface ProtocolSuccessStats {
  id: string;
  nameFa: string;
  attempts: number;
  successes: number;
  rollingRate: number; // 0-100
}

export interface LiveConnectionState {
  connectionStartTimestamp: number | null;
  sessionId: string | null;
  dataBytesDown: number;
  dataBytesUp: number;
  switchesPerformed: number;
  blockEventsAvoided: number;
  currentLatencyMs: number;
  latencySamples: number[];
  throughputHistory: ThroughputSample[];
  sessions: SessionRecord[];
  protocolStats: Record<string, ProtocolSuccessStats>;
  lastTickTs: number;
}

// ──────────────────────────────────────────────
// Real Network Measurement (v9.2)
// Single source of truth for live speed / latency / ISP readouts
// ──────────────────────────────────────────────

export interface NetworkMeasurementSample {
  ts: number;
  downloadMbps: number;
  uploadMbps: number;
  latencyMs: number;
}

export interface NetworkStatsState {
  status: 'idle' | 'testing' | 'ready' | 'error';
  downloadMbps: number | null;
  uploadMbps: number | null;
  latencyMs: number | null;
  jitterMs: number | null;
  packetLossPct: number | null;
  isp: string | null;
  ispFa: string | null;
  publicIp: string | null;
  asn: string | null;
  lastTestTs: number | null;
  history: NetworkMeasurementSample[];
  error: string | null;
}


// ──────────────────────────────────────────────
// Advanced Enterprise Features (v9.0)
// Smart Firewall, System Health, Emergency Command
// ──────────────────────────────────────────────

export interface SmartFirewallRule {
  id: string;
  app: string;
  appFa: string;
  domain: string;
  action: 'allow' | 'block' | 'dns-only';
  enabled: boolean;
  blockedAttempts: number;
  lastTriggered?: number;
}

export interface SmartFirewallState {
  mode: 'strict' | 'balanced' | 'permissive';
  learningEnabled: boolean;
  rules: SmartFirewallRule[];
  totalBlocked: number;
  dnsLeakBlocked: number;
  lastBlockTs: number;
}

export interface SystemHealthState {
  cpu: number;
  ram: number;
  netInMbps: number;
  netOutMbps: number;
  dnsLatencyMs: number;
  processCount: number;
  tempC: number;
  battery: number;
  charging: boolean;
  lastUpdateTs: number;
}

export interface EmergencyActionEntry {
  id: string;
  actionId: string;
  labelFa: string;
  severity: 'critical' | 'high' | 'medium';
  executedAt: number;
  descriptionFa: string;
  /** Result of a one-shot action (e.g. network reset) */
  result?: 'running' | 'success' | 'failed';
  /** New state after a toggle action (e.g. stealth mode enabled/disabled) */
  newState?: 'enabled' | 'disabled';
}

// ──────────────────────────────────────────────
// Iran AI Anti-Filtering Smart Scanner (v9.4)
// Dynamic, always-on scanning engine that classifies the live
// filtering level of the Iranian internet and auto-connects
// through the strongest route — no user intervention required.
// ──────────────────────────────────────────────

export type IranFilterLevel =
  | 'open'                 // اینترنت آزاد — بدون فیلترینگ معنادار
  | 'light'                // فیلترینگ سبک — مسدودسازی SNI سایت‌های شناخته‌شده
  | 'dpi'                  // بازرسی عمیق بسته‌ها — DPI فعال (TLS ClientHello Reset)
  | 'sni'                  // فیلترینگ SNI گسترده — کلمات کلیدی و دامنه‌ها
  | 'ip-block'             // مسدودسازی سطح IP — نشت در سطح مسیریابی
  | 'extreme'              // فیلترینگ چندلایه فوق‌سخت — DPI + SNI + IP + محدودسازی هم‌زمان
  | 'international-cutoff' // قطع کامل اینترنت بین‌الملل
  | 'national-only';       // فقط شبکه ملی (قطع ۱۰۰٪ بین‌الملل)

export interface IranProbeResult {
  id: string;
  nameFa: string;
  type: 'dns' | 'tcp' | 'tls' | 'quic' | 'egress' | 'national' | 'doh' | 'ipv6' | 'port' | 'throttle';
  targetFa: string;
  reachable: boolean;
  latencyMs: number;
  detailFa: string;
}

export interface IranScannerCycleRecord {
  id: string;
  timestamp: number;
  durationMs: number;
  filterLevel: IranFilterLevel;
  filterScore: number;
  probesOk: number;
  probesTotal: number;
  bestStrategyId: string;
  bestStrategyFa: string;
  autoConnected: boolean;
  routeCoreId: string;
  routeCoreNameFa: string;
  serverId: string;
  serverNameFa: string;
  multiPathUsed: boolean;
}

export interface IranScannerStrategy {
  id: string;
  level: IranFilterLevel;
  nameFa: string;
  descriptionFa: string;
  coreId: string;
  protocolId: string | null; // novel enterprise protocol (null = none needed)
  nationalEmergency: boolean; // activate national blackout shield
  stepsFa: string[];
}

export interface IranScannerState {
  // engine
  scanning: boolean;
  lastScanTs: number | null;
  scanCycleCount: number;
  nextScanInSec: number;
  intervalSec: number;
  autoPilot: boolean;

  // live classification
  filterLevel: IranFilterLevel;
  filterScore: number; // 0-100 filtering severity
  filterLevelFa: string;
  filterLevelDescFa: string;

  // probes of the last cycle
  probes: IranProbeResult[];

  // AI decision
  activeStrategyId: string;
  activeStrategyFa: string;
  aiReasonFa: string;
  routeCoreId: string;
  routeCoreNameFa: string;

  // history (for timeline + sparkline)
  history: IranScannerCycleRecord[];
  filterScoreHistory: number[];

  // dynamic environment model (drifts every cycle)
  environment: {
    dpiIntensity: number;
    sniCoverage: number;
    ipBlockCoverage: number;
    internationalReach: number;
    nationalReach: number;
    driftPhase: number;
  };

  // v10 — very-hard multi-layer filtering model (اندازه‌گیری هم‌زمان لایه‌های سخت)
  hardLayers: IranHardLayer[];

  // v10 — AI auto server discovery (کشف خودکار قوی‌ترین سرور در هر سطح)
  servers: IranServerNode[];
  activeServerId: string | null;
  serverScanTs: number;

  // v10 — self-learning strategy weights (یادگیری هوش مصنوعی از موفقیت استراتژی‌ها)
  learning: IranStrategyLearning[];

  // v10 — Iranian ISP national coverage (پوشش اپراتورهای ایران)
  ispCoverage: IranIspCoverage[];

  // v10 — filtering intensity forecast (پیش‌بینی هوشمند موج فیلترینگ)
  forecast: IranForecastPoint[];

  // v10 — quantum multi-path routing (مسیریابی کوانتومی چندمسیره)
  multiPath: {
    enabled: boolean;
    activePaths: IranMultiPathRoute[];
  };

  // v10 — adaptive scan engine + scan throughput stats
  adaptiveInterval: boolean;
  probesExecutedTotal: number;
  scansToday: number;
  lastScanDurationMs: number;
  avgDurationMs: number;
}

export const IRAN_SCANNER_STRATEGIES: IranScannerStrategy[] = [
  {
    id: 'direct-best',
    level: 'open',
    nameFa: 'اتصال مستقیم بهینه',
    descriptionFa: 'اینترنت باز است — بهترین هسته از نظر تأخیر انتخاب می‌شود',
    coreId: 'xray-gfw',
    protocolId: null,
    nationalEmergency: false,
    stepsFa: ['انتخاب هسته با کمترین تأخیر', 'فعال‌سازی DNS امن DoH', 'اتصال مستقیم بدون استتار'],
  },
  {
    id: 'light-sni-rotate',
    level: 'light',
    nameFa: 'چرخش هوشمند SNI',
    descriptionFa: 'فیلترینگ سبک — SNI های شناخته‌شده مسدودند، بقیه باز',
    coreId: 'hiddify',
    protocolId: null,
    nationalEmergency: false,
    stepsFa: ['دور زدن SNI های مسدود', 'تزریق SNI جایگزین بانکی', 'فعال‌سازی DNS-over-HTTPS'],
  },
  {
    id: 'dpi-fragment-reality',
    level: 'dpi',
    nameFa: 'شکستن DPI با Fragment + Reality',
    descriptionFa: 'بازرسی عمیق بسته فعال — الگوی TLS ClientHello خنثی می‌شود',
    coreId: 'mahsang',
    protocolId: 'shadowtls-mutator',
    nationalEmergency: false,
    stepsFa: ['تکه‌سازی پکت TLS ClientHello', 'استتار با پروتکل ShadowTLS جهش‌یافته', 'پدینگ تطبیقی آنتروپی'],
  },
  {
    id: 'sni-camouflage',
    level: 'sni',
    nameFa: 'استتار SNI با دامنه‌های مجاز',
    descriptionFa: 'فیلترینگ SNI گسترده — ترافیک در پوشش دامنه‌های بانکی مجاز',
    coreId: 'sing-box',
    protocolId: 'grpc-relay-mux',
    nationalEmergency: false,
    stepsFa: ['استتار با SNI شاپرک (sep.shaparak.ir)', 'مالتی‌پلکس gRPC روی TLS', 'جلوگیری از نشت SNI'],
  },
  {
    id: 'ip-rotation-cdn',
    level: 'ip-block',
    nameFa: 'چرخش IP از طریق رله CDN',
    descriptionFa: 'مسدودسازی سطح IP — اتصال از طریق رله‌های ابری داخلی/خارجی',
    coreId: 'xray-gfw',
    protocolId: 'hysteria2-brutal',
    nationalEmergency: false,
    stepsFa: ['چرخش خودکار IP اگریس', 'مسیریابی از رله CDN (آروان/ابرآروان)', 'تثبیت هویت با Hysteria2-Brutal'],
  },
  {
    id: 'national-quantum-shield',
    level: 'international-cutoff',
    nameFa: 'سپر ملی کوانتومی (قطع بین‌الملل)',
    descriptionFa: 'اینترنت بین‌الملل قطع است — اتصال از طریق رله‌های داخلی و استگانوگرافی کوانتومی',
    coreId: 'naira',
    protocolId: 'qns-v4',
    nationalEmergency: true,
    stepsFa: ['فعال‌سازی حالت اضطراری شبکه ملی', 'استگانوگرافی کوانتومی QNS-v4 در ترافیک مجاز داخلی', 'مسیریابی نامتقارن + رله داخلی', 'استتار کامل در ترافیک شاپرک و آپارات'],
  },
  {
    id: 'national-relay-qns',
    level: 'national-only',
    nameFa: 'رله ملی QNS (فقط شبکه داخلی)',
    descriptionFa: 'فقط اینترنت ملی در دسترس — ارتباط از طریق رله‌های داخلی و پروتکل‌های شناسایی‌نشده',
    coreId: 'naira',
    protocolId: 'qns-v4',
    nationalEmergency: true,
    stepsFa: ['اتصال به رله داخلی (آسیاتک/ایران‌تل)', 'فعال‌سازی QNS-v4 روی ترافیک مجاز', 'همگام‌سازی با گره‌های خروجی منطقه‌ای'],
  },
  {
    id: 'quantum-multipath-fragmentation',
    level: 'extreme',
    nameFa: 'مسیریابی کوانتومی چندمسیره + شکستن چندلایه',
    descriptionFa: 'فیلترینگ فوق‌سخت هم‌زمان روی DPI، SNI، IP و محدودسازی پهنای‌باند — تکه‌سازی پکت در ۳ مسیر موازی و بازسازی در مقصد',
    coreId: 'quazar',
    protocolId: 'qns-v4',
    nationalEmergency: false,
    stepsFa: ['تکه‌سازی پکت TLS در ۳ مسیر موازی (MPTCP کوانتومی)', 'استتار هم‌زمان SNI + Fragment + IP چرخشی', 'شفاف‌سازی در برابر محدودسازی پهنای‌باند (pacing adapt)', 'بازسازی ترتیب بسته‌ها در گره خروجی'],
  },
];

export const IRAN_FILTER_LEVELS: Record<IranFilterLevel, { labelFa: string; descFa: string; score: number }> = {
  'open': { labelFa: 'اینترنت باز', descFa: 'بدون فیلترینگ معنادار — دسترسی کامل به تمام سرویس‌ها', score: 6 },
  'light': { labelFa: 'فیلترینگ سبک', descFa: 'مسدودسازی SNI سایت‌های شناخته‌شده — اکثر سرویس‌ها در دسترس', score: 24 },
  'dpi': { labelFa: 'بازرسی عمیق بسته (DPI)', descFa: 'تشخیص و بازنشانی الگوی TLS ClientHello — نیاز به استتار پکت', score: 52 },
  'sni': { labelFa: 'فیلترینگ SNI گسترده', descFa: 'مسدودسازی بر اساس کلمات کلیدی و دامنه — نیاز به استتار SNI', score: 63 },
  'ip-block': { labelFa: 'مسدودسازی سطح IP', descFa: 'مسدودسازی آدرس‌های IP خارجی — نیاز به چرخش IP و رله CDN', score: 74 },
  'extreme': { labelFa: 'فیلترینگ چندلایه فوق‌سخت', descFa: 'DPI + SNI + IP + محدودسازی پهنای‌باند هم‌زمان — نیاز به مسیریابی کوانتومی چندمسیره', score: 84 },
  'international-cutoff': { labelFa: 'قطع اینترنت بین‌الملل', descFa: 'قطع کامل مسیر بین‌الملل — اتصال از طریق شبکه ملی', score: 92 },
  'national-only': { labelFa: 'فقط شبکه ملی', descFa: 'فقط اینترنت داخلی در دسترس — سپر کوانتومی ملی فعال', score: 96 },
};

// ──────────────────────────────────────────────
// v10 — Iran AI Quantum Scanner: hard layers, server discovery,
// strategy learning, ISP coverage, forecast, multi-path routing
// ──────────────────────────────────────────────

/** لایه‌های فیلتر سخت — اندازه‌گیری هم‌زمان هر مکانیزم سرکوب */
export interface IranHardLayer {
  id: 'throttling' | 'tls-fingerprint' | 'quic-block' | 'time-window' | 'port-block';
  nameFa: string;
  value: number; // 0..100 شدت لایه
  active: boolean;
  detailFa: string;
}

/** گره سرور/رله کشف‌شده — AI بهترین را انتخاب و به آن متصل می‌شود */
export interface IranServerNode {
  id: string;
  nameFa: string;
  regionFa: string;
  type: 'international' | 'national' | 'cdn-relay';
  protocolId: string;
  latencyMs: number;
  loadPct: number;
  score: number; // امتیاز ترکیبی AI (تأخیر، بار، سازگاری با سطح)
  reachable: boolean;
  lastProbeTs: number;
}

/** یادگیری استراتژی — نرخ موفقیت و وزن تطبیقی هر استراتژی */
export interface IranStrategyLearning {
  strategyId: string;
  attempts: number;
  successes: number;
  winRatePct: number;
  weight: number; // وزن AI در انتخاب
}

/** پوشش اپراتورهای ایران — سالم‌سازی زنده شبکه ملی */
export interface IranIspCoverage {
  id: string;
  nameFa: string;
  reachable: boolean;
  latencyMs: number;
  detailFa: string;
}

/** نقطه پیش‌بینی شدت فیلترینگ */
export interface IranForecastPoint {
  hour: number; // ساعت ۰..۲۳ به وقت ایران
  expectedScore: number; // 0..100
  band: 'low' | 'medium' | 'high' | 'critical';
}

/** مسیر فعال در مسیریابی کوانتومی چندمسیره */
export interface IranMultiPathRoute {
  id: string;
  pathFa: string;
  active: boolean;
  latencyMs: number;
  bytesPct: number; // سهم بار هر مسیر
}

export const IRAN_SERVER_POOL: Omit<IranServerNode, 'latencyMs' | 'loadPct' | 'score' | 'reachable' | 'lastProbeTs'>[] = [
  { id: 'srv-fra-1', nameFa: 'فرانکفورت', regionFa: 'آلمان', type: 'international', protocolId: 'shadowtls-mutator' },
  { id: 'srv-ams-1', nameFa: 'آمستردام', regionFa: 'هلند', type: 'international', protocolId: 'qns-v4' },
  { id: 'srv-ist-1', nameFa: 'استانبول', regionFa: 'ترکیه', type: 'international', protocolId: 'hysteria2-brutal' },
  { id: 'srv-dxb-1', nameFa: 'دبی', regionFa: 'امارات', type: 'international', protocolId: 'grpc-relay-mux' },
  { id: 'srv-arn-1', nameFa: 'آروان', regionFa: 'رله CDN داخلی', type: 'cdn-relay', protocolId: 'qns-v4' },
  { id: 'srv-shecan-1', nameFa: 'شبکه شاتل', regionFa: 'رله CDN داخلی', type: 'cdn-relay', protocolId: 'grpc-relay-mux' },
  { id: 'srv-asiatech-1', nameFa: 'آسیا‌تک', regionFa: 'رله ملی', type: 'national', protocolId: 'qns-v4' },
  { id: 'srv-irantel-1', nameFa: 'ایران‌تل', regionFa: 'رله ملی', type: 'national', protocolId: 'qns-v4' },
];

export const IRAN_ISP_LIST: { id: string; nameFa: string }[] = [
  { id: 'mci', nameFa: 'همراه اول (MCI)' },
  { id: 'irancell', nameFa: 'ایرانسل (MTN)' },
  { id: 'rightel', nameFa: 'رایتل' },
  { id: 'tci', nameFa: 'مخابرات (TCI/ADSL)' },
  { id: 'shatel', nameFa: 'شاتل' },
  { id: 'parsonline', nameFa: 'پارس‌آنلاین' },
  { id: 'asiatech', nameFa: 'آسیا‌تک' },
];


// ──────────────────────────────────────────────
// AI Load Balancer — adaptive traffic distribution
// across cores, protocols, tunnels and direct mode
// ──────────────────────────────────────────────
export type LoadBalancerMode = 'adaptive' | 'weighted' | 'failover' | 'quantum-multipath';

export interface CoreLoadShare {
  coreId: string;
  coreNameFa: string;
  weightPct: number;
  trafficSharePct: number;
  active: boolean;
  healthScore: number;
}

export interface ProtocolLoadShare {
  protocolId: string;
  protocolNameFa: string;
  coreId: string;
  weightPct: number;
  active: boolean;
  route: 'tunnel' | 'direct';
}

export interface AILoadBalancerState {
  enabled: boolean;
  mode: LoadBalancerMode;
  cores: CoreLoadShare[];
  protocols: ProtocolLoadShare[];
  activeProtocolIds: string[];
  directSharePct: number;
  tunnelSharePct: number;
  lastRebalanceTs: number;
  rebalanceCount: number;
  totalBandwidthMbps: number;
  reasonFa: string;
  aiDecision: {
    recommendedCoreId: string;
    recommendedProtocolId: string;
    confidencePct: number;
  };
}

export interface StealthRotationHistoryEntry {
  id: string;
  timestamp: number;
  fromCoreId: string;
  toCoreId: string;
  fromProtocolId: string;
  toProtocolId: string;
  fingerprintRisk: number;
  techniqueFa: string;
  reasonFa: string;
}

export interface StealthRotationState {
  enabled: boolean;
  activeSinceTs: number;
  lastRotationAt: number;
  rotationCount: number;
  fingerprintRisk: number;
  nextScheduledAt: number;
  jitterMs: number;
  lastCoreId: string;
  lastProtocolId: string;
  techniqueFa: string;
  reasonFa: string;
  history: StealthRotationHistoryEntry[];
}
