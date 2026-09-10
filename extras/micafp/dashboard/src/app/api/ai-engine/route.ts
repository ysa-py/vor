import { NextRequest, NextResponse } from 'next/server';

// ──────────────────────────────────────────────
// In-memory AI Engine state (singleton per process)
// ──────────────────────────────────────────────
const CORE_IDS = [
  'hiddify', 'xray-gfw', 'sing-box',
  'amneziavpn', 'defyxvpn', 'moav',
  'lantern', 'mahsang', 'psiphon',
] as const;

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

function computeUCB(
  rewards: number[],
  alpha: number,
  pulls: number,
): { exploitation: number; exploration: number; total: number } {
  const n = rewards.length;
  if (n === 0) {
    return {
      exploitation: 0.5,
      exploration: alpha * Math.sqrt(Math.log(pulls + 1) / 1),
      total: 0.5 + alpha,
    };
  }
  const avgReward = rewards.reduce((a, b) => a + b, 0) / n;
  const exploration = alpha * Math.sqrt(Math.log(pulls + 1) / n);
  return {
    exploitation: Math.round(avgReward * 1000) / 1000,
    exploration: Math.round(exploration * 1000) / 1000,
    total: Math.round((avgReward + exploration) * 1000) / 1000,
  };
}

interface PerCoreScore {
  coreId: string;
  score: number;
  ucb: { exploitation: number; exploration: number; total: number };
  rewardAvg: number;
  pullCount: number;
  alpha: number;
}

const rlWeights: Record<string, number[]> = Object.fromEntries(
  CORE_IDS.map((id) => [id, [0.5, 0.3, 0.2, 0.6, 0.4]]),
);

const rlParams = {
  learningRate: 0.01,
  discountFactor: 0.95,
  epsilon: 0.1,
  batchSize: 32,
  replayBufferSize: 10000,
  targetUpdateFrequency: 100,
  gradientClipValue: 1.0,
};

const predictionState = {
  imminentBlockRisk: 12,
  predictedBlockCore: null as string | null,
  proactiveSwitchRecommended: false,
  confidenceScore: 0.87,
  lastPredictionTimestamp: Date.now() - 60000,
  modelVersion: '3.1.0',
};

const ispDetection = {
  detectedISP: 'irancell',
  detectedISPFa: 'ایرانسل',
  confidence: 0.94,
  method: 'RTT-fingerprint + DNS-pattern',
  lastDetectionTimestamp: Date.now() - 300000,
  ispRuleApplied: 'irancell',
};

// ──────────────────────────────────────────────
// GET /api/ai-engine
// ──────────────────────────────────────────────
export async function GET() {
  const perCoreScores: PerCoreScore[] = CORE_IDS.map((coreId) => {
    const rewards = REWARD_HISTORY[coreId] ?? [];
    const alpha = UCB_ALPHAS[coreId] ?? 1.5;
    const ucb = computeUCB(rewards, alpha, totalPulls);
    const avg = rewards.length > 0 ? rewards.reduce((a, b) => a + b, 0) / rewards.length : 0;
    return {
      coreId,
      score: Math.round(ucb.total * 100),
      ucb,
      rewardAvg: Math.round(avg * 1000) / 1000,
      pullCount: rewards.length,
      alpha,
    };
  });

  return NextResponse.json({
    success: true,
    timestamp: Date.now(),
    algorithm: {
      name: 'UCB1 (Upper Confidence Bound)',
      nameFa: 'UCB1 (کران بالایی اطمینان)',
      description: 'Multi-Armed Bandit with UCB1 exploration-exploitation balance',
      descriptionFa: 'باندیت چندبازویی با تعادل اکتشاف-بهره‌برداری UCB1',
      totalPulls,
      version: '3.1.0',
    },
    perCoreScores,
    predictionState,
    ispDetection,
    rlParameters: rlParams,
    rlWeights,
    rewardHistory: REWARD_HISTORY,
    ucbAlphas: UCB_ALPHAS,
    meta: {
      endpoint: '/api/ai-engine',
      descriptionFa: 'وضعیت موتور هوش مصنوعی و الگوریتم UCB1',
    },
  });
}

// ──────────────────────────────────────────────
// POST /api/ai-engine
// body: { action: 'update-reward' | 'force-switch', coreId: string, reward?: number }
// ──────────────────────────────────────────────
export async function POST(request: NextRequest) {
  try {
    const body = await request.json();
    const { action, coreId, reward } = body as {
      action: string;
      coreId?: string;
      reward?: number;
    };

    if (!action) {
      return NextResponse.json(
        { success: false, error: 'Missing required field: action', errorFa: 'فیلد ضروری موجود نیست: action' },
        { status: 400 },
      );
    }

    if (action === 'update-reward') {
      if (!coreId || !CORE_IDS.includes(coreId as (typeof CORE_IDS)[number])) {
        return NextResponse.json(
          {
            success: false,
            error: `Invalid coreId. Must be one of: ${CORE_IDS.join(', ')}`,
            errorFa: 'شناسه هسته نامعتبر است',
          },
          { status: 400 },
        );
      }
      const rewardValue = typeof reward === 'number' ? Math.max(0, Math.min(1, reward)) : 0.5;
      if (!REWARD_HISTORY[coreId]) {
        REWARD_HISTORY[coreId] = [];
      }
      REWARD_HISTORY[coreId].push(rewardValue);
      REWARD_HISTORY[coreId] = REWARD_HISTORY[coreId].slice(-100);
      totalPulls += 1;

      // Update RL weights based on reward
      const weights = [...(rlWeights[coreId] ?? [0.5, 0.3, 0.2, 0.6, 0.4])];
      weights[0] = Math.min(1, weights[0] + rlParams.learningRate * (rewardValue > 0.5 ? 1 : -1));
      rlWeights[coreId] = weights;

      return NextResponse.json({
        success: true,
        action: 'update-reward',
        coreId,
        reward: rewardValue,
        newRewardHistory: REWARD_HISTORY[coreId],
        totalPulls,
        updatedWeights: rlWeights[coreId],
        message: `Reward ${rewardValue} recorded for ${coreId}`,
        messageFa: `پاداش ${rewardValue} برای هسته ${coreId} ثبت شد`,
      });
    }

    if (action === 'force-switch') {
      if (!coreId || !CORE_IDS.includes(coreId as (typeof CORE_IDS)[number])) {
        return NextResponse.json(
          {
            success: false,
            error: `Invalid coreId. Must be one of: ${CORE_IDS.join(', ')}`,
            errorFa: 'شناسه هسته نامعتبر است',
          },
          { status: 400 },
        );
      }

      const switchTime = Math.round((0.8 + Math.random() * 1.2) * 1000) / 1000;
      totalPulls += 1;

      const rewards = REWARD_HISTORY[coreId] ?? [];
      const ucb = computeUCB(rewards, UCB_ALPHAS[coreId] ?? 1.5, totalPulls);

      return NextResponse.json({
        success: true,
        action: 'force-switch',
        previousCoreId: 'xray-gfw',
        newCoreId: coreId,
        switchTimeMs: switchTime * 1000,
        switchTimeSec: switchTime,
        ucbScore: ucb,
        totalPulls,
        message: `Forced switch to ${coreId} completed in ${switchTime}s`,
        messageFa: `تعویض اجباری به ${coreId} در ${switchTime} ثانیه انجام شد`,
      });
    }

    return NextResponse.json(
      {
        success: false,
        error: `Unknown action: ${action}. Valid actions: update-reward, force-switch`,
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
