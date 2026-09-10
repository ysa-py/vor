// ─────────────────────────────────────────────────────────────
// AI Connectivity Decision — decides, from live scanner output,
// whether to use a full tunnel, a split tunnel, or go direct,
// and — when Iran's international internet is cut — whether to
// fall back to a national reverse relay, DNS tunneling, or both.
// ─────────────────────────────────────────────────────────────

import type { IranFilterLevel } from './unified-shield-types';

export type TunnelMode = 'direct' | 'split-tunnel' | 'full-tunnel';
export type BlackoutFallback = 'none' | 'national-reverse-relay' | 'dns-tunnel' | 'hybrid-multipath';

export interface ConnectivityEnvironmentInput {
  filterLevel: IranFilterLevel;
  internationalReach: number; // 0..100
  nationalReach: number; // 0..100
  dpiIntensity: number; // 0..100
  dnsOk: boolean;
}

export interface AIConnectivityDecision {
  mode: TunnelMode;
  useTunnel: boolean;
  blackoutFallback: BlackoutFallback;
  dnsTunnelEnabled: boolean;
  nationalRelayEnabled: boolean;
  recommendedProtocolId: string;
  recommendedCoreId: string;
  reasonFa: string;
  decidedAt: number | null;
  auto: boolean;
}

export const TUNNEL_MODE_LABEL: Record<TunnelMode, string> = {
  'direct': 'بدون تانل (مستقیم)',
  'split-tunnel': 'تانل تفکیکی (Split Tunnel)',
  'full-tunnel': 'تانل کامل',
};

export const BLACKOUT_FALLBACK_LABEL: Record<BlackoutFallback, string> = {
  'none': '—',
  'national-reverse-relay': 'رله معکوس ملی (NIN)',
  'dns-tunnel': 'تانلینگ DNS (DNSTT / EDNS0)',
  'hybrid-multipath': 'ترکیبی چندمسیره (رله ملی + DNS)',
};

function buildReason(
  level: IranFilterLevel,
  mode: TunnelMode,
  fallback: BlackoutFallback,
  env: ConnectivityEnvironmentInput,
): string {
  if (level === 'open') {
    return 'اینترنت باز است — نیازی به تانل نیست و اتصال مستقیم برای کمترین تأخیر انتخاب شد';
  }
  if (level === 'light') {
    return 'فیلترینگ سبک — تانل تفکیکی برای مسیرهای مسدودشده و اتصال مستقیم برای بقیه ترافیک';
  }
  if (level === 'international-cutoff' || level === 'national-only') {
    if (fallback === 'dns-tunnel') {
      return 'اینترنت بین‌الملل قطع است ولی DNS سالم است — تانلینگ DNS (بافر ۴۰۹۶ بایتی) برای عبور از خاموشی انتخاب شد';
    }
    if (fallback === 'national-reverse-relay') {
      return 'اینترنت بین‌الملل قطع است — رله معکوس از سرویس‌های داخلی (آپارات/آروان/تلوبیون) برای خروج منطقه‌ای فعال شد';
    }
    return 'اینترنت بین‌الملل قطع است — مسیر ترکیبی چندمسیره (رله ملی + تانل DNS) به‌صورت موازی فعال شد';
  }
  if (level === 'extreme') {
    return `فیلترینگ چندلایه فوق‌سخت (DPI ${env.dpiIntensity}٪) — تانل کامل کوانتومی با مسیریابی چندمسیره`;
  }
  return `سطح «${level}» شناسایی شد — تانل کامل با استتار پکت برای دور زدن فیلترینگ انتخاب شد`;
}

export function decideConnectivity(
  env: ConnectivityEnvironmentInput,
  now: number,
  auto = true,
): AIConnectivityDecision {
  let mode: TunnelMode = 'full-tunnel';
  let blackoutFallback: BlackoutFallback = 'none';
  let recommendedProtocolId = 'shadowtls-mutator';

  switch (env.filterLevel) {
    case 'open':
      mode = 'direct';
      recommendedProtocolId = '';
      break;
    case 'light':
      mode = 'split-tunnel';
      recommendedProtocolId = 'shadowtls-mutator';
      break;
    case 'dpi':
    case 'sni':
      mode = 'full-tunnel';
      recommendedProtocolId = 'shadowtls-mutator';
      break;
    case 'ip-block':
      mode = 'full-tunnel';
      recommendedProtocolId = 'hysteria2-brutal';
      break;
    case 'extreme':
      mode = 'full-tunnel';
      recommendedProtocolId = 'qns-v4';
      break;
    case 'international-cutoff':
    case 'national-only':
      mode = 'full-tunnel';
      recommendedProtocolId = 'qns-v4';
      const natOk = env.nationalReach >= 60;
      const dnsOk = env.dnsOk;
      if (natOk && dnsOk) {
        blackoutFallback = 'hybrid-multipath';
        recommendedProtocolId = 'qns-v4';
      } else if (natOk) {
        blackoutFallback = 'national-reverse-relay';
        recommendedProtocolId = 'qns-v4';
      } else if (dnsOk) {
        blackoutFallback = 'dns-tunnel';
        recommendedProtocolId = 'dnstt';
      } else {
        blackoutFallback = 'hybrid-multipath';
        recommendedProtocolId = 'qns-v4';
      }
      break;
  }

  // Which core should the AI actually connect to for this route?
  let recommendedCoreId = '';
  switch (env.filterLevel) {
    case 'open':
      recommendedCoreId = '';
      break;
    case 'light':
      recommendedCoreId = 'hiddify';
      break;
    case 'dpi':
    case 'sni':
      recommendedCoreId = 'mahsang';
      break;
    case 'ip-block':
      recommendedCoreId = 'sing-box';
      break;
    case 'extreme':
      recommendedCoreId = 'quazar';
      break;
    case 'international-cutoff':
    case 'national-only':
      recommendedCoreId =
        blackoutFallback === 'dns-tunnel'
          ? 'stormdns'
          : blackoutFallback === 'national-reverse-relay'
            ? 'naira'
            : 'quazar'; // hybrid-multipath
      break;
  }

  const dnsTunnelEnabled = blackoutFallback === 'dns-tunnel' || blackoutFallback === 'hybrid-multipath';
  const nationalRelayEnabled = blackoutFallback === 'national-reverse-relay' || blackoutFallback === 'hybrid-multipath';
  const useTunnel = mode !== 'direct';

  return {
    mode,
    useTunnel,
    blackoutFallback,
    dnsTunnelEnabled,
    nationalRelayEnabled,
    recommendedProtocolId,
    recommendedCoreId,
    reasonFa: buildReason(env.filterLevel, mode, blackoutFallback, env),
    decidedAt: now,
    auto,
  };
}
