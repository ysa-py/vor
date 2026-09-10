import { NextRequest, NextResponse } from 'next/server';

// ──────────────────────────────────────────────
// Kill Switch state (in-memory)
// ──────────────────────────────────────────────
interface KillSwitchState {
  enabled: boolean;
  blockAllOnDisconnect: boolean;
  allowedApps: string[];
  networkLock: boolean;
  lastTriggered: number | null;
  triggerCount: number;
  mode: 'full' | 'selective';
  dnsLeakProtection: boolean;
  ipv6LeakProtection: boolean;
  webRtcLeakProtection: boolean;
}

const killSwitchState: KillSwitchState = {
  enabled: true,
  blockAllOnDisconnect: true,
  allowedApps: [],
  networkLock: true,
  lastTriggered: Date.now() - 7200000,
  triggerCount: 3,
  mode: 'full',
  dnsLeakProtection: true,
  ipv6LeakProtection: true,
  webRtcLeakProtection: true,
};

const ALLOWED_APPS_OPTIONS = [
  { id: 'telegram', name: 'Telegram', nameFa: 'تلگرام', packageName: 'org.telegram.messenger' },
  { id: 'whatsapp', name: 'WhatsApp', nameFa: 'واتساپ', packageName: 'com.whatsapp' },
  { id: 'signal', name: 'Signal', nameFa: 'سیگنال', packageName: 'org.thoughtcrime.securesms' },
  { id: 'banking', name: 'Banking Apps', nameFa: 'اپلیکیشن‌های بانکی', packageName: 'ir.bank.*' },
  { id: 'imessage', name: 'iMessage', nameFa: 'آیمسیج', packageName: 'com.apple.imessage' },
];

// ──────────────────────────────────────────────
// GET /api/kill-switch
// ──────────────────────────────────────────────
export async function GET() {
  return NextResponse.json({
    success: true,
    timestamp: Date.now(),
    state: killSwitchState,
    config: {
      description: 'Kill switch blocks all network traffic when VPN disconnects unexpectedly',
      descriptionFa: 'کلید قطع ارتباط، تمام ترافیک شبکه را در صورت قطع غیرمنتظره VPN مسدود می‌کند',
      modeOptions: [
        { value: 'full', label: 'Full Block', labelFa: 'مسدودسازی کامل', description: 'Block all traffic when VPN disconnects', descriptionFa: 'مسدودسازی تمام ترافیک هنگام قطع VPN' },
        { value: 'selective', label: 'Selective Block', labelFa: 'مسدودسازی انتخابی', description: 'Only block non-allowed apps', descriptionFa: 'فقط اپلیکیشن‌های غیرمجاز مسدود می‌شوند' },
      ],
      allowedAppsOptions: ALLOWED_APPS_OPTIONS,
    },
    audit: {
      lastTriggered: killSwitchState.lastTriggered,
      triggerCount: killSwitchState.triggerCount,
      lastTriggeredFa: killSwitchState.lastTriggered
        ? new Date(killSwitchState.lastTriggered).toLocaleString('fa-IR')
        : 'هرگز',
    },
    meta: {
      endpoint: '/api/kill-switch',
      descriptionFa: 'مدیریت کلید قطع ارتباط',
    },
  });
}

// ──────────────────────────────────────────────
// POST /api/kill-switch
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

    if (action === 'toggle-enabled') {
      killSwitchState.enabled = !killSwitchState.enabled;
      return NextResponse.json({
        success: true,
        action: 'toggle-enabled',
        enabled: killSwitchState.enabled,
        state: killSwitchState,
        message: killSwitchState.enabled
          ? 'Kill switch enabled — network traffic will be blocked on VPN disconnect'
          : 'Kill switch disabled — WARNING: traffic may leak on VPN disconnect',
        messageFa: killSwitchState.enabled
          ? 'کلید قطع فعال شد — ترافیک شبکه در صورت قطع VPN مسدود خواهد شد'
          : 'کلید قطع غیرفعال شد — هشدار: ترافیک ممکن است در صورت قطع VPN نشت کند',
      });
    }

    if (action === 'toggle-network-lock') {
      killSwitchState.networkLock = !killSwitchState.networkLock;
      return NextResponse.json({
        success: true,
        action: 'toggle-network-lock',
        networkLock: killSwitchState.networkLock,
        state: killSwitchState,
        message: killSwitchState.networkLock
          ? 'Network lock enabled — OS-level firewall active'
          : 'Network lock disabled',
        messageFa: killSwitchState.networkLock
          ? 'قفل شبکه فعال شد — فایروال سطح سیستم‌عامل فعال'
          : 'قفل شبکه غیرفعال شد',
      });
    }

    if (action === 'toggle-dns-leak') {
      killSwitchState.dnsLeakProtection = !killSwitchState.dnsLeakProtection;
      return NextResponse.json({
        success: true,
        action: 'toggle-dns-leak',
        dnsLeakProtection: killSwitchState.dnsLeakProtection,
        state: killSwitchState,
        message: killSwitchState.dnsLeakProtection
          ? 'DNS leak protection enabled'
          : 'DNS leak protection disabled — WARNING: DNS queries may leak',
        messageFa: killSwitchState.dnsLeakProtection
          ? 'محافظت از نشت DNS فعال شد'
          : 'محافظت از نشت DNS غیرفعال شد — هشدار: درخواست‌های DNS ممکن است نشت کنند',
      });
    }

    if (action === 'toggle-ipv6-leak') {
      killSwitchState.ipv6LeakProtection = !killSwitchState.ipv6LeakProtection;
      return NextResponse.json({
        success: true,
        action: 'toggle-ipv6-leak',
        ipv6LeakProtection: killSwitchState.ipv6LeakProtection,
        state: killSwitchState,
        message: killSwitchState.ipv6LeakProtection
          ? 'IPv6 leak protection enabled'
          : 'IPv6 leak protection disabled',
        messageFa: killSwitchState.ipv6LeakProtection
          ? 'محافظت از نشت IPv6 فعال شد'
          : 'محافظت از نشت IPv6 غیرفعال شد',
      });
    }

    if (action === 'toggle-webrtc-leak') {
      killSwitchState.webRtcLeakProtection = !killSwitchState.webRtcLeakProtection;
      return NextResponse.json({
        success: true,
        action: 'toggle-webrtc-leak',
        webRtcLeakProtection: killSwitchState.webRtcLeakProtection,
        state: killSwitchState,
        message: killSwitchState.webRtcLeakProtection
          ? 'WebRTC leak protection enabled'
          : 'WebRTC leak protection disabled',
        messageFa: killSwitchState.webRtcLeakProtection
          ? 'محافظت از نشت WebRTC فعال شد'
          : 'محافظت از نشت WebRTC غیرفعال شد',
      });
    }

    if (action === 'set-mode') {
      const { mode } = body as { mode?: string };
      if (mode !== 'full' && mode !== 'selective') {
        return NextResponse.json(
          {
            success: false,
            error: 'Invalid mode. Must be "full" or "selective"',
            errorFa: 'حالت نامعتبر. باید "full" یا "selective" باشد',
          },
          { status: 400 },
        );
      }
      killSwitchState.mode = mode;
      return NextResponse.json({
        success: true,
        action: 'set-mode',
        mode: killSwitchState.mode,
        state: killSwitchState,
        message: `Kill switch mode set to ${mode}`,
        messageFa: `حالت کلید قطع به ${mode === 'full' ? 'کامل' : 'انتخابی'} تنظیم شد`,
      });
    }

    if (action === 'set-allowed-apps') {
      const { apps } = body as { apps?: string[] };
      if (!apps || !Array.isArray(apps)) {
        return NextResponse.json(
          {
            success: false,
            error: 'Invalid apps. Must be an array of app IDs',
            errorFa: 'اپلیکیشن‌ها نامعتبر. باید آرایه‌ای از شناسه اپلیکیشن باشد',
          },
          { status: 400 },
        );
      }
      const validAppIds = ALLOWED_APPS_OPTIONS.map((a) => a.id);
      const invalidApps = apps.filter((a: string) => !validAppIds.includes(a));
      if (invalidApps.length > 0) {
        return NextResponse.json(
          {
            success: false,
            error: `Invalid app IDs: ${invalidApps.join(', ')}. Valid: ${validAppIds.join(', ')}`,
            errorFa: 'شناسه اپلیکیشن نامعتبر',
          },
          { status: 400 },
        );
      }
      killSwitchState.allowedApps = apps;
      return NextResponse.json({
        success: true,
        action: 'set-allowed-apps',
        allowedApps: killSwitchState.allowedApps,
        state: killSwitchState,
        message: `Allowed apps updated: ${apps.join(', ')}`,
        messageFa: `اپلیکیشن‌های مجاز به‌روز شد`,
      });
    }

    if (action === 'trigger-test') {
      killSwitchState.lastTriggered = Date.now();
      killSwitchState.triggerCount += 1;
      return NextResponse.json({
        success: true,
        action: 'trigger-test',
        triggered: true,
        state: killSwitchState,
        message: 'Kill switch test triggered successfully — all non-VPN traffic should be blocked',
        messageFa: 'تست کلید قطع با موفقیت فعال شد — تمام ترافیک غیر VPN باید مسدود شود',
      });
    }

    return NextResponse.json(
      {
        success: false,
        error: `Unknown action: ${action}. Valid actions: toggle-enabled, toggle-network-lock, toggle-dns-leak, toggle-ipv6-leak, toggle-webrtc-leak, set-mode, set-allowed-apps, trigger-test`,
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
