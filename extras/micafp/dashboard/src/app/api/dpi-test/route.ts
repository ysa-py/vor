import { NextRequest, NextResponse } from 'next/server';

// ──────────────────────────────────────────────
// DPI Signatures (from unified-shield-types.ts IRAN_DPI_SIGNATURES)
// ──────────────────────────────────────────────
const IRAN_DPI_SIGNATURES = [
  { signature: 'TLS-ClientHello-Reset', descriptionFa: 'بازنشانی ClientHello TLS — شایع‌ترین روش DPI ایران', hex: '16 03 01', severity: 'critical' as const },
  { signature: 'HTTP-403-Block', descriptionFa: 'صفحه ۴۰۳ ایرانی — مسدودسازی HTTP', hex: '48 54 54 50 2F 31 2E 31 20 34 30 33', severity: 'high' as const },
  { signature: 'Null-Route', descriptionFa: 'مسیریابی صفر — قطعی بی‌صدا', hex: '00 00 00 00', severity: 'medium' as const },
  { signature: 'SNI-Filter', descriptionFa: 'فیلتر SNI — بررسی نام سرور در TLS', hex: 'SNI-Filter-Detected', severity: 'critical' as const },
  { signature: 'DNS-Poison', descriptionFa: 'مسمومیت DNS — پاسخ جعلی DNS', hex: 'DNS-Poison-Response', severity: 'high' as const },
  { signature: 'Protocol-Detect', descriptionFa: 'تشخیص پروتکل — شناسایی الگوی پروتکل', hex: 'Protocol-Pattern-Match', severity: 'medium' as const },
];

const CORE_PROFILES = [
  { coreId: 'hiddify', coreName: 'hiddify-core', coreNameFa: 'هیدیفای', primaryProtocol: 'vless-reality-xtls', baseLatency: 85 },
  { coreId: 'xray-gfw', coreName: 'GFW-knocker/Xray-core', coreNameFa: 'ایکس‌ری GFW', primaryProtocol: 'vless-fragment', baseLatency: 62 },
  { coreId: 'sing-box', coreName: 'sing-box', coreNameFa: 'سینگ‌باکس', primaryProtocol: 'hysteria2', baseLatency: 73 },
  { coreId: 'amneziavpn', coreName: 'AmneziaVPN (awg-go)', coreNameFa: 'آمنزیاوی‌پی‌ان', primaryProtocol: 'amneziawg-1.5', baseLatency: 91 },
  { coreId: 'defyxvpn', coreName: 'DefyxVPN', coreNameFa: 'دیفیکسوی‌پی‌ان', primaryProtocol: 'defyxvpn-layers', baseLatency: 105 },
  { coreId: 'moav', coreName: 'MoaV', coreNameFa: 'موآوی', primaryProtocol: 'moav-tunnel', baseLatency: 118 },
  { coreId: 'lantern', coreName: 'Lantern', coreNameFa: 'لنترن', primaryProtocol: 'lantern-df-pt', baseLatency: 142 },
  { coreId: 'mahsang', coreName: 'MahsaNG core', coreNameFa: 'مهساان‌جی', primaryProtocol: 'mahsang-obfs', baseLatency: 79 },
  { coreId: 'psiphon', coreName: 'Psiphon Tunnel Core', coreNameFa: 'سایفون', primaryProtocol: 'psiphon-ssh-obfs', baseLatency: 156 },
];

const TEST_ENDPOINTS = [
  { id: 'te-1', url: 'https://www.youtube.com', nameFa: 'یوتیوب', category: 'video-streaming', categoryFa: 'استریم ویدیو' },
  { id: 'te-2', url: 'https://twitter.com', nameFa: 'توییتر/ایکس', category: 'social-media', categoryFa: 'شبکه اجتماعی' },
  { id: 'te-3', url: 'https://www.instagram.com', nameFa: 'اینستاگرام', category: 'social-media', categoryFa: 'شبکه اجتماعی' },
  { id: 'te-4', url: 'https://telegram.org', nameFa: 'تلگرام', category: 'messaging', categoryFa: 'پیام‌رسان' },
  { id: 'te-5', url: 'https://www.google.com', nameFa: 'گوگل', category: 'search-engine', categoryFa: 'موتور جستجو' },
  { id: 'te-6', url: 'https://discord.com', nameFa: 'دیسکورد', category: 'messaging', categoryFa: 'پیام‌رسان' },
  { id: 'te-7', url: 'https://github.com', nameFa: 'گیت‌هاب', category: 'development', categoryFa: 'توسعه' },
  { id: 'te-8', url: 'https://www.wikipedia.org', nameFa: 'ویکی‌پدیا', category: 'reference', categoryFa: 'مرجع' },
];

// ──────────────────────────────────────────────
// GET /api/dpi-test
// ──────────────────────────────────────────────
export async function GET() {
  return NextResponse.json({
    success: true,
    timestamp: Date.now(),
    signatures: IRAN_DPI_SIGNATURES,
    signatureCount: IRAN_DPI_SIGNATURES.length,
    testEndpoints: TEST_ENDPOINTS,
    testEndpointCount: TEST_ENDPOINTS.length,
    supportedCores: CORE_PROFILES.map((c) => ({
      coreId: c.coreId,
      coreName: c.coreName,
      coreNameFa: c.coreNameFa,
      primaryProtocol: c.primaryProtocol,
    })),
    meta: {
      endpoint: '/api/dpi-test',
      descriptionFa: 'شبیه‌سازی و تست DPI — بازرسی عمیق بسته‌ها',
    },
  });
}

// ──────────────────────────────────────────────
// POST /api/dpi-test
// body: { coreIds?: string[], testAll?: boolean }
// ──────────────────────────────────────────────
export async function POST(request: NextRequest) {
  try {
    const body = await request.json();
    const { coreIds, testAll } = body as { coreIds?: string[]; testAll?: boolean };

    const validCoreIds = CORE_PROFILES.map((c) => c.coreId);
    let targets = CORE_PROFILES;

    if (testAll) {
      targets = CORE_PROFILES;
    } else if (coreIds && Array.isArray(coreIds)) {
      const invalidIds = coreIds.filter((id: string) => !validCoreIds.includes(id));
      if (invalidIds.length > 0) {
        return NextResponse.json(
          {
            success: false,
            error: `Invalid core IDs: ${invalidIds.join(', ')}. Valid IDs: ${validCoreIds.join(', ')}`,
            errorFa: 'شناسه‌های هسته نامعتبر',
          },
          { status: 400 },
        );
      }
      targets = CORE_PROFILES.filter((c) => coreIds.includes(c.coreId));
    }

    if (targets.length === 0) {
      return NextResponse.json(
        {
          success: false,
          error: 'No cores selected for DPI test. Provide coreIds array or set testAll to true.',
          errorFa: 'هسته‌ای برای تست DPI انتخاب نشده. coreIds یا testAll را مشخص کنید.',
        },
        { status: 400 },
      );
    }

    const results = targets.map((core) => {
      const connected = Math.random() > 0.15;
      const bypassLevel: 'full' | 'partial' | 'none' = connected
        ? (Math.random() > 0.3 ? 'full' : 'partial')
        : 'none';
      const sigEntry = IRAN_DPI_SIGNATURES[Math.floor(Math.random() * IRAN_DPI_SIGNATURES.length)];
      const latency = connected
        ? Math.round(core.baseLatency + Math.random() * 30)
        : 0;

      return {
        coreId: core.coreId,
        coreName: core.coreName,
        coreNameFa: core.coreNameFa,
        connected,
        latency,
        protocol: core.primaryProtocol,
        bypassLevel,
        dpiSignature: sigEntry.signature,
        dpiSignatureFa: sigEntry.descriptionFa,
        dpiSeverity: sigEntry.severity,
        timestamp: Date.now(),
        details: {
          sniFilterBypass: bypassLevel !== 'none',
          tlsFragmentEffective: core.primaryProtocol.includes('fragment') || core.primaryProtocol.includes('reality'),
          junkPacketInjected: core.primaryProtocol === 'amneziawg-1.5',
          quicSupported: core.primaryProtocol === 'hysteria2' || core.primaryProtocol === 'tuic-v5',
          domainFronting: core.primaryProtocol.includes('psiphon') || core.primaryProtocol.includes('lantern'),
          packetLossPercent: connected ? Math.round(Math.random() * 5 * 100) / 100 : 100,
          jitterMs: connected ? Math.round(Math.random() * 20) : 0,
        },
      };
    });

    const summary = {
      totalTested: results.length,
      fullBypass: results.filter((r) => r.bypassLevel === 'full').length,
      partialBypass: results.filter((r) => r.bypassLevel === 'partial').length,
      noBypass: results.filter((r) => r.bypassLevel === 'none').length,
      avgLatency: Math.round(
        results.filter((r) => r.connected).reduce((sum, r) => sum + r.latency, 0) /
        Math.max(1, results.filter((r) => r.connected).length),
      ),
      bestCore: results
        .filter((r) => r.connected)
        .sort((a, b) => a.latency - b.latency)[0]?.coreId ?? 'none',
    };

    return NextResponse.json({
      success: true,
      timestamp: Date.now(),
      testDurationMs: Math.round(500 + Math.random() * 2000),
      results,
      summary,
      message: `DPI test completed for ${targets.length} core(s)`,
      messageFa: `تست DPI برای ${targets.length} هسته انجام شد`,
    });
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
