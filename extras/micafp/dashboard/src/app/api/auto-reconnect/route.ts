import { NextRequest, NextResponse } from 'next/server';

// ──────────────────────────────────────────────
// Auto-Reconnect state (in-memory)
// ──────────────────────────────────────────────
interface AutoReconnectState {
  enabled: boolean;
  maxRetries: number;
  retryCount: number;
  retryInterval: number;
  exponentialBackoff: boolean;
  lastReconnectAttempt: number;
  reconnectStatus: 'idle' | 'reconnecting' | 'failed' | 'connected';
  totalReconnectAttempts: number;
  totalSuccessfulReconnects: number;
  averageReconnectTimeMs: number;
  preferredCoreOrder: string[];
  fallbackToPsiphon: boolean;
  reconnectOnDPI: boolean;
  reconnectOnBlock: boolean;
  reconnectOnDnsLeak: boolean;
}

const reconnectState: AutoReconnectState = {
  enabled: true,
  maxRetries: 10,
  retryCount: 0,
  retryInterval: 3000,
  exponentialBackoff: true,
  lastReconnectAttempt: 0,
  reconnectStatus: 'idle',
  totalReconnectAttempts: 14,
  totalSuccessfulReconnects: 11,
  averageReconnectTimeMs: 2300,
  preferredCoreOrder: ['xray-gfw', 'mahsang', 'hiddify', 'defyxvpn', 'amneziavpn', 'sing-box', 'moav', 'lantern', 'psiphon'],
  fallbackToPsiphon: true,
  reconnectOnDPI: true,
  reconnectOnBlock: true,
  reconnectOnDnsLeak: true,
};

// ──────────────────────────────────────────────
// GET /api/auto-reconnect
// ──────────────────────────────────────────────
export async function GET() {
  const successRate = reconnectState.totalReconnectAttempts > 0
    ? Math.round((reconnectState.totalSuccessfulReconnects / reconnectState.totalReconnectAttempts) * 100 * 10) / 10
    : 0;

  const currentDelay = reconnectState.exponentialBackoff
    ? reconnectState.retryInterval * Math.pow(2, reconnectState.retryCount)
    : reconnectState.retryInterval;

  return NextResponse.json({
    success: true,
    timestamp: Date.now(),
    state: reconnectState,
    stats: {
      successRate,
      averageReconnectTimeSec: Math.round(reconnectState.averageReconnectTimeMs / 100) / 10,
      nextRetryDelayMs: Math.min(currentDelay, 30000),
      nextRetryDelaySec: Math.round(Math.min(currentDelay, 30000) / 1000 * 10) / 10,
      isExponential: reconnectState.exponentialBackoff,
      retriesRemaining: reconnectState.maxRetries - reconnectState.retryCount,
    },
    config: {
      description: 'Auto-reconnect restores VPN connectivity when connection drops unexpectedly',
      descriptionFa: 'اتصال مجدد خودکار، اتصال VPN را هنگام قطع غیرمنتظره بازیابی می‌کند',
      retryIntervalOptions: [
        { value: 2000, label: '2 seconds', labelFa: '۲ ثانیه' },
        { value: 3000, label: '3 seconds', labelFa: '۳ ثانیه' },
        { value: 5000, label: '5 seconds', labelFa: '۵ ثانیه' },
        { value: 10000, label: '10 seconds', labelFa: '۱۰ ثانیه' },
      ],
      maxRetriesOptions: [
        { value: 5, label: '5 retries', labelFa: '۵ تلاش' },
        { value: 10, label: '10 retries', labelFa: '۱۰ تلاش' },
        { value: 20, label: '20 retries', labelFa: '۲۰ تلاش' },
        { value: 0, label: 'Unlimited', labelFa: 'نامحدود' },
      ],
      triggerEvents: [
        { id: 'reconnectOnDPI', name: 'DPI Detection', nameFa: 'تشخیص DPI', enabled: reconnectState.reconnectOnDPI },
        { id: 'reconnectOnBlock', name: 'Core Blocked', nameFa: 'مسدود شدن هسته', enabled: reconnectState.reconnectOnBlock },
        { id: 'reconnectOnDnsLeak', name: 'DNS Leak', nameFa: 'نشت DNS', enabled: reconnectState.reconnectOnDnsLeak },
      ],
    },
    history: [
      {
        timestamp: Date.now() - 7200000,
        fromCore: 'hiddify',
        toCore: 'mahsang',
        reason: 'block',
        reasonFa: 'مسدود شدن',
        success: true,
        durationMs: 1800,
      },
      {
        timestamp: Date.now() - 5400000,
        fromCore: 'mahsang',
        toCore: 'xray-gfw',
        reason: 'dpi-detect',
        reasonFa: 'تشخیص DPI',
        success: true,
        durationMs: 2100,
      },
      {
        timestamp: Date.now() - 3600000,
        fromCore: 'xray-gfw',
        toCore: 'xray-gfw',
        reason: 'dns-leak',
        reasonFa: 'نشت DNS',
        success: true,
        durationMs: 1200,
      },
      {
        timestamp: Date.now() - 1800000,
        fromCore: 'defyxvpn',
        toCore: 'hiddify',
        reason: 'block',
        reasonFa: 'مسدود شدن',
        success: false,
        durationMs: 5000,
      },
    ],
    meta: {
      endpoint: '/api/auto-reconnect',
      descriptionFa: 'مدیریت اتصال مجدد خودکار',
    },
  });
}

// ──────────────────────────────────────────────
// POST /api/auto-reconnect
// body: { action: string, ...params }
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

    if (action === 'trigger-reconnect') {
      if (!reconnectState.enabled) {
        return NextResponse.json(
          {
            success: false,
            error: 'Auto-reconnect is disabled',
            errorFa: 'اتصال مجدد خودکار غیرفعال است',
          },
          { status: 400 },
        );
      }

      if (reconnectState.reconnectStatus === 'reconnecting') {
        return NextResponse.json({
          success: true,
          action: 'trigger-reconnect',
          status: 'already-reconnecting',
          message: 'Reconnect already in progress',
          messageFa: 'اتصال مجدد از قبل در حال انجام است',
        });
      }

      reconnectState.reconnectStatus = 'reconnecting';
      reconnectState.retryCount += 1;
      reconnectState.lastReconnectAttempt = Date.now();
      reconnectState.totalReconnectAttempts += 1;

      // Simulate reconnect outcome
      const success = Math.random() > 0.3;
      const reconnectTime = Math.round(1000 + Math.random() * 3000);

      if (success) {
        reconnectState.reconnectStatus = 'connected';
        reconnectState.retryCount = 0;
        reconnectState.totalSuccessfulReconnects += 1;
        reconnectState.averageReconnectTimeMs = Math.round(
          ((reconnectState.averageReconnectTimeMs * (reconnectState.totalSuccessfulReconnects - 1)) + reconnectTime) /
          reconnectState.totalSuccessfulReconnects,
        );
      } else {
        if (reconnectState.retryCount >= reconnectState.maxRetries) {
          reconnectState.reconnectStatus = 'failed';
        } else {
          reconnectState.reconnectStatus = 'reconnecting';
        }
      }

      return NextResponse.json({
        success: true,
        action: 'trigger-reconnect',
        reconnectSuccessful: success,
        reconnectTimeMs: reconnectTime,
        newStatus: reconnectState.reconnectStatus,
        retryCount: reconnectState.retryCount,
        state: reconnectState,
        message: success
          ? `Reconnect successful in ${reconnectTime}ms`
          : `Reconnect failed, retry ${reconnectState.retryCount}/${reconnectState.maxRetries}`,
        messageFa: success
          ? `اتصال مجدد در ${reconnectTime} میلی‌ثانیه موفق بود`
          : `اتصال مجدد ناموفق، تلاش ${reconnectState.retryCount}/${reconnectState.maxRetries}`,
      });
    }

    if (action === 'toggle-enabled') {
      reconnectState.enabled = !reconnectState.enabled;
      if (reconnectState.enabled) {
        reconnectState.retryCount = 0;
        reconnectState.reconnectStatus = 'idle';
      }
      return NextResponse.json({
        success: true,
        action: 'toggle-enabled',
        enabled: reconnectState.enabled,
        state: reconnectState,
        message: reconnectState.enabled
          ? 'Auto-reconnect enabled'
          : 'Auto-reconnect disabled — you will need to reconnect manually',
        messageFa: reconnectState.enabled
          ? 'اتصال مجدد خودکار فعال شد'
          : 'اتصال مجدد خودکار غیرفعال شد — باید دستی وصل شوید',
      });
    }

    if (action === 'update-settings') {
      const {
        maxRetries,
        retryInterval,
        exponentialBackoff,
        fallbackToPsiphon,
        reconnectOnDPI,
        reconnectOnBlock,
        reconnectOnDnsLeak,
      } = body as {
        maxRetries?: number;
        retryInterval?: number;
        exponentialBackoff?: boolean;
        fallbackToPsiphon?: boolean;
        reconnectOnDPI?: boolean;
        reconnectOnBlock?: boolean;
        reconnectOnDnsLeak?: boolean;
      };

      if (typeof maxRetries === 'number' && maxRetries >= 0) {
        reconnectState.maxRetries = maxRetries;
      }
      if (typeof retryInterval === 'number' && retryInterval >= 1000 && retryInterval <= 60000) {
        reconnectState.retryInterval = retryInterval;
      }
      if (typeof exponentialBackoff === 'boolean') {
        reconnectState.exponentialBackoff = exponentialBackoff;
      }
      if (typeof fallbackToPsiphon === 'boolean') {
        reconnectState.fallbackToPsiphon = fallbackToPsiphon;
      }
      if (typeof reconnectOnDPI === 'boolean') {
        reconnectState.reconnectOnDPI = reconnectOnDPI;
      }
      if (typeof reconnectOnBlock === 'boolean') {
        reconnectState.reconnectOnBlock = reconnectOnBlock;
      }
      if (typeof reconnectOnDnsLeak === 'boolean') {
        reconnectState.reconnectOnDnsLeak = reconnectOnDnsLeak;
      }

      return NextResponse.json({
        success: true,
        action: 'update-settings',
        state: reconnectState,
        message: 'Auto-reconnect settings updated',
        messageFa: 'تنظیمات اتصال مجدد خودکار به‌روز شد',
      });
    }

    if (action === 'reset-retry-count') {
      reconnectState.retryCount = 0;
      reconnectState.reconnectStatus = 'idle';
      return NextResponse.json({
        success: true,
        action: 'reset-retry-count',
        state: reconnectState,
        message: 'Retry count reset to 0',
        messageFa: 'شمارنده تلاش‌ها به صفر بازنشانی شد',
      });
    }

    return NextResponse.json(
      {
        success: false,
        error: `Unknown action: ${action}. Valid actions: trigger-reconnect, toggle-enabled, update-settings, reset-retry-count`,
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
