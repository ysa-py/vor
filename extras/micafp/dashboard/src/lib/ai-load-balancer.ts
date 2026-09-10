// ─────────────────────────────────────────────────────────────
// AI Load Balancer — adaptive traffic distribution across
// cores, protocols, tunneled routes and direct (no-tunnel) mode.
// Pure + testable; driven by live core health and the internal
// AI engine's scoring matrix.
// ─────────────────────────────────────────────────────────────

import type { CoreAdapter, LoadBalancerMode, CoreLoadShare, ProtocolLoadShare } from './unified-shield-types';

export function coreHealthScore(core: CoreAdapter): number {
  if (core.status === 'error' || core.health.blocked) return 0;
  const latencyScore = Math.max(0, 100 - core.health.latency);
  const lossScore = Math.max(0, 100 - core.health.packetLoss * 20);
  const dpiScore = Math.max(0, 100 - core.health.dpiExposure * 2);
  const dnsScore = core.health.dnsLeak ? 0 : 100;
  const blockScore = Math.max(0, 100 - core.blockEvents24h * 15);
  return Math.round(
    latencyScore * 0.3 + lossScore * 0.2 + dpiScore * 0.25 + dnsScore * 0.15 + blockScore * 0.1,
  );
}

export function pickBalancerMode(
  filterLevel: string | undefined,
  connected: boolean,
): LoadBalancerMode {
  if (!connected) return 'failover';
  if (
    filterLevel === 'extreme' ||
    filterLevel === 'international-cutoff' ||
    filterLevel === 'national-only'
  ) {
    return 'quantum-multipath';
  }
  if (filterLevel === 'dpi' || filterLevel === 'sni' || filterLevel === 'ip-block') {
    return 'adaptive';
  }
  return 'weighted';
}

const PROTOCOL_NAME_FA: Record<string, string> = {
  'qns-v4': 'استگانوگرافی کوانتومی QNS-v4',
  'shadowtls-mutator': 'شدوتی‌ال‌اس جهش‌یافته',
  'hysteria2-brutal': 'هیستریا۲ بروتال',
  'grpc-relay-mux': 'رله مالتی‌پلکس gRPC',
  'naira-blackout': 'رله قطعی نایرا',
  'quazar-quantum': 'کوانتومی کوزار',
  'stormdns-dnstt': 'تانل DNS استورم',
  'dnstt': 'تانلینگ DNS (DNSTT)',
  'whitedns-utls': 'DNS پاکیزه uTLS',
};

function pickProtocol(core: CoreAdapter, filterLevel: string | undefined): string {
  if (filterLevel === 'extreme') return 'qns-v4';
  if (filterLevel === 'international-cutoff' || filterLevel === 'national-only') {
    if (core.capabilities.includes('naira-blackout')) return 'naira-blackout';
    if (core.capabilities.includes('stormdns-dnstt')) return 'stormdns-dnstt';
    return 'qns-v4';
  }
  if (filterLevel === 'ip-block') return 'hysteria2-brutal';
  if (filterLevel === 'dpi' || filterLevel === 'sni') return 'shadowtls-mutator';
  return 'shadowtls-mutator';
}

export interface LoadShareResult {
  mode: LoadBalancerMode;
  cores: CoreLoadShare[];
  protocols: ProtocolLoadShare[];
  directSharePct: number;
  tunnelSharePct: number;
  recommendedCoreId: string;
  recommendedProtocolId: string;
  confidencePct: number;
  reasonFa: string;
  totalBandwidthMbps: number;
}

export function computeLoadShares(
  cores: CoreAdapter[],
  scoringMatrix: Record<string, number>,
  filterLevel: string | undefined,
  connected: boolean,
): LoadShareResult {
  const mode = pickBalancerMode(filterLevel, connected);

  const healthy = cores.filter((c) => c.status !== 'error' && !c.health.blocked);
  const pool = healthy.length ? healthy : cores;

  const scored = pool
    .map((core) => ({
      core,
      health: coreHealthScore(core),
      ai: scoringMatrix[core.id] ?? 0,
    }))
    .sort((a, b) => (b.ai || b.health) - (a.ai || a.health));

  const top = scored.slice(0, 6);
  const totalHealth = top.reduce((a, x) => a + Math.max(1, x.health), 0) || 1;

  const coresShares: CoreLoadShare[] = top.map((x, i) => {
    const weightPct = Math.max(1, Math.round((Math.max(1, x.health) / totalHealth) * 100));
    return {
      coreId: x.core.id,
      coreNameFa: x.core.nameFa,
      weightPct,
      trafficSharePct: weightPct,
      active: connected && (i === 0 || mode === 'quantum-multipath' || mode === 'adaptive'),
      healthScore: x.health,
    };
  });

  // Direct (no-tunnel) vs tunnel split — open/light networks can route directly.
  const directSharePct =
    filterLevel === 'open' ? 100 : filterLevel === 'light' ? 55 : 0;
  const tunnelSharePct = 100 - directSharePct;

  // Distribute tunnel share across the tunneled protocol routes.
  const tunnelTotal = coresShares.reduce((a, c) => a + c.weightPct, 0) || 1;
  const protocols: ProtocolLoadShare[] = coresShares
    .slice(0, 5)
    .map((cs) => {
      const core = cores.find((c) => c.id === cs.coreId) ?? cores[0];
      return {
        protocolId: pickProtocol(core, filterLevel),
        protocolNameFa: PROTOCOL_NAME_FA[pickProtocol(core, filterLevel)] ?? pickProtocol(core, filterLevel),
        coreId: cs.coreId,
        weightPct: Math.max(1, Math.round((cs.weightPct / tunnelTotal) * tunnelSharePct)),
        active: cs.active,
        route: 'tunnel' as const,
      };
    });

  if (directSharePct > 0) {
    protocols.unshift({
      protocolId: 'direct',
      protocolNameFa: 'بدون تانل (مستقیم)',
      coreId: '',
      weightPct: directSharePct,
      active: true,
      route: 'direct',
    });
  }

  const best = scored[0];
  const recommendedProtocolId = pickProtocol(best.core, filterLevel);
  const confidencePct = Math.min(99, Math.round(58 + best.health * 0.42));
  const totalBandwidthMbps = Math.round(
    top.reduce((a, x) => a + (x.core.health.bandwidth?.down ?? 0), 0),
  );

  const modeFa: Record<LoadBalancerMode, string> = {
    adaptive: 'تطبیقی (AI Adaptive)',
    weighted: 'وزنی (Weighted)',
    failover: 'فیل‌اور (Failover)',
    'quantum-multipath': 'کوانتومی چندمسیره',
  };

  const reasonFa =
    filterLevel === 'open' || filterLevel === 'light'
      ? `شبکه باز/سبک — ${directSharePct}٪ ترافیک بدون تانل و ${tunnelSharePct}٪ از طریق مسیر امن توزیع شد`
      : `حالت «${modeFa[mode]}» — ترافیک روی ${coresShares.length} هسته برتر و ${protocols.filter((p) => p.route === 'tunnel').length} پروتکل تونل متعادل شد (${confidencePct}٪ اطمینان AI)`;

  return {
    mode,
    cores: coresShares,
    protocols,
    directSharePct,
    tunnelSharePct,
    recommendedCoreId: best?.core.id ?? '',
    recommendedProtocolId,
    confidencePct,
    reasonFa,
    totalBandwidthMbps,
  };
}
