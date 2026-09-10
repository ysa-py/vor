import { NextRequest, NextResponse } from 'next/server';

// ──────────────────────────────────────────────
// Orchestrator state (in-memory)
// ──────────────────────────────────────────────
const CORE_IDS = [
  'hiddify', 'xray-gfw', 'sing-box',
  'amneziavpn', 'defyxvpn', 'moav',
  'lantern', 'mahsang', 'psiphon',
];

const ISP_RULES = [
  { id: 'mci', name: 'MCI (Hamrahe Avval)', nameFa: 'همراه اول', preferredCores: ['mahsang', 'amneziavpn'] },
  { id: 'irancell', name: 'Irancell (MTN)', nameFa: 'ایرانسل', preferredCores: ['hiddify', 'defyxvpn'] },
  { id: 'shatel', name: 'Shatel', nameFa: 'شتل', preferredCores: ['amneziavpn', 'psiphon'] },
  { id: 'asiatech', name: 'Asiatech', nameFa: 'آسیاتک', preferredCores: ['mahsang', 'hiddify'] },
  { id: 'rightel', name: 'Rightel', nameFa: 'رایتل', preferredCores: ['defyxvpn', 'hiddify'] },
];

interface OrchestratorState {
  activeCoreId: string;
  shadowConnections: string[];
  scoringMatrix: Record<string, number>;
  ucbScores: Record<string, { exploitation: number; exploration: number; total: number }>;
  predictionState: {
    imminentBlockRisk: number;
    predictedBlockCore: string | null;
    proactiveSwitchRecommended: boolean;
  };
  rlWeights: Record<string, number[]>;
  learningRate: number;
  totalSwitches: number;
  successfulSwitches: number;
  averageSwitchTime: number;
  detectedISP: string;
  detectedISPFa: string;
  ispRuleApplied: string;
  healthScore: number;
  uptimeSeconds: number;
  lastOrchestrationCycle: number;
  cycleCount: number;
}

function computeUCB(
  rewards: number[],
  alpha: number,
  totalPulls: number,
): { exploitation: number; exploration: number; total: number } {
  const n = rewards.length;
  if (n === 0) {
    return {
      exploitation: 0.5,
      exploration: alpha * Math.sqrt(Math.log(totalPulls + 1) / 1),
      total: 0.5 + alpha,
    };
  }
  const avgReward = rewards.reduce((a, b) => a + b, 0) / n;
  const exploration = alpha * Math.sqrt(Math.log(totalPulls + 1) / n);
  return {
    exploitation: Math.round(avgReward * 1000) / 1000,
    exploration: Math.round(exploration * 1000) / 1000,
    total: Math.round((avgReward + exploration) * 1000) / 1000,
  };
}

const UCB_ALPHAS: Record<string, number> = {
  hiddify: 1.5, 'xray-gfw': 1.5, 'sing-box': 1.5,
  amneziavpn: 2.0, defyxvpn: 1.5, moav: 1.5,
  lantern: 1.5, mahsang: 1.5, psiphon: 0.5,
};

const REWARD_HISTORY: Record<string, number[]> = {
  hiddify: [0.8, 0.9, 0.7, 0.85, 0.9],
  'xray-gfw': [0.95, 0.92, 0.88, 0.9, 0.93],
  'sing-box': [0.75, 0.8, 0.82, 0.78, 0.85],
  amneziavpn: [0.7, 0.65, 0.8, 0.72, 0.68],
  defyxvpn: [0.6, 0.65, 0.7, 0.55, 0.62],
  moav: [0.55, 0.6, 0.5, 0.58, 0.52],
  lantern: [0.45, 0.5, 0.48, 0.42, 0.47],
  mahsang: [0.88, 0.92, 0.85, 0.9, 0.87],
  psiphon: [0.35, 0.3, 0.4, 0.32, 0.28],
};

let totalPulls = 47;

const BASE_LATENCY: Record<string, number> = {
  hiddify: 85, 'xray-gfw': 62, 'sing-box': 73,
  amneziavpn: 91, defyxvpn: 105, moav: 118,
  lantern: 142, mahsang: 79, psiphon: 156,
};

function buildScoringMatrix(): Record<string, number> {
  const matrix: Record<string, number> = {};
  for (const coreId of CORE_IDS) {
    const latency = BASE_LATENCY[coreId] ?? 100;
    const latencyScore = Math.max(0, 100 - latency);
    const blockEvents = Math.floor(Math.random() * 5);
    const blockScore = Math.max(0, 100 - blockEvents * 15);
    const dpiScore = Math.max(0, 100 - Math.random() * 30);
    const rewardAvg = REWARD_HISTORY[coreId]
      ? REWARD_HISTORY[coreId]!.reduce((a, b) => a + b, 0) / REWARD_HISTORY[coreId]!.length
      : 0.5;
    const rlScore = rewardAvg * 100;
    matrix[coreId] = Math.round(
      latencyScore * 0.3 + blockScore * 0.2 + dpiScore * 0.25 + rlScore * 0.25,
    );
  }
  return matrix;
}

function buildUCBScores(): Record<string, { exploitation: number; exploration: number; total: number }> {
  const scores: Record<string, { exploitation: number; exploration: number; total: number }> = {};
  for (const coreId of CORE_IDS) {
    const alpha = UCB_ALPHAS[coreId] ?? 1.5;
    const rewards = REWARD_HISTORY[coreId] ?? [];
    scores[coreId] = computeUCB(rewards, alpha, totalPulls);
  }
  return scores;
}

const orchestratorState: OrchestratorState = {
  activeCoreId: 'xray-gfw',
  shadowConnections: ['mahsang', 'hiddify'],
  scoringMatrix: buildScoringMatrix(),
  ucbScores: buildUCBScores(),
  predictionState: {
    imminentBlockRisk: 12,
    predictedBlockCore: null,
    proactiveSwitchRecommended: false,
  },
  rlWeights: Object.fromEntries(CORE_IDS.map((id) => [id, [0.5, 0.3, 0.2, 0.6, 0.4]])),
  learningRate: 0.01,
  totalSwitches: 47,
  successfulSwitches: 44,
  averageSwitchTime: 1.3,
  detectedISP: 'irancell',
  detectedISPFa: 'ایرانسل',
  ispRuleApplied: 'irancell',
  healthScore: 93,
  uptimeSeconds: 259200,
  lastOrchestrationCycle: Date.now() - 15000,
  cycleCount: 1247,
};

// ──────────────────────────────────────────────
// GET /api/orchestrator
// ──────────────────────────────────────────────
export async function GET() {
  // Refresh scoring matrix and UCB scores
  orchestratorState.scoringMatrix = buildScoringMatrix();
  orchestratorState.ucbScores = buildUCBScores();
  orchestratorState.uptimeSeconds += 15;
  orchestratorState.cycleCount += 1;
  orchestratorState.lastOrchestrationCycle = Date.now();

  const ispRule = ISP_RULES.find((r) => r.id === orchestratorState.detectedISP);

  return NextResponse.json({
    success: true,
    timestamp: Date.now(),
    orchestrator: orchestratorState,
    ispRule: ispRule ?? null,
    activeCoreDetails: {
      coreId: orchestratorState.activeCoreId,
      latency: BASE_LATENCY[orchestratorState.activeCoreId] ?? 0,
      scoringMatrixRank: Object.entries(orchestratorState.scoringMatrix)
        .sort(([, a], [, b]) => b - a)
        .findIndex(([id]) => id === orchestratorState.activeCoreId) + 1,
    },
    shadowCoreDetails: orchestratorState.shadowConnections.map((id) => ({
      coreId: id,
      latency: BASE_LATENCY[id] ?? 0,
      score: orchestratorState.scoringMatrix[id] ?? 0,
    })),
    switchSuccessRate: orchestratorState.totalSwitches > 0
      ? Math.round((orchestratorState.successfulSwitches / orchestratorState.totalSwitches) * 100 * 10) / 10
      : 0,
    meta: {
      endpoint: '/api/orchestrator',
      descriptionFa: 'وضعیت هماهنگ‌سازی هوشمند اتصال',
    },
  });
}

// ──────────────────────────────────────────────
// POST /api/orchestrator
// body: { action: 'switch' | 'health-check', targetCoreId?: string }
// ──────────────────────────────────────────────
export async function POST(request: NextRequest) {
  try {
    const body = await request.json();
    const { action, targetCoreId } = body as { action?: string; targetCoreId?: string };

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

    if (action === 'switch') {
      if (!targetCoreId || !CORE_IDS.includes(targetCoreId)) {
        return NextResponse.json(
          {
            success: false,
            error: `Invalid targetCoreId. Must be one of: ${CORE_IDS.join(', ')}`,
            errorFa: 'شناسه هسته هدف نامعتبر است',
          },
          { status: 400 },
        );
      }

      if (targetCoreId === orchestratorState.activeCoreId) {
        return NextResponse.json({
          success: true,
          action: 'switch',
          message: `Already connected to ${targetCoreId}, no switch needed`,
          messageFa: `از قبل به ${targetCoreId} متصل هستید، تعویض لازم نیست`,
          orchestrator: orchestratorState,
        });
      }

      const previousCoreId = orchestratorState.activeCoreId;
      const newShadows = [previousCoreId, ...orchestratorState.shadowConnections.filter((id) => id !== targetCoreId)].slice(0, 2);

      const switchTime = Math.round((0.8 + Math.random() * 1.2) * 100) / 100;
      const success = Math.random() > 0.05;

      orchestratorState.activeCoreId = targetCoreId;
      orchestratorState.shadowConnections = newShadows;
      orchestratorState.totalSwitches += 1;
      orchestratorState.successfulSwitches += success ? 1 : 0;
      orchestratorState.averageSwitchTime = Math.round(
        ((orchestratorState.averageSwitchTime * (orchestratorState.totalSwitches - 1)) + switchTime) /
        orchestratorState.totalSwitches * 100,
      ) / 100;
      orchestratorState.scoringMatrix = buildScoringMatrix();
      orchestratorState.ucbScores = buildUCBScores();
      totalPulls += 1;

      // Update prediction state
      orchestratorState.predictionState = {
        imminentBlockRisk: Math.round(Math.random() * 30),
        predictedBlockCore: Math.random() > 0.7 ? previousCoreId : null,
        proactiveSwitchRecommended: false,
      };

      return NextResponse.json({
        success: true,
        action: 'switch',
        previousCoreId,
        newCoreId: targetCoreId,
        newShadowConnections: newShadows,
        switchTimeSec: switchTime,
        switchSuccessful: success,
        orchestrator: orchestratorState,
        message: success
          ? `Switched from ${previousCoreId} to ${targetCoreId} in ${switchTime}s`
          : `Switch to ${targetCoreId} failed, retrying...`,
        messageFa: success
          ? `از ${previousCoreId} به ${targetCoreId} در ${switchTime} ثانیه تعویض شد`
          : `تعویض به ${targetCoreId} ناموفق بود، تلاش مجدد...`,
      });
    }

    if (action === 'health-check') {
      orchestratorState.scoringMatrix = buildScoringMatrix();
      orchestratorState.ucbScores = buildUCBScores();
      orchestratorState.cycleCount += 1;
      orchestratorState.lastOrchestrationCycle = Date.now();

      const activeScore = orchestratorState.scoringMatrix[orchestratorState.activeCoreId] ?? 0;
      const isHealthy = activeScore > 50;
      orchestratorState.healthScore = isHealthy
        ? Math.min(100, Math.round(80 + Math.random() * 20))
        : Math.round(20 + Math.random() * 30);

      return NextResponse.json({
        success: true,
        action: 'health-check',
        healthy: isHealthy,
        activeCoreScore: activeScore,
        orchestrator: orchestratorState,
        recommendations: {
          switchRecommended: !isHealthy,
          bestAlternative: Object.entries(orchestratorState.scoringMatrix)
            .sort(([, a], [, b]) => b - a)
            .filter(([id]) => id !== orchestratorState.activeCoreId)[0]?.[0] ?? null,
        },
        message: isHealthy
          ? `Orchestrator healthy, active core score: ${activeScore}`
          : `Orchestrator health degraded, active core score: ${activeScore}`,
        messageFa: isHealthy
          ? `هماهنگ‌کننده سالم، امتیاز هسته فعال: ${activeScore}`
          : `سلامت هماهنگ‌کننده کاهش یافته، امتیاز هسته فعال: ${activeScore}`,
      });
    }

    return NextResponse.json(
      {
        success: false,
        error: `Unknown action: ${action}. Valid actions: switch, health-check`,
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
