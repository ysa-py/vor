import { NextRequest, NextResponse } from 'next/server';
import {
  buildInitialNetworkAnalyzerState,
  updateNetworkMonitoring,
  detectNetworkType,
  calculateConnectionQuality,
  calculateStabilityIndex,
  analyzeTrafficByProtocol,
  generatePacketStats,
  generateDailyDataUsage,
  generateWeeklyDataUsage,
  generateMonthlyDataUsage,
  generateBandwidthSnapshot,
} from '@/lib/network-analyzer';

// ──────────────────────────────────────────────
// In-memory network analyzer state
// ──────────────────────────────────────────────
let analyzerState = buildInitialNetworkAnalyzerState();
let monitoringInterval: ReturnType<typeof setInterval> | null = null;

function startMonitoring() {
  if (monitoringInterval) return;
  monitoringInterval = setInterval(() => {
    analyzerState = updateNetworkMonitoring(analyzerState, true);
  }, 5000);
}

function stopMonitoring() {
  if (monitoringInterval) {
    clearInterval(monitoringInterval);
    monitoringInterval = null;
  }
  analyzerState = {
    ...analyzerState,
    isMonitoring: false,
    currentUploadMbps: 0,
    currentDownloadMbps: 0,
    currentLatencyMs: 0,
    currentPacketLoss: 0,
    currentJitter: 0,
  };
}

// ──────────────────────────────────────────────
// GET /api/network-analyzer
// Returns current network statistics
// ──────────────────────────────────────────────
export async function GET() {
  const networkInfo = detectNetworkType();
  const quality = calculateConnectionQuality(
    analyzerState.currentLatencyMs,
    analyzerState.currentPacketLoss,
    analyzerState.currentJitter,
  );
  const stability = calculateStabilityIndex(analyzerState.bandwidthHistory);

  // Generate fresh traffic breakdown
  const totalUp = analyzerState.totalDataUsedMb * 1024 * 1024 * 0.2;
  const totalDown = analyzerState.totalDataUsedMb * 1024 * 1024 * 0.8;
  const trafficBreakdown = analyzeTrafficByProtocol(totalUp, totalDown);

  return NextResponse.json({
    success: true,
    timestamp: Date.now(),
    monitoring: {
      isActive: analyzerState.isMonitoring,
      startTime: analyzerState.monitoringStartTime,
      uptimeMs: analyzerState.isMonitoring
        ? Date.now() - analyzerState.monitoringStartTime
        : 0,
      uptimeHuman: analyzerState.isMonitoring
        ? `${Math.floor((Date.now() - analyzerState.monitoringStartTime) / 3600000)}h ${Math.floor(((Date.now() - analyzerState.monitoringStartTime) % 3600000) / 60000)}m`
        : '۰',
    },
    bandwidth: {
      currentUploadMbps: analyzerState.currentUploadMbps,
      currentDownloadMbps: analyzerState.currentDownloadMbps,
      currentLatencyMs: analyzerState.currentLatencyMs,
      currentPacketLoss: analyzerState.currentPacketLoss,
      currentJitter: analyzerState.currentJitter,
      labelUploadFa: 'آپلود',
      labelDownloadFa: 'دانلود',
      labelLatencyFa: 'تأخیر',
      labelPacketLossFa: 'افت بسته',
      labelJitterFa: 'جیتر',
    },
    connectionQuality: {
      score: quality.score,
      label: quality.label,
      labelFa: quality.labelFa,
    },
    stability: {
      index: stability.index,
      label: stability.label,
      labelFa: stability.labelFa,
    },
    networkType: {
      type: networkInfo.type,
      nameFa: networkInfo.nameFa,
    },
    trafficBreakdown,
    packetStats: {
      ...analyzerState.packetStats,
      labelSentFa: 'ارسال‌شده',
      labelReceivedFa: 'دریافت‌شده',
      labelRetransmittedFa: 'بازارسال‌شده',
      labelLostFa: 'از دست‌رفته',
      labelRetransmitRateFa: 'نرخ بازارسال',
      labelLossRateFa: 'نرخ افت',
    },
    dataUsage: {
      daily: analyzerState.dataUsageDaily,
      weekly: analyzerState.dataUsageWeekly,
      monthly: analyzerState.dataUsageMonthly,
      totalUsedMb: analyzerState.totalDataUsedMb,
      totalUsedGb: Math.round(analyzerState.totalDataUsedMb / 1024 * 10) / 10,
      labelTotalFa: 'مصرف کل',
    },
    bandwidthHistory: analyzerState.bandwidthHistory.slice(-12),
    meta: {
      endpoint: '/api/network-analyzer',
      descriptionFa: 'تحلیلگر شبکه — پایش پهنای باند، کیفیت اتصال و مصرف داده',
    },
  });
}

// ──────────────────────────────────────────────
// POST /api/network-analyzer
// body: { action: 'start' | 'stop' | 'refresh' }
// ──────────────────────────────────────────────
export async function POST(request: NextRequest) {
  try {
    const body = await request.json();
    const { action } = body as { action?: string };

    if (!action) {
      return NextResponse.json(
        {
          success: false,
          error: 'Missing required field: action',
          errorFa: 'فیلد ضروری موجود نیست: action',
        },
        { status: 400 },
      );
    }

    if (action === 'start') {
      if (analyzerState.isMonitoring) {
        return NextResponse.json({
          success: true,
          action: 'start',
          message: 'Network monitoring is already running',
          messageFa: 'پایش شبکه از قبل در حال اجرا است',
        });
      }

      analyzerState = buildInitialNetworkAnalyzerState();
      startMonitoring();

      return NextResponse.json({
        success: true,
        action: 'start',
        message: 'Network monitoring started',
        messageFa: 'پایش شبکه شروع شد',
        state: {
          isMonitoring: analyzerState.isMonitoring,
          networkType: analyzerState.networkType,
          networkTypeFa: analyzerState.networkTypeFa,
          qualityScore: analyzerState.connectionQualityScore,
          qualityLabelFa: analyzerState.connectionQualityLabelFa,
          stabilityIndex: analyzerState.stabilityIndex,
          stabilityLabelFa: analyzerState.stabilityLabelFa,
        },
      });
    }

    if (action === 'stop') {
      stopMonitoring();

      return NextResponse.json({
        success: true,
        action: 'stop',
        message: 'Network monitoring stopped',
        messageFa: 'پایش شبکه متوقف شد',
        finalStats: {
          totalDataUsedMb: analyzerState.totalDataUsedMb,
          totalDataUsedGb: Math.round(analyzerState.totalDataUsedMb / 1024 * 10) / 10,
          packetStats: analyzerState.packetStats,
        },
      });
    }

    if (action === 'refresh') {
      // Force a fresh snapshot
      const snapshot = generateBandwidthSnapshot(analyzerState.isMonitoring);
      analyzerState = {
        ...analyzerState,
        currentUploadMbps: snapshot.uploadMbps,
        currentDownloadMbps: snapshot.downloadMbps,
        currentLatencyMs: snapshot.latencyMs,
        currentPacketLoss: snapshot.packetLoss,
        bandwidthHistory: [...analyzerState.bandwidthHistory.slice(-59), snapshot],
        packetStats: generatePacketStats(analyzerState.isMonitoring, analyzerState.packetStats),
        dataUsageDaily: generateDailyDataUsage(7),
        dataUsageWeekly: generateWeeklyDataUsage(4),
        dataUsageMonthly: generateMonthlyDataUsage(6),
      };

      const quality = calculateConnectionQuality(
        analyzerState.currentLatencyMs,
        analyzerState.currentPacketLoss,
        analyzerState.currentJitter,
      );
      analyzerState.connectionQualityScore = quality.score;
      analyzerState.connectionQualityLabel = quality.label;
      analyzerState.connectionQualityLabelFa = quality.labelFa;

      const stability = calculateStabilityIndex(analyzerState.bandwidthHistory);
      analyzerState.stabilityIndex = stability.index;
      analyzerState.stabilityLabel = stability.label;
      analyzerState.stabilityLabelFa = stability.labelFa;

      return NextResponse.json({
        success: true,
        action: 'refresh',
        message: 'Network stats refreshed',
        messageFa: 'آمار شبکه به‌روز شد',
        currentBandwidth: {
          uploadMbps: analyzerState.currentUploadMbps,
          downloadMbps: analyzerState.currentDownloadMbps,
          latencyMs: analyzerState.currentLatencyMs,
          packetLoss: analyzerState.currentPacketLoss,
        },
        qualityScore: analyzerState.connectionQualityScore,
        stabilityIndex: analyzerState.stabilityIndex,
      });
    }

    return NextResponse.json(
      {
        success: false,
        error: `Unknown action: ${action}. Valid actions: start, stop, refresh`,
        errorFa: `عملیات ناشناخته: ${action}`,
      },
      { status: 400 },
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
