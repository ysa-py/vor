// ─────────────────────────────────────────────────────────────
// v11 — Iran Auto Scanner Engine
// 20 specialized protocol categories, 5 execution modes, 5 dynamic
// discovery scales, carrier auto-detection, zero-touch auto-pilot,
// Gemini-style AI + on-device RL, and multi-core config exporters.
// Pure/derived logic lives here so the Zustand store stays thin.
// ─────────────────────────────────────────────────────────────

import type { AIConnectivityDecision } from './connectivity-decision';

export type IranScanCategoryId =
  | 'full-matrix'
  | 'iran-telcos'
  | 'white-dns'
  | 'storm-dns'
  | 'cotten-dns'
  | 'master-dns'
  | 'vless-reality'
  | 'hysteria2-tuic'
  | 'cf-clean-warp'
  | 'shadowtls-mimicry'
  | 'sni-fronting'
  | 'ipv4-ipv6-clean'
  | 'dnstt-edns0'
  | 'vaydns-noizdns'
  | 'ssh-wrappers'
  | 'naiveproxy'
  | 'doh-dot'
  | 'iran-intranet-nin'
  | 'dpi-rst-inspector'
  | 'tor-pluggable';

export interface IranScanCategory {
  id: IranScanCategoryId;
  nameFa: string;
  descFa: string;
  icon: string; // lucide icon key resolved in the panel
  protocols: string[];
  totalNodes: number;
  healthyNodes: number;
  bestLatencyMs: number | null;
  status: 'idle' | 'scanning' | 'ready';
  lastScanTs: number | null;
}

export type IranScanModeId =
  | 'turbo-parallel'
  | 'deep-dpi-audit'
  | 'operator-adaptive'
  | 'full-matrix-100'
  | 'ai-autonomous-blackout';

export interface IranScanMode {
  id: IranScanModeId;
  nameFa: string;
  descFa: string;
  parallelism: number; // independent coroutines
  targetNodes: number;
}

export type IranDiscoveryScaleId =
  | 'adaptive-fast'
  | 'deep-sweep'
  | 'massive-autonomous'
  | 'unlimited-ultra'
  | 'enterprise-quantum';

export interface IranDiscoveryScale {
  id: IranDiscoveryScaleId;
  nameFa: string;
  nodes: number;
  descFa: string;
}

export type IranCarrierNetworkType = 'cellular' | 'dsl' | 'fiber';

export interface IranCarrier {
  id: string;
  nameFa: string;
  asn: string;
  networkType: IranCarrierNetworkType;
  mtuClamp: number;
  detected: boolean;
  latencyMs: number | null;
  coveragePct: number;
}

export interface IranAutoPilotState {
  pathValidator: boolean; // real-time path validation every 6s
  autoHealingWatchdog: boolean; // >5% loss or >120ms ping → auto switch
  zeroTouch: boolean; // total quality drop → background re-scan + connect
  blackoutSolver: boolean; // 1-touch national relay solver
  batterySaver: boolean; // adaptive pulse frequency
  lastValidationTs: number | null;
  lastHealTs: number | null;
  validationEvents: number;
  healEvents: number;
}

export interface IranGeminiAnalysis {
  tlsSplitLength: number;
  recommendedProtocolId: string;
  confidencePct: number;
  reasoningFa: string;
  lastAnalysisTs: number | null;
}

export interface IranLocalRLRecord {
  carrierId: string;
  attempts: number;
  successes: number;
  winRatePct: number;
  avgSetupMs: number;
  bestNodeId: string | null;
  tlsSplitLength: number;
  lastUpdatedTs: number | null;
}

export interface IranConfigExports {
  singBox: string;
  xray: string;
  clashMeta: string;
  rawUris: string[];
}

export type IranTunnelKind = 'sing-box' | 'xray' | 'clash' | 'uris' | 'naira' | 'quazar' | 'stormdns';

export interface IranAppliedTunnel {
  active: boolean;
  kind: IranTunnelKind;
  coreId: string;
  protocolId: string;
  labelFa: string;
  appliedAt: number | null;
  autoApplied: boolean;
}

export interface IranAutoScannerEngineState {
  categories: IranScanCategory[];
  activeCategoryId: IranScanCategoryId | null;
  scanMode: IranScanModeId;
  discoveryScale: IranDiscoveryScaleId;
  discoveredNodes: number;
  totalCategoryScans: number;
  lastCategoryScanTs: number | null;
  backgroundScanActive: boolean;
  carriers: IranCarrier[];
  activeCarrierId: string | null;
  autoPilot: IranAutoPilotState;
  gemini: IranGeminiAnalysis;
  localRL: IranLocalRLRecord[];
  exports: IranConfigExports;
  autoApply: boolean;
  appliedTunnel: IranAppliedTunnel;
  connectivity: AIConnectivityDecision;
  autoDecide: boolean;
}

// ── 20 specialized protocol categories ───────────────────────

export const IRAN_SCAN_CATEGORIES: IranScanCategory[] = [
  { id: 'full-matrix', nameFa: 'تمام اسکنرها (Full Matrix)', descFa: 'اجرای تست تجمیعی روی تمامی پروتکل‌ها به‌صورت هم‌زمان', icon: 'grid', protocols: ['ALL'], totalNodes: 100, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'iran-telcos', nameFa: 'اسکنر اپراتورهای ایران', descFa: 'پروفایل‌های اختصاصی همراه اول، ایرانسل، مخابرات، رایتل، شاتل، آسیاتک، های‌وب، مبین‌نت و زیتل', icon: 'antenna', protocols: ['MCI AS44244', 'MTN AS35897', 'TCI AS58224', 'Rightel AS57218'], totalNodes: 36, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'white-dns', nameFa: 'WhiteDNS', descFa: 'کاوش سرورهای DNS پاکیزه با شبیه‌سازی دست‌تکانی uTLS مرورگر استاندارد', icon: 'cloud', protocols: ['uTLS-browser', 'DNS-clean'], totalNodes: 60, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'storm-dns', nameFa: 'StormDNS', descFa: 'پویش استریم مداوم TCP بدون Resource Leak برای عبور از مسدودی UDP', icon: 'zap', protocols: ['TCP-stream'], totalNodes: 48, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'cotten-dns', nameFa: 'CottenDNS (Adaptive FEC)', descFa: 'اسکن سرورهای Super-FEC 8:4 برای عبور از پکت‌لاس تا ۶۰٪', icon: 'layers', protocols: ['Super-FEC 8:4'], totalNodes: 42, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'master-dns', nameFa: 'MasterDns (ARQ Multi-Path)', descFa: 'پویش کلاستر ریسینگ ۸ مسیره برای خنثی‌سازی تله و پویزنینگ DNS کش کشوری (10.10.34.34)', icon: 'git-branch', protocols: ['ARQ-8path'], totalNodes: 32, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'vless-reality', nameFa: 'VLESS Reality & Vision', descFa: 'پویش سرورهای ریالیتی با امضای JA4 کروم، تقسیم ClientHello و Vision Padding', icon: 'fingerprint', protocols: ['vless-reality', 'vision'], totalNodes: 80, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'hysteria2-tuic', nameFa: 'Hysteria 2 & TUIC v5', descFa: 'پویش ترافیک QUIC با کنترل ازدحام Brutal، پورت‌هاپینگ پویا و رمزنگاری Salamander', icon: 'wind', protocols: ['hysteria2', 'tuic-v5'], totalNodes: 64, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'cf-clean-warp', nameFa: 'Cloudflare Clean IP & WARP', descFa: 'کاوش اندپوینت‌های پاکیزه Anycast وارپ و وایرگارد با Noise Handshake', icon: 'cloud-cog', protocols: ['warp', 'wireguard'], totalNodes: 96, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'shadowtls-mimicry', nameFa: 'ShadowTLS v3 & SNI Mimicry', descFa: 'دست‌تکانی دوگانه و استتار پشت CDNهای اپل، مایکروسافت و آمازون', icon: 'mask', protocols: ['shadowtls-v3'], totalNodes: 52, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'sni-fronting', nameFa: 'SNI & Domain Fronting', descFa: 'کاوش دامنه‌های وایت‌لیست و CDNهای معتبر (WhatsApp، Azure، Speedtest، GitHub)', icon: 'door-open', protocols: ['domain-fronting'], totalNodes: 58, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'ipv4-ipv6-clean', nameFa: 'IPv4 & IPv6 Clean Ranges', descFa: 'کاوش رنج‌های BGP Anycast با پینگ پایین و بدون مسدودی پورت', icon: 'waypoints', protocols: ['ipv4', 'ipv6-anycast'], totalNodes: 120, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'dnstt-edns0', nameFa: 'DNSTT & EDNS0', descFa: 'پویش تونلینگ بر بستر DNS با بافر ۴۰۹۶ بایتی و تغییر برچسب QNAME', icon: 'at-sign', protocols: ['dnstt', 'edns0'], totalNodes: 44, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'vaydns-noizdns', nameFa: 'VayDNS & NoizDNS', descFa: 'کاوش پروتکل‌های مبهم‌سازی نویز با دست‌تکانی Noise IK و جیتر تصادفی', icon: 'audio-waveform', protocols: ['vaydns', 'noizdns'], totalNodes: 40, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'ssh-wrappers', nameFa: 'SSH & Secure Wrappers', descFa: 'اسکن تانل‌های SSH بر بستر WebSocket، TLS 1.3 و CDN Fronting', icon: 'terminal', protocols: ['ssh-ws', 'tls1.3'], totalNodes: 50, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'naiveproxy', nameFa: 'NaiveProxy (Chromium JA4)', descFa: 'پویش پروکسی‌های مبتنی بر پشته شبکه Chromium با مولتی‌پلکس HTTP/3', icon: 'chrome', protocols: ['naiveproxy', 'http3'], totalNodes: 38, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'doh-dot', nameFa: 'DoH & DoT', descFa: 'کاوش استاندارد RFC 8484 و RFC 7858 روی سرورهای Anycast بین‌المللی', icon: 'shield-check', protocols: ['doh', 'dot'], totalNodes: 66, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'iran-intranet-nin', nameFa: 'Iran Intranet & Reverse Relay (NIN)', descFa: 'پویش درگاه‌های شبکه ملی و رله‌های معکوس آپارات، تلوبیون، اسنپ، دیجی‌کالا و ابرآروان', icon: 'radio', protocols: ['NIN-reverse-relay'], totalNodes: 24, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'dpi-rst-inspector', nameFa: 'DPI & TCP RST Inspector', descFa: 'سنجش تزریق Fake RST، اندازه‌گیری پنجره TCP و تشخیص تله‌های فیلترینگ', icon: 'scan-eye', protocols: ['fake-rst', 'tcp-window'], totalNodes: 46, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
  { id: 'tor-pluggable', nameFa: 'Tor & Pluggable Transports', descFa: 'پویش بروکرهای Snowflake مبتنی بر WebRTC و پل‌های obfs4', icon: 'orbit', protocols: ['snowflake', 'obfs4'], totalNodes: 70, healthyNodes: 0, bestLatencyMs: null, status: 'idle', lastScanTs: null },
];

// ── 5 scan execution modes ───────────────────────────────────

export const IRAN_SCAN_MODES: IranScanMode[] = [
  { id: 'turbo-parallel', nameFa: 'Turbo Parallel', descFa: 'پویش پرسرعت موازی با دسته‌های ۳۲ کوروتین مستقل و تأخیر کم', parallelism: 32, targetNodes: 500 },
  { id: 'deep-dpi-audit', nameFa: 'Deep DPI Audit', descFa: 'بازرسی موشکافانه پکت‌ها، سنجش جعل TCP RST و آزمودن تزریق DNS Poisoning', parallelism: 8, targetNodes: 120 },
  { id: 'operator-adaptive', nameFa: 'Operator-Adaptive', descFa: 'تشخیص خودکار اپراتور فعال و انتخاب هدفمندترین روش بای‌پس مخصوص آن', parallelism: 16, targetNodes: 300 },
  { id: 'full-matrix-100', nameFa: 'Full Matrix 100+', descFa: 'اسکن ماتریس کامل بیش از ۱۰۰ نود پایدار و صدها نود پویا', parallelism: 24, targetNodes: 100 },
  { id: 'ai-autonomous-blackout', nameFa: 'AI Autonomous Blackout', descFa: 'ارزیابی بلادرنگ ارتباط بین‌الملل، ورود خودکار به رله ملی و اعمال کانفیگ قطعی کامل', parallelism: 12, targetNodes: 60 },
];

// ── 5 dynamic discovery scales ───────────────────────────────

export const IRAN_DISCOVERY_SCALES: IranDiscoveryScale[] = [
  { id: 'adaptive-fast', nameFa: 'چابک (Adaptive Fast)', nodes: 250, descFa: 'تولید و کاوش ۲۵۰ نود پویا' },
  { id: 'deep-sweep', nameFa: 'پویش عمیق (Deep Sweep)', nodes: 750, descFa: 'کاوش ۷۵۰ نود در ساب‌نت‌های Anycast جهانی' },
  { id: 'massive-autonomous', nameFa: 'ماتریس عظیم', nodes: 2000, descFa: 'کاوش ۲۰۰۰ نود شامل رله‌های ملی و بین‌المللی' },
  { id: 'unlimited-ultra', nameFa: 'پویش فوق‌عظیم (Unlimited Ultra)', nodes: 5000, descFa: 'کاوش ۵۰۰۰ نود پویا با پشتیبانی هوش مصنوعی' },
  { id: 'enterprise-quantum', nameFa: 'ماتریس کوانتومی سازمانی', nodes: 10000, descFa: 'کاوش تا ۱۰,۰۰۰ نود بدون وقفه' },
];

// ── Iranian carriers (with ASN + MTU clamping) ───────────────

export const IRAN_CARRIERS: IranCarrier[] = [
  { id: 'mci', nameFa: 'همراه اول', asn: 'AS44244', networkType: 'cellular', mtuClamp: 1280, detected: false, latencyMs: null, coveragePct: 96 },
  { id: 'irancell', nameFa: 'ایرانسل', asn: 'AS35897', networkType: 'cellular', mtuClamp: 1280, detected: false, latencyMs: null, coveragePct: 94 },
  { id: 'rightel', nameFa: 'رایتل', asn: 'AS57218', networkType: 'cellular', mtuClamp: 1280, detected: false, latencyMs: null, coveragePct: 78 },
  { id: 'tci', nameFa: 'مخابرات ایران', asn: 'AS58224', networkType: 'dsl', mtuClamp: 1360, detected: false, latencyMs: null, coveragePct: 88 },
  { id: 'shatel', nameFa: 'شاتل', asn: 'AS31549', networkType: 'dsl', mtuClamp: 1360, detected: false, latencyMs: null, coveragePct: 82 },
  { id: 'asiatech', nameFa: 'آسیاتک', asn: 'AS43754', networkType: 'fiber', mtuClamp: 1500, detected: false, latencyMs: null, coveragePct: 74 },
  { id: 'hiweb', nameFa: 'های‌وب', asn: 'AS44889', networkType: 'dsl', mtuClamp: 1360, detected: false, latencyMs: null, coveragePct: 70 },
  { id: 'mobinnet', nameFa: 'مبین‌نت', asn: 'AS48434', networkType: 'cellular', mtuClamp: 1280, detected: false, latencyMs: null, coveragePct: 58 },
  { id: 'zitel', nameFa: 'زیتل', asn: 'AS207256', networkType: 'fiber', mtuClamp: 1500, detected: false, latencyMs: null, coveragePct: 42 },
];

// ── Tunnel config application (UnifiedShield / MICAFP client) ──

export const IRAN_TUNNEL_KIND_MAP: Record<IranTunnelKind, { coreId: string; protocolId: string; labelFa: string }> = {
  'sing-box': { coreId: 'sing-box', protocolId: 'hysteria2-brutal', labelFa: 'تانل Sing-Box (Hysteria2)' },
  'xray': { coreId: 'xray-gfw', protocolId: 'shadowtls-mutator', labelFa: 'تانل Xray (VLESS Reality)' },
  'clash': { coreId: 'mahsang', protocolId: 'grpc-relay-mux', labelFa: 'تانل Clash Meta (gRPC Mux)' },
  'uris': { coreId: 'amneziavpn', protocolId: 'qns-v4', labelFa: 'تانل کوانتومی (QNS-v4)' },
  'naira': { coreId: 'naira', protocolId: 'naira-blackout', labelFa: 'تانل NAIRA (رله ملی + DNS کوانتومی)' },
  'quazar': { coreId: 'quazar', protocolId: 'quazar-quantum', labelFa: 'تانل QUAZAR (کوانتومی ضد DPI)' },
  'stormdns': { coreId: 'stormdns', protocolId: 'stormdns-dnstt', labelFa: 'تانل StormDNS (DNS قطعی)' },
};

export function pickIranTunnelKindForProtocol(protocolId: string): IranTunnelKind {
  if (protocolId === 'hysteria2-brutal') return 'sing-box';
  if (protocolId === 'shadowtls-mutator') return 'xray';
  if (protocolId === 'grpc-relay-mux') return 'clash';
  if (protocolId === 'qns-v4') return 'uris';
  if (protocolId === 'naira-blackout' || protocolId === 'dnstt') return 'naira';
  if (protocolId === 'quazar-quantum') return 'quazar';
  if (protocolId === 'stormdns-dnstt') return 'stormdns';
  if (protocolId === 'meshrelay-p2p') return 'naira';
  if (protocolId === 'stegostream-web') return 'xray';
  if (protocolId === 'rstguard-ttl') return 'xray';
  if (protocolId === 'chronoshield-temporal') return 'xray';
  if (protocolId === 'dualmux-l4') return 'quazar';
  if (protocolId === 'cottendns-fec') return 'stormdns';
  if (protocolId === 'masterdns-arq') return 'stormdns';
  if (protocolId === 'oobkeys-sms') return 'naira';
  if (protocolId === 'whitedns-utls') return 'stormdns';
  return 'xray';
}

// ── Pure helpers ─────────────────────────────────────────────

function clampEng(v: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, v));
}

function seededNoise(seed: number): number {
  // deterministic-ish pseudo-random in [-1, 1] from an integer seed
  const x = Math.sin(seed * 12.9898 + 78.233) * 43758.5453;
  return (x - Math.floor(x)) * 2 - 1;
}

export function buildInitialAutoScannerEngineState(): IranAutoScannerEngineState {
  return {
    categories: IRAN_SCAN_CATEGORIES.map((c) => ({ ...c })),
    activeCategoryId: null,
    scanMode: 'operator-adaptive',
    discoveryScale: 'adaptive-fast',
    discoveredNodes: 0,
    totalCategoryScans: 0,
    lastCategoryScanTs: null,
    backgroundScanActive: false,
    carriers: IRAN_CARRIERS.map((c) => ({ ...c })),
    activeCarrierId: null,
    autoPilot: {
      pathValidator: true,
      autoHealingWatchdog: true,
      zeroTouch: true,
      blackoutSolver: true,
      batterySaver: false,
      lastValidationTs: null,
      lastHealTs: null,
      validationEvents: 0,
      healEvents: 0,
    },
    gemini: {
      tlsSplitLength: 120,
      recommendedProtocolId: 'shadowtls-mutator',
      confidencePct: 84,
      reasoningFa: 'پاسخ‌های اپراتور نشان‌دهنده بازرسی اثر انگشت TLS است — طول شکستن ۱۲۰ بایت توصیه می‌شود',
      lastAnalysisTs: null,
    },
    localRL: IRAN_CARRIERS.map((c) => ({
      carrierId: c.id,
      attempts: 0,
      successes: 0,
      winRatePct: 0,
      avgSetupMs: 0,
      bestNodeId: null,
      tlsSplitLength: c.networkType === 'cellular' ? 1280 : 1360,
      lastUpdatedTs: null,
    })),
    exports: buildIranConfigExports(),
    autoApply: true,
    appliedTunnel: {
      active: false,
      kind: 'xray',
      coreId: 'xray-gfw',
      protocolId: 'shadowtls-mutator',
      labelFa: '—',
      appliedAt: null,
      autoApplied: false,
    },
    connectivity: {
      mode: 'full-tunnel',
      useTunnel: true,
      blackoutFallback: 'none',
      dnsTunnelEnabled: false,
      nationalRelayEnabled: false,
      recommendedProtocolId: 'shadowtls-mutator',
      recommendedCoreId: '',
      reasonFa: 'در انتظار اولین اسکن — تصمیم اتصال پس از اسکن اولیه مشخص می‌شود',
      decidedAt: null,
      auto: true,
    },
    autoDecide: true,
  };
}

/** Simulate a dynamic scan of one category (values always change between runs). */
export function runIranCategoryScan(
  categories: IranScanCategory[],
  categoryId: IranScanCategoryId,
  now: number,
  scanMode: IranScanModeId,
  activeCarrier: IranCarrier | null
): IranScanCategory[] {
  const mode = IRAN_SCAN_MODES.find((m) => m.id === scanMode) ?? IRAN_SCAN_MODES[0];
  const carrierBoost = activeCarrier ? 0.85 + (activeCarrier.coveragePct / 100) * 0.3 : 1;
  const fullMatrix = categoryId === 'full-matrix';

  return categories.map((cat) => {
    const isTarget = fullMatrix || cat.id === categoryId;
    if (!isTarget) return cat;
    // healthy nodes drift every scan — never static
    const health = clampEng(0.35 + carrierBoost * (0.5 + seededNoise(now + cat.id.length) * 0.28), 0.05, 0.99);
    const healthyNodes = Math.max(0, Math.round(cat.totalNodes * health));
    const bestLatencyMs = Math.max(9, Math.round(18 + (cat.id.charCodeAt(0) % 40) + Math.random() * 60));
    const durationBudget = mode.parallelism;
    void durationBudget;
    return {
      ...cat,
      healthyNodes,
      bestLatencyMs,
      status: 'ready',
      lastScanTs: now,
    };
  });
}

/** Auto-detect the active carrier (simulated reverse lookup of ASN). */
export function detectIranCarrier(carriers: IranCarrier[], now: number): IranCarrier[] {
  const detectedIndex = Math.floor(Math.random() * carriers.length);
  return carriers.map((c, i) => {
    const detected = i === detectedIndex;
    return {
      ...c,
      detected,
      latencyMs: detected ? Math.max(8, Math.round(14 + Math.random() * 26)) : null,
      coveragePct: detected ? clampEng(c.coveragePct + Math.round(seededNoise(now + i) * 4), 30, 99) : c.coveragePct,
    };
  });
}

/** Gemini-style local analysis: recommend TLS split length from carrier + filters. */
export function runIranGeminiAnalysis(
  carriers: IranCarrier[],
  activeCarrierId: string | null,
  dpiIntensity: number,
  now: number
): IranGeminiAnalysis {
  const carrier = carriers.find((c) => c.id === activeCarrierId) ?? null;
  const splitCandidates = [64, 96, 120, 160, 180, 256];
  const idx = Math.floor(clampEng(dpiIntensity / 20, 0, splitCandidates.length - 1));
  const tlsSplitLength = splitCandidates[idx] + (carrier?.networkType === 'cellular' ? 0 : 16);
  const protocols = ['shadowtls-mutator', 'qns-v4', 'hysteria2-brutal', 'grpc-relay-mux'];
  const recommendedProtocolId = protocols[Math.floor(clampEng(dpiIntensity / 30, 0, protocols.length - 1))];
  const confidencePct = Math.round(clampEng(72 + Math.random() * 24, 60, 99));
  const reasoningFa = carrier
    ? `بر اساس رفتار «${carrier.nameFa}» (${carrier.asn}) و شدت DPI «${dpiIntensity}٪»، طول شکستن هدر TLS «${tlsSplitLength} بایت» و پروتکل «${recommendedProtocolId}» توصیه می‌شود`
    : `بدون شناسایی اپراتور فعال — طول شکستن «${tlsSplitLength} بایت» و پروتکل «${recommendedProtocolId}» به‌صورت عمومی توصیه می‌شود`;
  return { tlsSplitLength, recommendedProtocolId, confidencePct, reasoningFa, lastAnalysisTs: now };
}

/** Update the on-device RL outcome record (kept locally, no telemetry out). */
export function recordIranLocalRL(
  records: IranLocalRLRecord[],
  carrierId: string,
  success: boolean,
  setupMs: number,
  bestNodeId: string | null,
  now: number
): IranLocalRLRecord[] {
  return records.map((r) => {
    if (r.carrierId !== carrierId) return r;
    const attempts = r.attempts + 1;
    const successes = r.successes + (success ? 1 : 0);
    const winRatePct = Math.round((successes / attempts) * 100);
    const avgSetupMs = Math.round((r.avgSetupMs * (attempts - 1) + setupMs) / attempts);
    return { ...r, attempts, successes, winRatePct, avgSetupMs, bestNodeId: bestNodeId ?? r.bestNodeId, lastUpdatedTs: now };
  });
}

// ── Core exporters ───────────────────────────────────────────

export function buildIranConfigExports(): IranConfigExports {
  const now = Date.now();
  const rawUris = [
    `vless://auto-scanner-${now}@fra1.example.net:443?encryption=none&security=reality&sni=www.speedtest.net&fp=chrome&pbk=<reality-public-key>&flow=xtls-rprx-vision#Iran-Frankfurt-${now}`,
    `hysteria2://auto-scanner-${now}@ams1.example.net:8443/?insecure=0&sni=www.cloudflare.com#Iran-Amsterdam-${now}`,
    `ss://YWVzLTI1Ni1nY206YXV0by1zY2FubmVy@dxb1.example.net:2053#Iran-Dubai-${now}`,
  ];
  return {
    singBox: JSON.stringify(
      {
        log: { level: 'warn' },
        inbounds: [
          { type: 'tun', tag: 'tun-in', mtu: 1280, auto_route: true, stack: 'mixed' },
          { type: 'mixed', tag: 'mixed-in', listen: '127.0.0.1', listen_port: 2080 },
        ],
        outbounds: [
          { type: 'hysteria2', tag: 'iran-ams', server: 'ams1.example.net', server_port: 8443, up_mbps: 40, down_mbps: 200, password: '<auto>' },
          { type: 'vless', tag: 'iran-fra', server: 'fra1.example.net', server_port: 443, uuid: '<uuid>', flow: 'xtls-rprx-vision', tls: { enabled: true, server_name: 'www.speedtest.net', utls: { enabled: true, fingerprint: 'chrome' } }, reality: { enabled: true, public_key: '<pk>', short_id: 'auto' } },
          { type: 'direct', tag: 'direct' },
        ],
        route: { rules: [{ geoip: ['ir'], outbound: 'direct' }], auto_detect_interface: true },
      },
      null,
      2
    ),
    xray: JSON.stringify(
      {
        inbounds: [{ tag: 'mixed', listen: '127.0.0.1', port: 2080, protocol: 'socks', settings: { udp: true } }],
        outbounds: [
          { protocol: 'vless', tag: 'iran-reality', settings: { vnext: [{ address: 'fra1.example.net', port: 443, users: [{ id: '<uuid>', flow: 'xtls-rprx-vision', encryption: 'none' }] }] }, streamSettings: { network: 'tcp', security: 'reality', realitySettings: { serverName: 'www.speedtest.net', fingerprint: 'chrome', publicKey: '<pk>', shortId: 'auto' }, tcpSettings: { header: { type: 'none' } } } },
          { protocol: 'freedom', tag: 'direct' },
        ],
      },
      null,
      2
    ),
    clashMeta: `# Mihomo (Clash Meta) — Iran Auto Scanner\nmixed-port: 7890\nallow-lan: false\nmode: rule\nlog-level: warning\n\nproxies:\n  - name: IRAN-FRA-Reality\n    type: vless\n    server: fra1.example.net\n    port: 443\n    uuid: <uuid>\n    network: tcp\n    tls: true\n    udp: true\n    flow: xtls-rprx-vision\n    servername: www.speedtest.net\n    reality-opts:\n      public-key: <pk>\n      short-id: auto\n    client-fingerprint: chrome\n\n  - name: IRAN-AMS-Hysteria2\n    type: hysteria2\n    server: ams1.example.net\n    port: 8443\n    password: <auto>\n    sni: www.cloudflare.com\n    up: 40\n    down: 200\n\nproxy-groups:\n  - name: AUTO\n    type: url-test\n    proxies: [IRAN-FRA-Reality, IRAN-AMS-Hysteria2]\n    url: http://www.gstatic.com/generate_204\n    interval: 300\n\nrules:\n  - GEOIP,IR,DIRECT\n  - MATCH,AUTO\n`,
    rawUris,
  };
}

export function buildIranDiagnosticReport(state: IranAutoScannerEngineState): string {
  const lines: string[] = [];
  lines.push('════════════════════════════════════════════');
  lines.push('  Iran Auto Scanner Engine — Diagnostic Report');
  lines.push('════════════════════════════════════════════');
  lines.push(`Exported: ${new Date().toLocaleString('fa-IR')}`);
  lines.push(`Scan mode: ${IRAN_SCAN_MODES.find((m) => m.id === state.scanMode)?.nameFa ?? state.scanMode}`);
  lines.push(`Discovery scale: ${IRAN_DISCOVERY_SCALES.find((s) => s.id === state.discoveryScale)?.nameFa ?? state.discoveryScale} (${state.discoveredNodes} nodes)`);
  lines.push(`Total category scans: ${state.totalCategoryScans}`);
  lines.push('');
  lines.push('— Auto-pilot —');
  lines.push(`  Path validator: ${state.autoPilot.pathValidator ? 'ON' : 'OFF'} (${state.autoPilot.validationEvents} events)`);
  lines.push(`  Auto-heal watchdog: ${state.autoPilot.autoHealingWatchdog ? 'ON' : 'OFF'} (${state.autoPilot.healEvents} events)`);
  lines.push(`  Zero-touch: ${state.autoPilot.zeroTouch ? 'ON' : 'OFF'} | Blackout solver: ${state.autoPilot.blackoutSolver ? 'ON' : 'OFF'}`);
  lines.push(`  Battery saver: ${state.autoPilot.batterySaver ? 'ON' : 'OFF'}`);
  lines.push('');
  lines.push('— 20 categories —');
  state.categories.forEach((c) => lines.push(`  ${c.nameFa}: ${c.healthyNodes}/${c.totalNodes} healthy | ${c.bestLatencyMs ?? '—'}ms | ${c.status}`));
  lines.push('');
  lines.push('— Carriers —');
  state.carriers.forEach((c) => lines.push(`  ${c.nameFa} (${c.asn}): ${c.detected ? `DETECTED ${c.latencyMs}ms` : '—'} | MTU ${c.mtuClamp}`));
  lines.push('');
  lines.push('— Gemini AI —');
  lines.push(`  TLS split: ${state.gemini.tlsSplitLength} bytes | protocol: ${state.gemini.recommendedProtocolId} | confidence: ${state.gemini.confidencePct}%`);
  lines.push(`  ${state.gemini.reasoningFa}`);
  lines.push('');
  lines.push('— Local RL (on-device, no telemetry out) —');
  state.localRL.forEach((r) => lines.push(`  ${r.carrierId}: ${r.successes}/${r.attempts} (${r.winRatePct}%) avg ${r.avgSetupMs}ms | tlsSplit ${r.tlsSplitLength}`));
  lines.push('');
  lines.push('— Active tunnel (applied to UnifiedShield client) —');
  lines.push(`  Auto-apply: ${state.autoApply ? 'ON' : 'OFF'}`);
  lines.push(`  Applied: ${state.appliedTunnel.active ? `${state.appliedTunnel.labelFa} (${state.appliedTunnel.coreId}/${state.appliedTunnel.protocolId})${state.appliedTunnel.autoApplied ? ' [AUTO]' : ' [MANUAL]'}` : 'none'}`);
  lines.push('════════════════════════════════════════════');
  return lines.join('\n');
}
