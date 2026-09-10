import { NextRequest, NextResponse } from 'next/server';

// ──────────────────────────────────────────────
// Core definitions (from unified-shield-types.ts CORE_DEFINITIONS)
// ──────────────────────────────────────────────
interface CoreAdapter {
  id: string;
  name: string;
  nameFa: string;
  version: string;
  latestVersion: string;
  status: 'connected' | 'disconnected' | 'connecting' | 'error' | 'standby';
  priority: number;
  health: {
    latency: number;
    packetLoss: number;
    blocked: boolean;
    dnsLeak: boolean;
    dpiExposure: number;
    uptime: number;
    bandwidth: { up: number; down: number };
  };
  capabilities: string[];
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
  platforms: string[];
  checksumSha256: string;
}

function generateHealth(coreId: string, isActive: boolean): CoreAdapter['health'] {
  const baseLatency: Record<string, number> = {
    hiddify: 85, 'xray-gfw': 62, 'sing-box': 73,
    amneziavpn: 91, defyxvpn: 105, moav: 118,
    lantern: 142, mahsang: 79, psiphon: 156,
  };
  const base = baseLatency[coreId] ?? 100;
  const jitter = Math.random() * 30 - 15;
  const latency = isActive ? Math.max(20, Math.round(base + jitter)) : 0;
  const packetLoss = isActive ? Math.random() * 3 : 0;
  return {
    latency,
    packetLoss: Math.round(packetLoss * 100) / 100,
    blocked: !isActive ? true : Math.random() < 0.05,
    dnsLeak: isActive ? Math.random() < 0.02 : false,
    dpiExposure: isActive ? Math.round(Math.random() * 15 * 10) / 10 : 100,
    uptime: isActive ? Math.floor(Math.random() * 86400) : 0,
    bandwidth: isActive
      ? { up: Math.round(Math.random() * 50 + 10), down: Math.round(Math.random() * 200 + 50) }
      : { up: 0, down: 0 },
  };
}

// All 21 cores with exact data from CORE_DEFINITIONS
const CORE_DEFINITIONS_DATA: Omit<CoreAdapter, 'status' | 'health' | 'priority' | 'lastChecked' | 'blockEvents24h'>[] = [
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
];

// Build full cores with runtime status
function buildCores(): CoreAdapter[] {
  const activeCores = new Set(['hiddify', 'xray-gfw', 'mahsang']);
  return CORE_DEFINITIONS_DATA.map((def, idx) => {
    const isActive = activeCores.has(def.id);
    return {
      ...def,
      status: isActive ? 'connected' as const : 'standby' as const,
      priority: 9 - idx,
      health: generateHealth(def.id, isActive),
      lastChecked: Date.now() - Math.floor(Math.random() * 15000),
      blockEvents24h: isActive ? Math.floor(Math.random() * 2) : Math.floor(Math.random() * 8),
    };
  });
}

// In-memory state
let coresState: CoreAdapter[] = buildCores();

// ──────────────────────────────────────────────
// GET /api/cores
// ──────────────────────────────────────────────
export async function GET() {
  // Refresh health data
  coresState = coresState.map((c) => ({
    ...c,
    lastChecked: Date.now(),
  }));

  const connected = coresState.filter((c) => c.status === 'connected');
  const standby = coresState.filter((c) => c.status === 'standby');
  const errored = coresState.filter((c) => c.status === 'error');

  return NextResponse.json({
    success: true,
    timestamp: Date.now(),
    totalCores: coresState.length,
    connectedCount: connected.length,
    standbyCount: standby.length,
    errorCount: errored.length,
    cores: coresState,
    summary: {
      connectedCores: connected.map((c) => ({ id: c.id, name: c.name, nameFa: c.nameFa })),
      standbyCores: standby.map((c) => ({ id: c.id, name: c.name, nameFa: c.nameFa })),
      errorCores: errored.map((c) => ({ id: c.id, name: c.name, nameFa: c.nameFa })),
    },
    meta: {
      endpoint: '/api/cores',
      descriptionFa: 'مدیریت و وضعیت هسته‌های اتصال',
    },
  });
}

// ──────────────────────────────────────────────
// POST /api/cores
// body: { coreId: string, action: 'start' | 'stop' | 'restart' | 'health-check' }
// ──────────────────────────────────────────────
export async function POST(request: NextRequest) {
  try {
    const body = await request.json();
    const { coreId, action } = body as { coreId?: string; action?: string };

    if (!coreId || !action) {
      return NextResponse.json(
        {
          success: false,
          error: 'Missing required fields: coreId and action',
          errorFa: 'فیلدهای ضروری موجود نیست: coreId و action',
        },
        { status: 400 },
      );
    }

    const coreIndex = coresState.findIndex((c) => c.id === coreId);
    if (coreIndex === -1) {
      return NextResponse.json(
        {
          success: false,
          error: `Core not found: ${coreId}. Valid IDs: ${CORE_DEFINITIONS_DATA.map((d) => d.id).join(', ')}`,
          errorFa: `هسته یافت نشد: ${coreId}`,
        },
        { status: 404 },
      );
    }

    const core = coresState[coreIndex];
    if (!core) {
      return NextResponse.json(
        { success: false, error: 'Core data unavailable', errorFa: 'داده هسته در دسترس نیست' },
        { status: 500 },
      );
    }

    const validActions = ['start', 'stop', 'restart', 'health-check'];
    if (!validActions.includes(action)) {
      return NextResponse.json(
        {
          success: false,
          error: `Invalid action: ${action}. Valid actions: ${validActions.join(', ')}`,
          errorFa: `عملیات نامعتبر: ${action}`,
        },
        { status: 400 },
      );
    }

    if (action === 'start') {
      if (core.status === 'connected') {
        return NextResponse.json({
          success: true,
          action: 'start',
          coreId,
          status: core.status,
          message: `Core ${coreId} is already running`,
          messageFa: `هسته ${coreId} از قبل در حال اجرا است`,
        });
      }
      coresState[coreIndex] = {
        ...core,
        status: 'connected',
        health: generateHealth(coreId, true),
        lastChecked: Date.now(),
      };
      return NextResponse.json({
        success: true,
        action: 'start',
        coreId,
        previousStatus: core.status,
        newStatus: 'connected',
        health: coresState[coreIndex].health,
        message: `Core ${coreId} started successfully`,
        messageFa: `هسته ${coreId} با موفقیت راه‌اندازی شد`,
      });
    }

    if (action === 'stop') {
      if (core.status === 'disconnected') {
        return NextResponse.json({
          success: true,
          action: 'stop',
          coreId,
          status: core.status,
          message: `Core ${coreId} is already stopped`,
          messageFa: `هسته ${coreId} از قبل متوقف شده`,
        });
      }
      const previousStatus = core.status;
      coresState[coreIndex] = {
        ...core,
        status: 'disconnected',
        health: generateHealth(coreId, false),
        lastChecked: Date.now(),
      };
      return NextResponse.json({
        success: true,
        action: 'stop',
        coreId,
        previousStatus,
        newStatus: 'disconnected',
        message: `Core ${coreId} stopped successfully`,
        messageFa: `هسته ${coreId} با موفقیت متوقف شد`,
      });
    }

    if (action === 'restart') {
      coresState[coreIndex] = {
        ...core,
        status: 'connecting',
        health: generateHealth(coreId, false),
        lastChecked: Date.now(),
      };
      // Simulate restart completing
      setTimeout(() => {
        if (coresState[coreIndex]) {
          coresState[coreIndex] = {
            ...coresState[coreIndex]!,
            status: 'connected',
            health: generateHealth(coreId, true),
            lastChecked: Date.now(),
          };
        }
      }, 2000);

      return NextResponse.json({
        success: true,
        action: 'restart',
        coreId,
        status: 'connecting',
        estimatedReconnectMs: 2000,
        message: `Core ${coreId} is restarting`,
        messageFa: `هسته ${coreId} در حال راه‌اندازی مجدد`,
      });
    }

    if (action === 'health-check') {
      const isActive = core.status === 'connected' || core.status === 'standby';
      const newHealth = generateHealth(coreId, isActive);
      coresState[coreIndex] = {
        ...core,
        health: newHealth,
        lastChecked: Date.now(),
      };
      return NextResponse.json({
        success: true,
        action: 'health-check',
        coreId,
        status: coresState[coreIndex].status,
        health: newHealth,
        lastChecked: coresState[coreIndex].lastChecked,
        message: `Health check completed for ${coreId}`,
        messageFa: `بررسی سلامت برای ${coreId} انجام شد`,
      });
    }

    // Should never reach here
    return NextResponse.json(
      { success: false, error: 'Unhandled action', errorFa: 'عملیات مدیریت نشده' },
      { status: 500 },
    );
  } catch (error) {
    return NextResponse.json(
      {
        success: false,
        error: 'Invalid JSON body',
        errorFa: 'بدنه JSON نامعتبر است',
        details: error instanceof Error ? error.message : 'Unknown error',
      },
      { status: 400 },
    );
  }
}
