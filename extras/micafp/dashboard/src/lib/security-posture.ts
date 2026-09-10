// ─────────────────────────────────────────────────────────────
// Security Posture Engine — single source of truth.
// Every subsystem row and the aggregate score are derived here from the
// same inputs so the UI can never show a row that contradicts the total.
// ─────────────────────────────────────────────────────────────

import type { ThreatEntry } from './unified-shield-types';

export type PostureRowStatus = 'safe' | 'warning' | 'severe' | 'unknown';

export interface PostureCheck {
  id: string;
  labelFa: string;
  status: PostureRowStatus;
  weight: number;
  detailFa?: string;
}

export interface SecurityPosture {
  score: number;
  grade: 'A+' | 'A' | 'B' | 'C' | 'D';
  okCount: number;
  checks: PostureCheck[];
  threatStatus: PostureRowStatus;
  threatDetailFa: string;
}

export interface SecurityPostureInput {
  killSwitchEnabled: boolean;
  networkLock: boolean;
  dnsMode: string; // 'doh' | 'dot' | 'plain'
  firewallMode: string; // 'strict' | 'balanced' | 'permissive' | ...
  firewallLearningEnabled: boolean;
  totalBlocked: number;
  connected: boolean;
  threats: ThreatEntry[] | null | undefined;
  blockedDomainsCount: number;
  dpiIntensity: number; // 0..100 live DPI intensity from the Iran scanner
  filterScore: number; // 0..100 filtering severity / anomaly score
}

const SEVERITY_ORDER = ['low', 'medium', 'high', 'critical'] as const;

/**
 * Derive the threat posture status from the threat-monitoring module.
 * Only *active* (unmitigated) threats count. A missing signal (null/undefined)
 * becomes `unknown` — never `severe` — so a lost feed can't masquerade as an
 * active attack.
 */
export function deriveThreatStatus(threats: ThreatEntry[] | null | undefined): PostureRowStatus {
  if (threats == null) return 'unknown';
  const active = threats.filter((t) => !t.mitigated);
  if (active.length === 0) return 'safe';

  let maxSev: ThreatEntry['severity'] = 'low';
  for (const t of active) {
    if (SEVERITY_ORDER.indexOf(t.severity) > SEVERITY_ORDER.indexOf(maxSev)) {
      maxSev = t.severity;
    }
  }
  if (maxSev === 'critical' || maxSev === 'high') return 'severe';
  // low or medium unmitigated threat → warning
  return 'warning';
}

function buildThreatDetail(input: SecurityPostureInput, status: PostureRowStatus): string {
  if (status === 'unknown') {
    return 'منبع پایش تهدید در دسترس نیست — سیگنال گمشده هرگز به‌عنوان تهدید فعال نمایش داده نمی‌شود';
  }
  const active = (input.threats ?? []).filter((t) => !t.mitigated);
  if (status === 'safe') {
    return `هیچ تهدید فعال شناسایی نشده — ${input.blockedDomainsCount.toLocaleString('fa-IR')} دامنه مسدودشده، شدت DPI ${input.dpiIntensity}٪`;
  }
  const names = active.map((t) => t.typeFa).join('، ');
  const signature = input.dpiIntensity > 55 ? 'امضای DPI شناسایی شد' : 'بدون تطبیق امضای DPI جدید';
  return `${names} | IP/دامنه مسدود: ${input.blockedDomainsCount.toLocaleString('fa-IR')} | ${signature} | امتیاز آنومالی: ${input.filterScore}٪`;
}

export function computeSecurityPosture(input: SecurityPostureInput): SecurityPosture {
  const threatStatus = deriveThreatStatus(input.threats);
  const threatDetailFa = buildThreatDetail(input, threatStatus);

  const checks: PostureCheck[] = [
    { id: 'kill-switch', labelFa: 'کشتن سوئیچ', status: input.killSwitchEnabled ? 'safe' : 'warning', weight: 18 },
    { id: 'network-lock', labelFa: 'قفل شبکه', status: input.networkLock ? 'safe' : 'warning', weight: 14 },
    { id: 'dns', labelFa: 'DNS امن', status: input.dnsMode !== 'plain' ? 'safe' : 'warning', weight: 12 },
    { id: 'firewall', labelFa: 'فایروال هوشمند', status: input.firewallMode !== 'permissive' ? 'safe' : 'warning', weight: 14 },
    { id: 'threat', labelFa: 'سطح تهدید', status: threatStatus, weight: 18, detailFa: threatDetailFa },
    { id: 'connection', labelFa: 'اتصال امن', status: input.connected ? 'safe' : 'warning', weight: 12 },
    { id: 'learning', labelFa: 'یادگیری هوشمند', status: input.firewallLearningEnabled ? 'safe' : 'warning', weight: 6 },
    { id: 'blocking', labelFa: 'مسدودسازی فعال', status: input.totalBlocked > 0 ? 'safe' : 'warning', weight: 6 },
  ];

  const factor: Record<PostureRowStatus, number> = { safe: 1, warning: 0.5, severe: 0, unknown: 0.5 };
  const score = Math.round(checks.reduce((sum, c) => sum + c.weight * factor[c.status], 0));
  const okCount = checks.filter((c) => c.status === 'safe').length;
  const grade: SecurityPosture['grade'] =
    score >= 90 ? 'A+' : score >= 80 ? 'A' : score >= 65 ? 'B' : score >= 50 ? 'C' : 'D';

  return { score, grade, okCount, checks, threatStatus, threatDetailFa };
}
