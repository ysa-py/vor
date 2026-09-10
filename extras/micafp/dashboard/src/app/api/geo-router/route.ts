import { NextRequest, NextResponse } from 'next/server';
import {
  buildInitialGeoRouterState,
  buildServerList,
  buildLatencyMap,
  autoSelectBestCountry,
  selectServerCountry,
  runServerHealthCheck,
  calculateLoadDistribution,
  getIranBypassRules,
} from '@/lib/geo-router';

// ──────────────────────────────────────────────
// In-memory geo router state
// ──────────────────────────────────────────────
let geoRouterState = buildInitialGeoRouterState('irancell');

// ──────────────────────────────────────────────
// GET /api/geo-router
// Returns server list, latency map, and current routing state
// ──────────────────────────────────────────────
export async function GET() {
  const bypassRules = getIranBypassRules();

  // Sort server list by latency for display
  const sortedServers = [...geoRouterState.serverList].sort(
    (a, b) => a.avgLatencyMs - b.avgLatencyMs,
  );

  const selectedServer = geoRouterState.serverList.find(
    (s) => s.code === geoRouterState.selectedCountry,
  );

  const recommendedServer = geoRouterState.serverList.find(
    (s) => s.code === geoRouterState.recommendedCountry,
  );

  return NextResponse.json({
    success: true,
    timestamp: Date.now(),
    selectedCountry: {
      code: geoRouterState.selectedCountry,
      nameFa: geoRouterState.selectedCountryFa,
      server: selectedServer ?? null,
      labelFa: 'کشور انتخاب‌شده',
    },
    recommendation: {
      code: geoRouterState.recommendedCountry,
      nameFa: geoRouterState.recommendedCountryFa,
      reason: geoRouterState.recommendationReason,
      reasonFa: geoRouterState.recommendationReasonFa,
      server: recommendedServer ?? null,
      labelFa: 'پیشنهاد هوشمند',
    },
    serverList: sortedServers.map((server) => ({
      ...server,
      labelNameFa: 'نام',
      labelLatencyFa: 'تأخیر',
      labelLoadFa: 'بار',
      labelStatusFa: server.isHealthy ? 'سالم' : 'ناسالم',
      labelServersFa: 'سرورها',
    })),
    latencyMap: geoRouterState.latencyMap,
    loadBalancing: {
      ...geoRouterState.loadBalancing,
      labelStrategyFa: 'استراتژی',
      labelEnabledFa: 'فعال',
      labelDistributionFa: 'توزیع بار',
    },
    autoSelectEnabled: geoRouterState.autoSelectEnabled,
    iranInternalBypass: geoRouterState.iranInternalBypass,
    lastHealthCheck: geoRouterState.lastHealthCheck,
    healthCheckIntervalMs: geoRouterState.healthCheckInterval,
    bypassRules,
    summary: {
      totalServers: geoRouterState.serverList.reduce((sum, s) => sum + s.servers, 0),
      activeServers: geoRouterState.serverList.reduce((sum, s) => sum + s.activeServers, 0),
      healthyCountries: geoRouterState.serverList.filter((s) => s.isHealthy).length,
      totalCountries: geoRouterState.serverList.length,
      iranBypassCountries: geoRouterState.serverList.filter((s) => s.supportsIranBypass).length,
      labelTotalServersFa: 'کل سرورها',
      labelActiveServersFa: 'سرورهای فعال',
      labelHealthyFa: 'سالم',
      labelIranBypassFa: 'عبور از فیلترینگ ایران',
    },
    meta: {
      endpoint: '/api/geo-router',
      descriptionFa: 'مسیریاب جغرافیایی — انتخاب سرور، نقشه تأخیر و توزیع بار',
    },
  });
}

// ──────────────────────────────────────────────
// POST /api/geo-router
// body: { action: 'select' | 'health-check' | 'auto-select' | 'set-strategy', ...params }
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

    if (action === 'select') {
      const { countryCode } = body as { countryCode?: string };
      if (!countryCode) {
        return NextResponse.json(
          {
            success: false,
            error: 'Missing required field: countryCode',
            errorFa: 'فیلد ضروری موجود نیست: countryCode',
          },
          { status: 400 },
        );
      }

      const server = geoRouterState.serverList.find((s) => s.code === countryCode);
      if (!server) {
        return NextResponse.json(
          {
            success: false,
            error: `Country not found: ${countryCode}`,
            errorFa: `کشور یافت نشد: ${countryCode}`,
          },
          { status: 404 },
        );
      }

      if (!server.isHealthy) {
        return NextResponse.json(
          {
            success: false,
            error: `Server ${countryCode} is not healthy — cannot select`,
            errorFa: `سرور ${countryCode} سالم نیست — امکان انتخاب وجود ندارد`,
          },
          { status: 400 },
        );
      }

      geoRouterState = selectServerCountry(geoRouterState, countryCode);

      return NextResponse.json({
        success: true,
        action: 'select',
        countryCode,
        countryNameFa: geoRouterState.selectedCountryFa,
        latencyMs: server.avgLatencyMs,
        loadPercent: server.loadPercent,
        message: `Selected ${server.name} (${countryCode}) as VPN server country`,
        messageFa: `${server.nameFa} (${countryCode}) به عنوان کشور سرور VPN انتخاب شد`,
        loadDistribution: geoRouterState.loadBalancing.currentDistribution,
      });
    }

    if (action === 'health-check') {
      const beforeHealthy = geoRouterState.serverList.filter((s) => s.isHealthy).length;
      geoRouterState = runServerHealthCheck(geoRouterState);
      const afterHealthy = geoRouterState.serverList.filter((s) => s.isHealthy).length;

      return NextResponse.json({
        success: true,
        action: 'health-check',
        checkDurationMs: Math.round(2000 + Math.random() * 3000),
        healthyBefore: beforeHealthy,
        healthyAfter: afterHealthy,
        changesDetected: beforeHealthy !== afterHealthy,
        selectedCountry: geoRouterState.selectedCountry,
        selectedCountryFa: geoRouterState.selectedCountryFa,
        isStillHealthy: geoRouterState.serverList.find(
          (s) => s.code === geoRouterState.selectedCountry,
        )?.isHealthy ?? false,
        recommendation: {
          code: geoRouterState.recommendedCountry,
          nameFa: geoRouterState.recommendedCountryFa,
          reasonFa: geoRouterState.recommendationReasonFa,
        },
        loadDistribution: geoRouterState.loadBalancing.currentDistribution,
        message: `Health check complete — ${afterHealthy} of ${geoRouterState.serverList.length} countries healthy`,
        messageFa: `بررسی سلامت کامل — ${afterHealthy} از ${geoRouterState.serverList.length} کشور سالم`,
      });
    }

    if (action === 'auto-select') {
      const { detectedISP } = body as { detectedISP?: string };
      const isp = detectedISP ?? 'default';
      const autoSelect = autoSelectBestCountry(isp, geoRouterState.serverList);

      geoRouterState = {
        ...geoRouterState,
        selectedCountry: autoSelect.countryCode,
        selectedCountryFa: autoSelect.countryNameFa,
        recommendedCountry: autoSelect.countryCode,
        recommendedCountryFa: autoSelect.countryNameFa,
        recommendationReason: autoSelect.reason,
        recommendationReasonFa: autoSelect.reasonFa,
        autoSelectEnabled: true,
      };

      // Recalculate load distribution for the selected country
      geoRouterState = selectServerCountry(geoRouterState, autoSelect.countryCode);

      const server = geoRouterState.serverList.find(
        (s) => s.code === autoSelect.countryCode,
      );

      return NextResponse.json({
        success: true,
        action: 'auto-select',
        selectedCountry: autoSelect.countryCode,
        selectedCountryFa: autoSelect.countryNameFa,
        reason: autoSelect.reason,
        reasonFa: autoSelect.reasonFa,
        latencyMs: server?.avgLatencyMs ?? 0,
        message: `Auto-selected ${autoSelect.countryCode} based on ISP: ${isp}`,
        messageFa: `انتخاب خودکار ${autoSelect.countryNameFa} بر اساس ISP: ${isp}`,
      });
    }

    if (action === 'set-strategy') {
      const { strategy } = body as { strategy?: string };
      const validStrategies = ['round-robin', 'least-connections', 'lowest-latency', 'weighted'];

      if (!strategy || !validStrategies.includes(strategy)) {
        return NextResponse.json(
          {
            success: false,
            error: `Invalid strategy: ${strategy}. Valid: ${validStrategies.join(', ')}`,
            errorFa: `استراتژی نامعتبر: ${strategy}`,
          },
          { status: 400 },
        );
      }

      const strategyFa: Record<string, string> = {
        'round-robin': 'نوبت‌چرخشی',
        'least-connections': 'کمترین اتصال',
        'lowest-latency': 'کمترین تأخیر',
        'weighted': 'وزنی',
      };

      const newDistribution = calculateLoadDistribution(
        geoRouterState.serverList,
        strategy as 'round-robin' | 'least-connections' | 'lowest-latency' | 'weighted',
      );

      geoRouterState = {
        ...geoRouterState,
        loadBalancing: {
          ...geoRouterState.loadBalancing,
          strategy: strategy as 'round-robin' | 'least-connections' | 'lowest-latency' | 'weighted',
          strategyFa: strategyFa[strategy],
          currentDistribution: newDistribution,
        },
      };

      return NextResponse.json({
        success: true,
        action: 'set-strategy',
        strategy,
        strategyFa: strategyFa[strategy],
        distribution: newDistribution,
        message: `Load balancing strategy set to ${strategy}`,
        messageFa: `استراتژی توزیع بار به ${strategyFa[strategy]} تنظیم شد`,
      });
    }

    return NextResponse.json(
      {
        success: false,
        error: `Unknown action: ${action}. Valid actions: select, health-check, auto-select, set-strategy`,
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
