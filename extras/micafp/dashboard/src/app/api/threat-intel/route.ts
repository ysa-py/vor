import { NextRequest, NextResponse } from 'next/server';

// ──────────────────────────────────────────────
// Threat Intelligence state (in-memory)
// ──────────────────────────────────────────────
interface ThreatEntry {
  id: string;
  type: string;
  typeFa: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  description: string;
  descriptionFa: string;
  detectedAt: number;
  mitigated: boolean;
  countermeasure: string;
  countermeasureFa: string;
  affectedCores: string[];
  ispSpecific: boolean;
  region: string;
}

const activeThreats: ThreatEntry[] = [
  {
    id: 'threat-1',
    type: 'DPI Deep Packet Inspection',
    typeFa: 'بازرسی عمیق بسته‌ها (DPI)',
    severity: 'high',
    description: 'Active TLS SNI filtering detected on current ISP — ClientHello packets with targeted SNI fields are being reset by mid-network DPI boxes',
    descriptionFa: 'فیلترینگ SNI فعال روی ISP فعلی شناسایی شد — بسته‌های ClientHello با فیلدهای SNI هدف توسط جعبه‌های DPI میانی بازنشانی می‌شوند',
    detectedAt: Date.now() - 7200000,
    mitigated: true,
    countermeasure: 'VLESS Reality + XTLS',
    countermeasureFa: 'VLESS Reality + XTLS — تقلید دست‌دهی Reality',
    affectedCores: ['hiddify', 'xray-gfw', 'mahsang'],
    ispSpecific: true,
    region: 'iran',
  },
  {
    id: 'threat-2',
    type: 'DNS Poisoning',
    typeFa: 'مسمومیت DNS',
    severity: 'critical',
    description: 'DNS responses being tampered with — fake A records injected for targeted domains including social media and messaging platforms, returning Iranian government-controlled IPs',
    descriptionFa: 'پاسخ‌های DNS دستکاری می‌شوند — رکوردهای A جعلی برای دامنه‌های هدف شامل شبکه‌های اجتماعی و پیام‌رسان‌ها تزریق شده و IP‌های کنترل‌شده توسط دولت ایران برگردانده می‌شود',
    detectedAt: Date.now() - 5400000,
    mitigated: true,
    countermeasure: 'DNS over HTTPS (DoH) via Cloudflare',
    countermeasureFa: 'DNS over HTTPS (DoH) از طریق کلودفلر',
    affectedCores: ['hiddify', 'sing-box', 'xray-gfw'],
    ispSpecific: true,
    region: 'iran',
  },
  {
    id: 'threat-3',
    type: 'Protocol Fingerprinting',
    typeFa: 'اثر انگشت پروتکل',
    severity: 'high',
    description: 'WireGuard handshake pattern detected by DPI — the distinctive 148-byte initiation packets are being identified and blocked by Iran\'s national filtering infrastructure (FATAFilter)',
    descriptionFa: 'الگوی دست‌دهی WireGuard توسط DPI شناسایی شد — بسته‌های شروع‌کننده متمایز ۱۴۸ بایتی توسط زیرساخت فیلترینگ ملی ایران (FATAFilter) شناسایی و مسدود می‌شوند',
    detectedAt: Date.now() - 1800000,
    mitigated: true,
    countermeasure: 'AmneziaWG junk packet injection',
    countermeasureFa: 'تزریق بسته‌های جونک آمنزیاوی‌جی — پنهان‌سازی الگوی دست‌دهی',
    affectedCores: ['amneziavpn', 'defyxvpn'],
    ispSpecific: true,
    region: 'iran',
  },
  {
    id: 'threat-4',
    type: 'IP Blocking (GFW-style)',
    typeFa: 'مسدودسازی IP (سبک GFW)',
    severity: 'medium',
    description: 'Several VPN server IPs blocked using Great Firewall-style IP blocking — destination IP ranges added to national firewall blacklist, causing TCP RST and ICMP unreachable responses',
    descriptionFa: 'چندین IP سرور VPN با روش مسدودسازی IP سبک دیوار آتش کبیر مسدود شده — محدوده‌های IP مقصد به لیست سیاه فایروال ملی اضافه شده و پاسخ‌های TCP RST و ICMP غیرقابل دسترسی ارسال می‌شود',
    detectedAt: Date.now() - 3600000,
    mitigated: false,
    countermeasure: 'Domain fronting via CDN + multi-path routing',
    countermeasureFa: 'فرانتینگ دامنه از طریق CDN + مسیریابی چندمسیره',
    affectedCores: ['lantern', 'psiphon', 'hiddify'],
    ispSpecific: false,
    region: 'iran',
  },
  {
    id: 'threat-5',
    type: 'Null Routing (Blackholing)',
    typeFa: 'مسیریابی صفر (بلک‌هولینگ)',
    severity: 'medium',
    description: 'Silent packet dropping on specific routes — traffic to certain international IP ranges is being null-routed at ISP level without any error response, causing connection timeouts',
    descriptionFa: 'رها کردن بی‌صدا بسته‌ها در مسیرهای خاص — ترافیک به محدوده‌های IP بین‌المللی خاص در سطح ISP بدون هیچ پاسخ خطایی مسیریابی صفر می‌شود و باعث تایم‌اوت اتصال می‌گردد',
    detectedAt: Date.now() - 900000,
    mitigated: false,
    countermeasure: 'Multi-path routing with automatic failover + Psiphon fallback',
    countermeasureFa: 'مسیریابی چندمسیره با بکاپ خودکار + بکاپ سایفون',
    affectedCores: ['moav', 'lantern', 'psiphon'],
    ispSpecific: true,
    region: 'iran',
  },
];

type ThreatLevel = 'low' | 'medium' | 'high' | 'critical';

const threatIntelState = {
  lastScan: Date.now() - 1800000,
  threatLevel: 'high' as ThreatLevel,
  dpiPatternsUpdated: '2026.05.23-r2',
  blockedDomainsCount: 1247,
  activeCountermeasures: [
    'VLESS Reality + XTLS',
    'DNS over HTTPS (DoH)',
    'AmneziaWG junk packets',
    'TLS fragmentation',
    'Domain fronting via CDN',
  ],
  activeCountermeasuresFa: [
    'VLESS Reality + XTLS',
    'DNS over HTTPS (DoH)',
    'بسته‌های جونک آمنزیاوی‌جی',
    'تقسیم Fragment TLS',
    'فرانتینگ دامنه از طریق CDN',
  ],
  scanIntervalMs: 300000,
  threatFeedSources: [
    { id: 'iran-dpi-db', name: 'Iran DPI Signatures DB', nameFa: 'پایگاه داده امضاهای DPI ایران', lastUpdate: '2026.05.23-r2' },
    { id: 'gfw-list', name: 'GFW Domain List', nameFa: 'لیست دامنه‌های دیوار آتش', lastUpdate: '2026.05.22' },
    { id: 'blocked-ips', name: 'Iran Blocked IP Ranges', nameFa: 'محدوده‌های IP مسدود ایران', lastUpdate: '2026.05.23' },
  ],
};

// ──────────────────────────────────────────────
// GET /api/threat-intel
// ──────────────────────────────────────────────
export async function GET() {
  const mitigatedCount = activeThreats.filter((t) => t.mitigated).length;
  const unmitigatedCount = activeThreats.filter((t) => !t.mitigated).length;

  return NextResponse.json({
    success: true,
    timestamp: Date.now(),
    threatLevel: threatIntelState.threatLevel,
    threatLevelFa: threatIntelState.threatLevel === 'low'
      ? 'پایین'
      : threatIntelState.threatLevel === 'medium'
        ? 'متوسط'
        : threatIntelState.threatLevel === 'high'
          ? 'بالا'
          : 'بحرانی',
    activeThreats,
    activeThreatCount: activeThreats.length,
    mitigatedCount,
    unmitigatedCount,
    state: threatIntelState,
    severityBreakdown: {
      critical: activeThreats.filter((t) => t.severity === 'critical').length,
      high: activeThreats.filter((t) => t.severity === 'high').length,
      medium: activeThreats.filter((t) => t.severity === 'medium').length,
      low: activeThreats.filter((t) => t.severity === 'low').length,
    },
    countermeasures: threatIntelState.activeCountermeasures.map((cm, idx) => ({
      id: `cm-${idx + 1}`,
      name: cm,
      nameFa: threatIntelState.activeCountermeasuresFa[idx] ?? cm,
      active: true,
    })),
    recommendations: {
      immediate: [
        { action: 'Switch to VLESS Reality core', actionFa: 'تعویض به هسته VLESS Reality', priority: 'high' },
        { action: 'Enable DNS over HTTPS', actionFa: 'فعال‌سازی DNS over HTTPS', priority: 'high' },
        { action: 'Update DPI signatures to latest', actionFa: 'به‌روزرسانی امضاهای DPI به آخرین نسخه', priority: 'medium' },
      ],
      preventive: [
        { action: 'Enable shadow connections for redundancy', actionFa: 'فعال‌سازی اتصالات سایه برای افزونگی', priority: 'medium' },
        { action: 'Configure AmneziaWG for WireGuard-based connections', actionFa: 'پیکربندی آمنزیاوی‌جی برای اتصالات مبتنی بر وایرگارد', priority: 'medium' },
        { action: 'Enable proactive AI switching', actionFa: 'فعال‌سازی تعویض پیشگیرانه هوش مصنوعی', priority: 'low' },
      ],
    },
    meta: {
      endpoint: '/api/threat-intel',
      descriptionFa: 'اطلاعات تهدید و اقدامات متقابل',
    },
  });
}

// ──────────────────────────────────────────────
// POST /api/threat-intel
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

    if (action === 'scan') {
      threatIntelState.lastScan = Date.now();

      // Simulate potential new threat detection
      const newThreatDetected = Math.random() > 0.7;
      const scanDuration = Math.round(2000 + Math.random() * 3000);

      return NextResponse.json({
        success: true,
        action: 'scan',
        scanDurationMs: scanDuration,
        threatsFound: activeThreats.length,
        newThreatDetected,
        lastScan: threatIntelState.lastScan,
        threatLevel: threatIntelState.threatLevel,
        message: newThreatDetected
          ? `Scan complete — new threat detected, threat level: ${threatIntelState.threatLevel}`
          : `Scan complete — no new threats, threat level: ${threatIntelState.threatLevel}`,
        messageFa: newThreatDetected
          ? `اسکن کامل — تهدید جدید شناسایی شد، سطح تهدید: ${threatIntelState.threatLevel === 'high' ? 'بالا' : 'بحرانی'}`
          : `اسکن کامل — تهدید جدیدی شناسایی نشد، سطح تهدید: بالا`,
      });
    }

    if (action === 'mitigate') {
      const { threatId } = body as { threatId?: string };
      if (!threatId) {
        return NextResponse.json(
          {
            success: false,
            error: 'Missing required field: threatId',
            errorFa: 'فیلد ضروری موجود نیست: threatId',
          },
          { status: 400 },
        );
      }

      const threat = activeThreats.find((t) => t.id === threatId);
      if (!threat) {
        return NextResponse.json(
          {
            success: false,
            error: `Threat not found: ${threatId}`,
            errorFa: `تهدید یافت نشد: ${threatId}`,
          },
          { status: 404 },
        );
      }

      threat.mitigated = true;

      // Recalculate threat level
      const unmitigated = activeThreats.filter((t) => !t.mitigated);
      const hasCritical = unmitigated.some((t) => t.severity === 'critical');
      const hasHigh = unmitigated.some((t) => t.severity === 'high');

      if (unmitigated.length === 0) {
        threatIntelState.threatLevel = 'low';
      } else if (hasCritical) {
        threatIntelState.threatLevel = 'critical';
      } else if (hasHigh) {
        threatIntelState.threatLevel = 'high';
      } else {
        threatIntelState.threatLevel = 'medium';
      }

      return NextResponse.json({
        success: true,
        action: 'mitigate',
        threatId,
        countermeasure: threat.countermeasure,
        countermeasureFa: threat.countermeasureFa,
        newThreatLevel: threatIntelState.threatLevel,
        message: `Threat ${threatId} mitigated using ${threat.countermeasure}`,
        messageFa: `تهدید ${threatId} با ${threat.countermeasureFa} خنثی شد`,
      });
    }

    if (action === 'update-signatures') {
      threatIntelState.dpiPatternsUpdated = '2026.05.23-r3';
      threatIntelState.blockedDomainsCount += Math.floor(Math.random() * 50);

      return NextResponse.json({
        success: true,
        action: 'update-signatures',
        newVersion: threatIntelState.dpiPatternsUpdated,
        blockedDomainsCount: threatIntelState.blockedDomainsCount,
        message: 'DPI signatures updated successfully',
        messageFa: 'امضاهای DPI با موفقیت به‌روز شد',
      });
    }

    return NextResponse.json(
      {
        success: false,
        error: `Unknown action: ${action}. Valid actions: scan, mitigate, update-signatures`,
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
