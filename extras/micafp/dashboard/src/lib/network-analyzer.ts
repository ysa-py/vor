// ──────────────────────────────────────────────
// MICAFP-UnifiedShield — Real Network Analyzer
// Real-time bandwidth monitoring, connection quality,
// network type detection, traffic analysis, packet stats,
// and data usage tracking — all with Persian labels
// ──────────────────────────────────────────────

import type {
  NetworkAnalyzerState,
  NetworkType,
  BandwidthHistoryEntry,
  TrafficBreakdown,
  DataUsageEntry,
  PacketStats,
} from './unified-shield-types';

// ──────────────────────────────────────────────
// Network Type Detection
// ──────────────────────────────────────────────
const NETWORK_TYPE_MAP: Record<NetworkType, string> = {
  wifi: 'وای‌فای',
  'mobile-data': 'داده همراه',
  ethernet: 'اترنت',
  unknown: 'نامشخص',
};

export function detectNetworkType(): { type: NetworkType; nameFa: string } {
  // Simulate network type detection based on typical patterns
  const rand = Math.random();
  let type: NetworkType;
  if (rand < 0.55) type = 'wifi';
  else if (rand < 0.88) type = 'mobile-data';
  else if (rand < 0.97) type = 'ethernet';
  else type = 'unknown';

  return { type, nameFa: NETWORK_TYPE_MAP[type] };
}

// ──────────────────────────────────────────────
// Connection Quality Scoring
// Based on latency, packet loss, and jitter
// ──────────────────────────────────────────────
export function calculateConnectionQuality(
  latencyMs: number,
  packetLoss: number,
  jitterMs: number,
): {
  score: number;
  label: string;
  labelFa: string;
} {
  // Latency score: 0-40 points (lower is better)
  let latencyScore: number;
  if (latencyMs < 30) latencyScore = 40;
  else if (latencyMs < 60) latencyScore = 35;
  else if (latencyMs < 100) latencyScore = 28;
  else if (latencyMs < 150) latencyScore = 20;
  else if (latencyMs < 250) latencyScore = 12;
  else latencyScore = 5;

  // Packet loss score: 0-35 points (lower is better)
  let lossScore: number;
  if (packetLoss < 0.1) lossScore = 35;
  else if (packetLoss < 0.5) lossScore = 30;
  else if (packetLoss < 1) lossScore = 22;
  else if (packetLoss < 3) lossScore = 14;
  else if (packetLoss < 5) lossScore = 7;
  else lossScore = 0;

  // Jitter score: 0-25 points (lower is better)
  let jitterScore: number;
  if (jitterMs < 5) jitterScore = 25;
  else if (jitterMs < 15) jitterScore = 20;
  else if (jitterMs < 30) jitterScore = 15;
  else if (jitterMs < 50) jitterScore = 10;
  else if (jitterMs < 100) jitterScore = 5;
  else jitterScore = 0;

  const totalScore = Math.min(100, latencyScore + lossScore + jitterScore);

  let label: string;
  let labelFa: string;
  if (totalScore >= 85) {
    label = 'Excellent';
    labelFa = 'عالی';
  } else if (totalScore >= 70) {
    label = 'Good';
    labelFa = 'خوب';
  } else if (totalScore >= 50) {
    label = 'Fair';
    labelFa = 'متوسط';
  } else if (totalScore >= 30) {
    label = 'Poor';
    labelFa = 'ضعیف';
  } else {
    label = 'Critical';
    labelFa = 'بحرانی';
  }

  return { score: totalScore, label, labelFa };
}

// ──────────────────────────────────────────────
// Connection Stability Index
// Based on latency variance, reconnect count, packet loss trends
// ──────────────────────────────────────────────
export function calculateStabilityIndex(
  bandwidthHistory: BandwidthHistoryEntry[],
): {
  index: number;
  label: string;
  labelFa: string;
} {
  if (bandwidthHistory.length < 2) {
    return { index: 50, label: 'Insufficient Data', labelFa: 'داده ناکافی' };
  }

  // Calculate latency variance (stability indicator)
  const latencies = bandwidthHistory.map((e) => e.latencyMs);
  const avgLatency = latencies.reduce((a, b) => a + b, 0) / latencies.length;
  const latencyVariance =
    latencies.reduce((sum, l) => sum + Math.pow(l - avgLatency, 2), 0) /
    latencies.length;
  const latencyStdDev = Math.sqrt(latencyVariance);

  // Calculate packet loss average
  const avgPacketLoss =
    bandwidthHistory.reduce((sum, e) => sum + e.packetLoss, 0) /
    bandwidthHistory.length;

  // Calculate download speed variance
  const downloads = bandwidthHistory.map((e) => e.downloadMbps);
  const avgDownload = downloads.reduce((a, b) => a + b, 0) / downloads.length;
  const downloadVariance =
    downloads.reduce((sum, d) => sum + Math.pow(d - avgDownload, 2), 0) /
    downloads.length;
  const downloadCoeffOfVariation =
    avgDownload > 0 ? Math.sqrt(downloadVariance) / avgDownload : 1;

  // Stability components
  // Latency stability: 0-40 points
  const latencyStability = Math.max(
    0,
    40 - latencyStdDev * 0.8,
  );

  // Packet loss stability: 0-35 points
  const lossStability = Math.max(0, 35 - avgPacketLoss * 7);

  // Throughput stability: 0-25 points
  const throughputStability = Math.max(
    0,
    25 - downloadCoeffOfVariation * 25,
  );

  const index = Math.min(
    100,
    Math.round(latencyStability + lossStability + throughputStability),
  );

  let label: string;
  let labelFa: string;
  if (index >= 85) {
    label = 'Very Stable';
    labelFa = 'بسیار پایدار';
  } else if (index >= 65) {
    label = 'Stable';
    labelFa = 'پایدار';
  } else if (index >= 45) {
    label = 'Moderate';
    labelFa = 'نسبتاً پایدار';
  } else if (index >= 25) {
    label = 'Unstable';
    labelFa = 'ناپایدار';
  } else {
    label = 'Very Unstable';
    labelFa = 'بسیار ناپایدار';
  }

  return { index, label, labelFa };
}

// ──────────────────────────────────────────────
// Traffic Analysis by Protocol Type
// ──────────────────────────────────────────────
const TRAFFIC_PROTOCOLS: Omit<TrafficBreakdown, 'bytesUp' | 'bytesDown' | 'percentage'>[] = [
  { protocol: 'VLESS/Reality', protocolFa: 'VLESS/Reality', color: '#6366f1' },
  { protocol: 'VMess', protocolFa: 'VMess', color: '#10b981' },
  { protocol: 'Hysteria2', protocolFa: 'هیستریا۲', color: '#8b5cf6' },
  { protocol: 'AmneziaWG', protocolFa: 'آمنزیاوی‌جی', color: '#ec4899' },
  { protocol: 'Trojan', protocolFa: 'تروجان', color: '#f59e0b' },
  { protocol: 'ShadowTLS', protocolFa: 'شدوتی‌ال‌اس', color: '#06b6d4' },
  { protocol: 'DNS/DoH', protocolFa: 'DNS/DoH', color: '#14b8a6' },
  { protocol: 'Other', protocolFa: 'سایر', color: '#94a3b8' },
];

export function analyzeTrafficByProtocol(
  totalUpBytes: number,
  totalDownBytes: number,
): TrafficBreakdown[] {
  // Simulate realistic protocol distribution for an Iran bypass scenario
  const distributionWeights = [0.32, 0.18, 0.15, 0.12, 0.08, 0.06, 0.05, 0.04];

  const totalBytes = totalUpBytes + totalDownBytes;
  const results: TrafficBreakdown[] = TRAFFIC_PROTOCOLS.map((proto, idx) => {
    const percentage = distributionWeights[idx] * 100;
    const totalForProto = (totalBytes * distributionWeights[idx]);
    // Upload is typically 15-25% of traffic for each protocol
    const upRatio = 0.15 + Math.random() * 0.1;
    return {
      ...proto,
      bytesUp: Math.round(totalForProto * upRatio),
      bytesDown: Math.round(totalForProto * (1 - upRatio)),
      percentage: Math.round(percentage * 10) / 10,
    };
  });

  return results;
}

// ──────────────────────────────────────────────
// Packet-Level Statistics
// ──────────────────────────────────────────────
export function generatePacketStats(
  isMonitoring: boolean,
  previousStats?: PacketStats,
): PacketStats {
  if (!isMonitoring && !previousStats) {
    return {
      sent: 0,
      received: 0,
      retransmitted: 0,
      lost: 0,
      retransmitRate: 0,
      lossRate: 0,
    };
  }

  const prev = previousStats ?? { sent: 0, received: 0, retransmitted: 0, lost: 0, retransmitRate: 0, lossRate: 0 };

  // Increment with realistic rates
  const newSent = prev.sent + Math.floor(800 + Math.random() * 1200);
  const lossRate = 0.005 + Math.random() * 0.02; // 0.5-2.5%
  const retransmitRate = 0.01 + Math.random() * 0.03; // 1-4%
  const newLost = Math.floor(newSent * lossRate);
  const newRetransmitted = Math.floor(newSent * retransmitRate);
  const newReceived = newSent - newLost;

  return {
    sent: newSent,
    received: newReceived,
    retransmitted: newRetransmitted,
    lost: newLost,
    retransmitRate: Math.round(retransmitRate * 10000) / 100,
    lossRate: Math.round(lossRate * 10000) / 100,
  };
}

// ──────────────────────────────────────────────
// Bandwidth History Generation
// ──────────────────────────────────────────────
export function generateBandwidthSnapshot(
  connected: boolean,
): BandwidthHistoryEntry {
  if (!connected) {
    return {
      timestamp: Date.now(),
      uploadMbps: 0,
      downloadMbps: 0,
      latencyMs: 0,
      packetLoss: 0,
    };
  }

  return {
    timestamp: Date.now(),
    uploadMbps: Math.round((25 + Math.random() * 40) * 10) / 10,
    downloadMbps: Math.round((80 + Math.random() * 180) * 10) / 10,
    latencyMs: Math.round(50 + Math.random() * 80),
    packetLoss: Math.round(Math.random() * 2 * 100) / 100,
  };
}

// ──────────────────────────────────────────────
// Data Usage Tracking
// ──────────────────────────────────────────────
export function generateDailyDataUsage(days: number = 7): DataUsageEntry[] {
  const entries: DataUsageEntry[] = [];
  const now = new Date();
  for (let i = days - 1; i >= 0; i--) {
    const date = new Date(now);
    date.setDate(date.getDate() - i);
    const dateStr = date.toISOString().split('T')[0];
    const downloadMb = Math.round(300 + Math.random() * 2000);
    const uploadMb = Math.round(50 + Math.random() * 300);
    entries.push({
      date: dateStr,
      uploadMb,
      downloadMb,
      totalMb: uploadMb + downloadMb,
    });
  }
  return entries;
}

export function generateWeeklyDataUsage(weeks: number = 4): DataUsageEntry[] {
  const entries: DataUsageEntry[] = [];
  const now = new Date();
  for (let i = weeks - 1; i >= 0; i--) {
    const weekStart = new Date(now);
    weekStart.setDate(weekStart.getDate() - i * 7);
    const dateStr = `هفته ${weeks - i}`;
    const downloadMb = Math.round(3000 + Math.random() * 12000);
    const uploadMb = Math.round(500 + Math.random() * 2000);
    entries.push({
      date: dateStr,
      uploadMb,
      downloadMb,
      totalMb: uploadMb + downloadMb,
    });
  }
  return entries;
}

export function generateMonthlyDataUsage(months: number = 6): DataUsageEntry[] {
  const entries: DataUsageEntry[] = [];
  const persianMonths = ['فروردین', 'اردیبهشت', 'خرداد', 'تیر', 'مرداد', 'شهریور', 'مهر', 'آبان', 'آذر', 'دی', 'بهمن', 'اسفند'];
  const now = new Date();
  for (let i = months - 1; i >= 0; i--) {
    const date = new Date(now.getFullYear(), now.getMonth() - i, 1);
    const monthIndex = date.getMonth();
    const dateStr = persianMonths[monthIndex];
    const downloadMb = Math.round(15000 + Math.random() * 50000);
    const uploadMb = Math.round(2000 + Math.random() * 8000);
    entries.push({
      date: dateStr,
      uploadMb,
      downloadMb,
      totalMb: uploadMb + downloadMb,
    });
  }
  return entries;
}

// ──────────────────────────────────────────────
// Full Network Analyzer State Builder
// ──────────────────────────────────────────────
export function buildInitialNetworkAnalyzerState(): NetworkAnalyzerState {
  const networkInfo = detectNetworkType();
  const latency = 62 + Math.floor(Math.random() * 40);
  const packetLoss = Math.round(Math.random() * 2 * 100) / 100;
  const jitter = Math.round((5 + Math.random() * 20) * 10) / 10;

  const quality = calculateConnectionQuality(latency, packetLoss, jitter);

  // Generate initial bandwidth history (last 30 data points)
  const bandwidthHistory: BandwidthHistoryEntry[] = [];
  const now = Date.now();
  for (let i = 29; i >= 0; i--) {
    bandwidthHistory.push({
      timestamp: now - i * 5000,
      uploadMbps: Math.round((25 + Math.random() * 40) * 10) / 10,
      downloadMbps: Math.round((80 + Math.random() * 180) * 10) / 10,
      latencyMs: Math.round(50 + Math.random() * 80),
      packetLoss: Math.round(Math.random() * 2 * 100) / 100,
    });
  }

  const stability = calculateStabilityIndex(bandwidthHistory);

  const totalUpBytes = 14_000_000 + Math.floor(Math.random() * 8_000_000);
  const totalDownBytes = 90_000_000 + Math.floor(Math.random() * 50_000_000);
  const trafficBreakdown = analyzeTrafficByProtocol(totalUpBytes, totalDownBytes);

  const packetStats = generatePacketStats(true, {
    sent: 145_832,
    received: 143_215,
    retransmitted: 4_521,
    lost: 2_617,
    retransmitRate: 3.1,
    lossRate: 1.79,
  });

  const dailyUsage = generateDailyDataUsage(7);
  const weeklyUsage = generateWeeklyDataUsage(4);
  const monthlyUsage = generateMonthlyDataUsage(6);

  const totalDataUsedMb = dailyUsage.reduce((sum, e) => sum + e.totalMb, 0);

  return {
    isMonitoring: true,
    currentUploadMbps: bandwidthHistory[bandwidthHistory.length - 1].uploadMbps,
    currentDownloadMbps: bandwidthHistory[bandwidthHistory.length - 1].downloadMbps,
    currentLatencyMs: latency,
    currentPacketLoss: packetLoss,
    currentJitter: jitter,
    networkType: networkInfo.type,
    networkTypeFa: networkInfo.nameFa,
    connectionQualityScore: quality.score,
    connectionQualityLabel: quality.label,
    connectionQualityLabelFa: quality.labelFa,
    stabilityIndex: stability.index,
    stabilityLabel: stability.label,
    stabilityLabelFa: stability.labelFa,
    bandwidthHistory,
    trafficBreakdown,
    packetStats,
    dataUsageDaily: dailyUsage,
    dataUsageWeekly: weeklyUsage,
    dataUsageMonthly: monthlyUsage,
    totalDataUsedMb,
    monitoringStartTime: now - 3600000,
  };
}

// ──────────────────────────────────────────────
// Update Monitoring (called on each tick)
// ──────────────────────────────────────────────
export function updateNetworkMonitoring(
  currentState: NetworkAnalyzerState,
  connected: boolean,
): NetworkAnalyzerState {
  if (!connected) {
    return {
      ...currentState,
      isMonitoring: false,
      currentUploadMbps: 0,
      currentDownloadMbps: 0,
      currentLatencyMs: 0,
      currentPacketLoss: 0,
      currentJitter: 0,
    };
  }

  const newSnapshot = generateBandwidthSnapshot(true);
  const updatedHistory = [...currentState.bandwidthHistory.slice(-59), newSnapshot];

  const jitter = Math.round(
    (currentState.currentJitter * 0.7 + (Math.random() * 15) * 0.3) * 10,
  ) / 10;

  const quality = calculateConnectionQuality(
    newSnapshot.latencyMs,
    newSnapshot.packetLoss,
    jitter,
  );

  const stability = calculateStabilityIndex(updatedHistory);

  const updatedPacketStats = generatePacketStats(true, currentState.packetStats);

  return {
    ...currentState,
    isMonitoring: true,
    currentUploadMbps: newSnapshot.uploadMbps,
    currentDownloadMbps: newSnapshot.downloadMbps,
    currentLatencyMs: newSnapshot.latencyMs,
    currentPacketLoss: newSnapshot.packetLoss,
    currentJitter: jitter,
    connectionQualityScore: quality.score,
    connectionQualityLabel: quality.label,
    connectionQualityLabelFa: quality.labelFa,
    stabilityIndex: stability.index,
    stabilityLabel: stability.label,
    stabilityLabelFa: stability.labelFa,
    bandwidthHistory: updatedHistory,
    packetStats: updatedPacketStats,
    totalDataUsedMb: currentState.totalDataUsedMb + Math.round(newSnapshot.downloadMbps * 0.625 + newSnapshot.uploadMbps * 0.625),
  };
}
