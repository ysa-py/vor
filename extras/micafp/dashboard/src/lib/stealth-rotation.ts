// ─────────────────────────────────────────────────────────────
// Stealth Rotation Engine — AI-coordinated, anti-fingerprint
// switching of cores and protocols so the filter cannot identify
// (and therefore block) a stable connection signature.
//
// Pure + testable. Driven by live core health, DPI exposure,
// block history and the internal AI engine's scoring matrix.
// ─────────────────────────────────────────────────────────────

import type { CoreAdapter, NovelEnterpriseProtocol } from './unified-shield-types';

export interface RotationHistoryEntry {
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

export interface StealthRotationPlan {
  shouldRotate: boolean;
  nextCoreId: string;
  nextProtocolId: string;
  fingerprintRisk: number;
  techniqueFa: string;
  reasonFa: string;
  nextScheduledAt: number;
  jitterMs: number;
}

export const FINGERPRINT_ROTATE_THRESHOLD = 62;

const ROTATION_TECHNIQUES_FA = [
  'جهش پورت + چرخش SNI',
  'تکه‌سازی ClientHello',
  'تعویض JA4 + پدینگ تصادفی',
  'پرش پروتکل (Protocol Hop)',
  'تغییر MTU + بافر TLS',
  'چرخش اثر انگشت TLS',
];

// Risk that the active core+protocol combination is being fingerprinted.
export function computeFingerprintRisk(
  core: CoreAdapter,
  protocol: NovelEnterpriseProtocol | undefined,
  activeSinceTs: number,
  now: number,
): number {
  if (!core || core.status === 'error' || core.health.blocked) return 100;

  const ageMs = Math.max(0, now - activeSinceTs);
  const ageRisk = Math.min(30, (ageMs / (5 * 60_000)) * 30); // saturates after ~5 min

  const blockRisk = Math.min(30, (core.blockEvents24h ?? 0) * 6);
  const dpiRisk = Math.min(25, (core.health.dpiExposure ?? 0) * 0.5);
  const dnsRisk = core.health.dnsLeak ? 15 : 0;

  const protocolRisk = protocol
    ? Math.max(0, (100 - (protocol.dpiResistancePercent ?? 70)) * 0.35)
    : 20;

  return Math.round(Math.min(100, ageRisk + blockRisk + dpiRisk + dnsRisk + protocolRisk));
}

function jitterBetween(minMs: number, maxMs: number, rand: () => number = Math.random): number {
  return Math.round(minMs + rand() * (maxMs - minMs));
}

function pickLeastFingerprintedCore(
  cores: CoreAdapter[],
  scoringMatrix: Record<string, number>,
  avoidIds: Set<string>,
  activeSinceTs: number,
  now: number,
): CoreAdapter | undefined {
  const candidates = cores.filter(
    (c) => c.status !== 'error' && !c.health.blocked && !avoidIds.has(c.id),
  );
  const pool = candidates.length ? candidates : cores.filter((c) => c.status !== 'error' && !c.health.blocked);

  return pool
    .map((core) => ({
      core,
      score:
        (scoringMatrix[core.id] ?? 0) +
        (100 - computeFingerprintRisk(core, undefined, activeSinceTs, now)) +
        // small latency preference
        Math.max(0, 50 - core.health.latency),
    }))
    .sort((a, b) => b.score - a.score)[0]?.core;
}

function pickLeastFingerprintedProtocol(
  protocols: NovelEnterpriseProtocol[],
  avoidId: string,
  rand: () => number = Math.random,
): string {
  const candidates = protocols.filter((p) => p.id !== avoidId && p.status !== 'testing');
  const pool = candidates.length ? candidates : protocols;
  if (!pool.length) return avoidId;

  return pool
    .map((p) => ({
      id: p.id,
      score:
        (p.dpiResistancePercent ?? 70) +
        (p.stealthScore ?? 0) * 0.5 +
        (p.packetLossRecoveryPercent ?? 0) * 0.25 +
        rand() * 12, // small stochastic tie-break so the pattern is non-deterministic
    }))
    .sort((a, b) => b.score - a.score)[0].id;
}

export function buildStealthRotationPlan(input: {
  cores: CoreAdapter[];
  protocols: NovelEnterpriseProtocol[];
  scoringMatrix: Record<string, number>;
  activeCoreId: string;
  activeProtocolId: string;
  activeSinceTs: number;
  now: number;
  lastRotationAt: number;
  recentCoreIds: string[];
  rand?: () => number;
}): StealthRotationPlan {
  const {
    cores,
    protocols,
    scoringMatrix,
    activeCoreId,
    activeProtocolId,
    activeSinceTs,
    now,
    recentCoreIds,
  } = input;
  const rand = input.rand ?? Math.random;

  const activeCore = cores.find((c) => c.id === activeCoreId);
  const activeProtocol = protocols.find((p) => p.id === activeProtocolId);

  const risk = activeCore
    ? computeFingerprintRisk(activeCore, activeProtocol, activeSinceTs, now)
    : 100;

  // Avoid re-picking cores we just used, to break recognisable patterns.
  const avoidIds = new Set<string>(recentCoreIds.slice(0, 3));
  avoidIds.add(activeCoreId);

  const nextCore = pickLeastFingerprintedCore(cores, scoringMatrix, avoidIds, activeSinceTs, now);
  const nextProtocolId = pickLeastFingerprintedProtocol(
    protocols,
    activeProtocolId,
    rand,
  );

  const jitterMs = jitterBetween(20_000, 45_000, rand);
  const nextScheduledAt = now + jitterMs;

  const forced = !activeCore || activeCore.status === 'error' || activeCore.health.blocked;
  const shouldRotate = forced || risk >= FINGERPRINT_ROTATE_THRESHOLD;

  const techniqueFa = ROTATION_TECHNIQUES_FA[
    Math.floor(rand() * ROTATION_TECHNIQUES_FA.length)
  ];

  const reasonFa = forced
    ? 'هسته فعال از کار افتاده یا مسدود شده — چرخش اضطراری خودکار برای حفظ اتصال'
    : `ریسک اثر انگشت به ${risk}٪ رسید (آستانه ${FINGERPRINT_ROTATE_THRESHOLD}٪) — هسته و پروتکل قبل از شناسایی توسط فیلتر تعویض شد`;

  return {
    shouldRotate,
    nextCoreId: nextCore?.id ?? activeCoreId,
    nextProtocolId,
    fingerprintRisk: risk,
    techniqueFa,
    reasonFa,
    nextScheduledAt,
    jitterMs,
  };
}
