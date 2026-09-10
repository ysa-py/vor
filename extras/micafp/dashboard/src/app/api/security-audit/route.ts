import { NextRequest, NextResponse } from 'next/server';
import {
  buildInitialSecurityAuditState,
  runFullSecurityAudit,
  performDNSLeakTest,
  detectWebRTCLeak,
  detectIPv6Leak,
  verifyKillSwitch,
  assessEncryption,
  calculatePrivacyScore,
  generateSecurityRecommendations,
  updateRealTimeSecurityStatus,
} from '@/lib/security-audit';

// ──────────────────────────────────────────────
// In-memory security audit state
// ──────────────────────────────────────────────
let securityAuditState = buildInitialSecurityAuditState(
  true,
  'xray-gfw',
  'doh',
  true,
  true,
  true,
);

// ──────────────────────────────────────────────
// GET /api/security-audit
// Returns current audit results and security status
// ──────────────────────────────────────────────
export async function GET() {
  const criticalRecs = securityAuditState.recommendations.filter(
    (r) => r.severity === 'critical' && !r.implemented,
  );
  const warningRecs = securityAuditState.recommendations.filter(
    (r) => r.severity === 'warning' && !r.implemented,
  );

  return NextResponse.json({
    success: true,
    timestamp: Date.now(),

    overallStatus: {
      status: securityAuditState.overallSecurityStatus,
      statusFa: securityAuditState.overallSecurityStatusFa,
      privacyScore: securityAuditState.privacyScore,
      privacyScoreLabel: securityAuditState.privacyScoreLabel,
      privacyScoreLabelFa: securityAuditState.privacyScoreLabelFa,
      lastAuditTime: securityAuditState.lastAuditTime,
      isRunning: securityAuditState.isRunning,
      realTimeMonitoring: securityAuditState.realTimeMonitoring,
      labelStatusFa: 'وضعیت کلی',
      labelScoreFa: 'امتیاز حریم خصوصی',
    },

    dnsLeak: {
      ...securityAuditState.dnsLeak,
      labelIsLeakingFa: securityAuditState.dnsLeak.isLeaking ? 'نشت دارد' : 'بدون نشت',
      labelDetectedServersFa: 'سرورهای شناسایی‌شده',
      labelLeakCountFa: 'تعداد نشت',
    },

    webrtcLeak: {
      ...securityAuditState.webrtcLeak,
      labelIsLeakingFa: securityAuditState.webrtcLeak.isLeaking ? 'نشت دارد' : 'بدون نشت',
      labelDetectedIPsFa: 'IP‌های شناسایی‌شده',
    },

    ipv6Leak: {
      ...securityAuditState.ipv6Leak,
      labelIsLeakingFa: securityAuditState.ipv6Leak.isLeaking ? 'نشت دارد' : 'بدون نشت',
    },

    killSwitch: {
      verified: securityAuditState.killSwitchVerified,
      details: securityAuditState.killSwitchDetails,
      detailsFa: securityAuditState.killSwitchDetailsFa,
      labelVerifiedFa: securityAuditState.killSwitchVerified ? 'تأییدشده' : 'تأییدنشده',
    },

    encryption: {
      ...securityAuditState.encryptionAssessment,
      labelProtocolFa: 'پروتکل',
      labelKeyExchangeFa: 'تبادل کلید',
      labelCipherFa: 'رمزنگاری',
      labelStrengthFa: 'قدرت',
    },

    recommendations: {
      total: securityAuditState.recommendations.length,
      critical: criticalRecs.length,
      warning: warningRecs.length,
      items: securityAuditState.recommendations,
      labelCriticalFa: 'بحرانی',
      labelWarningFa: 'هشدار',
      labelTotalFa: 'کل پیشنهادات',
    },

    scoreBreakdown: {
      dns: securityAuditState.dnsLeak.isLeaking ? 10 : 25,
      webrtc: securityAuditState.webrtcLeak.isLeaking ? 5 : 20,
      ipv6: securityAuditState.ipv6Leak.isLeaking ? 3 : 15,
      killSwitch: securityAuditState.killSwitchVerified ? 15 : 3,
      encryption: Math.round((securityAuditState.encryptionAssessment.score / 100) * 25),
      labelDnsFa: 'امنیت DNS',
      labelWebrtcFa: 'امنیت WebRTC',
      labelIpv6Fa: 'امنیت IPv6',
      labelKillSwitchFa: 'کلید کشت',
      labelEncryptionFa: 'رمزنگاری',
    },

    meta: {
      endpoint: '/api/security-audit',
      descriptionFa: 'ممیزی امنیتی — بررسی نشت DNS، WebRTC، IPv6 و ارزیابی رمزنگاری',
    },
  });
}

// ──────────────────────────────────────────────
// POST /api/security-audit
// body: { action: 'run-audit' | 'test-dns' | 'test-webrtc' | 'test-ipv6' | 'test-kill-switch', ...params }
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

    if (action === 'run-audit') {
      const {
        vpnConnected = true,
        activeCoreId = 'xray-gfw',
        dnsMode = 'doh',
        killSwitchEnabled = true,
        networkLockEnabled = true,
        ipv6Enabled = true,
      } = body as {
        vpnConnected?: boolean;
        activeCoreId?: string;
        dnsMode?: 'doh' | 'dot' | 'plain';
        killSwitchEnabled?: boolean;
        networkLockEnabled?: boolean;
        ipv6Enabled?: boolean;
      };

      // Mark as running
      securityAuditState = { ...securityAuditState, isRunning: true };

      // Simulate audit duration
      const auditDuration = 3000 + Math.floor(Math.random() * 5000);

      // Run the full audit
      securityAuditState = runFullSecurityAudit(
        securityAuditState,
        vpnConnected,
        activeCoreId,
        dnsMode,
        killSwitchEnabled,
        networkLockEnabled,
        ipv6Enabled,
      );

      const criticalCount = securityAuditState.recommendations.filter(
        (r) => r.severity === 'critical' && !r.implemented,
      ).length;

      return NextResponse.json({
        success: true,
        action: 'run-audit',
        auditDurationMs: auditDuration,
        privacyScore: securityAuditState.privacyScore,
        privacyScoreLabelFa: securityAuditState.privacyScoreLabelFa,
        overallStatus: securityAuditState.overallSecurityStatus,
        overallStatusFa: securityAuditState.overallSecurityStatusFa,
        criticalIssuesFound: criticalCount,
        testsPerformed: ['DNS Leak', 'WebRTC Leak', 'IPv6 Leak', 'Kill Switch', 'Encryption'],
        testsPerformedFa: ['نشت DNS', 'نشت WebRTC', 'نشت IPv6', 'کلید کشت', 'رمزنگاری'],
        message: criticalCount > 0
          ? `Audit complete — ${criticalCount} critical issue(s) found, privacy score: ${securityAuditState.privacyScore}/100`
          : `Audit complete — no critical issues, privacy score: ${securityAuditState.privacyScore}/100`,
        messageFa: criticalCount > 0
          ? `ممیزی کامل — ${criticalCount} مشکل بحرانی شناسایی شد، امتیاز حریم خصوصی: ${securityAuditState.privacyScore}/۱۰۰`
          : `ممیزی کامل — بدون مشکل بحرانی، امتیاز حریم خصوصی: ${securityAuditState.privacyScore}/۱۰۰`,
        results: {
          dnsLeak: securityAuditState.dnsLeak.isLeaking,
          webrtcLeak: securityAuditState.webrtcLeak.isLeaking,
          ipv6Leak: securityAuditState.ipv6Leak.isLeaking,
          killSwitchVerified: securityAuditState.killSwitchVerified,
          encryptionStrength: securityAuditState.encryptionAssessment.strength,
        },
        recommendations: securityAuditState.recommendations,
      });
    }

    if (action === 'test-dns') {
      const { vpnConnected = true, dnsMode = 'doh' } = body as {
        vpnConnected?: boolean;
        dnsMode?: 'doh' | 'dot' | 'plain';
      };

      const result = performDNSLeakTest(vpnConnected, dnsMode);
      securityAuditState = { ...securityAuditState, dnsLeak: result };

      // Recalculate privacy score
      const privacyResult = calculatePrivacyScore({
        dnsLeak: result,
        webrtcLeak: securityAuditState.webrtcLeak,
        ipv6Leak: securityAuditState.ipv6Leak,
        killSwitchVerified: securityAuditState.killSwitchVerified,
        encryption: securityAuditState.encryptionAssessment,
        vpnConnected,
      });
      securityAuditState.privacyScore = privacyResult.score;
      securityAuditState.privacyScoreLabel = privacyResult.label;
      securityAuditState.privacyScoreLabelFa = privacyResult.labelFa;
      securityAuditState.overallSecurityStatus = privacyResult.status;
      securityAuditState.overallSecurityStatusFa = privacyResult.statusFa;

      return NextResponse.json({
        success: true,
        action: 'test-dns',
        result,
        updatedPrivacyScore: privacyResult.score,
        message: result.isLeaking
          ? `DNS leak detected — ${result.leakCount} of ${result.totalQueries} queries leaked`
          : `No DNS leak — all ${result.totalQueries} queries secure`,
        messageFa: result.isLeaking
          ? `نشت DNS شناسایی شد — ${result.leakCount} از ${result.totalQueries} درخواست نشت کرد`
          : `بدون نشت DNS — تمام ${result.totalQueries} درخواست امن هستند`,
      });
    }

    if (action === 'test-webrtc') {
      const { vpnConnected = true, killSwitchEnabled = true } = body as {
        vpnConnected?: boolean;
        killSwitchEnabled?: boolean;
      };

      const result = detectWebRTCLeak(vpnConnected, killSwitchEnabled);
      securityAuditState = { ...securityAuditState, webrtcLeak: result };

      const privacyResult = calculatePrivacyScore({
        dnsLeak: securityAuditState.dnsLeak,
        webrtcLeak: result,
        ipv6Leak: securityAuditState.ipv6Leak,
        killSwitchVerified: securityAuditState.killSwitchVerified,
        encryption: securityAuditState.encryptionAssessment,
        vpnConnected,
      });
      securityAuditState.privacyScore = privacyResult.score;
      securityAuditState.overallSecurityStatus = privacyResult.status;
      securityAuditState.overallSecurityStatusFa = privacyResult.statusFa;

      return NextResponse.json({
        success: true,
        action: 'test-webrtc',
        result,
        updatedPrivacyScore: privacyResult.score,
        message: result.isLeaking
          ? 'WebRTC leak detected — real IP exposed'
          : 'No WebRTC leak — IP properly protected',
        messageFa: result.isLeaking
          ? 'نشت WebRTC شناسایی شد — IP واقعی فاش شده'
          : 'بدون نشت WebRTC — IP به‌درستی محافظت می‌شود',
      });
    }

    if (action === 'test-ipv6') {
      const { vpnConnected = true, ipv6Enabled = true } = body as {
        vpnConnected?: boolean;
        ipv6Enabled?: boolean;
      };

      const result = detectIPv6Leak(vpnConnected, ipv6Enabled);
      securityAuditState = { ...securityAuditState, ipv6Leak: result };

      const privacyResult = calculatePrivacyScore({
        dnsLeak: securityAuditState.dnsLeak,
        webrtcLeak: securityAuditState.webrtcLeak,
        ipv6Leak: result,
        killSwitchVerified: securityAuditState.killSwitchVerified,
        encryption: securityAuditState.encryptionAssessment,
        vpnConnected,
      });
      securityAuditState.privacyScore = privacyResult.score;
      securityAuditState.overallSecurityStatus = privacyResult.status;
      securityAuditState.overallSecurityStatusFa = privacyResult.statusFa;

      return NextResponse.json({
        success: true,
        action: 'test-ipv6',
        result,
        updatedPrivacyScore: privacyResult.score,
        message: result.isLeaking
          ? 'IPv6 leak detected — IPv6 traffic bypassing VPN'
          : 'No IPv6 leak — IPv6 properly handled',
        messageFa: result.isLeaking
          ? 'نشت IPv6 شناسایی شد — ترافیک IPv6 از VPN عبور نمی‌کند'
          : 'بدون نشت IPv6 — IPv6 به‌درستی مدیریت می‌شود',
      });
    }

    if (action === 'test-kill-switch') {
      const { killSwitchEnabled = true, networkLockEnabled = true } = body as {
        killSwitchEnabled?: boolean;
        networkLockEnabled?: boolean;
      };

      const result = verifyKillSwitch(killSwitchEnabled, networkLockEnabled);
      securityAuditState = {
        ...securityAuditState,
        killSwitchVerified: result.verified,
        killSwitchDetails: result.details,
        killSwitchDetailsFa: result.detailsFa,
      };

      const privacyResult = calculatePrivacyScore({
        dnsLeak: securityAuditState.dnsLeak,
        webrtcLeak: securityAuditState.webrtcLeak,
        ipv6Leak: securityAuditState.ipv6Leak,
        killSwitchVerified: result.verified,
        encryption: securityAuditState.encryptionAssessment,
        vpnConnected: true,
      });
      securityAuditState.privacyScore = privacyResult.score;
      securityAuditState.overallSecurityStatus = privacyResult.status;
      securityAuditState.overallSecurityStatusFa = privacyResult.statusFa;

      return NextResponse.json({
        success: true,
        action: 'test-kill-switch',
        verified: result.verified,
        details: result.details,
        detailsFa: result.detailsFa,
        updatedPrivacyScore: privacyResult.score,
        message: result.verified
          ? 'Kill switch verified — traffic properly blocked on disconnect'
          : 'Kill switch test failed — traffic may leak on disconnect',
        messageFa: result.verified
          ? 'کلید کشت تأیید شد — ترافیک هنگام قطع به‌درستی مسدود می‌شود'
          : 'تست کلید کشت ناموفق — ترافیک ممکن است هنگام قطع نشت کند',
      });
    }

    return NextResponse.json(
      {
        success: false,
        error: `Unknown action: ${action}. Valid actions: run-audit, test-dns, test-webrtc, test-ipv6, test-kill-switch`,
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
