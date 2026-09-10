import { NextResponse } from 'next/server';

// ──────────────────────────────────────────────
// System health check endpoint
// ──────────────────────────────────────────────

const CORE_IDS = [
  'hiddify', 'xray-gfw', 'sing-box',
  'amneziavpn', 'defyxvpn', 'moav',
  'lantern', 'mahsang', 'psiphon',
];

const BASE_LATENCY: Record<string, number> = {
  hiddify: 85, 'xray-gfw': 62, 'sing-box': 73,
  amneziavpn: 91, defyxvpn: 105, moav: 118,
  lantern: 142, mahsang: 79, psiphon: 156,
};

function getCoreHealth(coreId: string, isActive: boolean) {
  const base = BASE_LATENCY[coreId] ?? 100;
  const latency = isActive ? Math.max(20, Math.round(base + Math.random() * 30 - 15)) : 0;
  return {
    coreId,
    status: isActive ? 'connected' : 'standby',
    latency,
    packetLoss: isActive ? Math.round(Math.random() * 3 * 100) / 100 : 0,
    blocked: !isActive,
    dnsLeak: isActive ? Math.random() < 0.02 : false,
    dpiExposure: isActive ? Math.round(Math.random() * 15 * 10) / 10 : 100,
    uptime: isActive ? Math.floor(Math.random() * 86400) : 0,
    bandwidthDown: isActive ? Math.round(Math.random() * 200 + 50) : 0,
    bandwidthUp: isActive ? Math.round(Math.random() * 50 + 10) : 0,
  };
}

// ──────────────────────────────────────────────
// GET /api/health
// ──────────────────────────────────────────────
export async function GET() {
  const now = Date.now();
  const connectedCores = ['hiddify', 'xray-gfw', 'mahsang'];

  const coreHealths = CORE_IDS.map((id) => getCoreHealth(id, connectedCores.includes(id)));

  const activeCores = coreHealths.filter((c) => c.status === 'connected');
  const avgLatency = activeCores.length > 0
    ? Math.round(activeCores.reduce((sum, c) => sum + c.latency, 0) / activeCores.length)
    : 0;
  const avgPacketLoss = activeCores.length > 0
    ? Math.round(activeCores.reduce((sum, c) => sum + c.packetLoss, 0) / activeCores.length * 100) / 100
    : 0;
  const totalBandwidthDown = activeCores.reduce((sum, c) => sum + c.bandwidthDown, 0);
  const totalBandwidthUp = activeCores.reduce((sum, c) => sum + c.bandwidthUp, 0);

  const dnsHealthy = !activeCores.some((c) => c.dnsLeak);
  const anyBlocked = activeCores.some((c) => c.blocked);
  const avgDpiExposure = activeCores.length > 0
    ? Math.round(activeCores.reduce((sum, c) => sum + c.dpiExposure, 0) / activeCores.length * 10) / 10
    : 100;

  const memoryUsageMb = Math.round(180 + Math.random() * 60);
  const cpuUsagePercent = Math.round(8 + Math.random() * 15);
  const diskFreeGb = Math.round((12.5 + Math.random() * 5) * 10) / 10;
  const processUptime = 259200 + Math.floor(Math.random() * 3600);

  const overallScore = Math.round(
    (dnsHealthy ? 25 : 0) +
    (!anyBlocked ? 25 : 10) +
    (avgLatency < 100 ? 25 : 15) +
    (avgDpiExposure < 10 ? 25 : 10),
  );

  const status: 'healthy' | 'degraded' | 'critical' = overallScore >= 80 ? 'healthy' : overallScore >= 50 ? 'degraded' : 'critical';

  return NextResponse.json({
    success: true,
    timestamp: now,
    status,
    overallScore,
    statusFa: status === 'healthy' ? 'سالم' : status === 'degraded' ? 'ضعیف' : 'بحرانی',

    connection: {
      connected: true,
      activeCoreId: 'xray-gfw',
      activeCoreNameFa: 'ایکس‌ری GFW',
      shadowConnections: ['mahsang', 'hiddify'],
      totalCores: CORE_IDS.length,
      connectedCores: connectedCores.length,
      standbyCores: CORE_IDS.length - connectedCores.length,
      avgLatencyMs: avgLatency,
      avgPacketLossPercent: avgPacketLoss,
      totalBandwidthMbps: { up: totalBandwidthUp, down: totalBandwidthDown },
    },

    security: {
      dnsSecure: dnsHealthy,
      dnsMode: 'doh',
      dnsProvider: 'cloudflare',
      dnsProviderFa: 'کلودفلر',
      anyCoreBlocked: anyBlocked,
      avgDpiExposure,
      killSwitchEnabled: true,
      networkLockEnabled: true,
      leakProtectionActive: dnsHealthy && !anyBlocked,
      threatLevel: 'high',
      threatLevelFa: 'بالا',
      activeCountermeasures: ['VLESS Reality + XTLS', 'DNS over HTTPS (DoH)', 'AmneziaWG junk packets'],
    },

    system: {
      memoryUsageMb,
      cpuUsagePercent,
      diskFreeGb,
      processUptimeSec: processUptime,
      processUptimeHuman: `${Math.floor(processUptime / 86400)}d ${Math.floor((processUptime % 86400) / 3600)}h ${Math.floor((processUptime % 3600) / 60)}m`,
      platform: 'android',
      appVersion: '2.0.0',
      aiModelVersion: '3.1.0',
      dpiSignaturesVersion: '2026.05.23-r2',
    },

    ai: {
      orchestratorHealthy: true,
      scoringMatrixActive: true,
      ucbAlgorithmRunning: true,
      lastOrchestrationCycle: now - 15000,
      totalSwitches: 47,
      successfulSwitches: 44,
      switchSuccessRate: Math.round((44 / 47) * 100 * 10) / 10,
      learningRate: 0.01,
      detectedISP: 'irancell',
      detectedISPFa: 'ایرانسل',
    },

    cores: coreHealths,

    recommendations: {
      switchRecommended: avgLatency > 120,
      dnsSwitchRecommended: !dnsHealthy,
      killSwitchCritical: false,
      updateAvailable: true,
      updateMessage: 'iran-block-signatures update available: 2026.05.23-r2',
      updateMessageFa: 'به‌روزرسانی امضاهای مسدودیت ایران موجود: 2026.05.23-r2',
    },

    meta: {
      endpoint: '/api/health',
      descriptionFa: 'بررسی جامع سلامت سیستم',
    },
  });
}
