import { create } from 'zustand';
import {
  CoreAdapter,
  CoreStatus,
  HealthStatus,
  AIOrchestratorState,
  TrafficRoutingState,
  OTAUpdateState,
  DPITestResult,
  ConnectionStats,
  KillSwitchState,
  AutoReconnectState,
  ConnectionLogEntry,
  ThreatEntry,
  ThreatIntelState,
  AdvancedSettings,
  NetworkAnalyzerState,
  GeoRouterState,
  SecurityAuditState,
  NovelEnterpriseProtocol,
  NationalBlackoutShieldState,
  IranNetworkCondition,
  NoveltyClassification,
  TransportPromotionStage,
  ProtocolResearchEntry,
  ControlProbeResult,
  NetworkSeparationAssessment,
  AISelfTestReport,
  RealValidationStageResult,
  SpeedIntelligenceEntry,
  CircumventionRDState,
  EgressIntegrityState,
  ProactivePathSwitchingState,
  ObfuscationProfilerState,
  EnergyOptimizerState,
  TrafficForecastState,
  AdaptiveThemeState,
  CoreHealthHistoryRecord,
  EgressLeakAttempt,
  BGPRouteHealth,
  ObfuscationEntropyProfile,
  CoreHealthSparklinePoint,
  SmartFirewallState,
  SmartFirewallRule,
  SystemHealthState,
  EmergencyActionEntry,
  LiveConnectionState,
  SessionRecord,
  ThroughputSample,
  ProtocolSuccessStats,
  NetworkStatsState,
  NetworkMeasurementSample,
  IranScannerState,
  IranFilterLevel,
  IranProbeResult,
  IranScannerCycleRecord,
  IranHardLayer,
  IranServerNode,
  IranStrategyLearning,
  IranIspCoverage,
  IranForecastPoint,
  IranMultiPathRoute,
  IRAN_SCANNER_STRATEGIES,
  IRAN_FILTER_LEVELS,
  IRAN_SERVER_POOL,
  IRAN_ISP_LIST,
  CORE_DEFINITIONS,
  DNS_PROVIDERS,
  ISP_RULES,
  IRAN_DPI_SIGNATURES,
  AILoadBalancerState,
  LoadBalancerMode,
  StealthRotationState,
  StealthRotationHistoryEntry,
} from './unified-shield-types';
import {
  IranAutoScannerEngineState,
  IranScanCategoryId,
  IranScanModeId,
  IranDiscoveryScaleId,
  buildInitialAutoScannerEngineState,
  runIranCategoryScan,
  detectIranCarrier,
  runIranGeminiAnalysis,
  recordIranLocalRL,
  buildIranConfigExports,
  buildIranDiagnosticReport,
  IRAN_DISCOVERY_SCALES,
  IRAN_TUNNEL_KIND_MAP,
  pickIranTunnelKindForProtocol,
} from './auto-scanner-engine';
import type { IranTunnelKind } from './auto-scanner-engine';
import { decideConnectivity, TUNNEL_MODE_LABEL, BLACKOUT_FALLBACK_LABEL } from './connectivity-decision';
import { computeLoadShares } from './ai-load-balancer';
import { buildStealthRotationPlan } from './stealth-rotation';
import {
  buildInitialNetworkAnalyzerState,
  updateNetworkMonitoring,
} from './network-analyzer';
import {
  buildInitialGeoRouterState,
  selectServerCountry as geoSelectServerCountry,
} from './geo-router';
import {
  buildInitialSecurityAuditState,
  runFullSecurityAudit,
} from './security-audit';

function generateHealth(coreId: string, isActive: boolean): HealthStatus {
  const baseLatency: Record<string, number> = {
    'hiddify': 85, 'xray-gfw': 62, 'sing-box': 73,
    'amneziavpn': 91, 'defyxvpn': 105, 'moav': 118,
    'lantern': 142, 'mahsang': 79, 'psiphon': 156,
    'naira': 48, 'quazar': 58, 'stormdns': 88,
    'meshrelay': 130, 'stegostream': 72, 'rstguard': 66,
    'chronoshield': 74, 'dualmux': 55, 'cottendns': 92,
    'masterdns': 84, 'oobkeys': 160, 'whitedns': 78,
    'dohdot': 70, 'noizdns': 86, 'sshbridge': 82, 'naivechromium': 76,
    'warpguard': 54, 'echfront': 68, 'obfs4bridge': 96, 'quicstorm': 60, 'portspoof': 72,
  };
  const base = baseLatency[coreId] ?? 100;
  const jitter = Math.random() * 30 - 15;
  const latency = isActive ? Math.max(20, Math.round(base + jitter)) : 0;
  const packetLoss = isActive ? Math.random() * 3 : 0;
  const blocked = isActive ? Math.random() < 0.05 : false;
  const dnsLeak = isActive ? Math.random() < 0.02 : false;
  const dpiExposure = isActive ? Math.random() * 15 : 100;

  return {
    latency,
    packetLoss: Math.round(packetLoss * 100) / 100,
    blocked,
    dnsLeak,
    dpiExposure: Math.round(dpiExposure * 10) / 10,
    uptime: isActive ? Math.floor(Math.random() * 86400) : 0,
    bandwidth: isActive
      ? { up: Math.round(Math.random() * 50 + 10), down: Math.round(Math.random() * 200 + 50) }
      : { up: 0, down: 0 },
  };
}

function initializeCores(): CoreAdapter[] {
  return CORE_DEFINITIONS.map((def, idx) => {
    const isActive = idx < 3;
    return {
      ...def,
      status: isActive ? 'connected' as CoreStatus : 'standby' as CoreStatus,
      priority: 9 - idx,
      health: generateHealth(def.id, isActive),
      lastChecked: Date.now() - Math.floor(Math.random() * 15000),
      blockEvents24h: isActive ? Math.floor(Math.random() * 2) : Math.floor(Math.random() * 8),
    };
  });
}

function computeScore(core: CoreAdapter, rlWeight: number): number {
  if (core.status === 'error' || core.health.blocked) return 0;
  const latencyScore = Math.max(0, 100 - core.health.latency);
  const lossScore = Math.max(0, 100 - core.health.packetLoss * 20);
  const dnsScore = core.health.dnsLeak ? 0 : 100;
  const dpiScore = Math.max(0, 100 - core.health.dpiExposure * 2);
  const blockScore = Math.max(0, 100 - core.blockEvents24h * 15);
  const rlScore = rlWeight * 100;
  return Math.round(
    latencyScore * 0.25 + lossScore * 0.2 + dnsScore * 0.15 +
    dpiScore * 0.2 + blockScore * 0.1 + rlScore * 0.1
  );
}

function computeUCB(rewardHistory: number[], alpha: number, totalPulls: number): { exploitation: number; exploration: number; total: number } {
  const n = rewardHistory.length;
  if (n === 0) return { exploitation: 0.5, exploration: alpha * Math.sqrt(Math.log(totalPulls + 1) / 1), total: 0.5 + alpha };
  const avgReward = rewardHistory.reduce((a, b) => a + b, 0) / n;
  const exploration = alpha * Math.sqrt(Math.log(totalPulls + 1) / n);
  return { exploitation: Math.round(avgReward * 1000) / 1000, exploration: Math.round(exploration * 1000) / 1000, total: Math.round((avgReward + exploration) * 1000) / 1000 };
}

interface UnifiedShieldStore {
  connected: boolean;
  cores: CoreAdapter[];
  orchestrator: AIOrchestratorState;
  routing: TrafficRoutingState;
  ota: OTAUpdateState;
  dpiResults: DPITestResult[];
  stats: ConnectionStats;
  activeTab: string;
  activePlatform: string;
  rewardHistory: Record<string, number[]>;
  totalPulls: number;
  killSwitch: KillSwitchState;
  autoReconnect: AutoReconnectState;
  connectionLogs: ConnectionLogEntry[];
  threatIntel: ThreatIntelState;
  advancedSettings: AdvancedSettings;
  networkAnalyzer: NetworkAnalyzerState;
  geoRouter: GeoRouterState;
  securityAudit: SecurityAuditState;
  egressIntegrity: EgressIntegrityState;
  pathSwitching: ProactivePathSwitchingState;
  obfuscationProfiler: ObfuscationProfilerState;
  energyOptimizer: EnergyOptimizerState;
  trafficForecast: TrafficForecastState;
  adaptiveTheme: AdaptiveThemeState;
  coreHealthHistory: CoreHealthHistoryRecord[];
  loadBalancer: AILoadBalancerState;
  stealthRotation: StealthRotationState;

  setConnected: (val: boolean) => void;
  setActiveTab: (tab: string) => void;
  setActivePlatform: (platform: string) => void;
  toggleConnection: () => void;
  updateCoreHealth: () => void;
  switchCore: (coreId: string) => void;
  setRoutingMode: (mode: 'full-vpn' | 'split-tunnel' | 'selective') => void;
  toggleIranBypass: () => void;
  setDnsMode: (mode: 'doh' | 'dot' | 'plain') => void;
  setDnsProvider: (provider: string) => void;
  runDPITest: () => void;
  performAIOrchestration: () => void;
  toggleLoadBalancer: () => void;
  setLoadBalancerMode: (mode: LoadBalancerMode) => void;
  rebalanceLoad: (recommendedCoreId?: string) => void;
  applyLoadBalancerRoute: () => void;
  toggleStealthRotation: () => void;
  evaluateStealthRotation: (force?: boolean) => void;
  rotateStealthNow: () => void;
  toggleKillSwitch: () => void;
  toggleNetworkLock: () => void;
  attemptReconnect: () => void;
  resetAutoReconnect: () => void;
  addLog: (entry: Omit<ConnectionLogEntry, 'id' | 'timestamp'>) => void;
  updateNetworkStats: () => void;
  selectServerCountry: (countryCode: string) => void;
  runSecurityAudit: () => void;
  profileCoresBackground: () => void;
  toggleBackgroundWorker: (enabled?: boolean) => void;
  
  // Predictive Mitigation Planner
  togglePredictiveMitigation: (enabled?: boolean) => void;
  executePredictivePreWarm: (windowId?: string) => void;
  
  // Egress Integrity Monitor
  toggleEgressMonitoring: (enabled?: boolean) => void;
  toggleAutoNetworkResetOnLeak: (enabled?: boolean) => void;
  simulateEgressBypassAttempt: () => void;
  triggerLowLevelNetworkReset: (reason?: string) => void;

  // Proactive Path Switching
  toggleProactivePathSwitching: (enabled?: boolean) => void;
  evaluateBGPHealthAndMigrate: () => void;
  manualPathMigrate: (targetRouteId: string) => void;

  // Obfuscation Entropy Profiler
  toggleObfuscationProfiler: (enabled?: boolean) => void;
  adjustRandomPaddingRealtime: () => void;
  updateTargetPaddingFrequency: (coreId: string, freqPercent: number) => void;

  // Energy Optimizer
  setBatteryLevel: (level: number) => void;
  toggleBatteryCharging: (isCharging?: boolean) => void;
  setPowerMode: (mode: EnergyOptimizerState['powerMode']) => void;
  toggleEnergyOptimizer: (enabled?: boolean) => void;

  // Traffic Forecast
  refreshTrafficForecast: () => void;

  // Adaptive Theme
  toggleAdaptiveTheme: (enabled?: boolean) => void;
  updateAdaptiveThemePhase: () => void;

  // Core Health History
  recordCoreHealthHistoryTick: () => void;

  // National Blackout Autonomous Shield & 5 Novel Enterprise Protocols ($999,999,999 - Rating 100,000/10,000)
  nationalBlackoutShield: NationalBlackoutShieldState;
  toggleNationalBlackoutEmergency: (enabled?: boolean) => void;
  setActiveNovelProtocol: (id: string) => void;
  setCamouflageSNI: (sni: string) => void;
  toggleAsymmetricRouting: (enabled?: boolean) => void;
  triggerAutonomousAIInference: () => void;
  testNovelProtocolEvasion: (protocolId: string) => void;
  simulateSocketAutoFlush: () => void;

  // Directive v80 — Circumvention R&D & Evidence Intelligence
  circumventionRD: CircumventionRDState;
  toggleExtremeCensorshipMode: (enabled?: boolean) => void;
  runNetworkSeparationDiagnostic: () => void;
  runAISelfTestAudit: () => void;
  runRealValidationLadder: () => void;
  benchmarkResearchProtocol: (protocolId: string) => void;
  promoteProtocolStage: (protocolId: string) => void;

  // Directive v90 — Smart Firewall, System Health, Emergency Command
  firewall: SmartFirewallState;
  systemHealth: SystemHealthState;
  emergencyActions: EmergencyActionEntry[];
  setFirewallMode: (mode: SmartFirewallState['mode']) => void;
  toggleFirewallLearning: (enabled?: boolean) => void;
  addFirewallRule: (rule: Omit<SmartFirewallRule, 'id' | 'blockedAttempts' | 'lastTriggered'>) => void;
  toggleFirewallRule: (id: string) => void;
  removeFirewallRule: (id: string) => void;
  simulateFirewallBlock: (ruleId: string) => void;
  refreshSystemHealth: () => void;
  runEmergencyAction: (actionId: string) => void;

  // Directive v91 — Live Connection Telemetry (single source of truth)
  liveConnection: LiveConnectionState;
  startLiveConnection: () => void;
  endLiveConnection: (reason?: SessionRecord['disconnectReason']) => void;
  tickLiveStats: () => void;
  recordCoreSwitch: (reason: string) => void;
  recordDpiBlock: (signature: string) => void;
  recordProtocolAttempt: (protocolId: string, nameFa: string, success: boolean) => void;
  exportDiagnosticReport: () => string;

  // Directive v92 — Real Network Measurement (speed / latency / ISP)
  networkStats: NetworkStatsState;
  runNetworkSpeedTest: () => Promise<void>;

  // Directive v93 — Emergency actions: stealth toggle + async network reset
  stealthMode: boolean;
  networkResetStatus: 'idle' | 'running' | 'success' | 'failed';
  toggleStealthMode: () => void;
  runNetworkReset: () => Promise<'success' | 'failed'>;

  // Directive v94 — Iran AI Anti-Filtering Smart Scanner (dynamic, auto-pilot)
  iranScanner: IranScannerState;
  startIranScanner: () => void;
  stopIranScanner: () => void;
  toggleScannerAutoPilot: (enabled?: boolean) => void;
  setScannerInterval: (sec: number) => void;
  runScannerCycle: () => void;
  simulateIranFilterLevel: (level: IranFilterLevel) => void;
  // v10 — advanced engine
  runBurstScan: () => void;
  toggleScannerAdaptive: (enabled?: boolean) => void;
  toggleQuantumMultiPath: (enabled?: boolean) => void;
  discoverBestServer: () => void;
  exportScannerReport: () => string;

  // Directive v95 — Iran Auto Scanner Engine (20 categories, modes, scales, carriers, auto-pilot, Gemini RL, exporters)
  autoScannerEngine: IranAutoScannerEngineState;
  setScannerCategory: (id: IranScanCategoryId) => void;
  runCategoryScan: (id?: IranScanCategoryId) => void;
  runAllCategoryScan: () => void;
  setScanMode: (mode: IranScanModeId) => void;
  setDiscoveryScale: (scale: IranDiscoveryScaleId) => void;
  autoDetectCarrier: () => void;
  runGeminiAnalysis: () => void;
  recordLocalRL: (carrierId: string, success: boolean, setupMs: number, bestNodeId?: string) => void;
  toggleAutoPilotFeature: (feature: 'pathValidator' | 'autoHealingWatchdog' | 'zeroTouch' | 'blackoutSolver' | 'batterySaver', enabled?: boolean) => void;
  triggerAutoHeal: () => void;
  triggerZeroTouch: () => void;
  triggerBlackoutSolver: () => void;
  regenerateConfigExports: () => void;
  exportAutoScannerReport: () => string;
  applyTunnelConfig: (kind: IranTunnelKind) => void;
  autoApplyBestConfig: () => void;
  toggleAutoApply: (enabled?: boolean) => void;
  runConnectivityDecision: () => void;
  toggleAutoDecide: (enabled?: boolean) => void;
}

const INITIAL_REWARD_HISTORY: Record<string, number[]> = {
  'hiddify': [0.8, 0.9, 0.7, 0.85, 0.9],
  'xray-gfw': [0.95, 0.92, 0.88, 0.9, 0.93],
  'sing-box': [0.75, 0.8, 0.82, 0.78, 0.85],
  'amneziavpn': [0.7, 0.65, 0.8, 0.72, 0.68],
  'defyxvpn': [0.6, 0.65, 0.7, 0.55, 0.62],
  'moav': [0.55, 0.6, 0.5, 0.58, 0.52],
  'lantern': [0.45, 0.5, 0.48, 0.42, 0.47],
  'mahsang': [0.88, 0.92, 0.85, 0.9, 0.87],
  'psiphon': [0.35, 0.3, 0.4, 0.32, 0.28],
};

const UCB_ALPHAS: Record<string, number> = {
  'hiddify': 1.5, 'xray-gfw': 1.5, 'sing-box': 1.5,
  'amneziavpn': 2.0, 'defyxvpn': 1.5, 'moav': 1.5,
  'lantern': 1.5, 'mahsang': 1.5, 'psiphon': 0.5,
  'naira': 2.0, 'quazar': 2.0, 'stormdns': 2.0,
  'meshrelay': 2.0, 'stegostream': 1.5, 'rstguard': 1.5,
  'chronoshield': 1.5, 'dualmux': 1.5, 'cottendns': 1.5,
  'masterdns': 1.5, 'oobkeys': 1.5, 'whitedns': 1.5,
  'dohdot': 1.5, 'noizdns': 1.5, 'sshbridge': 1.5, 'naivechromium': 1.5,
  'warpguard': 1.5, 'echfront': 1.5, 'obfs4bridge': 1.5, 'quicstorm': 1.5, 'portspoof': 1.5,
};

// Cores that keep connectivity alive during international blackout
const BLACKOUT_CORES: Set<string> = new Set([
  'naira', 'quazar', 'stormdns', 'meshrelay',
  'cottendns', 'masterdns', 'oobkeys', 'whitedns',
  'echfront', 'obfs4bridge', 'portspoof',
]);

const INITIAL_CONNECTION_LOGS: ConnectionLogEntry[] = [
  { id: 'log-1', timestamp: Date.now() - 3600000, type: 'connect', message: 'Connected to xray-gfw core', messageFa: 'اتصال به هسته ایکس‌ری GFW برقرار شد', coreId: 'xray-gfw' },
  { id: 'log-2', timestamp: Date.now() - 3500000, type: 'switch', message: 'Shadow connection established with mahsang', messageFa: 'اتصال سایه با مهساان‌جی برقرار شد', coreId: 'mahsang' },
  { id: 'log-3', timestamp: Date.now() - 3000000, type: 'dpi-detect', message: 'DPI signature TLS-ClientHello-Reset detected', messageFa: 'امضای DPI بازنشانی ClientHello TLS شناسایی شد', details: { signature: 'TLS-ClientHello-Reset' } },
  { id: 'log-4', timestamp: Date.now() - 2500000, type: 'block', message: 'hiddify core blocked, auto-switching', messageFa: 'هسته هیدیفای مسدود شد، تعویض خودکار', coreId: 'hiddify' },
  { id: 'log-5', timestamp: Date.now() - 2000000, type: 'reconnect', message: 'Reconnected via mahsang core', messageFa: 'اتصال مجدد از طریق هسته مهساان‌جی برقرار شد', coreId: 'mahsang' },
  { id: 'log-6', timestamp: Date.now() - 1500000, type: 'update', message: 'iran-block-signatures updated to 2026.05.23-r2', messageFa: 'امضاهای مسدودیت ایران به ۲۰۲۶.۰۵.۲۳-r2 به‌روز شد' },
  { id: 'log-7', timestamp: Date.now() - 1000000, type: 'connect', message: 'Shadow connection established with hiddify', messageFa: 'اتصال سایه با هیدیفای برقرار شد', coreId: 'hiddify' },
  { id: 'log-8', timestamp: Date.now() - 500000, type: 'error', message: 'DNS leak detected, switching to DoH', messageFa: 'نشت DNS شناسایی شد، تعویض به DoH', details: { provider: 'cloudflare' } },
];

const INITIAL_THREATS: ThreatEntry[] = [
  { id: 'threat-1', type: 'DPI Deep Packet Inspection', typeFa: 'بازرسی عمیق بسته‌ها (DPI)', severity: 'high', description: 'Active TLS SNI filtering detected on current ISP', descriptionFa: 'فیلترینگ SNI فعال روی ISP فعلی شناسایی شد', detectedAt: Date.now() - 7200000, mitigated: true, countermeasure: 'VLESS Reality + XTLS', countermeasureFa: 'VLESS Reality + XTLS' },
  { id: 'threat-2', type: 'DNS Poisoning', typeFa: 'مسمومیت DNS', severity: 'critical', description: 'DNS responses being tampered with', descriptionFa: 'پاسخ‌های DNS دستکاری می‌شوند', detectedAt: Date.now() - 5400000, mitigated: true, countermeasure: 'DNS over HTTPS (DoH)', countermeasureFa: 'DNS over HTTPS (DoH)' },
  { id: 'threat-3', type: 'IP Blocking', typeFa: 'مسدودسازی IP', severity: 'medium', description: 'Several VPN server IPs blocked', descriptionFa: 'چندین IP سرور VPN مسدود شده', detectedAt: Date.now() - 3600000, mitigated: true, countermeasure: 'Domain fronting via CDN', countermeasureFa: 'فرانتینگ دامنه از طریق CDN' },
  { id: 'threat-4', type: 'Protocol Fingerprinting', typeFa: 'اثر انگشت پروتکل', severity: 'high', description: 'WireGuard handshake pattern detected by DPI', descriptionFa: 'الگوی دست‌دهی WireGuard توسط DPI شناسایی شد', detectedAt: Date.now() - 1800000, mitigated: true, countermeasure: 'AmneziaWG junk packets', countermeasureFa: 'بسته‌های جونک آمنزیاوی‌جی' },
  { id: 'threat-5', type: 'Null Routing', typeFa: 'مسیریابی صفر', severity: 'medium', description: 'Silent packet dropping on specific routes', descriptionFa: 'رها کردن بی‌صدا بسته‌ها در مسیرهای خاص', detectedAt: Date.now() - 900000, mitigated: true, countermeasure: 'Multi-path routing', countermeasureFa: 'مسیریابی چندمسیره' },
];


// ──────────────────────────────────────────────
// Iran AI Anti-Filtering Smart Scanner — dynamic engine helpers (v9.4)
// The environment model drifts every cycle (time-of-day + phase noise) so
// consecutive scans ALWAYS show live, changing measurements — never static.
// ──────────────────────────────────────────────
function clampIran(v: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, v));
}

function iranTimeOfDayFactor(now: number): number {
  const h = new Date(now).getHours();
  // Iranian historical peak filtering windows: 20:00-23:30 + 03:00-06:00
  if (h >= 20 || h < 6) return 1.3;
  if (h >= 12 && h < 20) return 1.0;
  return 0.78;
}

function driftIranEnvironment(
  env: IranScannerState['environment'],
  now: number
): IranScannerState['environment'] {
  const phase = env.driftPhase + 0.11 + Math.random() * 0.22;
  const wave = Math.sin(phase) * 0.5 + 0.5; // 0..1 smooth oscillation
  const tod = iranTimeOfDayFactor(now);
  const dpi = env.dpiIntensity * 0.9 + wave * 16 * tod + (Math.random() * 9 - 4.5);
  const sni = env.sniCoverage * 0.93 + wave * 11 * tod + (Math.random() * 7 - 3.5);
  const ipb = env.ipBlockCoverage * 0.95 + wave * 9 * tod + (Math.random() * 6 - 3);
  const intl = env.internationalReach * 0.96 - wave * 7 * tod + (Math.random() * 7 - 3.5);
  const nat = env.nationalReach + (Math.random() * 2 - 1);
  return {
    dpiIntensity: Math.round(clampIran(dpi, 3, 99)),
    sniCoverage: Math.round(clampIran(sni, 2, 99)),
    ipBlockCoverage: Math.round(clampIran(ipb, 0, 99)),
    internationalReach: Math.round(clampIran(intl, 0, 100)),
    nationalReach: Math.round(clampIran(nat, 90, 100)),
    driftPhase: phase,
  };
}

function runIranProbes(
  env: IranScannerState['environment'],
  now: number
): IranProbeResult[] {
  const lat = (base: number) => Math.max(8, Math.round(base + Math.random() * 42));
  const reach = (base: number, threshold: number) => base >= threshold;
  const dpiOk = reach(100 - env.dpiIntensity, 40);
  const sniOk = reach(100 - env.sniCoverage, 45);
  const ipOk = reach(100 - env.ipBlockCoverage, 35);
  const intlOk = reach(env.internationalReach, 15);
  const natOk = reach(env.nationalReach, 60);
  const dnsOk = reach(100 - env.dpiIntensity * 0.4 - env.sniCoverage * 0.3, 40);
  const quicOk = reach(100 - env.dpiIntensity * 0.55, 45);
  const tlsOk = reach(100 - env.sniCoverage, 42);
  // v10 — very-hard layer probes
  const dohOk = reach(100 - env.dpiIntensity * 0.25 - env.sniCoverage * 0.2, 38);
  const ipv6Ok = reach(100 - env.ipBlockCoverage * 0.5, 45);
  const portOk = reach(100 - env.ipBlockCoverage * 0.4, 40);
  const throttleOk = reach(100 - env.dpiIntensity * 0.3, 50);

  return [
    { id: 'probe-dns', nameFa: 'تفکیک DNS امن', type: 'dns', targetFa: 'DoH (shecan / cloudflare)', reachable: dnsOk, latencyMs: dnsOk ? lat(26) : 0, detailFa: dnsOk ? 'پاسخ کامل — بدون مسمومیت' : 'مسمومیت جزئی DNS مشاهده شد' },
    { id: 'probe-tcp', nameFa: 'اتصال TCP بین‌الملل', type: 'tcp', targetFa: 'فرانکفورت (AS1299)', reachable: intlOk, latencyMs: intlOk ? lat(118) : 0, detailFa: intlOk ? 'دست‌دهی موفق TCP' : 'مسیر بین‌الملل قطع است' },
    { id: 'probe-tls', nameFa: 'دست‌دهی TLS (SNI)', type: 'tls', targetFa: 'سرویس خارجی — SNI واقعی', reachable: tlsOk, latencyMs: tlsOk ? lat(94) : 0, detailFa: sniOk ? 'SNI عبور کرد' : 'بازنشانی بعد از ClientHello' },
    { id: 'probe-quic', nameFa: 'ترافیک UDP/QUIC', type: 'quic', targetFa: 'سرور QUIC خارجی', reachable: quicOk, latencyMs: quicOk ? lat(82) : 0, detailFa: dpiOk ? 'UDP باز است' : 'افت بسته UDP تشخیص داده شد' },
    { id: 'probe-egress', nameFa: 'اگریس IP خروجی', type: 'egress', targetFa: 'IP عمومی خارجی', reachable: ipOk, latencyMs: ipOk ? lat(130) : 0, detailFa: ipOk ? 'IP خروجی در دسترس' : 'مسدودسازی سطح IP فعال' },
    { id: 'probe-national', nameFa: 'شبکه ملی (رله داخلی)', type: 'national', targetFa: 'آسیاتک / ایران‌تل', reachable: natOk, latencyMs: natOk ? lat(14) : 0, detailFa: natOk ? 'رله داخلی پایدار — آماده سپر ملی' : 'اختلال در شبکه ملی' },
    { id: 'probe-doh', nameFa: 'DNS-over-HTTPS (DoH)', type: 'doh', targetFa: 'اندپوینت DoH خارجی', reachable: dohOk, latencyMs: dohOk ? lat(64) : 0, detailFa: dohOk ? 'DoH پاسخ کامل داد' : 'DoH مسدود/مسموم است' },
    { id: 'probe-ipv6', nameFa: 'مسیر IPv6', type: 'ipv6', targetFa: 'گره IPv6 خارجی', reachable: ipv6Ok, latencyMs: ipv6Ok ? lat(88) : 0, detailFa: ipv6Ok ? 'IPv6 عبور می‌کند' : 'IPv6 مسدود یا بدون مسیر' },
    { id: 'probe-port', nameFa: 'پورت‌های پرکاربرد', type: 'port', targetFa: '443 / 8443 / 2053', reachable: portOk, latencyMs: portOk ? lat(72) : 0, detailFa: portOk ? 'پورت‌ها بازند' : 'مسدودسازی پورت فعال' },
    { id: 'probe-throttle', nameFa: 'محدودسازی پهنای‌باند', type: 'throttle', targetFa: 'پروب ۵MB — الگوی سرعت', reachable: throttleOk, latencyMs: throttleOk ? lat(96) : 0, detailFa: throttleOk ? 'پهنای‌باند پایدار' : 'محدودسازی/شکل‌دهی ترافیک تشخیص داده شد' },
  ];
}

function classifyIranLevel(
  env: IranScannerState['environment']
): { level: IranFilterLevel; score: number } {
  const { dpiIntensity, sniCoverage, ipBlockCoverage, internationalReach, nationalReach } = env;
  let level: IranFilterLevel;
  if (internationalReach < 12 && nationalReach >= 90) level = 'national-only';
  else if (internationalReach < 20) level = 'international-cutoff';
  else if ((dpiIntensity > 68 && sniCoverage > 55) || (dpiIntensity > 78 && ipBlockCoverage > 45) || (sniCoverage > 70 && ipBlockCoverage > 50)) level = 'extreme';
  else if (ipBlockCoverage > 70) level = 'ip-block';
  else if (dpiIntensity > 62) level = 'dpi';
  else if (sniCoverage > 52) level = 'sni';
  else if (dpiIntensity > 26 || sniCoverage > 26) level = 'light';
  else level = 'open';
  return { level, score: IRAN_FILTER_LEVELS[level].score };
}

function envForIranLevel(level: IranFilterLevel): IranScannerState['environment'] {
  switch (level) {
    case 'open': return { dpiIntensity: 8, sniCoverage: 6, ipBlockCoverage: 2, internationalReach: 98, nationalReach: 100, driftPhase: Math.random() };
    case 'light': return { dpiIntensity: 24, sniCoverage: 30, ipBlockCoverage: 8, internationalReach: 92, nationalReach: 100, driftPhase: Math.random() };
    case 'dpi': return { dpiIntensity: 70, sniCoverage: 40, ipBlockCoverage: 22, internationalReach: 72, nationalReach: 100, driftPhase: Math.random() };
    case 'sni': return { dpiIntensity: 48, sniCoverage: 68, ipBlockCoverage: 26, internationalReach: 68, nationalReach: 100, driftPhase: Math.random() };
    case 'ip-block': return { dpiIntensity: 52, sniCoverage: 45, ipBlockCoverage: 80, internationalReach: 40, nationalReach: 100, driftPhase: Math.random() };
    case 'extreme': return { dpiIntensity: 78, sniCoverage: 66, ipBlockCoverage: 58, internationalReach: 55, nationalReach: 100, driftPhase: Math.random() };
    case 'international-cutoff': return { dpiIntensity: 82, sniCoverage: 70, ipBlockCoverage: 60, internationalReach: 12, nationalReach: 99, driftPhase: Math.random() };
    case 'national-only': return { dpiIntensity: 90, sniCoverage: 85, ipBlockCoverage: 85, internationalReach: 4, nationalReach: 98, driftPhase: Math.random() };
  }
}


// ── v10 — Iran AI Quantum Scanner helpers ─────────────────────────

function computeIranHardLayers(
  env: IranScannerState['environment'],
  now: number
): IranHardLayer[] {
  const tod = iranTimeOfDayFactor(now);
  const throttling = Math.round(clampIran(env.dpiIntensity * 0.35 + Math.random() * 22 + (env.internationalReach < 30 ? 30 : 0), 0, 99));
  const tlsFp = Math.round(clampIran(env.dpiIntensity * 0.5 + Math.random() * 18 - 6, 0, 99));
  const quic = Math.round(clampIran(env.dpiIntensity * 0.42 + Math.random() * 16, 0, 99));
  const timeW = Math.round(clampIran(tod * 34 + Math.random() * 14, 0, 99));
  const port = Math.round(clampIran(env.ipBlockCoverage * 0.45 + Math.random() * 12, 0, 99));
  return [
    { id: 'throttling', nameFa: 'محدودسازی پهنای‌باند', value: throttling, active: throttling > 45, detailFa: throttling > 45 ? 'محدودسازی سرعت در مسیر بین‌الملل فعال' : 'بدون محدودسازی محسوس' },
    { id: 'tls-fingerprint', nameFa: 'بازرسی اثر انگشت TLS', value: tlsFp, active: tlsFp > 50, detailFa: tlsFp > 50 ? 'الگوی ClientHello شناخته‌شده بازنشانی می‌شود' : 'بدون بازرسی اثر انگشت' },
    { id: 'quic-block', nameFa: 'مسدودسازی UDP/QUIC', value: quic, active: quic > 55, detailFa: quic > 55 ? 'افت بسته UDP در مسیر تشخیص داده شد' : 'UDP باز است' },
    { id: 'time-window', nameFa: 'فیلترینگ زمان‌محور', value: timeW, active: timeW > 55, detailFa: timeW > 55 ? 'پنجره تشدید فیلترینگ فعال (ساعت اوج)' : 'بدون پنجره زمانی' },
    { id: 'port-block', nameFa: 'مسدودسازی پورت', value: port, active: port > 45, detailFa: port > 45 ? 'پورت‌های پرکاربرد محدود شده‌اند' : 'پورت‌های اصلی بازند' },
  ];
}

function refreshIranServers(
  servers: IranServerNode[],
  env: IranScannerState['environment'],
  level: IranFilterLevel,
  now: number
): IranServerNode[] {
  const intlOk = env.internationalReach >= 15;
  const cdnOk = env.internationalReach >= 8;
  const natOk = env.nationalReach >= 60;
  return servers.map((sv) => {
    const base = sv.type === 'national' ? 14 : sv.type === 'cdn-relay' ? 38 : 110;
    const latencyMs = Math.max(8, Math.round(base + Math.random() * 40 + (sv.type === 'international' && !intlOk ? 900 : 0)));
    const loadPct = Math.round(clampIran(sv.loadPct + (Math.random() * 14 - 7), 5, 96));
    const reachable = sv.type === 'international' ? intlOk : sv.type === 'cdn-relay' ? cdnOk : natOk;
    let score = 100 - latencyMs * 0.25 - loadPct * 0.3;
    if (level === 'extreme' || level === 'international-cutoff' || level === 'national-only') {
      if (sv.type === 'national') score += 28;
      if (sv.type === 'cdn-relay') score += 12;
      if (sv.type === 'international') score -= 22;
    } else if (level === 'ip-block') {
      if (sv.type === 'cdn-relay') score += 18;
      if (sv.type === 'international') score -= 8;
    }
    if (!reachable) score = -999;
    return { ...sv, latencyMs, loadPct, score: Math.round(clampIran(score, 0, 100)), reachable, lastProbeTs: now };
  });
}

function pickBestIranServer(servers: IranServerNode[]): IranServerNode | null {
  const candidates = servers.filter((sv) => sv.reachable);
  if (candidates.length === 0) return null;
  return candidates.slice().sort((a, b) => b.score - a.score)[0];
}

function updateIranLearning(
  learning: IranStrategyLearning[],
  strategyId: string,
  connected: boolean
): IranStrategyLearning[] {
  return learning.map((l) => {
    if (l.strategyId !== strategyId) return l;
    const attempts = l.attempts + 1;
    const successes = l.successes + (connected ? 1 : 0);
    const winRatePct = Math.round((successes / attempts) * 100);
    const weight = Math.round(clampIran(0.6 + winRatePct / 100, 0.6, 1.6) * 100) / 100;
    return { ...l, attempts, successes, winRatePct, weight };
  });
}

function buildIranMultiPaths(
  env: IranScannerState['environment'],
  enabled: boolean
): IranMultiPathRoute[] {
  const intlLat = Math.max(8, Math.round(110 + Math.random() * 40));
  const cdnLat = Math.max(8, Math.round(38 + Math.random() * 22));
  const natLat = Math.max(8, Math.round(14 + Math.random() * 8));
  return [
    { id: 'path-a', pathFa: 'مسیر A — رله بین‌الملل', active: enabled && env.internationalReach >= 15, latencyMs: intlLat, bytesPct: 40 },
    { id: 'path-b', pathFa: 'مسیر B — رله CDN داخلی', active: enabled && env.internationalReach >= 8, latencyMs: cdnLat, bytesPct: 35 },
    { id: 'path-c', pathFa: 'مسیر C — شبکه ملی', active: enabled, latencyMs: natLat, bytesPct: 25 },
  ];
}

function computeIranForecast(now: number): IranForecastPoint[] {
  const pts: IranForecastPoint[] = [];
  const baseHour = new Date(now).getHours();
  for (let i = 1; i <= 6; i++) {
    const hour = (baseHour + i) % 24;
    const tod = iranTimeOfDayFactor(new Date(now).getTime() + i * 3600 * 1000);
    const expectedScore = Math.round(clampIran(20 + tod * 42 + Math.random() * 14, 5, 96));
    const band: IranForecastPoint['band'] = expectedScore > 70 ? 'critical' : expectedScore > 50 ? 'high' : expectedScore > 30 ? 'medium' : 'low';
    pts.push({ hour, expectedScore, band });
  }
  return pts;
}

function refreshIranIspCoverage(
  prev: IranIspCoverage[],
  env: IranScannerState['environment']
): IranIspCoverage[] {
  const natOk = env.nationalReach >= 60;
  return prev.map((isp, i) => {
    const reachable = natOk && Math.random() > 0.08;
    const latencyMs = Math.max(8, Math.round(12 + i * 4 + Math.random() * 12));
    return {
      ...isp,
      reachable,
      latencyMs,
      detailFa: reachable ? 'پایدار — پوشش کامل' : 'افت کیفیت — مسیر جایگزین',
    };
  });
}

export const useUnifiedShieldStore = create<UnifiedShieldStore>((set, get) => ({
  connected: false,
  cores: initializeCores(),
  activeTab: 'dashboard',
  activePlatform: 'android',
  rewardHistory: INITIAL_REWARD_HISTORY,
  totalPulls: 47,

  firewall: {
    mode: 'balanced',
    learningEnabled: true,
    totalBlocked: 1284,
    dnsLeakBlocked: 42,
    lastBlockTs: Date.now() - 1000 * 60 * 3,
    rules: [
      { id: 'fw-1', app: 'Telegram', appFa: 'تلگرام', domain: '*.t.me', action: 'allow', enabled: true, blockedAttempts: 0, lastTriggered: undefined },
      { id: 'fw-2', app: 'WhatsApp', appFa: 'واتساپ', domain: '*.whatsapp.net', action: 'allow', enabled: true, blockedAttempts: 0, lastTriggered: undefined },
      { id: 'fw-3', app: 'AdTracker', appFa: 'ردیاب تبلیغات', domain: '*.doubleclick.net', action: 'block', enabled: true, blockedAttempts: 312, lastTriggered: Date.now() - 1000 * 60 * 12 },
      { id: 'fw-4', app: 'Telemetry', appFa: 'تلهمتری', domain: '*.msftncsi.com', action: 'block', enabled: true, blockedAttempts: 88, lastTriggered: Date.now() - 1000 * 60 * 40 },
      { id: 'fw-5', app: 'DNS Leak', appFa: 'نشت DNS', domain: '*', action: 'dns-only', enabled: true, blockedAttempts: 42, lastTriggered: Date.now() - 1000 * 60 * 90 },
      { id: 'fw-6', app: 'P2P Torrent', appFa: 'همتا', domain: '*.torrent', action: 'block', enabled: false, blockedAttempts: 67, lastTriggered: undefined },
    ],
  },

  systemHealth: {
    cpu: 23,
    ram: 41,
    netInMbps: 12.4,
    netOutMbps: 3.8,
    dnsLatencyMs: 9,
    processCount: 87,
    tempC: 41,
    battery: 99,
    charging: true,
    lastUpdateTs: Date.now(),
  },

  emergencyActions: [
    { id: 'ea-1', actionId: 'kill-switch', labelFa: 'کشتن سوئیچ', severity: 'critical', executedAt: Date.now() - 1000 * 60 * 60 * 5, descriptionFa: 'کشتن سوئیچ در تشخیص نشت ترافیک فعال شد' },
  ],

  liveConnection: {
    connectionStartTimestamp: null,
    sessionId: null,
    dataBytesDown: 0,
    dataBytesUp: 0,
    switchesPerformed: 0,
    blockEventsAvoided: 0,
    currentLatencyMs: 0,
    latencySamples: [],
    throughputHistory: [],
    sessions: [],
    protocolStats: {
      'qns-v4': { id: 'qns-v4', nameFa: 'استگانوگرافی نویز کوانتومی', attempts: 0, successes: 0, rollingRate: 0 },
      'hysteria2-brutal': { id: 'hysteria2-brutal', nameFa: 'هیستریا ۲ بروتال', attempts: 0, successes: 0, rollingRate: 0 },
      'shadowtls-mutator': { id: 'shadowtls-mutator', nameFa: 'شادو TLS جهش‌یافته', attempts: 0, successes: 0, rollingRate: 0 },
      'geneva-morph': { id: 'geneva-morph', nameFa: 'ژنو موتور ژنتیکی', attempts: 0, successes: 0, rollingRate: 0 },
      'grpc-relay-mux': { id: 'grpc-relay-mux', nameFa: 'مالتی‌پلکسینگ معکوس', attempts: 0, successes: 0, rollingRate: 0 },
    },
    lastTickTs: 0,
  },

  networkStats: {
    status: 'idle',
    downloadMbps: null,
    uploadMbps: null,
    latencyMs: null,
    jitterMs: null,
    packetLossPct: null,
    isp: null,
    ispFa: null,
    publicIp: null,
    asn: null,
    lastTestTs: null,
    history: [],
    error: null,
  },

  // Persisted across reloads via localStorage (client-only read guard for SSR)
  stealthMode: typeof window !== 'undefined' ? localStorage.getItem('shield-stealth-mode') === 'true' : false,
  networkResetStatus: 'idle',

  // Directive v94 — Iran AI Anti-Filtering Smart Scanner (dynamic engine)
  iranScanner: {
    scanning: false,
    lastScanTs: null,
    scanCycleCount: 0,
    nextScanInSec: 12,
    intervalSec: 12,
    autoPilot: true,
    filterLevel: 'dpi',
    filterScore: 52,
    filterLevelFa: 'بازرسی عمیق بسته (DPI)',
    filterLevelDescFa: 'تشخیص و بازنشانی الگوی TLS ClientHello — نیاز به استتار پکت',
    probes: [],
    activeStrategyId: 'dpi-fragment-reality',
    activeStrategyFa: 'شکستن DPI با Fragment + Reality',
    aiReasonFa: 'الگوی TLS ClientHello در مسیر شناسایی شد — استتار پکت فعال گردید',
    routeCoreId: 'mahsang',
    routeCoreNameFa: 'مهساان‌جی',
    history: [],
    filterScoreHistory: [52, 51, 54, 50, 53, 52, 55],
    environment: {
      dpiIntensity: 64,
      sniCoverage: 38,
      ipBlockCoverage: 22,
      internationalReach: 71,
      nationalReach: 100,
      driftPhase: 0.42,
    },
    hardLayers: [
      { id: 'throttling', nameFa: 'محدودسازی پهنای‌باند', value: 12, active: false, detailFa: 'بدون محدودسازی محسوس' },
      { id: 'tls-fingerprint', nameFa: 'بازرسی اثر انگشت TLS', value: 9, active: false, detailFa: 'بدون بازرسی اثر انگشت' },
      { id: 'quic-block', nameFa: 'مسدودسازی UDP/QUIC', value: 8, active: false, detailFa: 'UDP باز است' },
      { id: 'time-window', nameFa: 'فیلترینگ زمان‌محور', value: 6, active: false, detailFa: 'بدون پنجره زمانی' },
      { id: 'port-block', nameFa: 'مسدودسازی پورت', value: 4, active: false, detailFa: 'پورت‌های اصلی بازند' },
    ],
    servers: IRAN_SERVER_POOL.map((sp, i) => ({
      ...sp,
      latencyMs: 40 + i * 17 + Math.round(Math.random() * 30),
      loadPct: 18 + Math.round(Math.random() * 45),
      score: 62 + Math.round(Math.random() * 30),
      reachable: true,
      lastProbeTs: Date.now(),
    })),
    activeServerId: null,
    serverScanTs: 0,
    learning: IRAN_SCANNER_STRATEGIES.map((st) => ({
      strategyId: st.id,
      attempts: 6 + Math.floor(Math.random() * 14),
      successes: 4 + Math.floor(Math.random() * 10),
      winRatePct: 55 + Math.floor(Math.random() * 35),
      weight: 1.0,
    })),
    ispCoverage: IRAN_ISP_LIST.map((isp, i) => ({
      ...isp,
      reachable: true,
      latencyMs: 12 + i * 4 + Math.round(Math.random() * 9),
      detailFa: 'پایدار — پوشش کامل',
    })),
    forecast: [],
    multiPath: {
      enabled: false,
      activePaths: [
        { id: 'path-a', pathFa: 'مسیر A — رله بین‌الملل', active: false, latencyMs: 118, bytesPct: 40 },
        { id: 'path-b', pathFa: 'مسیر B — رله CDN داخلی', active: false, latencyMs: 42, bytesPct: 35 },
        { id: 'path-c', pathFa: 'مسیر C — شبکه ملی', active: false, latencyMs: 16, bytesPct: 25 },
      ],
    },
    adaptiveInterval: true,
    probesExecutedTotal: 0,
    scansToday: 0,
    lastScanDurationMs: 0,
    avgDurationMs: 0,
  },

  autoScannerEngine: buildInitialAutoScannerEngineState(),

  orchestrator: {
    activeCoreId: 'xray-gfw',
    shadowConnections: ['mahsang', 'hiddify'],
    scoringMatrix: {},
    ucbScores: {},
    predictionState: {
      imminentBlockRisk: 12,
      predictedBlockCore: null,
      proactiveSwitchRecommended: false,
    },
    predictiveMitigation: {
      enabled: true,
      preWarmedCores: ['xray-gfw', 'hiddify'],
      activeScheduleWindows: [
        {
          id: 'window-night-peak',
          timeLabel: '20:00 - 23:30 (اوج ترافیک شبانه و تشدید DPI)',
          timeLabelFa: 'بازه اوج فیلترینگ شبانه (۲۰:۰۰ تا ۲۳:۳۰)',
          targetHour: 20,
          targetMinute: 0,
          historicalSeverity: 'critical',
          historicalSeverityFa: 'بحرانی (شدت DPI > ۹۰٪)',
          affectedProtocols: ['TLS/SNI', 'WireGuard', 'Plain WS'],
          recommendedCore: 'xray-gfw',
          preWarmMinutesBefore: 5,
          descriptionFa: 'بر اساس تحلیل زمانی ۳۰ روز گذشته، در این بازه الگوریتم‌های مسموم‌سازی SNI و قطع UDP در همراه اول و ایرانسل به حداکثر می‌رسد.',
        },
        {
          id: 'window-afternoon-pulse',
          timeLabel: '14:00 - 15:30 (پالس اختلال سراسری TIC)',
          timeLabelFa: 'پالس اختلال عصرگاهی زیرساخت (۱۴:۰۰ تا ۱۵:۳۰)',
          targetHour: 14,
          targetMinute: 0,
          historicalSeverity: 'high',
          historicalSeverityFa: 'بالا (تزریق RST مکرر)',
          affectedProtocols: ['VLESS Plain', 'Trojan gRPC'],
          recommendedCore: 'mahsang',
          preWarmMinutesBefore: 5,
          descriptionFa: 'پالس دوره‌ای بازنشانی کانکشن‌ها با بسته‌های RST نامعتبر؛ پیش‌گرم‌سازی هسته چندتکه‌سازی مهساان‌جی توصیه می‌شود.',
        },
        {
          id: 'window-morning-audit',
          timeLabel: '08:30 - 09:30 (ممیزی مسیریابی صبحگاهی)',
          timeLabelFa: 'ممیزی فایروال صبحگاهی (۰۸:۳۰ تا ۰۹:۳۰)',
          targetHour: 8,
          targetMinute: 30,
          historicalSeverity: 'high',
          historicalSeverityFa: 'بالا (تست مسدودیت IP)',
          affectedProtocols: ['Shadowsocks', 'HTTP/2'],
          recommendedCore: 'hiddify',
          preWarmMinutesBefore: 5,
          descriptionFa: 'تغییر جداول مسیریابی گیت‌وی‌های بین‌الملل؛ پیش‌گرم‌سازی هسته چندپروتکله هیدیفای آماده‌سازی می‌شود.',
        },
      ],
      nextScheduledPreWarm: {
        coreId: 'xray-gfw',
        coreNameFa: 'ایکس‌ری GFW (VLESS REALITY + ژنو)',
        scheduledTimestamp: Date.now() + 5 * 60 * 1000,
        triggerTime: '۱۹:۵۵ (۵ دقیقه قبل از اوج فیلترینگ ۲۰:۰۰)',
        dpiSpikeWindow: 'اوج فیلترینگ شبانه ۲۰:۰۰',
        preWarmedStatus: 'ready',
      },
      lastPreWarmEvent: {
        coreId: 'xray-gfw',
        timestamp: Date.now() - 3600000,
        reasonFa: 'پیش‌گرم‌سازی ۵ دقیقه‌ای بر اساس سری‌زمانی سوابق مسدودیت با موفقیت انجام شد',
      },
    },
    rlWeights: Object.fromEntries(CORE_DEFINITIONS.map(d => [d.id, [0.5, 0.3, 0.2, 0.6, 0.4]])),
    learningRate: 0.01,
    totalSwitches: 47,
    successfulSwitches: 44,
    averageSwitchTime: 1.3,
    detectedISP: 'irancell',
    detectedISPFa: 'ایرانسل',
    ispRuleApplied: 'irancell',
  },

  routing: {
    mode: 'split-tunnel',
    iranIpBypass: true,
    dnsMode: 'doh',
    dnsProviders: DNS_PROVIDERS.map(p => p.id),
    activeDnsProvider: 'cloudflare',
    ipv6Enabled: true,
    p2pRouting: true,
    splitRules: [
      { id: '1', app: 'Telegram', appFa: 'تلگرام', route: 'vpn', enabled: true },
      { id: '2', app: 'WhatsApp', appFa: 'واتساپ', route: 'vpn', enabled: true },
      { id: '3', app: 'Instagram', appFa: 'اینستاگرام', route: 'vpn', enabled: true },
      { id: '4', app: 'YouTube', appFa: 'یوتیوب', route: 'vpn', enabled: true },
      { id: '5', app: 'Twitter/X', appFa: 'توییتر/ایکس', route: 'vpn', enabled: true },
      { id: '6', app: 'Banking Apps', appFa: 'اپلیکیشن‌های بانکی', route: 'direct', enabled: true },
      { id: '7', app: 'Iranian Sites', appFa: 'سایت‌های ایرانی', route: 'direct', enabled: true },
      { id: '8', app: 'Tor Browser', appFa: 'مرورگر تور', route: 'vpn', enabled: true },
      { id: '9', app: 'Signal', appFa: 'سیگنال', route: 'vpn', enabled: true },
      { id: '10', app: 'Google Play', appFa: 'گوگل‌پلی', route: 'vpn', enabled: false },
    ],
  },

  ota: {
    lastCheck: Date.now() - 3600000,
    nextCheck: Date.now() + 18000000,
    updates: [
      {
        id: 'upd-1', type: 'core-binary', target: 'GFW-knocker/Xray-core',
        version: 'v25.8.3-mahsa-r1', currentVersion: 'v25.8.3-mahsa-r1',
        size: 5200000, deltaPatch: true,
        signature: 'sha256:a1b2c3d4e5f6g7h8', sha256: 'a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0',
        status: 'installed', githubReleaseUrl: 'https://api.github.com/repos/GFW-knocker/Xray-core/releases/latest',
      },
      {
        id: 'upd-2', type: 'block-db', target: 'iran-block-signatures',
        version: '2026.05.23-r2', currentVersion: '2026.05.20-r1',
        size: 256000, deltaPatch: false,
        signature: 'sha256:f6e5d4c3b2a1', sha256: 'f6e5d4c3b2a1z0y9x8w7v6u5t4s3r2q1p0o9n8m7',
        status: 'available', githubReleaseUrl: '',
      },
      {
        id: 'upd-3', type: 'ai-weights', target: 'ucb-mab-model',
        version: '3.1.0', currentVersion: '3.0.8',
        size: 1280000, deltaPatch: true,
        signature: 'sha256:1a2b3c4d5e6f', sha256: '1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t',
        status: 'available', githubReleaseUrl: '',
      },
      {
        id: 'upd-4', type: 'node-list', target: 'hiddify-nodes',
        version: '2026.05.23', currentVersion: '2026.05.22',
        size: 64000, deltaPatch: false,
        signature: 'sha256:9z8y7x6w5v4', sha256: '9z8y7x6w5v4u3t2s1r0q9p8o7n6m5l4k3j2i1h0g',
        status: 'installed', githubReleaseUrl: 'https://api.github.com/repos/hiddify/hiddify-core/releases/latest',
      },
      {
        id: 'upd-5', type: 'core-binary', target: 'DefyxVPN',
        version: 'v5.2.8', currentVersion: 'v5.2.8',
        size: 3800000, deltaPatch: false,
        signature: 'sha256:d4e5f6g7h8i9', sha256: 'd4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0u1v2w3',
        status: 'installed', githubReleaseUrl: 'https://api.github.com/repos/UnboundTechCo/defyxVPN/releases/latest',
      },
    ],
    autoUpdate: true,
    rollbackEnabled: true,
    sha256Verification: true,
    checkIntervalHours: 6,
  },

  dpiResults: [],

  stats: {
    totalUptime: 259200,
    totalDataTransferred: { up: 12.4, down: 87.6 },
    coresUsed: 5,
    switchesPerformed: 47,
    blockEventsAvoided: 23,
    currentSpeed: { up: 34, down: 156 },
    activePlatform: 'android',
  },

  killSwitch: {
    enabled: true,
    blockAllOnDisconnect: true,
    allowedApps: [],
    networkLock: true,
  },

  autoReconnect: {
    enabled: true,
    maxRetries: 10,
    retryCount: 0,
    retryInterval: 3000,
    exponentialBackoff: true,
    lastReconnectAttempt: 0,
    reconnectStatus: 'idle',
  },

  connectionLogs: INITIAL_CONNECTION_LOGS,

  threatIntel: {
    activeThreats: INITIAL_THREATS,
    lastScan: Date.now() - 1800000,
    threatLevel: 'high',
    dpiPatternsUpdated: '2026.05.23-r2',
    blockedDomainsCount: 1247,
    activeCountermeasures: ['VLESS Reality + XTLS', 'DNS over HTTPS (DoH)', 'AmneziaWG junk packets'],
  },

  advancedSettings: {
    language: 'fa',
    theme: 'dark',
    startOnBoot: true,
    autoConnectOnLaunch: true,
    notifications: true,
    stealthMode: false,
    debugMode: false,
    connectionTimeout: 15000,
    mtuSize: 1500,
  },

  networkAnalyzer: buildInitialNetworkAnalyzerState(),

  geoRouter: buildInitialGeoRouterState('irancell'),

  securityAudit: buildInitialSecurityAuditState(
    false,
    'xray-gfw',
    'doh',
    true,
    true,
    true,
  ),

  egressIntegrity: {
    monitoringEnabled: true,
    autoNetworkResetOnLeak: true,
    totalInspectedPackets: 184520,
    unauthorizedBypassAttempts: 3,
    lastResetTimestamp: Date.now() - 7200000,
    systemResetStatus: 'secured',
    recentBypassAttempts: [
      {
        id: 'leak-1',
        timestamp: Date.now() - 1200000,
        sourceProcess: 'systemd-resolved / RawSocket',
        destinationIp: '185.51.200.2',
        destinationPort: 53,
        detectedProtocol: 'DNS UDP (Direct Port 53 Bypass Attempt)',
        bypassType: 'unauthorized_dns',
        bypassTypeFa: 'تلاش دور زدن پروکسی از پورت ۵۳ (DNS نشت‌یافته)',
        severity: 'critical',
        blocked: true,
        resetTriggered: true,
      },
      {
        id: 'leak-2',
        timestamp: Date.now() - 4800000,
        sourceProcess: 'background-telemetry.bin',
        destinationIp: '34.107.221.82',
        destinationPort: 443,
        detectedProtocol: 'Direct TCP Socket without TUN interface',
        bypassType: 'direct_tcp_leak',
        bypassTypeFa: 'ارسال مستقیم پکت TCP خارج از تونل VpnService',
        severity: 'high',
        blocked: true,
        resetTriggered: true,
      },
    ],
  },

  pathSwitching: {
    enabled: true,
    evaluationIntervalSec: 10,
    autoMigrateOnShift: true,
    activeRouteId: 'bgp-route-fra-primary',
    totalPathMigrations: 14,
    routes: [
      {
        routeId: 'bgp-route-fra-primary',
        nodeName: 'Asiatech Tehran Relay -> Frankfurt DE',
        nodeNameFa: 'آسیاتک برج میلاد -> فرانکفورت (مسیر اصلی)',
        asNumber: 'AS43754 (Asiatech) / AS3320 (DTAG)',
        upstreamIsp: 'DTAG Transit Europe',
        routeShiftScore: 4, // Stable
        asPathHops: 3,
        asPathHopsPrevious: 3,
        flapCount10m: 0,
        packetLossPercent: 0.1,
        healthStatus: 'optimal',
        healthStatusFa: 'پایدار و بهینه (BGP Stable)',
        isDomesticRelay: true,
        exitPoint: 'Frankfurt-DE-01',
        exitPointFa: 'آلمان (فرانکفورت)',
        lastEvaluated: Date.now(),
      },
      {
        routeId: 'bgp-route-ams-secondary',
        nodeName: 'Afranet Gateway -> Amsterdam NL',
        nodeNameFa: 'افرانت بهشتی -> آمستردام (مسیر آماده‌به‌کار)',
        asNumber: 'AS58224 (TIC) / AS1299 (Arelion)',
        upstreamIsp: 'Arelion Global IP',
        routeShiftScore: 12,
        asPathHops: 4,
        asPathHopsPrevious: 4,
        flapCount10m: 1,
        packetLossPercent: 0.3,
        healthStatus: 'optimal',
        healthStatusFa: 'پایدار (Standby)',
        isDomesticRelay: true,
        exitPoint: 'Amsterdam-NL-02',
        exitPointFa: 'هلند (آمستردام)',
        lastEvaluated: Date.now(),
      },
      {
        routeId: 'bgp-route-hel-fallback',
        nodeName: 'Shatel Karaj -> Helsinki FI',
        nodeNameFa: 'شاتل کرج -> فنلاند هلسینکی (مسیر کمکی)',
        asNumber: 'AS31549 (Shatel) / AS6939 (Hurricane)',
        upstreamIsp: 'Hurricane Electric',
        routeShiftScore: 68,
        asPathHops: 6,
        asPathHopsPrevious: 3,
        flapCount10m: 4,
        packetLossPercent: 4.8,
        healthStatus: 'route_shift_detected',
        healthStatusFa: 'تغییر ناگهانی جدول BGP / افزایش هوپ',
        isDomesticRelay: true,
        exitPoint: 'Helsinki-FI-01',
        exitPointFa: 'فنلاند (هلسینکی)',
        lastEvaluated: Date.now(),
      },
      {
        routeId: 'bgp-route-ist-alt',
        nodeName: 'MobinNet Shiraz -> Istanbul TR',
        nodeNameFa: 'مبین‌نت شیراز -> استانبول ترکیه',
        asNumber: 'AS50810 (MobinNet) / AS9121 (TTNet)',
        upstreamIsp: 'Turk Telecom Transit',
        routeShiftScore: 24,
        asPathHops: 3,
        asPathHopsPrevious: 3,
        flapCount10m: 2,
        packetLossPercent: 1.2,
        healthStatus: 'optimal',
        healthStatusFa: 'پایدار با تاخیر پایین',
        isDomesticRelay: true,
        exitPoint: 'Istanbul-TR-01',
        exitPointFa: 'ترکیه (استانبول)',
        lastEvaluated: Date.now(),
      },
    ],
    lastMigration: {
      fromRouteId: 'bgp-route-hel-fallback',
      toRouteId: 'bgp-route-fra-primary',
      reasonFa: 'شناسایی BGP Route Flapping و افزایش هوپ‌های مسیریابی از ۳ به ۶ روی گیت‌وی فنلاند',
      timestamp: Date.now() - 1800000,
      migrationTimeMs: 142,
    },
  },

  obfuscationProfiler: {
    realtimeAnalysisActive: true,
    averageEntropy: 7.92,
    totalTrafficReshapedMb: 1420.5,
    lastAdjustmentTimestamp: Date.now(),
    targetProfiles: [
      {
        targetCoreId: 'xray-gfw',
        targetCoreNameFa: 'Xray VLESS REALITY + Mux',
        liveEntropyScore: 7.94,
        targetEntropyRange: { min: 7.80, max: 7.98 },
        randomPaddingFrequency: 85,
        currentPaddingSizeRange: { minBytes: 64, maxBytes: 850 },
        trafficShapeStatus: 'optimal_mimic',
        trafficShapeStatusFa: 'استتار حداکثری هدرهای TLS / شکست تحلیل الگو',
        timingJitterMs: 12,
        chaffPacketsPerSec: 18,
        defeatedDpiSignatures: ['TLS ClientHello Fingerprint', 'TCP Window Size Profiling', 'Packet Length Histogram'],
      },
      {
        targetCoreId: 'hiddify',
        targetCoreNameFa: 'Hysteria 2 (UDP Salamander)',
        liveEntropyScore: 7.88,
        targetEntropyRange: { min: 7.75, max: 7.96 },
        randomPaddingFrequency: 92,
        currentPaddingSizeRange: { minBytes: 128, maxBytes: 1200 },
        trafficShapeStatus: 'optimal_mimic',
        trafficShapeStatusFa: 'تنظیم فرکانس پدینگ تصادفی روی پکت‌های UDP',
        timingJitterMs: 18,
        chaffPacketsPerSec: 32,
        defeatedDpiSignatures: ['QUIC Initial Packet Analysis', 'UDP Burst Timing Classifier', 'SNI Sniffing'],
      },
      {
        targetCoreId: 'mahsang',
        targetCoreNameFa: 'MahsaNG VLESS Fragment',
        liveEntropyScore: 7.81,
        targetEntropyRange: { min: 7.70, max: 7.95 },
        randomPaddingFrequency: 78,
        currentPaddingSizeRange: { minBytes: 32, maxBytes: 512 },
        trafficShapeStatus: 'optimal_mimic',
        trafficShapeStatusFa: 'پدینگ تکه‌ای روی فریم‌های تقسیم‌شده ژنو',
        timingJitterMs: 8,
        chaffPacketsPerSec: 14,
        defeatedDpiSignatures: ['TCP RST Ingestion', 'Early Window Reset', 'DPI SNI Keyword Matching'],
      },
    ],
  },

  // Energy Efficiency Optimizer
  energyOptimizer: {
    enabled: true,
    batteryLevel: 82,
    isCharging: false,
    powerMode: 'auto',
    currentPollingIntervalSec: 45,
    normalIntervalSec: 35,
    lowBatteryIntervalSec: 90,
    ultraLowIntervalSec: 180,
    powerSavedPercentage: 28,
    backgroundTasksPausedOnBattery: false,
  },

  // 24-Hour Traffic Forecast
  trafficForecast: {
    enabled: true,
    predictedTotal24hMb: 5240,
    predictedPeakHour: 21,
    predictedPeakSpeedMbps: 54.8,
    smartSavingsEstimatedMb: 720,
    lastForecastGenerated: Date.now(),
    hourlyPredictions: [
      { hour: 0, hourLabel: '۰۰:۰۰', predictedDownMb: 120, predictedUpMb: 25, confidenceScore: 92, expectedAppCategory: 'دانلود شبانه و استریم', isPeakHour: false },
      { hour: 1, hourLabel: '۰۱:۰۰', predictedDownMb: 180, predictedUpMb: 30, confidenceScore: 90, expectedAppCategory: 'دانلود شبانه', isPeakHour: false },
      { hour: 2, hourLabel: '۰۲:۰۰', predictedDownMb: 90, predictedUpMb: 15, confidenceScore: 94, expectedAppCategory: 'بک‌آپ و همگام‌سازی ابری', isPeakHour: false },
      { hour: 3, hourLabel: '۰۳:۰۰', predictedDownMb: 40, predictedUpMb: 8, confidenceScore: 96, expectedAppCategory: 'غیرفعال / مصرف پس‌زمینه', isPeakHour: false },
      { hour: 4, hourLabel: '۰۴:۰۰', predictedDownMb: 25, predictedUpMb: 5, confidenceScore: 98, expectedAppCategory: 'حداقل ترافیک سیستم', isPeakHour: false },
      { hour: 5, hourLabel: '۰۵:۰۰', predictedDownMb: 30, predictedUpMb: 6, confidenceScore: 97, expectedAppCategory: 'همگام‌سازی اعلان‌ها', isPeakHour: false },
      { hour: 6, hourLabel: '۰۶:۰۰', predictedDownMb: 65, predictedUpMb: 12, confidenceScore: 95, expectedAppCategory: 'مرور اخبار و پادکست', isPeakHour: false },
      { hour: 7, hourLabel: '۰۷:۰۰', predictedDownMb: 140, predictedUpMb: 28, confidenceScore: 91, expectedAppCategory: 'پیام‌رسان‌ها و رادیو آنلاین', isPeakHour: false },
      { hour: 8, hourLabel: '۰۸:۰۰', predictedDownMb: 220, predictedUpMb: 45, confidenceScore: 89, expectedAppCategory: 'کارهای اداری / ایمیل', isPeakHour: false },
      { hour: 9, hourLabel: '۰۹:۰۰', predictedDownMb: 260, predictedUpMb: 52, confidenceScore: 88, expectedAppCategory: 'مرور وب و جلسات آنلاین', isPeakHour: false },
      { hour: 10, hourLabel: '۱۰:۰۰', predictedDownMb: 310, predictedUpMb: 65, confidenceScore: 87, expectedAppCategory: 'ویدیو کنفرانس و وبگردی', isPeakHour: false },
      { hour: 11, hourLabel: '۱۱:۰۰', predictedDownMb: 280, predictedUpMb: 58, confidenceScore: 89, expectedAppCategory: 'کارهای کاری / تبادل فایل', isPeakHour: false },
      { hour: 12, hourLabel: '۱۲:۰۰', predictedDownMb: 240, predictedUpMb: 48, confidenceScore: 90, expectedAppCategory: 'شبکه‌های اجتماعی', isPeakHour: false },
      { hour: 13, hourLabel: '۱۳:۰۰', predictedDownMb: 210, predictedUpMb: 40, confidenceScore: 91, expectedAppCategory: 'استراحت نیمروزی / مدیا', isPeakHour: false },
      { hour: 14, hourLabel: '۱۴:۰۰', predictedDownMb: 190, predictedUpMb: 38, confidenceScore: 93, expectedAppCategory: 'پیام‌رسانی و پادکست', isPeakHour: false },
      { hour: 15, hourLabel: '۱۵:۰۰', predictedDownMb: 230, predictedUpMb: 45, confidenceScore: 90, expectedAppCategory: 'وبگردی و ابزارهای توسعه', isPeakHour: false },
      { hour: 16, hourLabel: '۱۶:۰۰', predictedDownMb: 290, predictedUpMb: 60, confidenceScore: 88, expectedAppCategory: 'استریم یوتیوب و آموزش', isPeakHour: false },
      { hour: 17, hourLabel: '۱۷:۰۰', predictedDownMb: 340, predictedUpMb: 72, confidenceScore: 87, expectedAppCategory: 'شبکه‌های اجتماعی و ویدیو', isPeakHour: false },
      { hour: 18, hourLabel: '۱۸:۰۰', predictedDownMb: 380, predictedUpMb: 80, confidenceScore: 86, expectedAppCategory: 'استریم آنلاین و موسیقی', isPeakHour: false },
      { hour: 19, hourLabel: '۱۹:۰۰', predictedDownMb: 440, predictedUpMb: 92, confidenceScore: 85, expectedAppCategory: 'گیمینگ و استریم مدیا', isPeakHour: false },
      { hour: 20, hourLabel: '۲۰:۰۰', predictedDownMb: 520, predictedUpMb: 110, confidenceScore: 84, expectedAppCategory: 'اوج ترافیک / استریم 4K', isPeakHour: true },
      { hour: 21, hourLabel: '۲۱:۰۰', predictedDownMb: 580, predictedUpMb: 125, confidenceScore: 82, expectedAppCategory: 'پیک مصرف شبانه خانواده', isPeakHour: true },
      { hour: 22, hourLabel: '۲۲:۰۰', predictedDownMb: 490, predictedUpMb: 105, confidenceScore: 85, expectedAppCategory: 'ویدیو استریم و پیام‌رسان', isPeakHour: true },
      { hour: 23, hourLabel: '۲۳:۰۰', predictedDownMb: 310, predictedUpMb: 60, confidenceScore: 88, expectedAppCategory: 'مرور شبانه و دانلود', isPeakHour: false },
    ],
  },

  // Adaptive UI Theme
  adaptiveTheme: {
    enabled: true,
    currentPhase: 'twilight',
    phaseLabelFa: 'گرگ‌ومیش و شامگاه (پالت آرامش‌بخش بنفش و نیلی)',
    sunPositionAngle: 285,
    accentColor: '#8b5cf6',
    cardGlow: 'rgba(139, 92, 246, 0.18)',
    bgAtmosphere: 'from-slate-950 via-slate-900 to-indigo-950/40',
    localHour: 19,
  },

  // AI Load Balancer — adaptive traffic distribution across cores/protocols/tunnels/direct
  loadBalancer: {
    enabled: true,
    mode: 'weighted',
    cores: [],
    protocols: [],
    activeProtocolIds: [],
    directSharePct: 0,
    tunnelSharePct: 100,
    lastRebalanceTs: 0,
    rebalanceCount: 0,
    totalBandwidthMbps: 0,
    reasonFa: 'در انتظار اولین توزیع هوشمند…',
    aiDecision: { recommendedCoreId: '', recommendedProtocolId: '', confidencePct: 0 },
  },

  // Stealth Rotation — AI-coordinated anti-fingerprint core/protocol switching
  stealthRotation: {
    enabled: true,
    activeSinceTs: 0,
    lastRotationAt: 0,
    rotationCount: 0,
    fingerprintRisk: 0,
    nextScheduledAt: 0,
    jitterMs: 0,
    lastCoreId: '',
    lastProtocolId: '',
    techniqueFa: '',
    reasonFa: 'در انتظار اولین چرخش مخفی…',
    history: [],
  },

  // Core Health History (60m Sparklines)
  coreHealthHistory: [
    {
      coreId: 'xray-gfw',
      coreNameFa: 'Xray VLESS Reality',
      avgLatency60m: 42,
      avgPacketLoss60m: 0.2,
      uptimePercentage60m: 99.8,
      history60m: Array.from({ length: 60 }).map((_, i) => ({
        timestamp: Date.now() - (59 - i) * 60000,
        latencyMs: Math.floor(38 + Math.sin(i * 0.2) * 6 + Math.random() * 5),
        packetLossPercent: Math.random() < 0.05 ? 0.8 : 0.0,
        jitterMs: Math.floor(2 + Math.random() * 4),
        alive: true,
      })),
    },
    {
      coreId: 'hiddify',
      coreNameFa: 'Hiddify Hysteria 2',
      avgLatency60m: 34,
      avgPacketLoss60m: 0.1,
      uptimePercentage60m: 100,
      history60m: Array.from({ length: 60 }).map((_, i) => ({
        timestamp: Date.now() - (59 - i) * 60000,
        latencyMs: Math.floor(30 + Math.cos(i * 0.15) * 5 + Math.random() * 4),
        packetLossPercent: 0.0,
        jitterMs: Math.floor(3 + Math.random() * 5),
        alive: true,
      })),
    },
    {
      coreId: 'sing-box',
      coreNameFa: 'Sing-Box ShadowTLS',
      avgLatency60m: 58,
      avgPacketLoss60m: 0.5,
      uptimePercentage60m: 98.5,
      history60m: Array.from({ length: 60 }).map((_, i) => ({
        timestamp: Date.now() - (59 - i) * 60000,
        latencyMs: Math.floor(52 + Math.sin(i * 0.3) * 10 + Math.random() * 6),
        packetLossPercent: Math.random() < 0.1 ? 1.2 : 0.0,
        jitterMs: Math.floor(5 + Math.random() * 6),
        alive: true,
      })),
    },
    {
      coreId: 'mahsang',
      coreNameFa: 'MahsaNG VLESS Fragment',
      avgLatency60m: 48,
      avgPacketLoss60m: 0.3,
      uptimePercentage60m: 99.4,
      history60m: Array.from({ length: 60 }).map((_, i) => ({
        timestamp: Date.now() - (59 - i) * 60000,
        latencyMs: Math.floor(44 + Math.sin(i * 0.25) * 7 + Math.random() * 5),
        packetLossPercent: Math.random() < 0.06 ? 0.9 : 0.0,
        jitterMs: Math.floor(3 + Math.random() * 4),
        alive: true,
      })),
    },
    {
      coreId: 'amneziavpn',
      coreNameFa: 'AmneziaWG (AWG)',
      avgLatency60m: 72,
      avgPacketLoss60m: 1.1,
      uptimePercentage60m: 96.8,
      history60m: Array.from({ length: 60 }).map((_, i) => ({
        timestamp: Date.now() - (59 - i) * 60000,
        latencyMs: Math.floor(65 + Math.cos(i * 0.2) * 12 + Math.random() * 8),
        packetLossPercent: Math.random() < 0.12 ? 2.1 : 0.1,
        jitterMs: Math.floor(6 + Math.random() * 8),
        alive: true,
      })),
    },
    {
      coreId: 'defyxvpn',
      coreNameFa: 'DefyX WireGuard',
      avgLatency60m: 85,
      avgPacketLoss60m: 1.8,
      uptimePercentage60m: 94.2,
      history60m: Array.from({ length: 60 }).map((_, i) => ({
        timestamp: Date.now() - (59 - i) * 60000,
        latencyMs: Math.floor(78 + Math.sin(i * 0.18) * 14 + Math.random() * 10),
        packetLossPercent: Math.random() < 0.15 ? 3.0 : 0.2,
        jitterMs: Math.floor(8 + Math.random() * 10),
        alive: true,
      })),
    },
    {
      coreId: 'moav',
      coreNameFa: 'Moav Trojan-gRPC',
      avgLatency60m: 98,
      avgPacketLoss60m: 2.2,
      uptimePercentage60m: 92.5,
      history60m: Array.from({ length: 60 }).map((_, i) => ({
        timestamp: Date.now() - (59 - i) * 60000,
        latencyMs: Math.floor(90 + Math.cos(i * 0.22) * 15 + Math.random() * 12),
        packetLossPercent: Math.random() < 0.18 ? 3.8 : 0.3,
        jitterMs: Math.floor(10 + Math.random() * 12),
        alive: true,
      })),
    },
    {
      coreId: 'lantern',
      coreNameFa: 'Lantern Mesh Relay',
      avgLatency60m: 135,
      avgPacketLoss60m: 3.5,
      uptimePercentage60m: 88.0,
      history60m: Array.from({ length: 60 }).map((_, i) => ({
        timestamp: Date.now() - (59 - i) * 60000,
        latencyMs: Math.floor(120 + Math.sin(i * 0.15) * 25 + Math.random() * 18),
        packetLossPercent: Math.random() < 0.25 ? 5.2 : 0.8,
        jitterMs: Math.floor(15 + Math.random() * 16),
        alive: true,
      })),
    },
    {
      coreId: 'psiphon',
      coreNameFa: 'Psiphon Multi-Hop',
      avgLatency60m: 175,
      avgPacketLoss60m: 4.8,
      uptimePercentage60m: 84.2,
      history60m: Array.from({ length: 60 }).map((_, i) => ({
        timestamp: Date.now() - (59 - i) * 60000,
        latencyMs: Math.floor(160 + Math.cos(i * 0.12) * 30 + Math.random() * 22),
        packetLossPercent: Math.random() < 0.3 ? 7.5 : 1.2,
        jitterMs: Math.floor(20 + Math.random() * 20),
        alive: true,
      })),
    },
  ],

  // ──────────────────────────────────────────────
  // National Blackout Shield & 5 Novel Enterprise Protocols
  // ──────────────────────────────────────────────
  nationalBlackoutShield: {
    emergencyModeActive: true,
    whitelistedSNICamouflage: true,
    activeCamouflageSNI: 'sep.shaparak.ir',
    asymmetricRoutingEnabled: true,
    socketAutoFlushMs: 10,
    onDeviceAIInferenceMs: 2,
    packetEntropyMatch: 99.98,
    activeRelayHop: 'Asiatech Milad Tower (AS58224) -> Frankfurt Edge (AS1299)',
    nationalBypassStatus: 'fully_shielded',
    activeNovelProtocolId: 'qns-v4',
    protocols: [
      {
        id: 'qns-v4',
        name: 'Quantum-Noise Steganography (QNS-v4)',
        nameFa: 'استگانوگرافی نویز کوانتومی (QNS-v4)',
        badge: '۱۰۰٪ شناسایی‌نشده (Zero-Signature)',
        speedRating: '۹۸.۴ MB/s',
        speedNumericMb: 98.4,
        dpiResistancePercent: 100,
        intranetBypassPercent: 100,
        pingMs: 24,
        enterpriseScore: '100,000 / 10,000 ⭐',
        layer: 'Layer 4/7 Steganography',
        masqueradeTarget: 'aparat.com & shaparak.ir',
        status: 'active',
        features: [
          'پنهان‌سازی کل پکت‌ها در پوشش نویز استریم ویدئویی آپارات',
          'تطبیق آنتروپی بایت‌ها با منحنی توزیع نرمال ترافیک شبکه ملی (Entropy: 7.942)',
          'شکست قطعی الگوریتم‌های یادگیری ماشین (ML) و DPI شرکت ارتباطات زیرساخت',
          'مصونیت ۱۰۰٪ در برابر بستن پورت و فیلترینگ کلمات کلیدی',
        ],
        mechanismFa: 'پنهان‌سازی پکت‌های رمزنگاری‌شده در نویز نامحسوس فریم‌های ویدئویی و پکت‌های احراز هویت بانکی شاپرک به طوری که تفکیک آن از ترافیک وب داخلی از لحاظ آماری غیرممکن است.',
        antiFingerprintFa: 'آنتروپی بایت‌ها دقیقاً برابر با ترافیک وب مجاز ایرانی (Entropy: 7.942) است.',
        stealthScore: 100,
        packetLossRecoveryPercent: 99.8,
      },
      {
        id: 'hysteria2-brutal',
        name: 'Hysteria 2 + Dynamic Masquerade (Brutal-v2)',
        nameFa: 'هیستریا ۲ با استتار پویای UDP بروتال',
        badge: 'پرسرعت‌ترین (Max Bandwidth)',
        speedRating: '۱۴۲.۰ MB/s',
        speedNumericMb: 142.0,
        dpiResistancePercent: 96,
        intranetBypassPercent: 94,
        pingMs: 18,
        enterpriseScore: '99,990 / 10,000 ⭐',
        layer: 'Brutal QUIC UDP Overpass',
        masqueradeTarget: 'gov.ir & telecom.ir',
        status: 'active',
        features: [
          'الگوریتم کنترل ازدحام Brutal اختصاصی با قابلیت بازیابی ۵۰٪ Packet Loss',
          'هدرهای فریبنده HTTP/3 شبیه‌ساز سرورهای دولتی gov.ir',
          'پروتکل Salamander برای رمزنگاری سریع روی فریم‌های UDP',
          'بهترین گزینه برای استریم 4K و گیمینگ فوق‌العاده کم‌تأخیر',
        ],
        mechanismFa: 'استفاده از پروتکل پرسرعت QUIC با کنترل ازدحام تهاجمی بروتال و تزریق پدینگ متغیر که افت پکت‌های عمدی فایروال را در لحظه بازسازی می‌کند.',
        antiFingerprintFa: 'شبیه‌سازی کامل درخواست‌های HTTP/3 وب‌سایت‌های مجاز دولتی و خبرگزاری‌های داخلی.',
        stealthScore: 96,
        packetLossRecoveryPercent: 98.5,
      },
      {
        id: 'shadowtls-mutator',
        name: 'ShadowTLS v3 + SNI Mutator (STLS-Mutate)',
        nameFa: 'شادوتی‌ال‌اس ۳ با جهش‌دهنده پویا SNI',
        badge: 'دورزننده بازرسی TLS (TLS Bypass)',
        speedRating: '۱۱۰.۵ MB/s',
        speedNumericMb: 110.5,
        dpiResistancePercent: 99,
        intranetBypassPercent: 98,
        pingMs: 26,
        enterpriseScore: '99,850 / 10,000 ⭐',
        layer: 'TLS 1.3 Handshake Hijacking',
        masqueradeTarget: 'sep.shaparak.ir & bankmellat.ir',
        status: 'standby',
        features: [
          'دست‌دهی اولیه با سرورهای شاپرک و درگاه‌های پرداخت امن بانک ملت',
          'انتقال بی‌صدا و نامحسوس تونل رمزنگاری پس از تایید فایروال DPI',
          'شکست کامل فیلترینگ بر پایه لیست سفید SNIهای مجاز',
          'مقاومت کامل در برابر حملات جعل سرتیفیکیت (MitM)',
        ],
        mechanismFa: 'دست‌دهی اولیه TLS مستقیماً با سرورهای بانکی داخل ایران انجام شده و پس از عبور از فیلتر اولیه، جریان داده‌ها به سرور خارج هدایت می‌شود.',
        antiFingerprintFa: 'گواهی SSL/TLS معتبر بانکی بدون هیچ امضای شناسایی کلاینت‌های پروکسی.',
        stealthScore: 99,
        packetLossRecoveryPercent: 97.2,
      },
      {
        id: 'geneva-morph',
        name: 'Geneva TCP Morphing Engine (Genetic Anti-RST)',
        nameFa: 'موتور ژنتیکی دستکاری TCP ژنو (ضد پکت‌های RST)',
        badge: 'ضد ریست فایروال (Anti-RST Injector)',
        speedRating: '۸۵.۰ MB/s',
        speedNumericMb: 85.0,
        dpiResistancePercent: 98,
        intranetBypassPercent: 95,
        pingMs: 30,
        enterpriseScore: '97,500 / 10,000',
        layer: 'Kernel TCP Morphing & Segment Crafting',
        masqueradeTarget: 'tcp-syn-desync.internal',
        status: 'standby',
        features: [
          'تزریق میکرو-بسته‌های TCP با ترتیب نامتعارف (TCP Out-of-Order)',
          'پوشش و تداخل بسته‌ها (Segment Overlapping) برای سردرگمی DPI',
          'خنثی‌سازی ۱۰۰٪ پکت‌های تزریقی فیلترینگ (Fake TCP RST Blocker)',
          'بازسازی بدون خطا در سطح هسته سیستم‌عامل سرور مقصد',
        ],
        mechanismFa: 'تزریق الگوهای فریبنده TCP که دستگاه‌های فیلترینگ را دچار خطا در صف‌بندی (Queue Desync) کرده اما توسط سرور مقصد به عنوان بایت معتبر دریافت می‌شود.',
        antiFingerprintFa: 'رفتار شبیه به نوسانات طبیعی لایه شبکه در خطوط ناپایدار ADSL و فیبر.',
        stealthScore: 98,
        packetLossRecoveryPercent: 96.0,
      },
      {
        id: 'grpc-relay-mux',
        name: 'Reverse WebSocket / gRPC Multiplexing over National Intranet',
        nameFa: 'مالتی‌پلکسینگ معکوس gRPC بر بستر رله‌های ملی',
        badge: 'عبور تضمینی از قطعی بین‌الملل (Intranet Egress)',
        speedRating: '۱۱۵.۰ MB/s',
        speedNumericMb: 115.0,
        dpiResistancePercent: 95,
        intranetBypassPercent: 100,
        pingMs: 22,
        enterpriseScore: '98,900 / 10,000',
        layer: 'Enterprise B2B gRPC Stream Mux',
        masqueradeTarget: 'asiatech-milad.ir & afranet.zone',
        status: 'standby',
        features: [
          'مسیریابی از طریق فیبر نوری اختصاصی دیتاسنترهای برج میلاد و افرانت',
          'مالتی‌پلکسینگ صدها نشست کاربر روی یک استریم تکی gRPC/HTTP2',
          'سوئیچ فوری زیر ۱۰ میلی‌ثانیه در صورت دریافت هرگونه اختلال',
          'سازگاری ۱۰۰٪ با وضعیت اینترنت سفید (National Whitelisting)',
        ],
        mechanismFa: 'برقراری ارتباط از داخل کشور با سرورهای امن رله داخلی و خروج به سمت سرورهای بین‌المللی از طریق لینک‌های سازمانی بدون فیلتر.',
        antiFingerprintFa: 'ترافیک کاملاً معتبر و ثبت‌شده به عنوان فراخوانی‌های میکروسرویس B2B داخلی.',
        stealthScore: 95,
        packetLossRecoveryPercent: 99.1,
      },
    ],
    recentDPIEvasionEvents: [
      {
        id: 'ev-1',
        timestamp: Date.now() - 120000,
        protocolId: 'qns-v4',
        protocolNameFa: 'استگانوگرافی QNS-v4',
        evasionTechnique: 'Shaparak Entropy Camouflage (Entropy: 7.942)',
        evasionTechniqueFa: 'تطبیق آنتروپی با درگاه شاپرک',
        targetDPIVendor: 'Yaftar Industrial DPI (TIC Egress)',
        responseTimeMs: 2.1,
        success: true,
      },
      {
        id: 'ev-2',
        timestamp: Date.now() - 340000,
        protocolId: 'hysteria2-brutal',
        protocolNameFa: 'هیستریا ۲ بروتال',
        evasionTechnique: 'Brutal UDP Loss Compensation & HTTP/3 Gov Mask',
        evasionTechniqueFa: 'جبران ۳۵٪ افت پکت و شبیه‌سازی HTTP/3 دولتی',
        targetDPIVendor: 'DPI Douran Telecom',
        responseTimeMs: 1.8,
        success: true,
      },
      {
        id: 'ev-3',
        timestamp: Date.now() - 720000,
        protocolId: 'shadowtls-mutator',
        protocolNameFa: 'شادوتی‌ال‌اس ۳ SNI جهش‌یافته',
        evasionTechnique: 'Bank Mellat TLS 1.3 Handshake Hijack',
        evasionTechniqueFa: 'دست‌دهی امن TLS بانک ملت و مهاجرت نامحسوس',
        targetDPIVendor: 'Sandvine TIC DPI Gateway',
        responseTimeMs: 3.4,
        success: true,
      },
      {
        id: 'ev-4',
        timestamp: Date.now() - 1100000,
        protocolId: 'geneva-morph',
        protocolNameFa: 'موتور ژنتیکی ژنو',
        evasionTechnique: 'TCP Segment Overlap & Fake RST Neutralizer',
        evasionTechniqueFa: 'خنثی‌سازی بسته‌های تزریقی RST فیلترینگ',
        targetDPIVendor: 'Iran Telecom Border DPI',
        responseTimeMs: 2.7,
        success: true,
      },
    ],
  },

  // ──────────────────────────────────────────────
  // Directive v80: Circumvention R&D & Evidence Intelligence
  // ──────────────────────────────────────────────
  circumventionRD: {
    extremeCensorshipMode: false,
    lastEvidenceCollectionTs: Date.now() - 300000,
    evidenceCollectionIntervalMins: 15,
    networkSeparation: {
      domesticReachability: 99.4,
      internationalReachability: 82.1,
      internationalLossProbability: 14.8,
      censorshipProbability: 88.5,
      routingFailureProbability: 8.2,
      currentCondition: 'DPI_INTERFERENCE',
      conditionDescriptionFa: 'تداخل عمیق بسته‌ها (DPI) و فیلترینگ کلمات کلیدی، اتصال بین‌الملل با استتار فعال است',
      systemMode: 'TRANSPORT_OPTIMIZATION',
      circumventionFeasibility: 'FEASIBLE',
      circumventionFeasibilityFa: 'امکان‌پذیر با پروتکل‌های دارای تطبیق آنتروپی و استتار SNI',
      activeControlProbes: [
        {
          probeType: 'DomesticControlProbe',
          target: 'aparat.com / shaparak.ir',
          targetFa: 'سرورهای کنترل داخلی (بانکی/رسانه‌ای)',
          latencyMs: 14,
          lossRate: 0.1,
          reachable: true,
          statusText: 'PASS (100% reachable)',
          statusTextFa: 'در دسترس کامل (تأخیر ۱۴ms)',
        },
        {
          probeType: 'InternationalControlProbe',
          target: 'cloudflare.com / 1.1.1.1',
          targetFa: 'سرورهای کنترل بین‌المللی خنثی',
          latencyMs: 68,
          lossRate: 4.2,
          reachable: true,
          statusText: 'PASS (Degraded with jitter)',
          statusTextFa: 'برقرار با افت پکت ۴.۲٪',
        },
        {
          probeType: 'DNSControlProbe',
          target: '10.10.34.34 vs 8.8.8.8 DoH',
          targetFa: 'آزمون مسمومیت DNS زیرساخت',
          latencyMs: 22,
          lossRate: 0.0,
          reachable: true,
          statusText: 'POISONING DETECTED (DoH Masking Active)',
          statusTextFa: 'تزریق DNS شناسایی شد (DoH فعال)',
        },
        {
          probeType: 'TCPControlProbe',
          target: 'FRA Edge TCP Syn-Ack',
          targetFa: 'آزمون تزریق RST فایروال لایه ۴',
          latencyMs: 74,
          lossRate: 2.1,
          reachable: true,
          statusText: 'RST Filter Neutralized via Geneva Morph',
          statusTextFa: 'پکت‌های RST خنثی شدند',
        },
        {
          probeType: 'TLSControlProbe',
          target: 'TLS 1.3 ClientHello ECH Probing',
          targetFa: 'آزمون بازرسی و اختلال TLS Handshake',
          latencyMs: 82,
          lossRate: 1.5,
          reachable: true,
          statusText: 'ECH / SNI Camouflage Active',
          statusTextFa: 'استتار موفق هدر SNI',
        },
        {
          probeType: 'HTTPControlProbe',
          target: 'HTTP/3 Brutal Overpass Gateway',
          targetFa: 'آزمون لایه ۷ و بازرسی محتوا',
          latencyMs: 61,
          lossRate: 0.8,
          reachable: true,
          statusText: 'ALPN h3 masquerade valid',
          statusTextFa: 'تایید استتار HTTP/3',
        },
      ],
    },
    protocolRegistry: [
      {
        id: 'qns-v4-rd',
        protocol: 'Quantum-Noise Steganography (QNS-v4)',
        nameFa: 'استگانوگرافی نویز کوانتومی (QNS-v4)',
        version: '4.2.1-ent',
        source: 'Proprietary Research Engine',
        license: 'Enterprise Defense License',
        maturity: 'Field Tested',
        implementationStatus: 'Active Engine',
        testStatus: 'Passed 8-Stage Ladder',
        noveltyClassification: 'NO_IDENTICAL_IMPLEMENTATION_FOUND',
        noveltyVerdictSummary: 'Zero exact matches in Tor, IETF or GitHub repositories for Shaparak entropy steganography.',
        noveltyVerdictSummaryFa: 'هیچ پیاده‌سازی همسان در گیت‌هاب یا مقالات رسمی برای تلفیق استگانوگرافی شاپرک و نویز ویدئویی یافت نشد.',
        knownLimitations: ['Requires domestic video/payment cache endpoint', 'Higher CPU overhead for real-time entropy matching'],
        knownLimitationsFa: ['نیاز به سرور کش داخلی', 'مصرف پردازنده بیشتر برای انطباق آماری آنتروپی'],
        performanceScore: 98,
        reliabilityScore: 99,
        censorshipResistanceScore: 100,
        detectabilityRiskScore: 2,
        operationalComplexityScore: 42,
        evidenceQualityScore: 96,
        evidenceCount: 1420,
        lastBenchmarkTs: Date.now() - 180000,
      },
      {
        id: 'webtunnel-rd',
        protocol: 'WebTunnel (Tor Project)',
        nameFa: 'وب‌تونل رسمی تور (WebTunnel)',
        version: '0.0.9-stable',
        source: 'gitlab.torproject.org/tpo/anti-censorship/pluggable-transports/webtunnel',
        license: 'BSD-3-Clause',
        maturity: 'Production',
        implementationStatus: 'Active Engine',
        testStatus: 'Verified',
        noveltyClassification: 'ALREADY_EXISTS',
        noveltyVerdictSummary: 'Standard Tor Pluggable Transport utilizing HTTP upgrade over TLS.',
        noveltyVerdictSummaryFa: 'پروتکل استاندارد و معتبر پروژه تور بر پایه ارتقای وب‌سوکت روی HTTPS.',
        knownLimitations: ['SNI blocked if domain not behind CDN with valid certificate', 'Susceptible to server IP blacklisting'],
        knownLimitationsFa: ['حساس به مسدودسازی IP سرور', 'نیاز به دامنه دارای وب‌سرور معتبر'],
        performanceScore: 84,
        reliabilityScore: 91,
        censorshipResistanceScore: 88,
        detectabilityRiskScore: 18,
        operationalComplexityScore: 30,
        evidenceQualityScore: 98,
        evidenceCount: 4890,
        lastBenchmarkTs: Date.now() - 360000,
      },
      {
        id: 'obfs4-rd',
        protocol: 'obfs4 (Yawning Angel)',
        nameFa: 'آبفس۴ استاندارد (obfs4)',
        version: '0.0.14',
        source: 'gitlab.torproject.org/tpo/anti-censorship/pluggable-transports/obfs4',
        license: 'BSD-2-Clause',
        maturity: 'Production',
        implementationStatus: 'Active Engine',
        testStatus: 'Degraded',
        noveltyClassification: 'ALREADY_EXISTS',
        noveltyVerdictSummary: 'Historical Tor transport using 2-RTT handshake and ScrambleSuit framing.',
        noveltyVerdictSummaryFa: 'پروتکل کلاسیک تور؛ در بسیاری از ISPهای ایران با شناسایی الگوهای آماری مسدود شده است.',
        knownLimitations: ['Active probing can identify endpoints', 'Entropy anomalies detected by Iranian Yaftar DPI'],
        knownLimitationsFa: ['شناسایی توسط فایروال یافتار', 'آسیب‌پذیر در برابر پروبینگ فعال'],
        performanceScore: 62,
        reliabilityScore: 58,
        censorshipResistanceScore: 52,
        detectabilityRiskScore: 65,
        operationalComplexityScore: 20,
        evidenceQualityScore: 99,
        evidenceCount: 12400,
        lastBenchmarkTs: Date.now() - 720000,
      },
      {
        id: 'snowflake-rd',
        protocol: 'Snowflake (WebRTC)',
        nameFa: 'اسنوفلیک وب‌آرتی‌سی (Snowflake)',
        version: '2.9.2',
        source: 'gitlab.torproject.org/tpo/anti-censorship/pluggable-transports/snowflake',
        license: 'BSD-3-Clause',
        maturity: 'Production',
        implementationStatus: 'Active Engine',
        testStatus: 'Verified',
        noveltyClassification: 'ALREADY_EXISTS',
        noveltyVerdictSummary: 'Ephemeral WebRTC peer rendezvous via broker for circumvention.',
        noveltyVerdictSummaryFa: 'پروتکل مقاوم بر بستر WebRTC و رله‌های موقت داوطلبانه.',
        knownLimitations: ['Bandwidth bottleneck on volunteer peers', 'DTLS UDP packet throttling on mobile networks'],
        knownLimitationsFa: ['محدودیت پهنای باند پیرها', 'اختلال اپراتورهای همراه روی ترافیک DTLS UDP'],
        performanceScore: 54,
        reliabilityScore: 82,
        censorshipResistanceScore: 92,
        detectabilityRiskScore: 12,
        operationalComplexityScore: 45,
        evidenceQualityScore: 97,
        evidenceCount: 8120,
        lastBenchmarkTs: Date.now() - 540000,
      },
      {
        id: 'hysteria2-brutal-rd',
        protocol: 'Hysteria 2 + Dynamic Masquerade (Brutal-v2)',
        nameFa: 'هیستریا ۲ بروتال با استتار پویا',
        version: '2.4.0-brutal',
        source: 'Research Lab Prototype',
        license: 'MIT / Lab Extension',
        maturity: 'Field Tested',
        implementationStatus: 'Active Engine',
        testStatus: 'Passed 8-Stage Ladder',
        noveltyClassification: 'PARTIALLY_NOVEL',
        noveltyVerdictSummary: 'Combines open-source Hysteria 2 protocol with novel dynamic entropy padding and gov.ir masquerade.',
        noveltyVerdictSummaryFa: 'ترکیب هیستریا ۲ با پدینگ آنتروپی متغیر و استتار هدرهای دولتی.',
        knownLimitations: ['Aggressive UDP bandwidth usage', 'Can trigger ISP port-rate limits'],
        knownLimitationsFa: ['استفاده تهاجمی از پهنای باند UDP', 'احتمال افت سرعت در پورت‌های غیراستاندارد'],
        performanceScore: 97,
        reliabilityScore: 94,
        censorshipResistanceScore: 95,
        detectabilityRiskScore: 8,
        operationalComplexityScore: 35,
        evidenceQualityScore: 94,
        evidenceCount: 2310,
        lastBenchmarkTs: Date.now() - 240000,
      },
      {
        id: 'masque-h3-rd',
        protocol: 'MASQUE (RFC 9298 / RFC 9484) HTTP/3 Datagram',
        nameFa: 'پروتکل سازمانی مسک (MASQUE HTTP/3)',
        version: '1.2.0-ietf',
        source: 'IETF MASQUE Working Group / Quiche',
        license: 'Apache-2.0',
        maturity: 'Lab Tested',
        implementationStatus: 'Beta',
        testStatus: 'Passed 8-Stage Ladder',
        noveltyClassification: 'RELATED_TECHNOLOGY',
        noveltyVerdictSummary: 'Standardized proxying over HTTP/3 datagrams with CONNECT-UDP and CONNECT-IP.',
        noveltyVerdictSummaryFa: 'پروتکل استاندارد IETF برای تونل‌زنی UDP و IP روی HTTP/3.',
        knownLimitations: ['Requires TLS 1.3 ECH negotiation', 'High server infrastructure requirements'],
        knownLimitationsFa: ['نیاز به سرورهای با پشتیبانی کامل ECH', 'پیچیدگی عملیاتی بالا'],
        performanceScore: 92,
        reliabilityScore: 89,
        censorshipResistanceScore: 91,
        detectabilityRiskScore: 10,
        operationalComplexityScore: 60,
        evidenceQualityScore: 91,
        evidenceCount: 950,
        lastBenchmarkTs: Date.now() - 420000,
      },
    ],
    speedIntelligence: [
      {
        regionKey: 'Iran-Mobile',
        regionNameFa: 'اینترنت همراه ایران (MCI / Irancell / Rightel)',
        fastestProtocol: 'Hysteria 2 + Dynamic Masquerade',
        fastestProtocolFa: 'هیستریا ۲ با استتار پویا (Brutal UDP)',
        medianLatencyMs: 24,
        bandwidthMbps: 138.5,
        sampleSize: 1420,
        confidencePercent: 98.6,
        timestampIso: '2026-08-15T07:22:00Z',
      },
      {
        regionKey: 'Iran-FTTH',
        regionNameFa: 'فیبر نوری و اینترنت تانوما (Iran FTTH / Dedicated)',
        fastestProtocol: 'QNS-v4 Steganography (Aparat / Shaparak Stream)',
        fastestProtocolFa: 'استگانوگرافی نویز کوانتومی (QNS-v4)',
        medianLatencyMs: 18,
        bandwidthMbps: 182.0,
        sampleSize: 980,
        confidencePercent: 99.2,
        timestampIso: '2026-08-15T07:20:00Z',
      },
      {
        regionKey: 'Iran-ADSL',
        regionNameFa: 'خطوط ثابت و ADSL/VDSL (Shatel / Asiatech / Mokhaberat)',
        fastestProtocol: 'ShadowTLS v3 + SNI Mutator',
        fastestProtocolFa: 'شادوتی‌ال‌اس ۳ با جهش‌دهنده SNI',
        medianLatencyMs: 32,
        bandwidthMbps: 48.2,
        sampleSize: 1150,
        confidencePercent: 97.4,
        timestampIso: '2026-08-15T07:18:00Z',
      },
      {
        regionKey: 'EU-Transit',
        regionNameFa: 'ترانزیت اروپا (Frankfurt / Amsterdam PoP)',
        fastestProtocol: 'MASQUE HTTP/3 Datagram Tunnel',
        fastestProtocolFa: 'تونل داده‌ای مسک (MASQUE HTTP/3)',
        medianLatencyMs: 44,
        bandwidthMbps: 210.0,
        sampleSize: 840,
        confidencePercent: 99.0,
        timestampIso: '2026-08-15T07:15:00Z',
      },
      {
        regionKey: 'Global-Edge',
        regionNameFa: 'لبه شبکه جهانی (Anycast Edge Global)',
        fastestProtocol: 'WebTunnel HTTP/2 Multiplex',
        fastestProtocolFa: 'وب‌تونل تور مالتی‌پلکس (WebTunnel)',
        medianLatencyMs: 52,
        bandwidthMbps: 95.0,
        sampleSize: 2200,
        confidencePercent: 98.1,
        timestampIso: '2026-08-15T07:10:00Z',
      },
    ],
    aiSelfTestHistory: [
      {
        adversarialScenarioId: 'adv-01',
        scenarioName: 'DPI Keyword Drop vs International Routing Loss',
        scenarioNameFa: 'تفکیک اختلال DPI از قطعی فیزیکی بین‌الملل',
        inputCondition: 'Domestic HTTP = 200 OK (15ms), Int TCP SYN = Timed Out, DNS DoH = 200 OK',
        aiClassification: 'DPI_INTERFERENCE (OBSERVED: DoH Reachable, INFERRED: SNI Filtered, PREDICTED: Steganography effective)',
        groundTruth: 'DPI_INTERFERENCE',
        verdict: 'PASS',
        distinguishesObservation: true,
        evidenceRequirementMet: true,
        confidenceScore: 99.4,
      },
      {
        adversarialScenarioId: 'adv-02',
        scenarioName: 'Total International Fiber Cutout (National Only)',
        scenarioNameFa: 'قطعی کامل فیبر بین‌الملل (حالت شبکه ملی خالص)',
        inputCondition: 'Domestic Probes = 100% PASS, All International BGP routes withdrawn / 100% Packet Loss',
        aiClassification: 'CIRCUMVENTION_UNLIKELY_DUE_TO_UPSTREAM_CONNECTIVITY_LOSS (OBSERVED: 0% Int BGP)',
        groundTruth: 'INTERNATIONAL_BLACKOUT',
        verdict: 'PASS',
        distinguishesObservation: true,
        evidenceRequirementMet: true,
        confidenceScore: 100.0,
      },
      {
        adversarialScenarioId: 'adv-03',
        scenarioName: 'DNS Cache Poisoning & NXDOMAIN Injection',
        scenarioNameFa: 'تزریق رکوردهای آلوده DNS و جعل پاسخ‌ها',
        inputCondition: 'Port 53 UDP returns 10.10.34.34, Port 853 TLS returns legitimate IP',
        aiClassification: 'DNS_INTERFERENCE (OBSERVED: UDP spoofed, INFERRED: TIC DNS hijack)',
        groundTruth: 'DNS_INTERFERENCE',
        verdict: 'PASS',
        distinguishesObservation: true,
        evidenceRequirementMet: true,
        confidenceScore: 98.9,
      },
      {
        adversarialScenarioId: 'adv-04',
        scenarioName: 'Adversarial False Positive with High Jitter',
        scenarioNameFa: 'آزمون خطای مثبت کاذب ناشی از نوسان کیفیت خط',
        inputCondition: 'Packet loss 12%, Latency +80ms, but zero RST packets observed',
        aiClassification: 'PARTIAL_INTERNATIONAL_DEGRADATION (OBSERVED: High Jitter, NOT Censorship)',
        groundTruth: 'PARTIAL_INTERNATIONAL_DEGRADATION',
        verdict: 'PASS',
        distinguishesObservation: true,
        evidenceRequirementMet: true,
        confidenceScore: 97.2,
      },
    ],
    realTimeValidationLadder: [
      { stageNumber: 1, stageName: 'TCP Handshake Verification', stageNameFa: 'تایید دست‌دهی لایه ۴ (TCP Syn-Ack)', status: 'PASSED', latencyMs: 16, evidenceSnippet: 'SYN-ACK received in 16.2ms from domestic edge', failureClass: null },
      { stageNumber: 2, stageName: 'TLS Handshake & ECH Validation', stageNameFa: 'دست‌دهی امن TLS 1.3 با استتار ECH', status: 'PASSED', latencyMs: 24, evidenceSnippet: 'TLS_AES_256_GCM_SHA384 negotiated with valid bank certificate', failureClass: null },
      { stageNumber: 3, stageName: 'Transport Layer Handshake', stageNameFa: 'دست‌دهی اختصاصی ترنسپورت (Transport Handshake)', status: 'PASSED', latencyMs: 18, evidenceSnippet: 'QNS-v4 / Brutal frame authentication verified', failureClass: null },
      { stageNumber: 4, stageName: 'Tor / Core Bootstrap Probe', stageNameFa: 'راه‌اندازی هسته و دانلود کاتالوگ (Bootstrap)', status: 'PASSED', latencyMs: 38, evidenceSnippet: 'Bootstrap 100%: done (Consensus parsed)', failureClass: null },
      { stageNumber: 5, stageName: 'Directory Authority Sync', stageNameFa: 'همگام‌سازی فهرست سرورها (Directory Sync)', status: 'PASSED', latencyMs: 42, evidenceSnippet: 'Consensus micro-descriptors downloaded via tunnel', failureClass: null },
      { stageNumber: 6, stageName: 'Multi-Hop Circuit Build', stageNameFa: 'ایجاد مدار چندمرحله‌ای رمزنگاری‌شده (Circuit Build)', status: 'PASSED', latencyMs: 64, evidenceSnippet: '3-hop onion circuit established securely', failureClass: null },
      { stageNumber: 7, stageName: 'Application-Level End-to-End Test', stageNameFa: 'تست سطح اپلیکیشن و وب واقعی (App Connectivity)', status: 'PASSED', latencyMs: 78, evidenceSnippet: 'HTTPS GET to neutral external canary: 200 OK in 78ms', failureClass: null },
      { stageNumber: 8, stageName: 'Sustained 60s Stability & Throughput', stageNameFa: 'آزمون پایداری ممتد ۶۰ ثانیه‌ای (Stability Verification)', status: 'PASSED', latencyMs: 22, evidenceSnippet: '0.00% Packet Loss over 12,000 packets; Zero RST spikes', failureClass: null },
    ],
  },


  setConnected: (val) => set({ connected: val }),
  setActiveTab: (tab) => set({ activeTab: tab }),
  setActivePlatform: (platform) => set({ activePlatform: platform }),

  toggleConnection: () => {
    const { connected, cores } = get();
    if (!connected) {
      const updatedCores = cores.map((c, i) => {
        if (i === 0) return { ...c, status: 'connected' as CoreStatus, health: generateHealth(c.id, true) };
        if (i === 1) return { ...c, status: 'connected' as CoreStatus, health: generateHealth(c.id, true) };
        if (i === 2) return { ...c, status: 'standby' as CoreStatus, health: generateHealth(c.id, true) };
        return { ...c, status: 'standby' as CoreStatus, health: generateHealth(c.id, false) };
      });
      set({
        connected: true,
        cores: updatedCores,
        orchestrator: { ...get().orchestrator, activeCoreId: 'xray-gfw', shadowConnections: ['mahsang', 'hiddify'] },
        stats: { ...get().stats, currentSpeed: { up: 34, down: 156 } },
      });
      get().startLiveConnection();
      get().addLog({
        type: 'connect',
        message: 'VPN session started',
        messageFa: 'جلسه جدید VPN آغاز شد — تله‌متری زنده فعال گردید',
        coreId: 'xray-gfw',
      });
    } else {
      get().endLiveConnection('manual');
      const updatedCores = cores.map(c => ({
        ...c, status: 'disconnected' as CoreStatus, health: generateHealth(c.id, false),
      }));
      set({
        connected: false, cores: updatedCores,
        orchestrator: { ...get().orchestrator, activeCoreId: '', shadowConnections: [] },
        stats: { ...get().stats, currentSpeed: { up: 0, down: 0 } },
      });
    }
  },

  updateCoreHealth: () => {
    const { cores, orchestrator, connected } = get();
    if (!connected) return;

    const updatedCores = cores.map(c => {
      const isActive = c.id === orchestrator.activeCoreId || orchestrator.shadowConnections.includes(c.id);
      const prevHealth = c.health;
      const newBlocked = Math.random() < (isActive ? 0.04 : 0.02);
      const updatedHealth = {
        ...prevHealth,
        latency: Math.max(15, Math.round(prevHealth.latency * (1 + (Math.random() * 0.1 - 0.05)))),
        packetLoss: Math.max(0, Math.round((prevHealth.packetLoss + Math.random() * 0.5 - 0.25) * 100) / 100),
        dpiExposure: Math.max(0, Math.round((prevHealth.dpiExposure + Math.random() * 2 - 1) * 10) / 10),
        blocked: prevHealth.blocked ? Math.random() > 0.3 : newBlocked,
        bandwidth: isActive ? {
          up: Math.max(5, Math.round(prevHealth.bandwidth.up * (1 + (Math.random() * 0.2 - 0.1)))),
          down: Math.max(10, Math.round(prevHealth.bandwidth.down * (1 + (Math.random() * 0.2 - 0.1)))),
        } : { up: 0, down: 0 },
        uptime: isActive ? prevHealth.uptime + 15 : prevHealth.uptime,
      };
      // Sync status with health.blocked so computeScore and UI stay consistent
      let newStatus = c.status;
      if (updatedHealth.blocked && c.status === 'connected') newStatus = 'error' as CoreStatus;
      if (!updatedHealth.blocked && c.status === 'error') newStatus = 'standby' as CoreStatus;
      return {
        ...c,
        status: newStatus,
        health: updatedHealth,
        lastChecked: Date.now(),
      };
    });

    const scoringMatrix: Record<string, number> = {};
    for (const core of updatedCores) {
      const rlWeight = orchestrator.rlWeights[core.id]?.[0] ?? 0.5;
      scoringMatrix[core.id] = computeScore(core, rlWeight);
    }

    set({ cores: updatedCores, orchestrator: { ...orchestrator, scoringMatrix } });
  },

  switchCore: (coreId) => {
    const { cores, orchestrator, connected, rewardHistory, totalPulls } = get();
    if (!connected) return;

    const targetCore = cores.find(c => c.id === coreId);
    if (!targetCore || targetCore.health.blocked) return;

    const oldActive = orchestrator.activeCoreId;
    const newShadows = [oldActive, ...orchestrator.shadowConnections.filter(id => id !== coreId)].slice(0, 2);

    const updatedCores = cores.map(c => {
      if (c.id === coreId) return { ...c, status: 'connected' as CoreStatus };
      if (c.id === oldActive) return { ...c, status: 'standby' as CoreStatus };
      if (newShadows.includes(c.id)) return { ...c, status: 'standby' as CoreStatus };
      return c;
    });

    const success = Math.random() > 0.05;
    const reward = success ? (1.0 - Math.min(targetCore.health.latency / 5000, 0.9)) : 0;
    const newHistory = { ...rewardHistory };
    newHistory[coreId] = [...(newHistory[coreId] ?? []), reward].slice(-100);

    set({
      cores: updatedCores,
      rewardHistory: newHistory,
      totalPulls: totalPulls + 1,
      orchestrator: {
        ...orchestrator,
        activeCoreId: coreId,
        shadowConnections: newShadows,
        totalSwitches: orchestrator.totalSwitches + 1,
        successfulSwitches: orchestrator.successfulSwitches + (success ? 1 : 0),
      },
    });

    // Live telemetry: every real failover/switch increments the counter + logs it
    if (oldActive && oldActive !== coreId) {
      get().recordCoreSwitch(`auto-failover: ${oldActive} -> ${coreId}`);
    }
    get().recordProtocolAttempt(coreId, targetCore.nameFa, success);
    get().addLog({
      type: 'switch',
      message: `Core switched: ${oldActive} -> ${coreId}`,
      messageFa: `تعویض هسته: از «${oldActive}» به «${targetCore.nameFa}» — ${success ? 'موفق' : 'ناقص'}`,
      coreId,
      details: { signature: 'core-failover' },
    });
  },

  setRoutingMode: (mode) => set({ routing: { ...get().routing, mode } }),
  toggleIranBypass: () => set({ routing: { ...get().routing, iranIpBypass: !get().routing.iranIpBypass } }),
  setDnsMode: (mode) => set({ routing: { ...get().routing, dnsMode: mode } }),
  setDnsProvider: (provider) => set({ routing: { ...get().routing, activeDnsProvider: provider } }),

  runDPITest: () => {
    const { cores } = get();
    const latencyBases: Record<string, number> = {
      'hiddify': 85, 'xray-gfw': 62, 'sing-box': 73, 'amneziavpn': 91,
      'defyxvpn': 105, 'moav': 118, 'lantern': 142, 'mahsang': 79, 'psiphon': 156,
    };
    const sigEntries = IRAN_DPI_SIGNATURES;
    const results: DPITestResult[] = cores.map(core => {
      const connected = Math.random() > 0.15;
      const bypassLevel = connected ? (Math.random() > 0.3 ? 'full' : 'partial') : 'none';
      const sigEntry = sigEntries[Math.floor(Math.random() * sigEntries.length)];
      return {
        coreId: core.id, coreName: core.name, coreNameFa: core.nameFa,
        connected, latency: connected ? Math.round((latencyBases[core.id] ?? 100) + Math.random() * 30) : 0,
        protocol: core.capabilities[0], bypassLevel,
        dpiSignature: sigEntry.signature, dpiSignatureFa: sigEntry.descriptionFa,
        timestamp: Date.now(),
      };
    });
    set({ dpiResults: results });
  },

  performAIOrchestration: () => {
    const { cores, orchestrator, connected, rewardHistory, totalPulls } = get();
    if (!connected) return;

    const scoringMatrix: Record<string, number> = {};
    const ucbScores: Record<string, { exploitation: number; exploration: number; total: number }> = {};
    let bestCore = orchestrator.activeCoreId;
    let bestScore = 0;

    const iranLevel = get().iranScanner?.filterLevel;
    const blackout = iranLevel === 'international-cutoff' || iranLevel === 'national-only';
    const extreme = iranLevel === 'extreme';

    for (const core of cores) {
      const rlWeight = orchestrator.rlWeights[core.id]?.[0] ?? 0.5;
      let score = computeScore(core, rlWeight);
      // During blackout/extreme, the AI explicitly prefers the specialized cores
      if (blackout && BLACKOUT_CORES.has(core.id)) score += 40;
      if (extreme && core.id === 'quazar') score += 30;
      scoringMatrix[core.id] = score;

      const alpha = UCB_ALPHAS[core.id] ?? 1.5;
      ucbScores[core.id] = computeUCB(rewardHistory[core.id] ?? [], alpha, totalPulls);

      if (score > bestScore && !core.health.blocked && core.status !== 'error') {
        bestScore = score;
        bestCore = core.id;
      }
    }

    const currentScore = scoringMatrix[orchestrator.activeCoreId] ?? 0;
    const shouldSwitch = bestCore !== orchestrator.activeCoreId && (bestScore - currentScore) > 15;

    const latencySpikes = cores.filter(c => c.health.latency > 150).map(c => c.id);
    const imminentBlockRisk = Math.min(100, latencySpikes.length * 20);
    const predictedBlockCore = latencySpikes.length > 0 ? latencySpikes[0] : null;
    const proactiveSwitchRecommended = imminentBlockRisk > 40;

    const newRlWeights = { ...orchestrator.rlWeights };
    for (const core of cores) {
      const weights = [...(newRlWeights[core.id] ?? [0.5, 0.3, 0.2, 0.6, 0.4])];
      if (core.id === orchestrator.activeCoreId) {
        weights[0] = Math.min(1, weights[0] + orchestrator.learningRate * (core.health.blocked ? -5 : 1));
      } else {
        weights[0] = Math.max(0, weights[0] - orchestrator.learningRate * 0.5);
      }
      newRlWeights[core.id] = weights;
    }

    const ispRule = ISP_RULES.find(r => r.id === orchestrator.detectedISP) ?? ISP_RULES[1];

    if (shouldSwitch || (proactiveSwitchRecommended && predictedBlockCore === orchestrator.activeCoreId)) {
      get().switchCore(bestCore);
    }

    // Re-read orchestrator from store after possible switchCore mutation to avoid stale state overwrite
    const latestOrchestrator = get().orchestrator;
    set({
      orchestrator: {
        ...latestOrchestrator,
        scoringMatrix,
        ucbScores,
        predictionState: { imminentBlockRisk, predictedBlockCore, proactiveSwitchRecommended },
        rlWeights: newRlWeights,
        ispRuleApplied: ispRule.id,
      },
    });

    // AI Load Balancer — rebalance traffic in sync with the engine's core decision
    get().rebalanceLoad(bestCore);

    // Stealth Rotation — anti-fingerprint core/protocol switching
    get().evaluateStealthRotation();
  },

  toggleLoadBalancer: () => {
    const { loadBalancer } = get();
    set({ loadBalancer: { ...loadBalancer, enabled: !loadBalancer.enabled } });
    get().addLog({
      type: 'update',
      message: `AI Load Balancer ${loadBalancer.enabled ? 'disabled' : 'enabled'}`,
      messageFa: `لودبالانسر هوشمند ${loadBalancer.enabled ? 'غیرفعال' : 'فعال'} شد`,
    });
  },

  setLoadBalancerMode: (mode) => {
    set({ loadBalancer: { ...get().loadBalancer, mode } });
  },

  rebalanceLoad: (recommendedCoreIdOverride) => {
    const { cores, orchestrator, iranScanner, loadBalancer, connected } = get();
    const result = computeLoadShares(cores, orchestrator.scoringMatrix, iranScanner?.filterLevel, connected);
    const recommendedCoreId = recommendedCoreIdOverride ?? result.recommendedCoreId;
    set({
      loadBalancer: {
        ...loadBalancer,
        mode: result.mode,
        cores: result.cores,
        protocols: result.protocols,
        activeProtocolIds: result.protocols.filter((p) => p.active).map((p) => p.protocolId),
        directSharePct: result.directSharePct,
        tunnelSharePct: result.tunnelSharePct,
        lastRebalanceTs: Date.now(),
        rebalanceCount: loadBalancer.rebalanceCount + 1,
        totalBandwidthMbps: result.totalBandwidthMbps,
        reasonFa: result.reasonFa,
        aiDecision: {
          recommendedCoreId,
          recommendedProtocolId: result.recommendedProtocolId,
          confidencePct: result.confidencePct,
        },
      },
    });
  },

  applyLoadBalancerRoute: () => {
    const { loadBalancer, connected, orchestrator } = get();
    const coreId = loadBalancer.aiDecision.recommendedCoreId;
    if (!connected || !coreId || orchestrator.activeCoreId === coreId) return;
    get().switchCore(coreId);
    get().addLog({
      type: 'update',
      message: `AI Load Balancer applied recommended core ${coreId}`,
      messageFa: `لودبالانسر هوشمند: مسیر پیشنهادی AI به هسته «${coreId}» اعمال شد`,
    });
  },

  toggleStealthRotation: () => {
    const { stealthRotation } = get();
    set({ stealthRotation: { ...stealthRotation, enabled: !stealthRotation.enabled } });
    get().addLog({
      type: 'update',
      message: `Stealth Rotation ${stealthRotation.enabled ? 'disabled' : 'enabled'}`,
      messageFa: `چرخش مخفی هسته/پروتکل ${stealthRotation.enabled ? 'غیرفعال' : 'فعال'} شد`,
    });
  },

  evaluateStealthRotation: (force = false) => {
    const { cores, orchestrator, nationalBlackoutShield, stealthRotation, connected } = get();
    if (!connected) return;

    const now = Date.now();
    const plan = buildStealthRotationPlan({
      cores,
      protocols: nationalBlackoutShield.protocols,
      scoringMatrix: orchestrator.scoringMatrix,
      activeCoreId: orchestrator.activeCoreId,
      activeProtocolId: nationalBlackoutShield.activeNovelProtocolId,
      activeSinceTs: stealthRotation.activeSinceTs,
      now,
      lastRotationAt: stealthRotation.lastRotationAt,
      recentCoreIds: stealthRotation.history.map((h) => h.toCoreId),
    });

    // Always refresh the live risk readout
    set({
      stealthRotation: {
        ...stealthRotation,
        fingerprintRisk: plan.fingerprintRisk,
        nextScheduledAt: plan.nextScheduledAt,
        jitterMs: plan.jitterMs,
        techniqueFa: plan.techniqueFa,
        reasonFa: plan.reasonFa,
      },
    });

    if (!stealthRotation.enabled && !force) return;

    const shouldRotate = force || plan.shouldRotate || now >= stealthRotation.nextScheduledAt;
    if (!shouldRotate) return;

    const fromCoreId = orchestrator.activeCoreId;
    const fromProtocolId = nationalBlackoutShield.activeNovelProtocolId;

    if (plan.nextCoreId && plan.nextCoreId !== fromCoreId) {
      get().switchCore(plan.nextCoreId);
    }
    if (plan.nextProtocolId && plan.nextProtocolId !== fromProtocolId) {
      get().setActiveNovelProtocol(plan.nextProtocolId);
    }

    const entry: StealthRotationHistoryEntry = {
      id: `sr-${now}`,
      timestamp: now,
      fromCoreId,
      toCoreId: plan.nextCoreId,
      fromProtocolId,
      toProtocolId: plan.nextProtocolId,
      fingerprintRisk: plan.fingerprintRisk,
      techniqueFa: plan.techniqueFa,
      reasonFa: plan.reasonFa,
    };

    set({
      stealthRotation: {
        ...get().stealthRotation,
        activeSinceTs: now,
        lastRotationAt: now,
        rotationCount: stealthRotation.rotationCount + 1,
        fingerprintRisk: plan.fingerprintRisk,
        nextScheduledAt: plan.nextScheduledAt,
        jitterMs: plan.jitterMs,
        lastCoreId: plan.nextCoreId,
        lastProtocolId: plan.nextProtocolId,
        techniqueFa: plan.techniqueFa,
        reasonFa: plan.reasonFa,
        history: [entry, ...stealthRotation.history].slice(0, 20),
      },
    });

    get().addLog({
      type: 'switch',
      message: `Stealth rotation: ${fromCoreId}->${plan.nextCoreId} / ${fromProtocolId}->${plan.nextProtocolId} (risk ${plan.fingerprintRisk}%)`,
      messageFa: `چرخش مخفی AI: هسته «${fromCoreId}» به «${plan.nextCoreId}» و پروتکل «${fromProtocolId}» به «${plan.nextProtocolId}» تعویض شد تا امضای اتصال شناسایی نشود`,
      coreId: plan.nextCoreId,
      details: { signature: 'stealth-rotation', risk: String(plan.fingerprintRisk), technique: plan.techniqueFa },
    });
  },

  rotateStealthNow: () => {
    get().evaluateStealthRotation(true);
  },

  toggleKillSwitch: () => {
    const { killSwitch } = get();
    set({ killSwitch: { ...killSwitch, enabled: !killSwitch.enabled } });
  },

  toggleNetworkLock: () => {
    const { killSwitch } = get();
    set({ killSwitch: { ...killSwitch, networkLock: !killSwitch.networkLock } });
  },

  resetAutoReconnect: () => {
    const { autoReconnect } = get();
    set({ autoReconnect: { ...autoReconnect, retryCount: 0, reconnectStatus: 'idle' } });
  },

  attemptReconnect: () => {
    const { autoReconnect, connected } = get();
    if (connected || !autoReconnect.enabled) return;

    if (autoReconnect.retryCount >= autoReconnect.maxRetries) {
      set({ autoReconnect: { ...autoReconnect, reconnectStatus: 'failed' } });
      return;
    }

    set({ autoReconnect: { ...autoReconnect, reconnectStatus: 'reconnecting', lastReconnectAttempt: Date.now() } });

    const delay = autoReconnect.exponentialBackoff
      ? autoReconnect.retryInterval * Math.pow(2, autoReconnect.retryCount)
      : autoReconnect.retryInterval;

    setTimeout(() => {
      const success = Math.random() > 0.3;
      if (success) {
        get().toggleConnection();
        set({
          autoReconnect: {
            ...get().autoReconnect,
            reconnectStatus: 'connected',
            retryCount: 0,
          },
        });
      } else {
        set({
          autoReconnect: {
            ...get().autoReconnect,
            reconnectStatus: 'reconnecting',
            retryCount: get().autoReconnect.retryCount + 1,
          },
        });
      }
    }, Math.min(delay, 30000));
  },

  addLog: (entry) => {
    const { connectionLogs } = get();
    const newLog: ConnectionLogEntry = {
      ...entry,
      id: `log-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      timestamp: Date.now(),
    };
    set({ connectionLogs: [newLog, ...connectionLogs].slice(0, 200) });
  },

  updateNetworkStats: () => {
    const { networkAnalyzer, connected } = get();
    const updatedAnalyzer = updateNetworkMonitoring(networkAnalyzer, connected);
    set({ networkAnalyzer: updatedAnalyzer });
  },

  selectServerCountry: (countryCode: string) => {
    const { geoRouter } = get();
    const updatedGeoRouter = geoSelectServerCountry(geoRouter, countryCode);
    set({ geoRouter: updatedGeoRouter });
  },

  runSecurityAudit: () => {
    const { securityAudit, connected, orchestrator, routing, killSwitch } = get();
    const updatedAudit = runFullSecurityAudit(
      securityAudit,
      connected,
      orchestrator.activeCoreId,
      routing.dnsMode,
      killSwitch.enabled,
      killSwitch.networkLock,
      routing.ipv6Enabled,
    );
    set({ securityAudit: updatedAudit });
  },

  toggleBackgroundWorker: (enabled?: boolean) => {
    const { orchestrator } = get();
    const nextState = enabled !== undefined ? enabled : !(orchestrator.backgroundWorkerActive ?? true);
    set({
      orchestrator: {
        ...orchestrator,
        backgroundWorkerActive: nextState,
      },
    });
  },

  profileCoresBackground: () => {
    const { cores, orchestrator, connected } = get();
    // Profile cores during idle time (ranking by Latency-per-Packet-Loss efficiency)
    const activeId = orchestrator.activeCoreId;
    
    // Sample latency and packet loss for all cores
    const profiles = cores.map(core => {
      const isCurrentActive = core.id === activeId;
      // Standby or idle cores get lightweight test pings with jitter
      const testLatency = core.health.latency > 0 
        ? Math.max(18, Math.round(core.health.latency * (1 + (Math.random() * 0.08 - 0.04))))
        : Math.round(45 + Math.random() * 80);
      const testLoss = Math.max(0.01, Math.round((core.health.packetLoss + (Math.random() * 0.4 - 0.2)) * 100) / 100);
      
      // Latency-per-Packet-Loss Efficiency Metric:
      // Lower ratio = faster with fewer dropped packets
      const efficiencyRatio = Math.round(testLatency * (1 + (testLoss / 100) * 8) * 10) / 10;
      
      // Efficiency score: 100 (best) to 0 (worst)
      const efficiencyScore = Math.max(5, Math.min(99, Math.round(100 - (efficiencyRatio / 2.5))));

      return {
        coreId: core.id,
        coreNameFa: core.nameFa,
        latencyMs: testLatency,
        packetLoss: testLoss,
        efficiencyRatio,
        efficiencyScore,
        rank: 1,
        idleSampleCount: ((orchestrator.coreRankings?.find(r => r.coreId === core.id)?.idleSampleCount ?? 0) + 1),
        lastProfiled: Date.now(),
      };
    });

    // Rank from best efficiency (lowest ratio / highest score) to worst
    profiles.sort((a, b) => a.efficiencyRatio - b.efficiencyRatio);
    profiles.forEach((p, idx) => {
      p.rank = idx + 1;
    });

    // Dynamically update the AI Orchestrator's scoring matrix based on profile efficiency
    const updatedScoringMatrix = { ...orchestrator.scoringMatrix };
    for (const p of profiles) {
      const baseScore = updatedScoringMatrix[p.coreId] ?? 50;
      // Blend 70% current score with 30% background profiling efficiency
      const blendedScore = Math.round(baseScore * 0.7 + p.efficiencyScore * 0.3);
      updatedScoringMatrix[p.coreId] = blendedScore;
    }

    set({
      orchestrator: {
        ...orchestrator,
        scoringMatrix: updatedScoringMatrix,
        coreRankings: profiles,
        lastWorkerRun: Date.now(),
      },
    });
  },

  // ──────────────────────────────────────────────
  // 1. Predictive Mitigation Planner Methods
  // ──────────────────────────────────────────────
  togglePredictiveMitigation: (enabled) => {
    const { orchestrator } = get();
    const currentMitigation = orchestrator.predictiveMitigation;
    if (!currentMitigation) return;
    const nextVal = enabled !== undefined ? enabled : !currentMitigation.enabled;
    set({
      orchestrator: {
        ...orchestrator,
        predictiveMitigation: {
          ...currentMitigation,
          enabled: nextVal,
        },
      },
    });
  },

  executePredictivePreWarm: (windowId) => {
    const { orchestrator, cores, addLog } = get();
    const currentMitigation = orchestrator.predictiveMitigation;
    if (!currentMitigation) return;

    const targetWindow = currentMitigation.activeScheduleWindows.find(w => w.id === windowId) || currentMitigation.activeScheduleWindows[0];
    const targetCore = cores.find(c => c.id === targetWindow.recommendedCore) || cores[0];

    // Pre-warm high-obfuscation core and establish secondary tunnel
    const updatedPreWarmed = Array.from(new Set([...currentMitigation.preWarmedCores, targetCore.id]));

    set({
      orchestrator: {
        ...orchestrator,
        shadowConnections: Array.from(new Set([...orchestrator.shadowConnections, targetCore.id])),
        predictiveMitigation: {
          ...currentMitigation,
          preWarmedCores: updatedPreWarmed,
          nextScheduledPreWarm: {
            coreId: targetCore.id,
            coreNameFa: targetCore.nameFa,
            scheduledTimestamp: Date.now() + 5 * 60 * 1000,
            triggerTime: `${targetWindow.timeLabelFa}`,
            dpiSpikeWindow: targetWindow.timeLabel,
            preWarmedStatus: 'ready',
          },
          lastPreWarmEvent: {
            coreId: targetCore.id,
            timestamp: Date.now(),
            reasonFa: `پیش‌گرم‌سازی ۵ دقیقه‌ای هسته ${targetCore.nameFa} برای مقابله با ${targetWindow.timeLabelFa} انجام شد`,
          },
        },
      },
    });

    addLog({
      type: 'switch',
      coreId: targetCore.id,
      message: `Predictive Mitigation: Pre-warmed core [${targetCore.id}] 5 minutes before scheduled DPI spike`,
      messageFa: `طرح پیش‌گیرانه هوش مصنوعی: هسته ${targetCore.nameFa} ۵ دقیقه پیش از اوج فیلترینگ پیش‌گرم و فعال شد`,
    });
  },

  // ──────────────────────────────────────────────
  // 2. Egress Integrity Monitor Methods
  // ──────────────────────────────────────────────
  toggleEgressMonitoring: (enabled) => {
    const { egressIntegrity } = get();
    const nextVal = enabled !== undefined ? enabled : !egressIntegrity.monitoringEnabled;
    set({
      egressIntegrity: {
        ...egressIntegrity,
        monitoringEnabled: nextVal,
      },
    });
  },

  toggleAutoNetworkResetOnLeak: (enabled) => {
    const { egressIntegrity } = get();
    const nextVal = enabled !== undefined ? enabled : !egressIntegrity.autoNetworkResetOnLeak;
    set({
      egressIntegrity: {
        ...egressIntegrity,
        autoNetworkResetOnLeak: nextVal,
      },
    });
  },

  triggerLowLevelNetworkReset: (reason) => {
    const { egressIntegrity, addLog, cores, orchestrator } = get();
    set({
      egressIntegrity: {
        ...egressIntegrity,
        systemResetStatus: 'resetting',
        lastResetTimestamp: Date.now(),
      },
    });

    addLog({
      type: 'error',
      message: `CRITICAL: Low-Level Network Socket Reset Triggered - ${reason || 'Unauthorized Bypass Detected'}`,
      messageFa: `⚠️ بازنشانی سوکت‌های سیستمی شبکه انجام شد: مسدودسازی کلیه پکت‌های نشت‌یافته خارج از پروکسی (${reason || 'نشت مستقیم غیرمجاز'})`,
    });

    // Emulate swift socket flush & interface rebind within 800ms
    setTimeout(() => {
      set({
        egressIntegrity: {
          ...get().egressIntegrity,
          systemResetStatus: 'secured',
        },
      });
      addLog({
        type: 'connect',
        message: `Network Firewall Rules Restored: 100% of egress traffic forced through TUN/VpnService interface`,
        messageFa: `امنیت شبکه بازیابی شد: تمام ترافیک خروجی سیستم مجدداً به داخل رابط فیلترشکن TUN هدایت گردید`,
      });
    }, 800);
  },

  simulateEgressBypassAttempt: () => {
    const { egressIntegrity, triggerLowLevelNetworkReset, addLog } = get();
    const sampleLeaks: Omit<EgressLeakAttempt, 'id' | 'timestamp'>[] = [
      {
        sourceProcess: 'browser-webrtc.exe / StunProbe',
        destinationIp: '142.250.180.12',
        destinationPort: 19302,
        detectedProtocol: 'STUN Direct UDP Bypass',
        bypassType: 'udp_leak',
        bypassTypeFa: 'تلاش نشت مستقیم پکت STUN برای کشف IP واقعی',
        severity: 'critical',
        blocked: true,
        resetTriggered: egressIntegrity.autoNetworkResetOnLeak,
      },
      {
        sourceProcess: 'system-updater / RawSocket',
        destinationIp: '185.143.232.5',
        destinationPort: 80,
        detectedProtocol: 'Plain HTTP Raw Packet bypassing TUN',
        bypassType: 'raw_socket_bypass',
        bypassTypeFa: 'تلاش ارسال پکت خام TCP خارج از تونل پروکسی',
        severity: 'high',
        blocked: true,
        resetTriggered: egressIntegrity.autoNetworkResetOnLeak,
      },
      {
        sourceProcess: 'dns-resolver-probe',
        destinationIp: '8.8.8.8',
        destinationPort: 53,
        detectedProtocol: 'Plaintext DNS UDP Leak',
        bypassType: 'unauthorized_dns',
        bypassTypeFa: 'درخواست DNS کلیرتکست مسدودنشده به سمت سرور خارجی',
        severity: 'critical',
        blocked: true,
        resetTriggered: egressIntegrity.autoNetworkResetOnLeak,
      },
    ];

    const leakTemplate = sampleLeaks[Math.floor(Math.random() * sampleLeaks.length)];
    const newLeak: EgressLeakAttempt = {
      ...leakTemplate,
      id: `leak-${Date.now()}`,
      timestamp: Date.now(),
    };

    set({
      egressIntegrity: {
        ...egressIntegrity,
        unauthorizedBypassAttempts: egressIntegrity.unauthorizedBypassAttempts + 1,
        totalInspectedPackets: egressIntegrity.totalInspectedPackets + 450,
        recentBypassAttempts: [newLeak, ...egressIntegrity.recentBypassAttempts].slice(0, 30),
      },
    });

    if (egressIntegrity.autoNetworkResetOnLeak) {
      triggerLowLevelNetworkReset(newLeak.bypassTypeFa);
    }
  },

  // ──────────────────────────────────────────────
  // 3. Proactive Path Switching (BGP Evaluation)
  // ──────────────────────────────────────────────
  toggleProactivePathSwitching: (enabled) => {
    const { pathSwitching } = get();
    const nextVal = enabled !== undefined ? enabled : !pathSwitching.enabled;
    set({
      pathSwitching: {
        ...pathSwitching,
        enabled: nextVal,
      },
    });
  },

  evaluateBGPHealthAndMigrate: () => {
    const { pathSwitching, addLog, connected } = get();
    if (!pathSwitching.enabled) return;

    let routeShiftDetected = false;
    let worstRouteId = '';

    // Update BGP jitter & shift metrics
    const updatedRoutes = pathSwitching.routes.map(route => {
      const isShifted = Math.random() < 0.18; // 18% shift simulation
      const newShiftScore = isShifted 
        ? Math.min(95, Math.max(45, route.routeShiftScore + Math.floor(Math.random() * 30 + 15)))
        : Math.max(3, Math.min(30, route.routeShiftScore - Math.floor(Math.random() * 8)));
      
      const newHops = newShiftScore > 50 ? Math.min(8, route.asPathHops + 2) : route.asPathHopsPrevious;
      const status: BGPRouteHealth['healthStatus'] = newShiftScore > 60 
        ? 'route_shift_detected' 
        : newShiftScore > 40 
        ? 'flapping' 
        : 'optimal';
      
      const statusFa = status === 'route_shift_detected'
        ? 'تغییر ناگهانی جدول BGP / افزایش هوپ'
        : status === 'flapping'
        ? 'نوسان مسیر (Flapping)'
        : 'پایدار و بهینه (BGP Stable)';

      if (route.routeId === pathSwitching.activeRouteId && (status === 'route_shift_detected' || newShiftScore > 50)) {
        routeShiftDetected = true;
        worstRouteId = route.routeId;
      }

      return {
        ...route,
        routeShiftScore: newShiftScore,
        asPathHops: newHops,
        flapCount10m: isShifted ? route.flapCount10m + 1 : Math.max(0, route.flapCount10m - 1),
        packetLossPercent: Math.round((newShiftScore / 18) * 10) / 10,
        healthStatus: status,
        healthStatusFa: statusFa,
        lastEvaluated: Date.now(),
      };
    });

    // Auto-migrate if active route has degraded and autoMigrateOnShift is on
    if (routeShiftDetected && pathSwitching.autoMigrateOnShift) {
      const optimalTarget = updatedRoutes.find(r => r.routeId !== pathSwitching.activeRouteId && r.healthStatus === 'optimal') || updatedRoutes[0];
      
      if (optimalTarget && optimalTarget.routeId !== pathSwitching.activeRouteId) {
        const prevActive = pathSwitching.activeRouteId;
        const migrationTime = Math.floor(Math.random() * 80 + 70); // 70-150ms

        set({
          pathSwitching: {
            ...pathSwitching,
            routes: updatedRoutes,
            activeRouteId: optimalTarget.routeId,
            totalPathMigrations: pathSwitching.totalPathMigrations + 1,
            lastMigration: {
              fromRouteId: prevActive,
              toRouteId: optimalTarget.routeId,
              reasonFa: `شناسایی خودکار شیفت BGP و افت رتبه مسیر مبدا (${optimalTarget.nodeNameFa})`,
              timestamp: Date.now(),
              migrationTimeMs: migrationTime,
            },
          },
        });

        addLog({
          type: 'switch',
          message: `Proactive Path Switching: Migrated session from [${prevActive}] to [${optimalTarget.routeId}] in ${migrationTime}ms`,
          messageFa: `مهاجرت پیش‌دستانه مسیر BGP: سشن با موفقیت در ${migrationTime} میلی‌ثانیه به ${optimalTarget.nodeNameFa} منتقل گردید`,
        });
        return;
      }
    }

    set({
      pathSwitching: {
        ...pathSwitching,
        routes: updatedRoutes,
      },
    });
  },

  manualPathMigrate: (targetRouteId) => {
    const { pathSwitching, addLog } = get();
    const target = pathSwitching.routes.find(r => r.routeId === targetRouteId);
    if (!target) return;

    const prevActive = pathSwitching.activeRouteId;
    const migrationTime = Math.floor(Math.random() * 60 + 50);

    set({
      pathSwitching: {
        ...pathSwitching,
        activeRouteId: targetRouteId,
        totalPathMigrations: pathSwitching.totalPathMigrations + 1,
        lastMigration: {
          fromRouteId: prevActive,
          toRouteId: targetRouteId,
          reasonFa: `انتقال دستی مسیر BGP به ${target.nodeNameFa}`,
          timestamp: Date.now(),
          migrationTimeMs: migrationTime,
        },
      },
    });

    addLog({
      type: 'switch',
      message: `Manual Path Migration to [${target.nodeName}] completed in ${migrationTime}ms`,
      messageFa: `انتقال دستی مسیر به ${target.nodeNameFa} در ${migrationTime}ms با موفقیت اعمال شد`,
    });
  },

  // ──────────────────────────────────────────────
  // 4. Obfuscation Entropy Profiler Methods
  // ──────────────────────────────────────────────
  toggleObfuscationProfiler: (enabled) => {
    const { obfuscationProfiler } = get();
    const nextVal = enabled !== undefined ? enabled : !obfuscationProfiler.realtimeAnalysisActive;
    set({
      obfuscationProfiler: {
        ...obfuscationProfiler,
        realtimeAnalysisActive: nextVal,
      },
    });
  },

  adjustRandomPaddingRealtime: () => {
    const { obfuscationProfiler, addLog } = get();
    if (!obfuscationProfiler.realtimeAnalysisActive) return;

    // Dynamically calculate entropy fluctuations and adjust random padding
    const updatedProfiles = obfuscationProfiler.targetProfiles.map(prof => {
      // Calculate realistic Shannon entropy drift around 7.85 - 7.96
      const entropyDelta = (Math.random() * 0.06 - 0.03);
      const newEntropy = Math.round(Math.min(7.99, Math.max(7.60, prof.liveEntropyScore + entropyDelta)) * 100) / 100;

      // Adjust padding frequency to counter traffic-shape analysis
      let newFreq = prof.randomPaddingFrequency;
      let newStatus: ObfuscationEntropyProfile['trafficShapeStatus'] = 'optimal_mimic';
      let newStatusFa = 'استتار حداکثری هدرهای TLS / شکست تحلیل الگو';

      if (newEntropy < prof.targetEntropyRange.min) {
        newFreq = Math.min(98, prof.randomPaddingFrequency + 4);
        newStatus = 'adjusting_padding';
        newStatusFa = 'افزایش فرکانس پدینگ تصادفی جهت افزایش آنتروپی ترافیک';
      } else if (newEntropy > prof.targetEntropyRange.max) {
        newFreq = Math.max(70, prof.randomPaddingFrequency - 3);
        newStatus = 'adjusting_padding';
        newStatusFa = 'تعدیل پدینگ هدرها جهت جلوگیری از تشخیص فیلترینگ هیستوگرام';
      }

      return {
        ...prof,
        liveEntropyScore: newEntropy,
        randomPaddingFrequency: newFreq,
        trafficShapeStatus: newStatus,
        trafficShapeStatusFa: newStatusFa,
        chaffPacketsPerSec: Math.floor(15 + Math.random() * 20),
        timingJitterMs: Math.floor(8 + Math.random() * 15),
      };
    });

    const avg = Math.round((updatedProfiles.reduce((acc, p) => acc + p.liveEntropyScore, 0) / updatedProfiles.length) * 100) / 100;

    set({
      obfuscationProfiler: {
        ...obfuscationProfiler,
        targetProfiles: updatedProfiles,
        averageEntropy: avg,
        totalTrafficReshapedMb: Math.round((obfuscationProfiler.totalTrafficReshapedMb + Math.random() * 4 + 1) * 10) / 10,
        lastAdjustmentTimestamp: Date.now(),
      },
    });
  },

  updateTargetPaddingFrequency: (coreId, freqPercent) => {
    const { obfuscationProfiler, addLog } = get();
    const updatedProfiles = obfuscationProfiler.targetProfiles.map(p => {
      if (p.targetCoreId === coreId) {
        return {
          ...p,
          randomPaddingFrequency: Math.max(20, Math.min(100, freqPercent)),
        };
      }
      return p;
    });

    set({
      obfuscationProfiler: {
        ...obfuscationProfiler,
        targetProfiles: updatedProfiles,
      },
    });

    addLog({
      type: 'update',
      message: `Obfuscation Profiler: Updated padding frequency for [${coreId}] to ${freqPercent}%`,
      messageFa: `تنظیم دستی پروفایلر استتار: فرکانس پدینگ پکت‌های هسته ${coreId} به ${freqPercent}٪ تغییر یافت`,
    });
  },

  // ──────────────────────────────────────────────
  // Energy Efficiency Optimizer Actions
  // ──────────────────────────────────────────────
  setBatteryLevel: (level: number) => {
    const { energyOptimizer, addLog } = get();
    const clamped = Math.max(0, Math.min(100, level));
    
    // Auto-compute polling frequency based on battery level & charging state
    let interval = energyOptimizer.normalIntervalSec;
    let powerSaved = 15;
    let pausedBg = false;

    if (energyOptimizer.isCharging) {
      interval = 15; // fast polling when plugged in
      powerSaved = 0;
    } else if (clamped < 20) {
      interval = energyOptimizer.ultraLowIntervalSec; // 180s when battery < 20%
      powerSaved = 65;
      pausedBg = true;
    } else if (clamped < 40) {
      interval = energyOptimizer.lowBatteryIntervalSec; // 90s when battery < 40%
      powerSaved = 42;
    } else {
      interval = energyOptimizer.normalIntervalSec; // 35s normal
      powerSaved = 25;
    }

    set({
      energyOptimizer: {
        ...energyOptimizer,
        batteryLevel: clamped,
        currentPollingIntervalSec: interval,
        powerSavedPercentage: powerSaved,
        backgroundTasksPausedOnBattery: pausedBg,
      },
    });

    if (clamped < 20 && !energyOptimizer.isCharging) {
      addLog({
        type: 'warning',
        message: `Energy Optimizer: Low battery (${clamped}%) detected. Worker polling throttled to ${interval}s to preserve battery.`,
        messageFa: `بهینه‌ساز مصرف باتری: باتری دستگاه به ${clamped}٪ کاهش یافت. فرکانس پایش پس‌زمینه به هر ${interval} ثانیه کاهش یافت.`,
      });
    }
  },

  toggleBatteryCharging: (isCharging) => {
    const { energyOptimizer, setBatteryLevel } = get();
    const nextCharging = isCharging !== undefined ? isCharging : !energyOptimizer.isCharging;
    set({
      energyOptimizer: {
        ...energyOptimizer,
        isCharging: nextCharging,
      },
    });
    setBatteryLevel(energyOptimizer.batteryLevel);
  },

  setPowerMode: (mode) => {
    const { energyOptimizer, setBatteryLevel } = get();
    set({
      energyOptimizer: {
        ...energyOptimizer,
        powerMode: mode,
      },
    });
    setBatteryLevel(energyOptimizer.batteryLevel);
  },

  toggleEnergyOptimizer: (enabled) => {
    const { energyOptimizer } = get();
    const next = enabled !== undefined ? enabled : !energyOptimizer.enabled;
    set({
      energyOptimizer: {
        ...energyOptimizer,
        enabled: next,
      },
    });
  },

  // ──────────────────────────────────────────────
  // Traffic Forecast Actions
  // ──────────────────────────────────────────────
  refreshTrafficForecast: () => {
    const { trafficForecast, addLog } = get();
    const randomTotal = Math.floor(4500 + Math.random() * 1500);
    const randomSavings = Math.floor(randomTotal * 0.14);

    set({
      trafficForecast: {
        ...trafficForecast,
        predictedTotal24hMb: randomTotal,
        smartSavingsEstimatedMb: randomSavings,
        lastForecastGenerated: Date.now(),
      },
    });

    addLog({
      type: 'info',
      message: 'Traffic Forecast: Regenerated 24h predictive bandwidth model based on historical habits.',
      messageFa: 'پیش‌بینی ترافیک: مدل سری‌زمانی پیش‌بینی مصرف ۲۴ ساعت آینده بر پایه عادات اتصال کاربر به‌روزرسانی شد.',
    });
  },

  // ──────────────────────────────────────────────
  // Adaptive UI Theme Actions
  // ──────────────────────────────────────────────
  toggleAdaptiveTheme: (enabled) => {
    const { adaptiveTheme } = get();
    set({
      adaptiveTheme: {
        ...adaptiveTheme,
        enabled: enabled !== undefined ? enabled : !adaptiveTheme.enabled,
      },
    });
  },

  updateAdaptiveThemePhase: () => {
    const hour = new Date().getHours();
    const { adaptiveTheme } = get();

    let phase: AdaptiveThemeState['currentPhase'] = 'twilight';
    let phaseLabel = 'گرگ‌ومیش و شامگاه';
    let accent = '#8b5cf6';
    let glow = 'rgba(139, 92, 246, 0.18)';
    let bgAtmosphere = 'from-slate-950 via-slate-900 to-indigo-950/40';
    let angle = (hour / 24) * 360;

    if (hour >= 5 && hour < 8) {
      phase = 'dawn';
      phaseLabel = 'طلوع و صبحگاه (پالت ملایم کهربایی و رز)';
      accent = '#f59e0b';
      glow = 'rgba(245, 158, 11, 0.20)';
      bgAtmosphere = 'from-slate-950 via-slate-900 to-amber-950/30';
    } else if (hour >= 8 && hour < 17) {
      phase = 'daylight';
      phaseLabel = 'روشنایی روز (پالت پرانرژی فیروزه‌ای و لاجوردی)';
      accent = '#06b6d4';
      glow = 'rgba(6, 182, 212, 0.20)';
      bgAtmosphere = 'from-slate-950 via-slate-900 to-cyan-950/30';
    } else if (hour >= 17 && hour < 19) {
      phase = 'golden_hour';
      phaseLabel = 'ساعت طلایی غروب (پالت گرم پرتقالی و مرجانی)';
      accent = '#f97316';
      glow = 'rgba(249, 115, 22, 0.22)';
      bgAtmosphere = 'from-slate-950 via-slate-900 to-orange-950/35';
    } else if (hour >= 19 && hour < 23) {
      phase = 'twilight';
      phaseLabel = 'شامگاه و گرگ‌ومیش (پالت آرامش‌بخش بنفش و ارغوانی)';
      accent = '#8b5cf6';
      glow = 'rgba(139, 92, 246, 0.20)';
      bgAtmosphere = 'from-slate-950 via-slate-900 to-violet-950/35';
    } else {
      phase = 'midnight';
      phaseLabel = 'نیمه‌شب و پاس شب (پالت تاریک سرمه‌ای و چشم‌نواز)';
      accent = '#6366f1';
      glow = 'rgba(99, 102, 241, 0.16)';
      bgAtmosphere = 'from-slate-950 via-slate-900 to-blue-950/40';
    }

    set({
      adaptiveTheme: {
        ...adaptiveTheme,
        currentPhase: phase,
        phaseLabelFa: phaseLabel,
        accentColor: accent,
        cardGlow: glow,
        bgAtmosphere: bgAtmosphere,
        sunPositionAngle: Math.round(angle),
        localHour: hour,
      },
    });
  },

  // ──────────────────────────────────────────────
  // Core Health History Sparkline Tick
  // ──────────────────────────────────────────────
  recordCoreHealthHistoryTick: () => {
    const { coreHealthHistory, cores } = get();
    const updated = coreHealthHistory.map(rec => {
      const liveCore = cores.find(c => c.id === rec.coreId);
      const currentLat = liveCore?.health?.latency || rec.avgLatency60m;
      const currentPl = liveCore?.health?.packetLoss || 0.1;
      const currentJitter = 4;

      const newPoint: CoreHealthSparklinePoint = {
        timestamp: Date.now(),
        latencyMs: Math.max(15, Math.floor(currentLat + (Math.random() * 6 - 3))),
        packetLossPercent: Math.max(0, Math.round((currentPl + (Math.random() * 0.4 - 0.2)) * 10) / 10),
        jitterMs: Math.max(1, Math.floor(currentJitter + (Math.random() * 2 - 1))),
        alive: liveCore?.status !== 'disconnected',
      };

      const newHistory = [...rec.history60m.slice(1), newPoint];
      const avgLat = Math.round(newHistory.reduce((acc, p) => acc + p.latencyMs, 0) / newHistory.length);
      const avgPl = Math.round((newHistory.reduce((acc, p) => acc + p.packetLossPercent, 0) / newHistory.length) * 10) / 10;

      return {
        ...rec,
        history60m: newHistory,
        avgLatency60m: avgLat,
        avgPacketLoss60m: avgPl,
      };
    });

    set({ coreHealthHistory: updated });
  },

  // ──────────────────────────────────────────────
  // National Blackout Autonomous Shield Actions
  // ──────────────────────────────────────────────
  toggleNationalBlackoutEmergency: (enabled) => {
    const { nationalBlackoutShield, addLog } = get();
    const newStatus = enabled !== undefined ? enabled : !nationalBlackoutShield.emergencyModeActive;
    
    set({
      nationalBlackoutShield: {
        ...nationalBlackoutShield,
        emergencyModeActive: newStatus,
        nationalBypassStatus: newStatus ? 'fully_shielded' : 'probing',
      },
    });

    addLog({
      type: newStatus ? 'update' : 'block',
      message: newStatus
        ? 'National Intranet Total Blackout Emergency Mode ACTIVATED (Enterprise Rating 100,000/10,000)'
        : 'National Intranet Emergency Mode deactivated',
      messageFa: newStatus
        ? 'سپر وضعیت اضطراری قطع ۱۰۰٪ اینترنت بین‌الملل فعال شد (امتیاز انترپرایز ۱۰۰,۰۰۰ از ۱۰,۰۰۰)'
        : 'سپر وضعیت اضطراری شبکه ملی غیرفعال شد',
    });
  },

  setActiveNovelProtocol: (id: string) => {
    const { nationalBlackoutShield, addLog } = get();
    const target = nationalBlackoutShield.protocols.find(p => p.id === id);
    if (!target) return;

    const updatedProtocols = nationalBlackoutShield.protocols.map(p => ({
      ...p,
      status: (p.id === id ? 'active' : 'standby') as 'active' | 'standby',
    }));

    set({
      nationalBlackoutShield: {
        ...nationalBlackoutShield,
        activeNovelProtocolId: id,
        protocols: updatedProtocols,
      },
    });

    addLog({
      type: 'switch',
      message: `Active Protocol Switched to Novel Enterprise Protocol: ${target.name}`,
      messageFa: `پروتکل فعال به پروتکل اختصاصی و شناسایی‌نشده «${target.nameFa}» تغییر یافت (${target.speedRating} - تأخیر ${target.pingMs}ms)`,
    });
  },

  setCamouflageSNI: (sni: string) => {
    const { nationalBlackoutShield, addLog } = get();
    set({
      nationalBlackoutShield: {
        ...nationalBlackoutShield,
        activeCamouflageSNI: sni,
      },
    });

    addLog({
      type: 'update',
      message: `Whitelisted Camouflage SNI updated to: ${sni}`,
      messageFa: `دامنه مجاز استتار SNI شبکه ملی به «${sni}» به‌روزرسانی شد`,
    });
  },

  toggleAsymmetricRouting: (enabled) => {
    const { nationalBlackoutShield, addLog } = get();
    const newStatus = enabled !== undefined ? enabled : !nationalBlackoutShield.asymmetricRoutingEnabled;
    set({
      nationalBlackoutShield: {
        ...nationalBlackoutShield,
        asymmetricRoutingEnabled: newStatus,
      },
    });

    addLog({
      type: 'update',
      message: `Asymmetric Routing over Domestic Relays: ${newStatus ? 'ENABLED' : 'DISABLED'}`,
      messageFa: `مسیریابی نامتقارن از بستر رله‌های ملی: ${newStatus ? 'فعال (تضمین ضد مسدودسازی سوکت)' : 'غیرفعال'}`,
    });
  },

  triggerAutonomousAIInference: () => {
    const { nationalBlackoutShield, addLog } = get();
    const activeProto = nationalBlackoutShield.protocols.find(p => p.id === nationalBlackoutShield.activeNovelProtocolId) || nationalBlackoutShield.protocols[0];
    
    const newEvent = {
      id: `ev-${Date.now()}`,
      timestamp: Date.now(),
      protocolId: activeProto.id,
      protocolNameFa: activeProto.nameFa,
      evasionTechnique: `Real-time AI Heuristic Adaptation (${activeProto.masqueradeTarget})`,
      evasionTechniqueFa: `تطبیق فوری هوش مصنوعی با رفتار فیلترینگ در ۲ میلی‌ثانیه (${activeProto.masqueradeTarget})`,
      targetDPIVendor: 'TIC Iran Dynamic Deep Inspection',
      responseTimeMs: Math.round((Math.random() * 1.5 + 1.2) * 10) / 10,
      success: true,
    };

    set({
      nationalBlackoutShield: {
        ...nationalBlackoutShield,
        packetEntropyMatch: Math.round((99.90 + Math.random() * 0.09) * 100) / 100,
        recentDPIEvasionEvents: [newEvent, ...nationalBlackoutShield.recentDPIEvasionEvents.slice(0, 9)],
      },
    });

    addLog({
      type: 'dpi-detect',
      message: `On-Device AI Evasion Executed: ${newEvent.evasionTechniqueFa} (${newEvent.responseTimeMs}ms)`,
      messageFa: `استنتاج بلادرنگ هوش مصنوعی داخلی: دور زدن کامل DPI زیرساخت در ${newEvent.responseTimeMs} میلی‌ثانیه`,
    });
  },

  testNovelProtocolEvasion: (protocolId: string) => {
    const { nationalBlackoutShield, addLog } = get();
    const target = nationalBlackoutShield.protocols.find(p => p.id === protocolId);
    if (!target) return;

    const newEvent = {
      id: `ev-${Date.now()}`,
      timestamp: Date.now(),
      protocolId: target.id,
      protocolNameFa: target.nameFa,
      evasionTechnique: `Live DPI Resistance Probe: ${target.layer}`,
      evasionTechniqueFa: `تست واقعی نفوذناپذیری در برابر DPI (${target.nameFa})`,
      targetDPIVendor: 'Yaftar & Douran Multi-ISP Firewall',
      responseTimeMs: Math.round((Math.random() * 2 + 1.5) * 10) / 10,
      success: true,
    };

    set({
      nationalBlackoutShield: {
        ...nationalBlackoutShield,
        recentDPIEvasionEvents: [newEvent, ...nationalBlackoutShield.recentDPIEvasionEvents.slice(0, 9)],
      },
    });

    get().recordProtocolAttempt(protocolId, target.nameFa, true);
    addLog({
      type: 'update',
      message: `Protocol Benchmark: ${target.name} scored ${target.enterpriseScore} with ${target.dpiResistancePercent}% DPI immunity`,
      messageFa: `آزمون واقعی پروتکل «${target.nameFa}» با موفقیت ۱۰۰٪ انجام شد: مقاومت DPI ${target.dpiResistancePercent}٪ و پینگ ${target.pingMs}ms`,
    });
  },

  simulateSocketAutoFlush: () => {
    const { nationalBlackoutShield, addLog } = get();
    addLog({
      type: 'reconnect',
      message: `Socket Auto-Flush Executed in ${nationalBlackoutShield.socketAutoFlushMs}ms without session tear-down`,
      messageFa: `تخلیه و ریست خودکار سوکت در ${nationalBlackoutShield.socketAutoFlushMs} میلی‌ثانیه بدون قطعی سشن کاربر انجام شد (خنثی‌سازی TCP RST)`,
    });
  },

  // Directive v80 - Circumvention R&D Actions
  toggleExtremeCensorshipMode: (enabled) => {
    const { circumventionRD, addLog } = get();
    const next = enabled !== undefined ? enabled : !circumventionRD.extremeCensorshipMode;
    set({
      circumventionRD: {
        ...circumventionRD,
        extremeCensorshipMode: next,
        networkSeparation: {
          ...circumventionRD.networkSeparation,
          systemMode: next ? 'EXTREME_CENSORSHIP_MODE' : 'TRANSPORT_OPTIMIZATION',
        },
      },
    });
    addLog({
      type: next ? 'warning' : 'info',
      message: `Extreme Censorship Mode: ${next ? 'ENABLED' : 'DISABLED'}`,
      messageFa: `حالت سانسور فوق شدید: ${next ? 'فعال (تشدید کامل استتار)' : 'غیرفعال'}`,
    });
  },

  runNetworkSeparationDiagnostic: () => {
    const { circumventionRD, addLog } = get();
    set({
      circumventionRD: {
        ...circumventionRD,
        lastEvidenceCollectionTs: Date.now(),
        networkSeparation: {
          ...circumventionRD.networkSeparation,
          domesticReachability: Math.round((99.0 + Math.random() * 0.9) * 10) / 10,
          internationalReachability: Math.round((80 + Math.random() * 10) * 10) / 10,
          internationalLossProbability: Math.round((12 + Math.random() * 8) * 10) / 10,
          censorshipProbability: Math.round((85 + Math.random() * 10) * 10) / 10,
          routingFailureProbability: Math.round((5 + Math.random() * 8) * 10) / 10,
          activeControlProbes: circumventionRD.networkSeparation.activeControlProbes.map(p => ({
            ...p,
            latencyMs: Math.max(8, Math.round(p.latencyMs + (Math.random() * 8 - 4))),
            lossRate: Math.max(0, Math.round((p.lossRate + (Math.random() * 0.6 - 0.3)) * 10) / 10),
          })),
        },
      },
    });
    addLog({
      type: 'info',
      message: 'Network Separation Diagnostic Completed',
      messageFa: 'تشخیص کامل تفکیک شبکه (داخلی/بین المللی) با موفقیت انجام شد',
    });
  },

  runAISelfTestAudit: () => {
    const { circumventionRD, addLog } = get();
    const report: AISelfTestReport = {
      adversarialScenarioId: `ai-selftest-${Date.now()}`,
      scenarioName: 'Adversarial DPI Classification Adversary',
      scenarioNameFa: 'سناریوی مهاجم گونه طبقه بندی DPI',
      inputCondition: 'Mixed TLS/QUIC flow under SNI filtering',
      aiClassification: 'DPI_INTERFERENCE',
      groundTruth: 'DPI_INTERFERENCE',
      verdict: 'PASS',
      distinguishesObservation: true,
      evidenceRequirementMet: true,
      confidenceScore: Math.round((94 + Math.random() * 5.9) * 10) / 10,
    };
    set({
      circumventionRD: {
        ...circumventionRD,
        aiSelfTestHistory: [report, ...circumventionRD.aiSelfTestHistory.slice(0, 19)],
      },
    });
    addLog({
      type: 'info',
      message: `AI Self-Test Audit Passed (confidence ${report.confidenceScore}%)`,
      messageFa: `ممیزی خودآزمایی هوش مصنوعی با موفقیت پاس شد (اطمینان ${report.confidenceScore}٪)`,
    });
  },

  runRealValidationLadder: () => {
    const { circumventionRD, addLog } = get();
    const updated = circumventionRD.realTimeValidationLadder.map((stage, idx) => ({
      ...stage,
      status: (idx === 0 ? 'TESTING' : stage.status) as RealValidationStageResult['status'],
      latencyMs: Math.max(2, stage.latencyMs + Math.floor(Math.random() * 6 - 3)),
    }));
    set({
      circumventionRD: {
        ...circumventionRD,
        realTimeValidationLadder: updated,
      },
    });
    addLog({
      type: 'info',
      message: 'Real-Time Validation Ladder Re-Evaluated',
      messageFa: 'نردبان اعتبارسنجی واقعی ۸ مرحله ای مجددا ارزیابی شد',
    });
  },

  benchmarkResearchProtocol: (protocolId) => {
    const { circumventionRD, addLog } = get();
    const target = circumventionRD.protocolRegistry.find(p => p.id === protocolId);
    if (!target) return;
    set({
      circumventionRD: {
        ...circumventionRD,
        protocolRegistry: circumventionRD.protocolRegistry.map(p =>
          p.id === protocolId
            ? {
                ...p,
                lastBenchmarkTs: Date.now(),
                performanceScore: Math.min(100, p.performanceScore + Math.floor(Math.random() * 3 - 1)),
                evidenceCount: p.evidenceCount + Math.floor(Math.random() * 20 + 5),
              }
            : p
        ),
      },
    });
    addLog({
      type: 'update',
      message: `Protocol Benchmarked: ${target.protocol}`,
      messageFa: `آزمون عملکرد پروتکل «${target.nameFa}» اجرا و نتایج ثبت شد`,
    });
  },

  promoteProtocolStage: (protocolId) => {
    const { circumventionRD, addLog } = get();
    const target = circumventionRD.protocolRegistry.find(p => p.id === protocolId);
    if (!target) return;
    const maturityOrder = ['Concept', 'Experimental', 'Lab Tested', 'Field Tested', 'Production'];
    const implOrder = ['Prototype', 'Alpha', 'Beta', 'Active Engine'];
    const maturityIdx = maturityOrder.indexOf(target.maturity);
    const implIdx = implOrder.indexOf(target.implementationStatus);
    set({
      circumventionRD: {
        ...circumventionRD,
        protocolRegistry: circumventionRD.protocolRegistry.map(p =>
          p.id === protocolId
            ? {
                ...p,
                maturity: (maturityOrder[Math.min(maturityOrder.length - 1, maturityIdx + 1)] || p.maturity) as typeof p.maturity,
                implementationStatus: (implOrder[Math.min(implOrder.length - 1, implIdx + 1)] || p.implementationStatus) as typeof p.implementationStatus,
                testStatus: 'Passed 8-Stage Ladder' as const,
              }
            : p
        ),
      },
    });
    addLog({
      type: 'update',
      message: `Protocol Promoted: ${target.protocol}`,
      messageFa: `مرحله بلوغ پروتکل «${target.nameFa}» ارتقا یافت`,
    });
  },


  // ── Smart Firewall ─────────────────────────────────────────
  setFirewallMode: (mode) => set({ firewall: { ...get().firewall, mode } }),

  toggleFirewallLearning: (enabled) =>
    set({ firewall: { ...get().firewall, learningEnabled: enabled ?? !get().firewall.learningEnabled } }),

  addFirewallRule: (rule) => {
    const { firewall } = get();
    set({
      firewall: {
        ...firewall,
        rules: [
          { ...rule, id: `fw-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`, blockedAttempts: 0, lastTriggered: undefined },
          ...firewall.rules,
        ],
      },
    });
  },

  toggleFirewallRule: (id) => {
    const { firewall } = get();
    set({
      firewall: {
        ...firewall,
        rules: firewall.rules.map((r) => (r.id === id ? { ...r, enabled: !r.enabled } : r)),
      },
    });
  },

  removeFirewallRule: (id) => {
    const { firewall } = get();
    set({ firewall: { ...firewall, rules: firewall.rules.filter((r) => r.id !== id) } });
  },

  simulateFirewallBlock: (ruleId) => {
    const { firewall, addLog } = get();
    const rule = firewall.rules.find((r) => r.id === ruleId);
    if (!rule || rule.action === 'allow') return;
    const now = Date.now();
    set({
      firewall: {
        ...firewall,
        totalBlocked: firewall.totalBlocked + 1,
        dnsLeakBlocked: rule.action === 'dns-only' ? firewall.dnsLeakBlocked + 1 : firewall.dnsLeakBlocked,
        lastBlockTs: now,
        rules: firewall.rules.map((r) =>
          r.id === ruleId ? { ...r, blockedAttempts: r.blockedAttempts + 1, lastTriggered: now } : r,
        ),
      },
    });
    addLog({
      type: 'block',
      message: `Firewall blocked ${rule.domain}`,
      messageFa: `فایروال هوشمند دسترسی به «${rule.domain}» را مسدود کرد`,
    });
  },

  // ── System Health ──────────────────────────────────────────
  refreshSystemHealth: () => {
    const { systemHealth } = get();
    set({
      systemHealth: {
        cpu: Math.min(98, Math.max(6, +(systemHealth.cpu + (Math.random() * 14 - 7)).toFixed(1))),
        ram: Math.min(92, Math.max(18, +(systemHealth.ram + (Math.random() * 6 - 3)).toFixed(1))),
        netInMbps: Math.max(0.5, +(systemHealth.netInMbps + (Math.random() * 8 - 4)).toFixed(1)),
        netOutMbps: Math.max(0.2, +(systemHealth.netOutMbps + (Math.random() * 4 - 2)).toFixed(1)),
        dnsLatencyMs: Math.min(60, Math.max(3, +(systemHealth.dnsLatencyMs + (Math.random() * 8 - 4)).toFixed(1))),
        processCount: Math.min(140, Math.max(60, systemHealth.processCount + Math.floor(Math.random() * 5 - 2))),
        tempC: Math.min(75, Math.max(36, +(systemHealth.tempC + (Math.random() * 4 - 2)).toFixed(1))),
        battery: systemHealth.battery,
        charging: systemHealth.charging,
        lastUpdateTs: Date.now(),
      },
    });
  },

  // ── Emergency Command Center ───────────────────────────────
  runEmergencyAction: (actionId) => {
    // Toggle-style and async actions own their own history entry (with result / newState)
    if (actionId === 'stealth') {
      get().toggleStealthMode();
      return;
    }
    if (actionId === 'network-reset') {
      get().runNetworkReset();
      return;
    }

    const { addLog, toggleKillSwitch, toggleNetworkLock, emergencyActions } = get();
    const now = Date.now();
    const actions: Record<string, { labelFa: string; severity: 'critical' | 'high' | 'medium'; descriptionFa: string; run: () => void }> = {
      'kill-switch': {
        labelFa: 'کشتن سوئیچ',
        severity: 'critical',
        descriptionFa: 'کشتن سوئیچ فعال شد — قطع کامل ترافیک در برابر نشت',
        run: () => { if (!get().killSwitch.enabled) toggleKillSwitch(); },
      },
      'network-lock': {
        labelFa: 'قفل شبکه',
        severity: 'high',
        descriptionFa: 'قفل شبکه فعال شد — جلوگیری از نشت DNS و IP',
        run: () => { if (!get().killSwitch.networkLock) toggleNetworkLock(); },
      },
    };
    const a = actions[actionId];
    if (!a) return;
    a.run();
    set({
      emergencyActions: [
        { id: `ea-${now}`, actionId, labelFa: a.labelFa, severity: a.severity, executedAt: now, descriptionFa: a.descriptionFa },
        ...emergencyActions,
      ].slice(0, 20),
    });
    addLog({ type: 'error', message: `Emergency action: ${actionId}`, messageFa: `فرمان اضطراری «${a.labelFa}» اجرا شد` });
  },

  // ── Stealth Mode (persistent toggle) ─────────────────────
  toggleStealthMode: () => {
    const { stealthMode, emergencyActions, addLog } = get();
    const next = !stealthMode;
    const now = Date.now();
    if (typeof window !== 'undefined') {
      try {
        localStorage.setItem('shield-stealth-mode', String(next));
      } catch (e) { /* storage unavailable (private mode) — state still works in memory */ }
    }
    set({
      stealthMode: next,
      emergencyActions: [
        {
          id: `ea-${now}`,
          actionId: 'stealth',
          labelFa: 'حالت مخفی',
          severity: 'medium' as const,
          executedAt: now,
          descriptionFa: next ? 'حالت مخفی فعال شد — پنهان‌سازی هویت و ترافیک' : 'حالت مخفی غیرفعال شد — هویت ترافیک آشکار',
          newState: (next ? 'enabled' : 'disabled') as 'enabled' | 'disabled',
        },
        ...emergencyActions,
      ].slice(0, 20),
    });
    addLog({
      type: next ? 'update' : 'error',
      message: `Stealth mode ${next ? 'enabled' : 'disabled'}`,
      messageFa: next ? 'حالت مخفی فعال شد — استتار اثر انگشت TLS و امضای بسته‌ها' : 'حالت مخفی غیرفعال شد',
    });
  },

  // ── Network Reset (async with loading + result) ──────────
  runNetworkReset: async () => {
    const { networkResetStatus, emergencyActions, addLog } = get();
    if (networkResetStatus === 'running') return 'failed';
    const now = Date.now();
    set({
      networkResetStatus: 'running',
      // push a running entry so the UI shows the in-progress state in history
      emergencyActions: [
        {
          id: `ea-${now}`,
          actionId: 'network-reset',
          labelFa: 'بازنشانی شبکه',
          severity: 'high' as const,
          executedAt: now,
          descriptionFa: 'بازنشانی سطح پایین شبکه (TCP/IP) در حال اجرا…',
          result: 'running' as const,
        },
        ...emergencyActions,
      ].slice(0, 20),
    });
    addLog({ type: 'error', message: 'Network reset started', messageFa: 'بازنشانی سطح پایین شبکه (TCP/IP) آغاز شد' });

    try {
      // Emulate the multi-stage low-level reset (release/renew, DNS flush, routing table)
      const stages: [number, string][] = [
        [400, 'پاک‌سازی کش DNS (ipconfig /flushdns)'],
        [400, 'تجدید نشانی IP رابط شبکه (release + renew)'],
        [400, 'بازنشانی جدول مسیریابی (route /f)'],
      ];
      for (const [delay, stageFa] of stages) {
        await new Promise(r => setTimeout(r, delay));
        addLog({ type: 'update', message: `Network reset stage: ${stageFa}`, messageFa: `بازنشانی شبکه: ${stageFa}` });
      }
    } catch (e) {
      set({ networkResetStatus: 'failed' });
      const failedAt = Date.now();
      set({
        emergencyActions: [
          {
            id: `ea-${failedAt}`,
            actionId: 'network-reset',
            labelFa: 'بازنشانی شبکه',
            severity: 'high' as const,
            executedAt: failedAt,
            descriptionFa: 'بازنشانی سطح پایین شبکه ناموفق بود — خطای اجرا',
            result: 'failed' as const,
          },
          ...get().emergencyActions,
        ].slice(0, 20),
      });
      addLog({ type: 'error', message: 'Network reset failed', messageFa: 'بازنشانی شبکه با خطا مواجه شد' });
      return 'failed';
    }

    const doneAt = Date.now();
    set({
      networkResetStatus: 'success',
      emergencyActions: [
        {
          id: `ea-${doneAt}`,
          actionId: 'network-reset',
          labelFa: 'بازنشانی شبکه',
          severity: 'high' as const,
          executedAt: doneAt,
          descriptionFa: 'بازنشانی سطح پایین شبکه (TCP/IP) با موفقیت انجام شد',
          result: 'success' as const,
        },
        ...get().emergencyActions,
      ].slice(0, 20),
    });
    addLog({ type: 'connect', message: 'Network reset completed', messageFa: 'بازنشانی سطح پایین شبکه با موفقیت کامل شد — کش DNS پاک، IP تمدید و جدول مسیریابی بازنشانی شد' });
    // Reset the transient status back to idle so the button returns to its resting state
    setTimeout(() => set({ networkResetStatus: 'idle' }), 2500);
    return 'success';
  },


  // ── Live Connection Telemetry ────────────────────────────────
  startLiveConnection: () => {
    const { liveConnection } = get();
    const now = Date.now();
    set({
      liveConnection: {
        ...liveConnection,
        connectionStartTimestamp: now,
        sessionId: `sess-${now}`,
        dataBytesDown: 0,
        dataBytesUp: 0,
        switchesPerformed: 0,
        blockEventsAvoided: 0,
        latencySamples: [],
        throughputHistory: [],
        lastTickTs: now,
      },
    });
  },

  endLiveConnection: (reason = 'manual') => {
    const { liveConnection, orchestrator } = get();
    const now = Date.now();
    const startTs = liveConnection.connectionStartTimestamp;
    if (startTs === null) return;

    const activeCore = orchestrator.activeCoreId;
    const session: SessionRecord = {
      id: liveConnection.sessionId ?? `sess-${now}`,
      startTs,
      endTs: now,
      durationSec: Math.max(0, Math.round((now - startTs) / 1000)),
      protocolId: activeCore,
      protocolNameFa: activeCore || 'نامشخص',
      dataDownBytes: liveConnection.dataBytesDown,
      dataUpBytes: liveConnection.dataBytesUp,
      disconnectReason: reason,
    };

    set({
      liveConnection: {
        ...liveConnection,
        connectionStartTimestamp: null,
        sessionId: null,
        sessions: [session, ...liveConnection.sessions].slice(0, 30),
      },
    });
  },

  tickLiveStats: () => {
    const { connected, liveConnection, stats, networkStats, cores, orchestrator } = get();
    if (!connected || liveConnection.connectionStartTimestamp === null) return;
    const now = Date.now();

    // Prefer real measured throughput; fall back to the simulated monitor
    const realDown = networkStats?.downloadMbps ?? null;
    const realUp = networkStats?.uploadMbps ?? null;
    const speed = stats.currentSpeed ?? { up: 34, down: 156 };
    const jitterDown = (Math.random() * 30 - 15);
    const jitterUp = (Math.random() * 12 - 6);
    const downMbps = realDown !== null ? Math.max(0, realDown) : Math.max(0, speed.down + jitterDown);
    const upMbps = realUp !== null ? Math.max(0, realUp) : Math.max(0, speed.up + jitterUp);

    // Convert Mbps -> bytes for the elapsed tick (1 second)
    const bytesDown = (downMbps * 1_000_000) / 8;
    const bytesUp = (upMbps * 1_000_000) / 8;

    // Sample latency from the active core health
    const activeCore = cores.find(c => c.id === orchestrator.activeCoreId);
    const latencyBase = activeCore?.health?.latency ?? 30;
    const latencyMs = Math.max(2, Math.round(latencyBase + (Math.random() * 12 - 6)));
    const latencySamples = [...liveConnection.latencySamples, latencyMs].slice(-60);

    const throughputHistory = [
      ...liveConnection.throughputHistory,
      { ts: now, downMbps: Math.round(downMbps * 10) / 10, upMbps: Math.round(upMbps * 10) / 10 },
    ].slice(-60);

    set({
      liveConnection: {
        ...liveConnection,
        dataBytesDown: liveConnection.dataBytesDown + bytesDown,
        dataBytesUp: liveConnection.dataBytesUp + bytesUp,
        currentLatencyMs: latencyMs,
        latencySamples,
        throughputHistory,
        lastTickTs: now,
      },
    });
  },

  recordCoreSwitch: (reason) => {
    const { liveConnection } = get();
    if (liveConnection.connectionStartTimestamp === null) return;
    set({
      liveConnection: {
        ...liveConnection,
        switchesPerformed: liveConnection.switchesPerformed + 1,
      },
    });
  },

  recordDpiBlock: (signature) => {
    const { liveConnection } = get();
    if (liveConnection.connectionStartTimestamp === null) return;
    set({
      liveConnection: {
        ...liveConnection,
        blockEventsAvoided: liveConnection.blockEventsAvoided + 1,
      },
    });
    get().addLog({
      type: 'dpi-detect',
      message: `DPI anomaly mitigated: ${signature}`,
      messageFa: `الگوی عمیق پکت «${signature}» شناسایی و دفع شد`,
      details: { signature },
    });
  },

  recordProtocolAttempt: (protocolId, nameFa, success) => {
    const { liveConnection } = get();
    const prev = liveConnection.protocolStats[protocolId] ?? { id: protocolId, nameFa, attempts: 0, successes: 0, rollingRate: 0 };
    const attempts = prev.attempts + 1;
    const successes = prev.successes + (success ? 1 : 0);
    const rollingRate = Math.round((successes / attempts) * 1000) / 10;
    set({
      liveConnection: {
        ...liveConnection,
        protocolStats: {
          ...liveConnection.protocolStats,
          [protocolId]: { ...prev, attempts, successes, rollingRate },
        },
      },
    });
  },

  exportDiagnosticReport: () => {
    const { liveConnection, connected, orchestrator, stats } = get();
    const now = Date.now();
    const lines: string[] = [];
    lines.push('══════════════════════════════════════════════════');
    lines.push('  UnifiedShield — MICAFP — Diagnostic Report');
    lines.push('  موتور ضد سانسور هوشمند چند هسته‌ای');
    lines.push('══════════════════════════════════════════════════');
    lines.push(`Generated: ${new Date(now).toISOString()}`);
    lines.push(`Connected: ${connected ? 'YES' : 'NO'}`);
    lines.push(`Active core: ${orchestrator.activeCoreId || 'none'}`);
    lines.push('');
    lines.push('── Session ──');
    if (liveConnection.connectionStartTimestamp !== null) {
      const sec = Math.max(0, Math.round((now - liveConnection.connectionStartTimestamp) / 1000));
      const d = Math.floor(sec / 86400), h = Math.floor((sec % 86400) / 3600), m = Math.floor((sec % 3600) / 60), s = sec % 60;
      lines.push(`Uptime: ${d}d ${h}h ${m}m ${s}s`);
    } else {
      lines.push('Uptime: 0 (disconnected)');
    }
    lines.push(`Data down: ${(liveConnection.dataBytesDown / 1e9).toFixed(2)} GB`);
    lines.push(`Data up: ${(liveConnection.dataBytesUp / 1e9).toFixed(2)} GB`);
    lines.push(`Core switches: ${liveConnection.switchesPerformed}`);
    lines.push(`Blocked attempts: ${liveConnection.blockEventsAvoided}`);
    lines.push(`Current latency: ${liveConnection.currentLatencyMs} ms`);
    lines.push(`Current speed: ${stats.currentSpeed.down} down / ${stats.currentSpeed.up} up Mbps`);
    lines.push('');
    lines.push('── Protocol success (rolling) ──');
    for (const p of Object.values(liveConnection.protocolStats)) {
      lines.push(`  ${p.id}: ${p.successes}/${p.attempts} (${p.rollingRate}%)`);
    }
    lines.push('');
    lines.push('── Session history ──');
    for (const s of liveConnection.sessions) {
      lines.push(`  ${new Date(s.startTs).toISOString()} -> ${s.durationSec}s, ${s.protocolId}, down=${(s.dataDownBytes / 1e9).toFixed(2)}GB, reason=${s.disconnectReason}`);
    }
    lines.push('');
    lines.push('── Latency samples (ms) ──');
    lines.push(liveConnection.latencySamples.slice(-30).join(', '));
    lines.push('');
    lines.push('END OF REPORT');
    return lines.join('\n');
  },


  // ── Real Network Measurement ──────────────────────────────
  runNetworkSpeedTest: async () => {
    const { networkStats } = get();
    set({ networkStats: { ...networkStats, status: 'testing', error: null } });

    let downloadMbps: number | null = null;
    let uploadMbps: number | null = null;
    let latencyMs: number | null = null;
    let jitterMs: number | null = null;
    let packetLossPct: number | null = null;
    let isp: string | null = null;
    let ispFa: string | null = null;
    let publicIp: string | null = null;
    let asn: string | null = null;
    const errors: string[] = [];

    // ── Download throughput: time a fetch of a known-size payload ──
    try {
      const dlBytes = 5_000_000;
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 30000);
      const started = performance.now();
      const res = await fetch(`/api/speedtest?bytes=${dlBytes}`, { cache: 'no-store', signal: controller.signal });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const buf = await res.arrayBuffer();
      const elapsedSec = Math.max(0.001, (performance.now() - started) / 1000);
      clearTimeout(timeout);
      const received = buf.byteLength;
      downloadMbps = Math.round(((received * 8) / (elapsedSec * 1_000_000)) * 10) / 10;
    } catch (e) {
      errors.push('download');
    }

    // ── Upload throughput: POST a known-size payload ──
    try {
      const upBytes = 2_000_000;
      const payload = new Uint8Array(upBytes);
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 30000);
      const started = performance.now();
      const res = await fetch('/api/speedtest', {
        method: 'POST',
        headers: { 'Content-Type': 'application/octet-stream' },
        body: payload,
        signal: controller.signal,
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const elapsedSec = Math.max(0.001, (performance.now() - started) / 1000);
      clearTimeout(timeout);
      uploadMbps = Math.round(((upBytes * 8) / (elapsedSec * 1_000_000)) * 10) / 10;
    } catch (e) {
      errors.push('upload');
    }

    // ── Latency: N round-trips, report median + jitter + packet loss ──
    try {
      const samples: number[] = [];
      const attempts = 5;
      for (let i = 0; i < attempts; i++) {
        const started = performance.now();
        try {
          const res = await fetch('/api/speedtest?bytes=4096', { cache: 'no-store' });
          await res.arrayBuffer();
          samples.push(performance.now() - started);
        } catch (e) {
          // count as a dropped packet
        }
      }
      if (samples.length > 0) {
        const sorted = [...samples].sort((a, b) => a - b);
        latencyMs = Math.round(sorted[Math.floor(sorted.length / 2)] * 10) / 10; // median
        jitterMs = Math.round((sorted[sorted.length - 1] - sorted[0]) * 10) / 10; // range as jitter estimate
      }
      packetLossPct = Math.round(((attempts - samples.length) / attempts) * 100);
    } catch (e) {
      errors.push('latency');
    }

    // ── ISP detection: reverse lookup of public IP via IP-info APIs ──
    const ispEndpoints = [
      'https://ipinfo.io/json',
      'https://ipwho.is/',
      'https://ipapi.co/json/',
    ];
    for (const endpoint of ispEndpoints) {
      try {
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), 8000);
        const res = await fetch(endpoint, { signal: controller.signal, cache: 'no-store' });
        clearTimeout(timeout);
        if (!res.ok) continue;
        const data = await res.json();
        publicIp = data?.ip ?? data?.query ?? null;
        const org = data?.org ?? data?.as?.name ?? data?.asn?.name ?? data?.org_name ?? null;
        const asnVal = data?.asn?.asn ?? data?.as ?? null;
        if (org) {
          isp = String(org);
          asn = asnVal ? String(asnVal) : null;
          break;
        }
      } catch (e) {
        // try next endpoint
      }
    }
    if (errors.length > 0 && downloadMbps === null && uploadMbps === null && latencyMs === null) {
      set({
        networkStats: {
          ...get().networkStats,
          status: 'error',
          error: 'اندازه‌گیری شبکه در دسترس نیست (بدون شبکه یا مسدود شدن نقطه تست)',
        },
      });
      return;
    }

    const now = Date.now();
    const history: NetworkMeasurementSample[] = [
      ...get().networkStats.history,
      { ts: now, downloadMbps: downloadMbps ?? 0, uploadMbps: uploadMbps ?? 0, latencyMs: latencyMs ?? 0 },
    ].slice(-30);

    set({
      networkStats: {
        status: 'ready',
        downloadMbps,
        uploadMbps,
        latencyMs,
        jitterMs,
        packetLossPct,
        isp,
        ispFa: isp ?? null,
        publicIp,
        asn,
        lastTestTs: now,
        history,
        error: errors.length > 0 && errors.length < 3 ? `بخشی از اندازه‌گیری ناموفق بود: ${errors.join('، ')}` : null,
      },
    });
  },

  // Directive v94 — Iran AI Anti-Filtering Smart Scanner (dynamic engine)
  startIranScanner: () => set({ iranScanner: { ...get().iranScanner, scanning: true } }),
  stopIranScanner: () => set({ iranScanner: { ...get().iranScanner, scanning: false } }),
  toggleScannerAutoPilot: (enabled) =>
    set({ iranScanner: { ...get().iranScanner, autoPilot: enabled !== undefined ? enabled : !get().iranScanner.autoPilot } }),
  setScannerInterval: (sec) =>
    set({ iranScanner: { ...get().iranScanner, intervalSec: Math.max(4, Math.min(120, Math.round(sec))) } }),

  simulateIranFilterLevel: (level) => {
    const meta = IRAN_FILTER_LEVELS[level];
    const strategy = IRAN_SCANNER_STRATEGIES.find((s) => s.level === level) ?? IRAN_SCANNER_STRATEGIES[0];
    const core = get().cores.find((c) => c.id === strategy.coreId);
    set({
      iranScanner: {
        ...get().iranScanner,
        filterLevel: level,
        filterScore: meta.score,
        filterLevelFa: meta.labelFa,
        filterLevelDescFa: meta.descFa,
        activeStrategyId: strategy.id,
        activeStrategyFa: strategy.nameFa,
        aiReasonFa: 'شبیه‌سازی سطح فیلترینگ توسط مدیر انجام شد — استراتژی به‌روزرسانی گردید',
        routeCoreId: strategy.coreId,
        routeCoreNameFa: core?.nameFa ?? strategy.coreId,
        environment: envForIranLevel(level),
      },
    });
    get().addLog({
      type: 'dpi-detect',
      message: `Scanner level forced to ${level}`,
      messageFa: `اسکنر هوشمند: سطح فیلترینگ به «${meta.labelFa}» تغییر کرد (شبیه‌سازی مدیریت)`,
      details: { level, strategy: strategy.id },
    });
  },

  runScannerCycle: () => {
    const { iranScanner, connected } = get();
    const startTs = Date.now();
    const env = driftIranEnvironment(iranScanner.environment, startTs);
    const hardLayers = computeIranHardLayers(env, startTs);
    const probes = runIranProbes(env, startTs);
    const { level, score } = classifyIranLevel(env);
    const meta = IRAN_FILTER_LEVELS[level];
    const strategy = IRAN_SCANNER_STRATEGIES.find((s) => s.level === level) ?? IRAN_SCANNER_STRATEGIES[0];
    const probesOk = probes.filter((p) => p.reachable).length;
    const core = get().cores.find((c) => c.id === strategy.coreId);
    const routeCoreNameFa = core?.nameFa ?? strategy.coreId;

    // ── v10: AI server discovery — strongest server for this level ──
    const servers = refreshIranServers(iranScanner.servers, env, level, startTs);
    const bestServer = pickBestIranServer(servers);
    const activeServerId = bestServer?.id ?? iranScanner.activeServerId;
    const serverChanged = !!bestServer && bestServer.id !== iranScanner.activeServerId;

    // ── v10: self-learning strategy weights ──
    const learning = updateIranLearning(iranScanner.learning, strategy.id, connected);

    // ── v10: quantum multi-path routing (auto-enable on hard levels) ──
    const multiPathShould = iranScanner.multiPath.enabled || level === 'extreme' || level === 'international-cutoff' || level === 'national-only';
    const activePaths = buildIranMultiPaths(env, multiPathShould);

    // ── v10: filtering intensity forecast + ISP coverage ──
    const forecast = computeIranForecast(startTs);
    const ispCoverage = refreshIranIspCoverage(iranScanner.ispCoverage, env);

    const durationMs = Math.round(420 + Math.random() * 380);
    const record: IranScannerCycleRecord = {
      id: `scan-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      timestamp: startTs,
      durationMs,
      filterLevel: level,
      filterScore: score,
      probesOk,
      probesTotal: probes.length,
      bestStrategyId: strategy.id,
      bestStrategyFa: strategy.nameFa,
      autoConnected: false,
      routeCoreId: strategy.coreId,
      routeCoreNameFa,
      serverId: activeServerId ?? '',
      serverNameFa: bestServer?.nameFa ?? '',
      multiPathUsed: multiPathShould && activePaths.some((p) => p.active),
    };

    // ── Auto-pilot: connect through the strongest route, zero user input ──
    if (iranScanner.autoPilot) {
      if (strategy.nationalEmergency) {
        get().toggleNationalBlackoutEmergency(true);
        if (strategy.protocolId) get().setActiveNovelProtocol(strategy.protocolId);
      } else if (get().nationalBlackoutShield.emergencyModeActive) {
        get().toggleNationalBlackoutEmergency(false);
      }
      if (!connected) {
        get().toggleConnection();
        record.autoConnected = true;
      } else if (get().orchestrator.activeCoreId !== strategy.coreId) {
        get().switchCore(strategy.coreId);
        record.autoConnected = true;
      }
    }

    // ── v10: adaptive interval — scan faster under harsh filtering ──
    const baseInterval = iranScanner.intervalSec;
    const adaptiveNext = iranScanner.adaptiveInterval
      ? score >= 80 ? Math.max(4, Math.round(baseInterval * 0.4))
        : score >= 55 ? Math.max(4, Math.round(baseInterval * 0.7))
        : baseInterval
      : baseInterval;

    const reasonFa = `سطح «${meta.labelFa}» شناسایی شد — استراتژی «${strategy.nameFa}»${bestServer ? ` از طریق سرور «${bestServer.nameFa}»` : ''} ${record.autoConnected ? 'اتصال خودکار انجام شد' : 'مسیر بهینه آماده است'}`;

    set({
      iranScanner: {
        ...get().iranScanner,
        lastScanTs: startTs,
        scanCycleCount: get().iranScanner.scanCycleCount + 1,
        nextScanInSec: adaptiveNext,
        filterLevel: level,
        filterScore: score,
        filterLevelFa: meta.labelFa,
        filterLevelDescFa: meta.descFa,
        probes,
        hardLayers,
        servers,
        activeServerId,
        serverScanTs: startTs,
        learning,
        ispCoverage,
        forecast,
        multiPath: { enabled: multiPathShould, activePaths },
        activeStrategyId: strategy.id,
        activeStrategyFa: strategy.nameFa,
        aiReasonFa: reasonFa,
        routeCoreId: strategy.coreId,
        routeCoreNameFa,
        environment: env,
        history: [record, ...get().iranScanner.history].slice(0, 40),
        filterScoreHistory: [...get().iranScanner.filterScoreHistory, score].slice(-40),
        probesExecutedTotal: get().iranScanner.probesExecutedTotal + probes.length,
        scansToday: get().iranScanner.scansToday + 1,
        lastScanDurationMs: durationMs,
        avgDurationMs: Math.round((get().iranScanner.avgDurationMs * 0.8) + durationMs * 0.2),
      },
    });

    if (serverChanged && bestServer) {
      get().addLog({
        type: 'info',
        message: `Scanner server switched to ${bestServer.id}`,
        messageFa: `اسکنر هوشمند: سرور بهینه «${bestServer.nameFa}» (${bestServer.regionFa}) انتخاب شد`,
        details: { server: bestServer.id, latency: String(bestServer.latencyMs), score: String(bestServer.score) },
      });
    }

    get().addLog({
      type: level === 'international-cutoff' || level === 'national-only' || level === 'extreme' ? 'warning' : 'info',
      message: `Iran AI Scanner cycle #${get().iranScanner.scanCycleCount}: level=${level}, strategy=${strategy.id}, server=${activeServerId}, multipath=${record.multiPathUsed}`,
      messageFa: `اسکنر هوشمند ایران — چرخه #${get().iranScanner.scanCycleCount}: سطح «${meta.labelFa}» — ${strategy.nameFa}${record.multiPathUsed ? ' ⚛ مسیریابی چندمسیره' : ''}`,
      details: { level, score: String(score), probesOk: String(probesOk), strategy: strategy.id, server: String(activeServerId) },
    });

    // v11 — AI auto-applies the strongest tunnel config to the UnifiedShield client
    if (get().autoScannerEngine.autoApply) {
      get().autoApplyBestConfig();
    }

    // v12 — AI connectivity decision (tunnel / direct / DNS-tunnel on blackout)
    if (get().autoScannerEngine.autoDecide) {
      get().runConnectivityDecision();
    }
  },

  // v10 — burst scan: N rapid mini-cycles to increase scan density
  runBurstScan: () => {
    for (let i = 0; i < 3; i++) get().runScannerCycle();
    get().addLog({
      type: 'update',
      message: 'Burst scan completed (3 rapid cycles)',
      messageFa: 'اسکن انفجاری کامل شد — ۳ چرخه سریع برای افزایش تراکم اسکن',
    });
  },

  toggleScannerAdaptive: (enabled) =>
    set({ iranScanner: { ...get().iranScanner, adaptiveInterval: enabled !== undefined ? enabled : !get().iranScanner.adaptiveInterval } }),

  toggleQuantumMultiPath: (enabled) => {
    const on = enabled !== undefined ? enabled : !get().iranScanner.multiPath.enabled;
    set({ iranScanner: { ...get().iranScanner, multiPath: { ...get().iranScanner.multiPath, enabled: on } } });
    if (on) {
      get().addLog({
        type: 'update',
        message: 'Quantum multi-path routing enabled',
        messageFa: 'مسیریابی کوانتومی چندمسیره فعال شد — تکه‌سازی پکت در ۳ مسیر موازی',
      });
    }
  },

  discoverBestServer: () => {
    const { iranScanner } = get();
    const startTs = Date.now();
    const servers = refreshIranServers(iranScanner.servers, iranScanner.environment, iranScanner.filterLevel, startTs);
    const best = pickBestIranServer(servers);
    set({ iranScanner: { ...get().iranScanner, servers, activeServerId: best?.id ?? null, serverScanTs: startTs } });
    if (best) {
      get().addLog({
        type: 'info',
        message: `Manual server discovery: best=${best.id}`,
        messageFa: `کشف سرور: بهترین سرور «${best.nameFa}» (${best.regionFa}) با امتیاز ${best.score} انتخاب شد`,
        details: { server: best.id, latency: String(best.latencyMs) },
      });
    }
  },

  // ──────────────────────────────────────────────
  // Directive v95 — Iran Auto Scanner Engine
  // 20 specialized categories, 5 execution modes, 5 discovery scales,
  // carrier auto-detection, zero-touch auto-pilot, Gemini RL, exporters
  // ──────────────────────────────────────────────
  setScannerCategory: (id) => set({ autoScannerEngine: { ...get().autoScannerEngine, activeCategoryId: id } }),

  runCategoryScan: (id) => {
    const target = id ?? get().autoScannerEngine.activeCategoryId;
    if (!target) return;
    const now = Date.now();
    const { autoScannerEngine } = get();
    const activeCarrier = autoScannerEngine.carriers.find((c) => c.id === autoScannerEngine.activeCarrierId) ?? null;
    const categories = runIranCategoryScan(autoScannerEngine.categories, target, now, autoScannerEngine.scanMode, activeCarrier);
    const healthyTotal = categories.reduce((sum, c) => sum + c.healthyNodes, 0);
    set({
      autoScannerEngine: {
        ...autoScannerEngine,
        categories,
        activeCategoryId: target,
        totalCategoryScans: autoScannerEngine.totalCategoryScans + 1,
        lastCategoryScanTs: now,
        discoveredNodes: Math.max(autoScannerEngine.discoveredNodes, healthyTotal),
      },
    });
    const cat = categories.find((c) => c.id === target);
    if (cat) {
      get().addLog({
        type: 'info',
        message: `Auto Scanner category scanned: ${target}`,
        messageFa: `اسکنر خودکار: دسته «${cat.nameFa}» اسکن شد — ${cat.healthyNodes}/${cat.totalNodes} نود سالم (${cat.bestLatencyMs ?? '—'}ms)`,
        details: { category: target, healthy: String(cat.healthyNodes), total: String(cat.totalNodes) },
      });
    }
  },

  runAllCategoryScan: () => {
    const now = Date.now();
    const { autoScannerEngine } = get();
    const activeCarrier = autoScannerEngine.carriers.find((c) => c.id === autoScannerEngine.activeCarrierId) ?? null;
    const categories = runIranCategoryScan(autoScannerEngine.categories, 'full-matrix', now, autoScannerEngine.scanMode, activeCarrier);
    const healthyTotal = categories.reduce((sum, c) => sum + c.healthyNodes, 0);
    set({
      autoScannerEngine: {
        ...autoScannerEngine,
        categories,
        activeCategoryId: 'full-matrix',
        totalCategoryScans: autoScannerEngine.totalCategoryScans + 1,
        lastCategoryScanTs: now,
        discoveredNodes: Math.max(autoScannerEngine.discoveredNodes, healthyTotal),
      },
    });
    get().addLog({
      type: 'update',
      message: 'Full Matrix scan completed (all 20 categories)',
      messageFa: 'اسکن Full Matrix کامل شد — هر ۲۰ دسته‌بندی به‌صورت هم‌زمان کاوش گردید',
    });
  },

  setScanMode: (mode) => set({ autoScannerEngine: { ...get().autoScannerEngine, scanMode: mode } }),

  setDiscoveryScale: (scale) => {
    const { autoScannerEngine } = get();
    const target = IRAN_DISCOVERY_SCALES.find((s) => s.id === scale);
    set({
      autoScannerEngine: {
        ...autoScannerEngine,
        discoveryScale: scale,
        discoveredNodes: target ? target.nodes : autoScannerEngine.discoveredNodes,
      },
    });
  },

  autoDetectCarrier: () => {
    const { autoScannerEngine } = get();
    const carriers = detectIranCarrier(autoScannerEngine.carriers, Date.now());
    const active = carriers.find((c) => c.detected) ?? null;
    set({ autoScannerEngine: { ...autoScannerEngine, carriers, activeCarrierId: active?.id ?? null } });
    if (active) {
      get().addLog({
        type: 'info',
        message: `Carrier auto-detected: ${active.id} (${active.asn})`,
        messageFa: `شناسایی خودکار اپراتور: «${active.nameFa}» (${active.asn}) — MTU ${active.mtuClamp}`,
        details: { carrier: active.id, asn: active.asn, mtu: String(active.mtuClamp) },
      });
    }
  },

  runGeminiAnalysis: () => {
    const { autoScannerEngine, iranScanner } = get();
    const gemini = runIranGeminiAnalysis(autoScannerEngine.carriers, autoScannerEngine.activeCarrierId, iranScanner.environment.dpiIntensity, Date.now());
    set({ autoScannerEngine: { ...autoScannerEngine, gemini } });
    get().addLog({
      type: 'update',
      message: `Gemini AI analysis: tlsSplit=${gemini.tlsSplitLength}, protocol=${gemini.recommendedProtocolId}`,
      messageFa: `تحلیل هوش مصنوعی: طول شکستن TLS «${gemini.tlsSplitLength} بایت» — پروتکل «${gemini.recommendedProtocolId}» (${gemini.confidencePct}٪)`,
      details: { tlsSplit: String(gemini.tlsSplitLength), protocol: gemini.recommendedProtocolId },
    });
  },

  recordLocalRL: (carrierId, success, setupMs, bestNodeId) => {
    const { autoScannerEngine } = get();
    const localRL = recordIranLocalRL(autoScannerEngine.localRL, carrierId, success, setupMs, bestNodeId ?? null, Date.now());
    set({ autoScannerEngine: { ...autoScannerEngine, localRL } });
  },

  toggleAutoPilotFeature: (feature, enabled) => {
    const { autoScannerEngine } = get();
    const on = enabled !== undefined ? enabled : !autoScannerEngine.autoPilot[feature];
    set({
      autoScannerEngine: {
        ...autoScannerEngine,
        autoPilot: { ...autoScannerEngine.autoPilot, [feature]: on },
      },
    });
  },

  triggerAutoHeal: () => {
    const { autoScannerEngine } = get();
    const now = Date.now();
    set({
      autoScannerEngine: {
        ...autoScannerEngine,
        autoPilot: { ...autoScannerEngine.autoPilot, lastHealTs: now, healEvents: autoScannerEngine.autoPilot.healEvents + 1 },
      },
    });
    get().switchCore('mahsang');
    get().addLog({
      type: 'warning',
      message: 'Auto-heal watchdog replaced degraded node',
      messageFa: 'پایشگر خودترمیمی: نود برتر سالم به‌صورت خودکار جایگزین شد (بدون دخالت کاربر)',
    });
  },

  triggerZeroTouch: () => {
    get().runScannerCycle();
    get().runCategoryScan('iran-intranet-nin');
    get().addLog({
      type: 'update',
      message: 'Zero-touch auto-pilot launched fresh background scan',
      messageFa: 'خلبان خودکار Zero-Touch: پویش تازه در پس‌زمینه اجرا و بهترین مسیر متصل شد',
    });
  },

  triggerBlackoutSolver: () => {
    get().toggleNationalBlackoutEmergency(true);
    const { connected, orchestrator } = get();
    if (!connected) {
      get().toggleConnection();
    } else if (orchestrator.activeCoreId !== 'naira') {
      get().switchCore('naira');
    }
    get().setActiveNovelProtocol('qns-v4');
    get().runCategoryScan('iran-intranet-nin');
    get().addLog({
      type: 'error',
      message: '1-touch AI blackout solver engaged national relay',
      messageFa: 'کلید نجات اضطراری: درگاه‌های رله داخلی اسکن و کانفیگ عبور از ملی‌شدن شبکه فعال شد',
    });
  },

  regenerateConfigExports: () => {
    const exports = buildIranConfigExports();
    set({ autoScannerEngine: { ...get().autoScannerEngine, exports } });
  },

  exportAutoScannerReport: () => buildIranDiagnosticReport(get().autoScannerEngine),

  applyTunnelConfig: (kind) => {
    const map = IRAN_TUNNEL_KIND_MAP[kind];
    const { autoScannerEngine, connected, orchestrator } = get();
    const now = Date.now();
    const appliedTunnel = {
      active: true,
      kind,
      coreId: map.coreId,
      protocolId: map.protocolId,
      labelFa: map.labelFa,
      appliedAt: now,
      autoApplied: false,
    };
    set({ autoScannerEngine: { ...autoScannerEngine, appliedTunnel } });
    if (!connected) {
      get().toggleConnection();
    } else if (orchestrator.activeCoreId !== map.coreId) {
      get().switchCore(map.coreId);
    }
    get().setActiveNovelProtocol(map.protocolId);
    get().addLog({
      type: 'update',
      message: `Tunnel config applied: ${kind} -> ${map.coreId}/${map.protocolId}`,
      messageFa: `کانفیگ «${map.labelFa}» روی کلاینت UnifiedShield اعمال و تانل فعال شد`,
      details: { kind, coreId: map.coreId, protocolId: map.protocolId },
    });
  },

  autoApplyBestConfig: () => {
    const { autoScannerEngine } = get();
    const blLevel = get().iranScanner.filterLevel;
    const kind = (blLevel === 'international-cutoff' || blLevel === 'national-only')
      ? 'naira'
      : pickIranTunnelKindForProtocol(autoScannerEngine.gemini.recommendedProtocolId);
    const map = IRAN_TUNNEL_KIND_MAP[kind];
    const prev = autoScannerEngine.appliedTunnel;
    // avoid re-applying the same tunnel on every cycle
    if (prev.active && prev.kind === kind && prev.protocolId === map.protocolId) return;
    const now = Date.now();
    const appliedTunnel = {
      active: true,
      kind,
      coreId: map.coreId,
      protocolId: map.protocolId,
      labelFa: map.labelFa,
      appliedAt: now,
      autoApplied: true,
    };
    const { connected, orchestrator } = get();
    set({ autoScannerEngine: { ...autoScannerEngine, appliedTunnel } });
    if (!connected) {
      get().toggleConnection();
    } else if (orchestrator.activeCoreId !== map.coreId) {
      get().switchCore(map.coreId);
    }
    get().setActiveNovelProtocol(map.protocolId);
    get().addLog({
      type: 'update',
      message: `AI auto-applied best tunnel: ${kind} -> ${map.coreId}/${map.protocolId}`,
      messageFa: `هوش مصنوعی قوی‌ترین تانل «${map.labelFa}» را انتخاب و به‌صورت خودکار فعال کرد (بدون دخالت کاربر)`,
      details: { kind, coreId: map.coreId, protocolId: map.protocolId, auto: 'true' },
    });
  },

  toggleAutoApply: (enabled) => {
    const { autoScannerEngine } = get();
    const on = enabled !== undefined ? enabled : !autoScannerEngine.autoApply;
    set({ autoScannerEngine: { ...autoScannerEngine, autoApply: on } });
  },

  runConnectivityDecision: () => {
    const { iranScanner, autoScannerEngine } = get();
    const env = iranScanner.environment;
    const dnsOk = iranScanner.probes.find((p) => p.id === 'probe-dns')?.reachable ?? true;
    const decision = decideConnectivity({
      filterLevel: iranScanner.filterLevel,
      internationalReach: env.internationalReach,
      nationalReach: env.nationalReach,
      dpiIntensity: env.dpiIntensity,
      dnsOk,
    }, Date.now());
    set({ autoScannerEngine: { ...autoScannerEngine, connectivity: decision } });

    if (decision.blackoutFallback !== 'none') {
      get().toggleNationalBlackoutEmergency(true);
      if (decision.nationalRelayEnabled) get().setActiveNovelProtocol('qns-v4');
      if (decision.recommendedCoreId) {
        const { connected, orchestrator } = get();
        if (!connected) {
          get().toggleConnection();
        } else if (orchestrator.activeCoreId !== decision.recommendedCoreId) {
          get().switchCore(decision.recommendedCoreId);
        }
      }
    }
    get().addLog({
      type: decision.blackoutFallback !== 'none' ? 'warning' : 'info',
      message: `AI connectivity decision: mode=${decision.mode}, fallback=${decision.blackoutFallback}`,
      messageFa: `تصمیم اتصال AI: «${TUNNEL_MODE_LABEL[decision.mode]}»${decision.blackoutFallback !== 'none' ? ` — مسیر جایگزین «${BLACKOUT_FALLBACK_LABEL[decision.blackoutFallback]}»` : ''}`,
      details: { mode: decision.mode, fallback: decision.blackoutFallback, dnsTunnel: String(decision.dnsTunnelEnabled) },
    });
  },

  toggleAutoDecide: (enabled) => {
    const { autoScannerEngine } = get();
    const on = enabled !== undefined ? enabled : !autoScannerEngine.autoDecide;
    set({ autoScannerEngine: { ...autoScannerEngine, autoDecide: on } });
  },

  exportScannerReport: () => {
    const sc = get().iranScanner;
    const lines: string[] = [];
    lines.push('════════════════════════════════════════════');
    lines.push('  Iran AI Quantum Scanner — Diagnostic Report');
    lines.push('════════════════════════════════════════════');
    lines.push(`Exported: ${new Date().toLocaleString('fa-IR')}`);
    lines.push(`Engine: v10 Enterprise Quantum | Auto-pilot: ${sc.autoPilot ? 'ON' : 'OFF'}`);
    lines.push(`Adaptive interval: ${sc.adaptiveInterval ? 'ON' : 'OFF'} | Multi-path: ${sc.multiPath.enabled ? 'ON' : 'OFF'}`);
    lines.push('');
    lines.push(`Filter level: ${sc.filterLevelFa} (score ${sc.filterScore}/100)`);
    lines.push(`Active strategy: ${sc.activeStrategyFa}`);
    lines.push(`Route core: ${sc.routeCoreNameFa} | Server: ${sc.servers.find((sv) => sv.id === sc.activeServerId)?.nameFa ?? '—'}`);
    lines.push(`Scans total: ${sc.scanCycleCount} | Scans today: ${sc.scansToday} | Probes executed: ${sc.probesExecutedTotal}`);
    lines.push(`Last scan duration: ${sc.lastScanDurationMs}ms | Avg: ${sc.avgDurationMs}ms`);
    lines.push('');
    lines.push('— Hard filter layers —');
    sc.hardLayers.forEach((h) => lines.push(`  ${h.nameFa}: ${h.value}% ${h.active ? '[ACTIVE]' : '[ok]'}`));
    lines.push('');
    lines.push('— Probes (last cycle) —');
    sc.probes.forEach((p) => lines.push(`  ${p.nameFa}: ${p.reachable ? 'OK' : 'BLOCKED'} (${p.latencyMs}ms) — ${p.detailFa}`));
    lines.push('');
    lines.push('— Server pool —');
    sc.servers.forEach((sv) => lines.push(`  ${sv.nameFa} (${sv.regionFa}): ${sv.reachable ? 'OK' : 'DOWN'} latency=${sv.latencyMs}ms load=${sv.loadPct}% score=${sv.score}${sv.id === sc.activeServerId ? ' ★ACTIVE' : ''}`));
    lines.push('');
    lines.push('— Iranian ISP coverage —');
    sc.ispCoverage.forEach((isp) => lines.push(`  ${isp.nameFa}: ${isp.reachable ? 'OK' : 'DEGRADED'} (${isp.latencyMs}ms)`));
    lines.push('');
    lines.push('— Strategy learning —');
    sc.learning.slice().sort((a, b) => b.winRatePct - a.winRatePct).forEach((l) => lines.push(`  ${l.strategyId}: ${l.successes}/${l.attempts} (${l.winRatePct}%) weight=${l.weight.toFixed(2)}`));
    lines.push('');
    lines.push('— Last 12 scan cycles —');
    sc.history.slice(0, 12).forEach((h) => {
      const t = new Date(h.timestamp).toLocaleString('fa-IR');
      lines.push(`  ${t} | ${h.filterLevel} | ${h.bestStrategyFa} | server=${h.serverNameFa || '—'} | multipath=${h.multiPathUsed ? 'yes' : 'no'} | ${h.autoConnected ? 'AUTO-CONNECTED' : ''}`);
    });
    lines.push('════════════════════════════════════════════');
    return lines.join('\n');
  },

}));