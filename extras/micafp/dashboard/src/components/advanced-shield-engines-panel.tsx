'use client';

import React, { useState, useEffect, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  ShieldAlert, ShieldCheck, Flame, RotateCcw, AlertTriangle,
  Zap, Clock, Radio, Activity, Cpu, Sparkles, RefreshCw,
  GitBranch, Sliders, Waves, Layers, CheckCircle2, Lock,
  CornerDownLeft, Shield, AlertOctagon, Terminal, ArrowRightLeft,
  Server, Power, Eye, EyeOff, BatteryCharging, Battery, BatteryLow,
  LineChart, TrendingUp, BarChart2, Check, ArrowUpRight, Waypoints, Network, Shuffle, Fingerprint, Radar
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';
import type { LoadBalancerMode } from '@/lib/unified-shield-types';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

// Sparkline SVG Path generator for Latency
function generateLatencySvgPath(points: { latencyMs: number }[], width: number, height: number): { pathD: string; areaD: string } {
  if (!points || points.length < 2) return { pathD: '', areaD: '' };

  const minVal = Math.min(...points.map(p => p.latencyMs), 15);
  const maxVal = Math.max(...points.map(p => p.latencyMs), 120);
  const range = maxVal - minVal || 1;

  const coords = points.map((p, i) => {
    const x = (i / (points.length - 1)) * width;
    const y = height - ((p.latencyMs - minVal) / range) * (height - 8) - 4;
    return { x, y };
  });

  const pathD = coords.reduce((acc, pt, i) => i === 0 ? `M ${pt.x.toFixed(1)} ${pt.y.toFixed(1)}` : `${acc} L ${pt.x.toFixed(1)} ${pt.y.toFixed(1)}`, '');
  const areaD = `${pathD} L ${width} ${height} L 0 ${height} Z`;

  return { pathD, areaD };
}

// Sparkline SVG Path generator for Packet Loss
function generateLossSvgPath(points: { packetLossPercent: number }[], width: number, height: number): { pathD: string; areaD: string } {
  if (!points || points.length < 2) return { pathD: '', areaD: '' };

  const minVal = 0;
  const maxVal = Math.max(...points.map(p => p.packetLossPercent), 5);
  const range = maxVal - minVal || 1;

  const coords = points.map((p, i) => {
    const x = (i / (points.length - 1)) * width;
    const y = height - ((p.packetLossPercent - minVal) / range) * (height - 6) - 3;
    return { x, y };
  });

  const pathD = coords.reduce((acc, pt, i) => i === 0 ? `M ${pt.x.toFixed(1)} ${pt.y.toFixed(1)}` : `${acc} L ${pt.x.toFixed(1)} ${pt.y.toFixed(1)}`, '');
  const areaD = `${pathD} L ${width} ${height} L 0 ${height} Z`;

  return { pathD, areaD };
}

export function AdvancedShieldEnginesPanel() {
  const {
    connected,
    orchestrator,
    egressIntegrity,
    pathSwitching,
    obfuscationProfiler,
    energyOptimizer,
    coreHealthHistory,
    togglePredictiveMitigation,
    executePredictivePreWarm,
    toggleEgressMonitoring,
    toggleAutoNetworkResetOnLeak,
    simulateEgressBypassAttempt,
    triggerLowLevelNetworkReset,
    toggleProactivePathSwitching,
    evaluateBGPHealthAndMigrate,
    manualPathMigrate,
    toggleObfuscationProfiler,
    adjustRandomPaddingRealtime,
    updateTargetPaddingFrequency,
    setBatteryLevel,
    toggleBatteryCharging,
    setPowerMode,
    toggleEnergyOptimizer,
    recordCoreHealthHistoryTick,
    loadBalancer,
    toggleLoadBalancer,
    setLoadBalancerMode,
    rebalanceLoad,
    applyLoadBalancerRoute,
    stealthRotation,
    toggleStealthRotation,
    rotateStealthNow,
  } = useUnifiedShieldStore();

  const [activeSubTab, setActiveSubTab] = useState<'egress' | 'predictive' | 'bgp' | 'obfuscation' | 'history' | 'energy' | 'loadbalancer' | 'stealthrotation'>('egress');

  // Background ticker for BGP health checks, entropy profiling, and core health history
  useEffect(() => {
    const historyInterval = setInterval(() => {
      recordCoreHealthHistoryTick();
    }, 15000); // Record sparkline tick every 15s

    const bgpInterval = setInterval(() => {
      if (pathSwitching.enabled) {
        evaluateBGPHealthAndMigrate();
      }
    }, 10000); // 10s evaluation for BGP health

    const entropyInterval = setInterval(() => {
      if (obfuscationProfiler.realtimeAnalysisActive) {
        adjustRandomPaddingRealtime();
      }
    }, 6000); // Real-time entropy jitter every 6s

    return () => {
      clearInterval(historyInterval);
      clearInterval(bgpInterval);
      clearInterval(entropyInterval);
    };
  }, [pathSwitching.enabled, obfuscationProfiler.realtimeAnalysisActive, evaluateBGPHealthAndMigrate, adjustRandomPaddingRealtime, recordCoreHealthHistoryTick]);

  const activeBgpRoute = pathSwitching.routes.find(r => r.routeId === pathSwitching.activeRouteId) || pathSwitching.routes[0];

  return (
    <div className="space-y-4">
      {/* Overview Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        {/* Card 1: Egress Integrity */}
        <Card className="shield-surface relative overflow-hidden">
          <div className="absolute top-0 right-0 w-1.5 h-full bg-red-500" />
          <CardHeader className="pb-2">
            <CardTitle className="text-slate-200 text-sm flex items-center justify-between">
              <span className="flex items-center gap-1.5"><ShieldAlert className="w-4 h-4 text-red-400" />پایش خروجی Egress</span>
              <Badge className={egressIntegrity.systemResetStatus === 'secured' ? 'bg-emerald-500/20 text-emerald-300 border-emerald-500/30' : 'bg-red-500/20 text-red-300 border-red-500/30'}>
                {egressIntegrity.systemResetStatus === 'secured' ? 'ایمن (بدون نشت)' : 'در حال بازنشانی...'}
              </Badge>
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <div className="flex justify-between items-center text-xs">
              <span className="text-slate-400">تلاش‌های غیرمجاز بلاک‌شده:</span>
              <span className="text-red-400 font-bold font-mono">{toPersianNum(egressIntegrity.unauthorizedBypassAttempts)}</span>
            </div>
            <div className="flex justify-between items-center text-xs">
              <span className="text-slate-400">پکت‌های بازرسی‌شده:</span>
              <span className="text-slate-200 font-mono">{toPersianNum(egressIntegrity.totalInspectedPackets.toLocaleString())}</span>
            </div>
            <Button
              size="sm"
              variant="outline"
              onClick={() => simulateEgressBypassAttempt()}
              className="w-full text-xs h-7 border-red-500/30 text-red-300 hover:bg-red-500/20"
            >
              <Flame className="w-3.5 h-3.5 ml-1 text-red-400" />
              تست نشت و بازنشانی سوکت
            </Button>
          </CardContent>
        </Card>

        {/* Card 2: Predictive Mitigation */}
        <Card className="shield-surface relative overflow-hidden">
          <div className="absolute top-0 right-0 w-1.5 h-full bg-violet-500" />
          <CardHeader className="pb-2">
            <CardTitle className="text-slate-200 text-sm flex items-center justify-between">
              <span className="flex items-center gap-1.5"><Sparkles className="w-4 h-4 text-violet-400" />پیش‌گرم‌سازی ۵ دقیقه‌ای</span>
              <Badge className="bg-violet-500/20 text-violet-300 border-violet-500/30">
                {orchestrator.predictiveMitigation?.enabled ? 'فعال' : 'غیرفعال'}
              </Badge>
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <div className="flex justify-between items-center text-xs">
              <span className="text-slate-400">رویداد بعدی:</span>
              <span className="text-violet-300 font-bold">{orchestrator.predictiveMitigation?.nextScheduledPreWarm?.dpiSpikeWindow || 'اوج شبانه ۲۰:۰۰'}</span>
            </div>
            <div className="flex justify-between items-center text-xs">
              <span className="text-slate-400">هسته‌های آماده:</span>
              <span className="text-emerald-400 font-mono text-[11px]">{orchestrator.predictiveMitigation?.preWarmedCores.join(', ') || 'xray-gfw, hiddify'}</span>
            </div>
            <Button
              size="sm"
              variant="outline"
              onClick={() => executePredictivePreWarm()}
              className="w-full text-xs h-7 border-violet-500/30 text-violet-300 hover:bg-violet-500/20"
            >
              <Zap className="w-3.5 h-3.5 ml-1 text-violet-400" />
              اجرای پیش‌گرم‌سازی دستی
            </Button>
          </CardContent>
        </Card>

        {/* Card 3: Proactive Path Switching */}
        <Card className="shield-surface relative overflow-hidden">
          <div className="absolute top-0 right-0 w-1.5 h-full bg-cyan-500" />
          <CardHeader className="pb-2">
            <CardTitle className="text-slate-200 text-sm flex items-center justify-between">
              <span className="flex items-center gap-1.5"><GitBranch className="w-4 h-4 text-cyan-400" />مهاجرت مسیر BGP (۱۰s)</span>
              <Badge className="bg-cyan-500/20 text-cyan-300 border-cyan-500/30">
                {pathSwitching.enabled ? 'پایش ۱۰ ثانیه‌ای' : 'غیرفعال'}
              </Badge>
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <div className="flex justify-between items-center text-xs">
              <span className="text-slate-400">مسیر فعال جاری:</span>
              <span className="text-cyan-300 font-bold truncate max-w-[130px]">{activeBgpRoute.nodeNameFa}</span>
            </div>
            <div className="flex justify-between items-center text-xs">
              <span className="text-slate-400">مهاجرت‌های خودکار:</span>
              <span className="text-emerald-400 font-mono">{toPersianNum(pathSwitching.totalPathMigrations)} بار</span>
            </div>
            <Button
              size="sm"
              variant="outline"
              onClick={() => evaluateBGPHealthAndMigrate()}
              className="w-full text-xs h-7 border-cyan-500/30 text-cyan-300 hover:bg-cyan-500/20"
            >
              <RefreshCw className="w-3.5 h-3.5 ml-1 text-cyan-400" />
              ارزیابی فوری BGP
            </Button>
          </CardContent>
        </Card>

        {/* Card 4: Obfuscation Profiler */}
        <Card className="shield-surface relative overflow-hidden">
          <div className="absolute top-0 right-0 w-1.5 h-full bg-emerald-500" />
          <CardHeader className="pb-2">
            <CardTitle className="text-slate-200 text-sm flex items-center justify-between">
              <span className="flex items-center gap-1.5"><Waves className="w-4 h-4 text-emerald-400" />پروفایلر آنتروپی ترافیک</span>
              <Badge className="bg-emerald-500/20 text-emerald-300 border-emerald-500/30">
                آنتروپی: {toPersianNum(obfuscationProfiler.averageEntropy)} / ۸.۰
              </Badge>
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <div className="flex justify-between items-center text-xs">
              <span className="text-slate-400">حجم استتارشده:</span>
              <span className="text-emerald-300 font-mono">{toPersianNum(obfuscationProfiler.totalTrafficReshapedMb)} MB</span>
            </div>
            <div className="flex justify-between items-center text-xs">
              <span className="text-slate-400">شکست تحلیل الگو:</span>
              <span className="text-cyan-400 font-bold">۱۰۰٪ پایدار</span>
            </div>
            <Button
              size="sm"
              variant="outline"
              onClick={() => adjustRandomPaddingRealtime()}
              className="w-full text-xs h-7 border-emerald-500/30 text-emerald-300 hover:bg-emerald-500/20"
            >
              <Sliders className="w-3.5 h-3.5 ml-1 text-emerald-400" />
              تنظیم در لحظه پدینگ
            </Button>
          </CardContent>
        </Card>
      </div>

      {/* Main Feature Tabs */}
      <Card className="shield-surface">
        <CardHeader className="pb-3 border-b border-slate-700/40">
          <div className="flex items-center justify-between flex-wrap gap-2">
            <div>
              <CardTitle className="text-slate-100 text-base flex items-center gap-2">
                <ShieldCheck className="w-5 h-5 text-emerald-400" />
                مجموعه ماژول‌های امنیتی و ضدفیلترینگ فوق‌پیشرفته نسل چهارم
              </CardTitle>
              <CardDescription className="text-slate-400 text-xs mt-0.5">
                مسیریابی پیش‌دستانه BGP، پایش جامع خروجی Egress، پیش‌گرم‌سازی ۵ دقیقه‌ای سری‌زمانی و پروفایلر آنتروپی
              </CardDescription>
            </div>
          </div>
        </CardHeader>

        <CardContent className="p-4">
          <Tabs value={activeSubTab} onValueChange={(v: any) => setActiveSubTab(v)} className="space-y-4">
            <TabsList className="bg-[#0a0f1c]/85 border border-white/10 grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 h-auto p-1 gap-1">
              <TabsTrigger value="egress" className="data-[state=active]:bg-red-500/20 data-[state=active]:text-red-300 text-xs py-2">
                <ShieldAlert className="w-3.5 h-3.5 ml-1 text-red-400" />
                پایش خروجی Egress
              </TabsTrigger>
              <TabsTrigger value="predictive" className="data-[state=active]:bg-violet-500/20 data-[state=active]:text-violet-300 text-xs py-2">
                <Clock className="w-3.5 h-3.5 ml-1 text-violet-400" />
                پیش‌گرم‌سازی ۵m
              </TabsTrigger>
              <TabsTrigger value="bgp" className="data-[state=active]:bg-cyan-500/20 data-[state=active]:text-cyan-300 text-xs py-2">
                <GitBranch className="w-3.5 h-3.5 ml-1 text-cyan-400" />
                مهاجرت BGP
              </TabsTrigger>
              <TabsTrigger value="obfuscation" className="data-[state=active]:bg-emerald-500/20 data-[state=active]:text-emerald-300 text-xs py-2">
                <Waves className="w-3.5 h-3.5 ml-1 text-emerald-400" />
                پروفایلر آنتروپی
              </TabsTrigger>
              <TabsTrigger value="history" className="data-[state=active]:bg-amber-500/20 data-[state=active]:text-amber-300 text-xs py-2">
                <LineChart className="w-3.5 h-3.5 ml-1 text-amber-400" />
                تاریخچه ۶۰m هسته‌ها
              </TabsTrigger>
              <TabsTrigger value="energy" className="data-[state=active]:bg-lime-500/20 data-[state=active]:text-lime-300 text-xs py-2">
                <BatteryCharging className="w-3.5 h-3.5 ml-1 text-lime-400" />
                بهینه‌ساز باتری
              </TabsTrigger>
              <TabsTrigger value="loadbalancer" className="data-[state=active]:bg-cyan-500/20 data-[state=active]:text-cyan-300 text-xs py-2">
                <Waypoints className="w-3.5 h-3.5 ml-1 text-cyan-400" />
                لودبالانسر AI
              </TabsTrigger>
              <TabsTrigger value="stealthrotation" className="data-[state=active]:bg-fuchsia-500/20 data-[state=active]:text-fuchsia-300 text-xs py-2">
                <Fingerprint className="w-3.5 h-3.5 ml-1 text-fuchsia-400" />
                چرخش مخفی AI
              </TabsTrigger>
            </TabsList>

            {/* ────────────────────────────────────────────────────────── */}
            {/* SUB-TAB 1: EGRESS INTEGRITY MONITOR */}
            {/* ────────────────────────────────────────────────────────── */}
            <TabsContent value="egress" className="space-y-4 pt-2">
              <div className="bg-[#0a0f1c]/70 border border-white/10 rounded-xl p-4 space-y-4">
                <div className="flex items-center justify-between flex-wrap gap-3">
                  <div>
                    <h3 className="text-sm font-bold text-slate-200 flex items-center gap-2">
                      <ShieldAlert className="w-4 h-4 text-red-400" />
                      سامانه مانیتورینگ سلامت خروجی شبکه (Egress Integrity Monitor)
                    </h3>
                    <p className="text-xs text-slate-400 mt-1">
                      شناسایی هرگونه تلاش نرم‌افزارها برای ارسال بسته به خارج از رابط پروکسی (نشت DNS، Raw Sockets، STUN) و اجرای بلادرنگ بازنشانی لایه پایین شبکه.
                    </p>
                  </div>
                  <div className="flex items-center gap-4">
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-slate-400">ریست خودکار سوکت:</span>
                      <Switch
                        checked={egressIntegrity.autoNetworkResetOnLeak}
                        onCheckedChange={() => toggleAutoNetworkResetOnLeak()}
                      />
                    </div>
                    <Button
                      size="sm"
                      variant="destructive"
                      onClick={() => triggerLowLevelNetworkReset('درخواست دستی بازنشانی شبکه')}
                      className="text-xs h-8 bg-red-600/80 hover:bg-red-600 text-white"
                    >
                      <RotateCcw className="w-3.5 h-3.5 ml-1" />
                      اجرای فوری بازنشانی سوکت‌های سیستم
                    </Button>
                  </div>
                </div>

                {/* Status Bar */}
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 text-xs bg-slate-950/80 p-3 rounded-lg border border-white/10">
                  <div className="flex items-center gap-2">
                    <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
                    <div>
                      <span className="text-slate-400 block">وضعیت رابط TUN:</span>
                      <span className="text-emerald-300 font-bold font-mono">100% Locked (VpnService Bound)</span>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0" />
                    <div>
                      <span className="text-slate-400 block">تلاش‌های نشت شناسایی‌شده:</span>
                      <span className="text-amber-300 font-bold font-mono">{toPersianNum(egressIntegrity.unauthorizedBypassAttempts)} مورد</span>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Terminal className="w-4 h-4 text-cyan-400 shrink-0" />
                    <div>
                      <span className="text-slate-400 block">آخرین بازنشانی اضطراری:</span>
                      <span className="text-cyan-300 font-mono">{new Date(egressIntegrity.lastResetTimestamp ?? Date.now()).toLocaleTimeString('fa-IR')}</span>
                    </div>
                  </div>
                </div>

                {/* Logs of Egress Leaks */}
                <div>
                  <h4 className="text-xs font-bold text-slate-300 mb-2 flex items-center gap-1.5">
                    <AlertOctagon className="w-3.5 h-3.5 text-red-400" />
                    لاگ تلاش‌های غیرمجاز دور زدن پروکسی و عملیات بازنشانی سوکت (Audit Log)
                  </h4>
                  <div className="space-y-2 max-h-72 overflow-y-auto pr-1">
                    {egressIntegrity.recentBypassAttempts.map(leak => (
                      <div
                        key={leak.id}
                        className="bg-[#070b13]/95 border border-red-500/20 rounded-lg p-3 space-y-1.5 hover:border-red-500/40 transition-colors"
                      >
                        <div className="flex items-center justify-between flex-wrap gap-2 text-xs">
                          <div className="flex items-center gap-2">
                            <Badge className="bg-red-500/20 text-red-300 border-red-500/40 text-[10px]">
                              {leak.bypassTypeFa}
                            </Badge>
                            <span className="font-mono text-slate-200 font-bold">{leak.sourceProcess}</span>
                          </div>
                          <div className="flex items-center gap-2">
                            <span className="text-[10px] text-slate-400 font-mono">
                              {new Date(leak.timestamp).toLocaleTimeString('fa-IR')}
                            </span>
                            {leak.resetTriggered && (
                              <Badge variant="outline" className="text-[10px] border-amber-500/40 text-amber-300">
                                بازنشانی سوکت اجرا شد
                              </Badge>
                            )}
                          </div>
                        </div>

                        <div className="text-[11px] text-slate-400 flex items-center justify-between flex-wrap gap-2">
                          <div>
                            <span className="text-slate-500 ml-1">مقصد مسدودشده:</span>
                            <span className="font-mono text-cyan-300">{leak.destinationIp}:{leak.destinationPort}</span>
                            <span className="text-slate-500 mr-3 ml-1">پروتکل شناسایی‌شده:</span>
                            <span className="font-mono text-slate-300">{leak.detectedProtocol}</span>
                          </div>
                          <span className="text-emerald-400 font-bold text-[10px]">مسدودسازی قطعی در فایروال</span>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            </TabsContent>

            {/* ────────────────────────────────────────────────────────── */}
            {/* SUB-TAB 2: PREDICTIVE MITIGATION PLANNER */}
            {/* ────────────────────────────────────────────────────────── */}
            <TabsContent value="predictive" className="space-y-4 pt-2">
              <div className="bg-[#0a0f1c]/70 border border-white/10 rounded-xl p-4 space-y-4">
                <div className="flex items-center justify-between flex-wrap gap-3">
                  <div>
                    <h3 className="text-sm font-bold text-slate-200 flex items-center gap-2">
                      <Sparkles className="w-4 h-4 text-violet-400" />
                      طرح پیش‌گیرانه و پیش‌گرم‌سازی ۵ دقیقه‌ای (Predictive Mitigation Planner)
                    </h3>
                    <p className="text-xs text-slate-400 mt-1">
                      تحلیل سری‌زمانی سوابق مسدودسازی و آماده‌سازی و برقراری تونل‌های سایه روی هسته‌های فوق‌استتار دقیقاً ۵ دقیقه پیش از پیک فیلترینگ روزانه.
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-slate-400">پیش‌گرم‌سازی هوشمند:</span>
                    <Switch
                      checked={orchestrator.predictiveMitigation?.enabled ?? true}
                      onCheckedChange={() => togglePredictiveMitigation()}
                    />
                  </div>
                </div>

                {/* Next Event Card */}
                {orchestrator.predictiveMitigation?.nextScheduledPreWarm && (
                  <div className="bg-gradient-to-r from-violet-950/40 via-slate-900 to-slate-950 border border-violet-500/30 rounded-xl p-4">
                    <div className="flex items-center justify-between flex-wrap gap-3">
                      <div className="space-y-1">
                        <span className="text-[11px] text-violet-400 font-bold flex items-center gap-1.5">
                          <Clock className="w-3.5 h-3.5" />
                          نوبت بعدی پیش‌گرم‌سازی خودکار هوش مصنوعی:
                        </span>
                        <p className="text-base font-bold text-slate-100">
                          {orchestrator.predictiveMitigation.nextScheduledPreWarm.triggerTime}
                        </p>
                        <p className="text-xs text-slate-400">
                          هسته هدف پیش‌گرم: <span className="text-violet-300 font-bold">{orchestrator.predictiveMitigation.nextScheduledPreWarm.coreNameFa}</span>
                        </p>
                      </div>

                      <Button
                        size="sm"
                        onClick={() => executePredictivePreWarm()}
                        className="bg-violet-600 hover:bg-violet-500 text-white text-xs"
                      >
                        <Zap className="w-3.5 h-3.5 ml-1" />
                        اجرای فوری و گرم‌سازی تونل سایه
                      </Button>
                    </div>
                  </div>
                )}

                {/* Scheduled Spike Windows Table */}
                <div className="space-y-3">
                  <h4 className="text-xs font-bold text-slate-300 flex items-center gap-1.5">
                    <Flame className="w-3.5 h-3.5 text-amber-400" />
                    پنجره‌های زمانی اوج فیلترینگ بر اساس سری‌زمانی سوابق ۳۰ روز اخیر در ایران
                  </h4>

                  <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
                    {orchestrator.predictiveMitigation?.activeScheduleWindows.map(win => (
                      <div
                        key={win.id}
                        className="bg-slate-950/80 border border-white/10 hover:border-violet-500/40 rounded-xl p-3.5 space-y-2.5 transition-all"
                      >
                        <div className="flex items-center justify-between">
                          <Badge className={win.historicalSeverity === 'critical' ? 'bg-red-500/20 text-red-300 border-red-500/30' : 'bg-amber-500/20 text-amber-300 border-amber-500/30'}>
                            {win.historicalSeverityFa}
                          </Badge>
                          <span className="text-[10px] text-slate-400 font-mono">۵ دقیقه پیش‌گرم</span>
                        </div>

                        <div>
                          <h5 className="text-xs font-bold text-slate-200">{win.timeLabelFa}</h5>
                          <p className="text-[11px] text-slate-400 mt-1 leading-relaxed">
                            {win.descriptionFa}
                          </p>
                        </div>

                        <div className="space-y-1 text-[10px] bg-[#0a0f1c]/90 p-2 rounded border border-white/10">
                          <div className="flex justify-between text-slate-400">
                            <span>پروتکل‌های تحت خطر:</span>
                            <span className="text-red-300 font-mono">{win.affectedProtocols.join(', ')}</span>
                          </div>
                          <div className="flex justify-between text-slate-400">
                            <span>هسته توصیه‌شده:</span>
                            <span className="text-emerald-300 font-mono font-bold">{win.recommendedCore}</span>
                          </div>
                        </div>

                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => executePredictivePreWarm(win.id)}
                          className="w-full text-[11px] h-7 border-slate-700 hover:bg-violet-500/20 hover:text-violet-200 text-slate-300"
                        >
                          پیش‌گرم‌سازی این سناریو
                        </Button>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            </TabsContent>

            {/* ────────────────────────────────────────────────────────── */}
            {/* SUB-TAB 3: PROACTIVE PATH SWITCHING (BGP) */}
            {/* ────────────────────────────────────────────────────────── */}
            <TabsContent value="bgp" className="space-y-4 pt-2">
              <div className="bg-[#0a0f1c]/70 border border-white/10 rounded-xl p-4 space-y-4">
                <div className="flex items-center justify-between flex-wrap gap-3">
                  <div>
                    <h3 className="text-sm font-bold text-slate-200 flex items-center gap-2">
                      <GitBranch className="w-4 h-4 text-cyan-400" />
                      تعویض پیش‌دستانه مسیر بر اساس جداول BGP (Proactive Path Switching)
                    </h3>
                    <p className="text-xs text-slate-400 mt-1">
                      ارزیابی پیوسته سلامت مسیر BGP رله‌های داخلی به خروجی‌های بین‌الملل هر ۱۰ ثانیه؛ مهاجرت خودکار سشن در صورت تشخیص افزایش هوپ یا Route Flapping.
                    </p>
                  </div>
                  <div className="flex items-center gap-4">
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-slate-400">مهاجرت خودکار در شیفت BGP:</span>
                      <Switch
                        checked={pathSwitching.autoMigrateOnShift}
                        onCheckedChange={() => toggleProactivePathSwitching()}
                      />
                    </div>
                    <Button
                      size="sm"
                      onClick={() => evaluateBGPHealthAndMigrate()}
                      className="text-xs h-8 bg-cyan-600/80 hover:bg-cyan-600 text-white"
                    >
                      <RefreshCw className="w-3.5 h-3.5 ml-1" />
                      سنجش مجدد جداول BGP
                    </Button>
                  </div>
                </div>

                {/* Last Migration Notice */}
                {pathSwitching.lastMigration && (
                  <div className="bg-cyan-950/30 border border-cyan-500/30 rounded-lg p-3 flex items-center justify-between text-xs">
                    <div className="flex items-center gap-2">
                      <CheckCircle2 className="w-4 h-4 text-cyan-400 shrink-0" />
                      <div>
                        <span className="text-cyan-300 font-bold block">آخرین مهاجرت موفق سشن بدون قطعی:</span>
                        <span className="text-slate-400">{pathSwitching.lastMigration.reasonFa}</span>
                      </div>
                    </div>
                    <div className="text-left font-mono">
                      <Badge className="bg-cyan-500/20 text-cyan-300 border-cyan-500/40">
                        {pathSwitching.lastMigration.migrationTimeMs} ms
                      </Badge>
                    </div>
                  </div>
                )}

                {/* Routes Grid */}
                <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                  {pathSwitching.routes.map(route => {
                    const isActive = route.routeId === pathSwitching.activeRouteId;
                    const isShifted = route.healthStatus === 'route_shift_detected' || route.healthStatus === 'flapping';

                    return (
                      <div
                        key={route.routeId}
                        className={`rounded-xl p-4 border transition-all ${
                          isActive
                            ? 'bg-[#0a0f1c]/90 border-cyan-500 shadow-lg shadow-cyan-500/10'
                            : isShifted
                            ? 'bg-slate-950/80 border-amber-500/30'
                            : 'bg-slate-950/60 border-white/10 hover:border-slate-700'
                        }`}
                      >
                        <div className="flex items-center justify-between mb-2">
                          <div className="flex items-center gap-2">
                            <Server className={`w-4 h-4 ${isActive ? 'text-cyan-400' : 'text-slate-400'}`} />
                            <span className="font-bold text-xs text-slate-100">{route.nodeNameFa}</span>
                          </div>
                          <Badge className={
                            route.healthStatus === 'optimal'
                              ? 'bg-emerald-500/20 text-emerald-300 border-emerald-500/30 text-[10px]'
                              : 'bg-red-500/20 text-red-300 border-red-500/30 text-[10px]'
                          }>
                            {route.healthStatusFa}
                          </Badge>
                        </div>

                        <div className="space-y-1.5 text-xs text-slate-400 mb-3">
                          <div className="flex justify-between">
                            <span>شماره خودمختار (ASN):</span>
                            <span className="font-mono text-slate-200">{route.asNumber}</span>
                          </div>
                          <div className="flex justify-between">
                            <span>تعداد گام‌های مسیر (AS Path Hops):</span>
                            <span className="font-mono text-cyan-300">{route.asPathHops} Hops</span>
                          </div>
                          <div className="flex justify-between">
                            <span>میزان نوسان مسیر (Flap Count ۱۰m):</span>
                            <span className="font-mono text-amber-300">{route.flapCount10m} رویداد</span>
                          </div>
                          <div className="flex justify-between">
                            <span>پکت‌لاس مسیر بین‌الملل:</span>
                            <span className="font-mono text-slate-200">{toPersianNum(route.packetLossPercent)}٪</span>
                          </div>
                        </div>

                        <div className="flex items-center justify-between pt-2 border-t border-white/10">
                          <span className="text-[10px] text-slate-500">نقطه خروج: {route.exitPointFa}</span>
                          {isActive ? (
                            <Badge className="bg-emerald-500/20 text-emerald-300 border-emerald-500/40 text-[11px]">
                              مسیر فعال و آنلاین
                            </Badge>
                          ) : (
                            <Button
                              size="sm"
                              variant="outline"
                              onClick={() => manualPathMigrate(route.routeId)}
                              className="text-xs h-7 border-slate-700 text-slate-300 hover:bg-cyan-500/20 hover:text-cyan-200"
                            >
                              مهاجرت دستی به این مسیر
                            </Button>
                          )}
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            </TabsContent>

            {/* ────────────────────────────────────────────────────────── */}
            {/* SUB-TAB 4: OBFUSCATION PROFILER */}
            {/* ────────────────────────────────────────────────────────── */}
            <TabsContent value="obfuscation" className="space-y-4 pt-2">
              <div className="bg-[#0a0f1c]/70 border border-white/10 rounded-xl p-4 space-y-4">
                <div className="flex items-center justify-between flex-wrap gap-3">
                  <div>
                    <h3 className="text-sm font-bold text-slate-200 flex items-center gap-2">
                      <Waves className="w-4 h-4 text-emerald-400" />
                      پروفایلر آنتروپی و تنظیم بلادرنگ پدینگ تصادفی (Obfuscation Profiler)
                    </h3>
                    <p className="text-xs text-slate-400 mt-1">
                      محاسبه آنتروپی اطلاعاتی (Shannon Entropy) ترافیک عبوری از هسته‌های VLESS و Hysteria 2 و تغییر فرکانس پدینگ پکت‌ها در لحظه برای شکست تحلیل آماری فایروال.
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-slate-400">تحلیل آنتروپی زنده:</span>
                    <Switch
                      checked={obfuscationProfiler.realtimeAnalysisActive}
                      onCheckedChange={() => toggleObfuscationProfiler()}
                    />
                  </div>
                </div>

                {/* Live Target Core Profiles */}
                <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
                  {obfuscationProfiler.targetProfiles.map(prof => (
                    <div
                      key={prof.targetCoreId}
                      className="bg-slate-950/80 border border-white/10 hover:border-emerald-500/40 rounded-xl p-4 space-y-3 transition-all"
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          <Cpu className="w-4 h-4 text-emerald-400" />
                          <span className="font-bold text-xs text-slate-200">{prof.targetCoreNameFa}</span>
                        </div>
                        <Badge className="bg-emerald-500/20 text-emerald-300 border-emerald-500/30 text-[10px] font-mono">
                          آنتروپی: {toPersianNum(prof.liveEntropyScore)}
                        </Badge>
                      </div>

                      <div className="space-y-1.5">
                        <div className="flex justify-between text-xs">
                          <span className="text-slate-400">فرکانس پدینگ تصادفی:</span>
                          <span className="text-emerald-300 font-bold font-mono">{toPersianNum(prof.randomPaddingFrequency)}٪</span>
                        </div>
                        <Progress value={prof.randomPaddingFrequency} className="h-2" />
                      </div>

                      <div className="space-y-1 text-[10px] bg-[#0a0f1c]/90 p-2.5 rounded border border-white/10">
                        <div className="flex justify-between text-slate-400">
                          <span>بازه تزریق بایت زباله:</span>
                          <span className="text-cyan-300 font-mono">{toPersianNum(prof.currentPaddingSizeRange.minBytes)} - {toPersianNum(prof.currentPaddingSizeRange.maxBytes)} B</span>
                        </div>
                        <div className="flex justify-between text-slate-400">
                          <span>پکت‌های چاف (Chaff/Sec):</span>
                          <span className="text-violet-300 font-mono">{toPersianNum(prof.chaffPacketsPerSec)} pps</span>
                        </div>
                        <div className="flex justify-between text-slate-400">
                          <span>نوسان زمانی (Timing Jitter):</span>
                          <span className="text-amber-300 font-mono">{toPersianNum(prof.timingJitterMs)} ms</span>
                        </div>
                      </div>

                      <div className="space-y-1">
                        <span className="text-[10px] text-slate-400 block">امضاهای DPI خنثی‌شده:</span>
                        <div className="flex flex-wrap gap-1">
                          {prof.defeatedDpiSignatures.map(sig => (
                            <Badge key={sig} variant="outline" className="text-[9px] border-slate-700 text-slate-400">
                              {sig}
                            </Badge>
                          ))}
                        </div>
                      </div>

                      <div className="pt-2 border-t border-white/10 flex items-center justify-between gap-2">
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => updateTargetPaddingFrequency(prof.targetCoreId, Math.min(100, prof.randomPaddingFrequency + 5))}
                          className="text-[10px] h-6 px-2 border-slate-700 text-slate-300"
                        >
                          +۵٪ فرکانس
                        </Button>
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => updateTargetPaddingFrequency(prof.targetCoreId, Math.max(20, prof.randomPaddingFrequency - 5))}
                          className="text-[10px] h-6 px-2 border-slate-700 text-slate-300"
                        >
                          -۵٪ فرکانس
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </TabsContent>

            {/* ────────────────────────────────────────────────────────── */}
            {/* SUB-TAB 5: CORE HEALTH HISTORY (60-MIN SPARKLINE GRAPHS) */}
            {/* ────────────────────────────────────────────────────────── */}
            <TabsContent value="history" className="space-y-4 pt-2">
              <div className="bg-[#0a0f1c]/70 border border-white/10 rounded-xl p-4 space-y-4">
                <div className="flex items-center justify-between flex-wrap gap-3">
                  <div>
                    <h3 className="text-sm font-bold text-slate-200 flex items-center gap-2">
                      <LineChart className="w-4 h-4 text-amber-400" />
                      تاریخچه پایش سلامت هسته‌ها (Core Health History - ۶۰ دقیقه گذشته)
                    </h3>
                    <p className="text-xs text-slate-400 mt-1">
                      نمودارهای جرقه‌ای (Sparkline) بلادرنگ برای پایش روند تأخیر (Latency)، پکت‌لاس (Packet Loss) و درصد پایداری آپ‌تایم تمام ۳۰ هسته سیستم.
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <Badge variant="outline" className="text-xs border-amber-500/40 text-amber-300 bg-amber-500/10">
                      پایش زنده هر ۱۵ ثانیه
                    </Badge>
                  </div>
                </div>

                {/* Grid of 9 Cores Sparklines */}
                <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
                  {coreHealthHistory.map((rec) => {
                    const latSvg = generateLatencySvgPath(rec.history60m, 260, 48);
                    const lossSvg = generateLossSvgPath(rec.history60m, 260, 24);
                    const latestPoint = rec.history60m[rec.history60m.length - 1];

                    return (
                      <div
                        key={rec.coreId}
                        className="bg-slate-950/80 border border-white/10 hover:border-slate-700/80 transition-all rounded-xl p-3.5 space-y-3"
                      >
                        {/* Header */}
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-2">
                            <div className="w-2.5 h-2.5 rounded-full bg-emerald-400 animate-pulse" />
                            <span className="font-bold text-xs text-slate-200">{rec.coreNameFa}</span>
                          </div>
                          <Badge variant="outline" className="text-[10px] border-slate-700 text-slate-400 font-mono">
                            آپ‌تایم: {toPersianNum(rec.uptimePercentage60m)}٪
                          </Badge>
                        </div>

                        {/* Latency Sparkline */}
                        <div className="space-y-1">
                          <div className="flex items-center justify-between text-[11px]">
                            <span className="text-slate-400 flex items-center gap-1">
                              <span>تأخیر (Latency):</span>
                              <span className="text-amber-300 font-mono font-bold">{toPersianNum(latestPoint?.latencyMs || rec.avgLatency60m)} ms</span>
                            </span>
                            <span className="text-slate-500 text-[10px] font-mono">
                              میانگین: {toPersianNum(rec.avgLatency60m)} ms
                            </span>
                          </div>
                          <div className="bg-[#0a0f1c]/90 rounded-lg p-1.5 border border-white/10 overflow-hidden">
                            <svg viewBox="0 0 260 48" className="w-full h-12">
                              <defs>
                                <linearGradient id={`grad-lat-${rec.coreId}`} x1="0" y1="0" x2="0" y2="1">
                                  <stop offset="0%" stopColor="#f59e0b" stopOpacity="0.3" />
                                  <stop offset="100%" stopColor="#f59e0b" stopOpacity="0.0" />
                                </linearGradient>
                              </defs>
                              <path d={latSvg.areaD} fill={`url(#grad-lat-${rec.coreId})`} />
                              <path d={latSvg.pathD} fill="none" stroke="#f59e0b" strokeWidth="2" strokeLinecap="round" />
                            </svg>
                          </div>
                        </div>

                        {/* Packet Loss Sparkline */}
                        <div className="space-y-1">
                          <div className="flex items-center justify-between text-[11px]">
                            <span className="text-slate-400 flex items-center gap-1">
                              <span>افت بسته (Packet Loss):</span>
                              <span className={`font-mono font-bold ${(latestPoint?.packetLossPercent || 0) > 1 ? 'text-red-400' : 'text-emerald-400'}`}>
                                {toPersianNum(latestPoint?.packetLossPercent || 0)}٪
                              </span>
                            </span>
                            <span className="text-slate-500 text-[10px] font-mono">
                              جیتر: {toPersianNum(latestPoint?.jitterMs || 3)} ms
                            </span>
                          </div>
                          <div className="bg-[#0a0f1c]/90 rounded-lg p-1 border border-white/10 overflow-hidden">
                            <svg viewBox="0 0 260 24" className="w-full h-6">
                              <defs>
                                <linearGradient id={`grad-loss-${rec.coreId}`} x1="0" y1="0" x2="0" y2="1">
                                  <stop offset="0%" stopColor="#ef4444" stopOpacity="0.4" />
                                  <stop offset="100%" stopColor="#ef4444" stopOpacity="0.0" />
                                </linearGradient>
                              </defs>
                              <path d={lossSvg.areaD} fill={`url(#grad-loss-${rec.coreId})`} />
                              <path d={lossSvg.pathD} fill="none" stroke="#ef4444" strokeWidth="1.5" strokeLinecap="round" />
                            </svg>
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            </TabsContent>

            {/* ────────────────────────────────────────────────────────── */}
            {/* SUB-TAB 6: ENERGY EFFICIENCY OPTIMIZER */}
            {/* ────────────────────────────────────────────────────────── */}
            <TabsContent value="energy" className="space-y-4 pt-2">
              <div className="bg-[#0a0f1c]/70 border border-white/10 rounded-xl p-4 space-y-4">
                <div className="flex items-center justify-between flex-wrap gap-3">
                  <div>
                    <h3 className="text-sm font-bold text-slate-200 flex items-center gap-2">
                      <BatteryCharging className="w-4 h-4 text-lime-400" />
                      بهینه‌ساز هوشمند مصرف باتری (Energy Efficiency Optimizer)
                    </h3>
                    <p className="text-xs text-slate-400 mt-1">
                      تنظیم تطبیقی فرکانس مانیتورینگ ورکر پس‌زمینه بر اساس درصد شارژ باتری دستگاه جهت افزایش طول عمر باتری.
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-slate-400">ماژول فعال:</span>
                    <Switch
                      checked={energyOptimizer.enabled}
                      onCheckedChange={() => toggleEnergyOptimizer()}
                    />
                  </div>
                </div>

                {/* Energy Status Grid */}
                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3 text-xs">
                  <div className="bg-slate-950 p-3 rounded-lg border border-white/10 space-y-1.5">
                    <div className="flex items-center justify-between text-slate-400">
                      <span>درصد باتری دستگاه:</span>
                      {energyOptimizer.isCharging ? (
                        <BatteryCharging className="w-4 h-4 text-lime-400" />
                      ) : energyOptimizer.batteryLevel < 20 ? (
                        <BatteryLow className="w-4 h-4 text-red-400 animate-pulse" />
                      ) : (
                        <Battery className="w-4 h-4 text-slate-300" />
                      )}
                    </div>
                    <div className="text-xl font-bold font-mono text-slate-100 flex items-baseline gap-1">
                      <span>{toPersianNum(energyOptimizer.batteryLevel)}</span>
                      <span className="text-xs text-slate-400">٪</span>
                    </div>
                    <Progress value={energyOptimizer.batteryLevel} className="h-1.5 bg-slate-800" />
                  </div>

                  <div className="bg-slate-950 p-3 rounded-lg border border-white/10 space-y-1">
                    <span className="text-slate-400 block">فاصله پایش پس‌زمینه:</span>
                    <div className="text-xl font-bold font-mono text-cyan-300 flex items-baseline gap-1">
                      <span>هر {toPersianNum(energyOptimizer.currentPollingIntervalSec)}</span>
                      <span className="text-xs text-slate-400">ثانیه</span>
                    </div>
                    <span className="text-[10px] text-slate-500">
                      {energyOptimizer.batteryLevel < 20 ? 'حالت فوق‌العاده کم‌مصرف فعال' : 'حالت بهینه مصرف انرژی'}
                    </span>
                  </div>

                  <div className="bg-slate-950 p-3 rounded-lg border border-white/10 space-y-1">
                    <span className="text-slate-400 block">صرفه‌جویی تخمینی باتری:</span>
                    <div className="text-xl font-bold font-mono text-lime-400 flex items-baseline gap-1">
                      <span>+{toPersianNum(energyOptimizer.powerSavedPercentage)}</span>
                      <span className="text-xs text-slate-400">٪</span>
                    </div>
                    <span className="text-[10px] text-slate-500">کاهش WakeLock و انتقال رادیویی</span>
                  </div>

                  <div className="bg-slate-950 p-3 rounded-lg border border-white/10 space-y-1">
                    <span className="text-slate-400 block">وضعیت اتصال به شارژر:</span>
                    <div className="text-sm font-bold mt-1">
                      {energyOptimizer.isCharging ? (
                        <span className="text-lime-400 flex items-center gap-1">
                          <Check className="w-3.5 h-3.5" />
                          متصل به برق (پایش سریع ۱۵s)
                        </span>
                      ) : (
                        <span className="text-amber-400">روی باتری (پایش تطبیقی)</span>
                      )}
                    </div>
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => toggleBatteryCharging()}
                      className="text-[10px] h-6 px-2 mt-1 border-slate-700 text-slate-300 w-full"
                    >
                      {energyOptimizer.isCharging ? 'شبیه‌سازی قطع شارژر' : 'شبیه‌سازی اتصال شارژر'}
                    </Button>
                  </div>
                </div>

                {/* Battery Level Simulator & Power Mode */}
                <div className="bg-slate-950 p-4 rounded-xl border border-white/10 space-y-3">
                  <div className="flex items-center justify-between flex-wrap gap-2">
                    <span className="text-xs font-bold text-slate-200">شبیه‌ساز و تست سطح شارژ باتری:</span>
                    <div className="flex items-center gap-1 flex-wrap">
                      <Button
                        size="sm"
                        variant={energyOptimizer.batteryLevel === 12 ? 'default' : 'outline'}
                        onClick={() => setBatteryLevel(12)}
                        className="text-xs h-7 px-2.5 border-red-500/40 text-red-300"
                      >
                        ۱۲٪ (بحرانی &lt; ۲۰٪)
                      </Button>
                      <Button
                        size="sm"
                        variant={energyOptimizer.batteryLevel === 35 ? 'default' : 'outline'}
                        onClick={() => setBatteryLevel(35)}
                        className="text-xs h-7 px-2.5 border-amber-500/40 text-amber-300"
                      >
                        ۳۵٪ (متوسط)
                      </Button>
                      <Button
                        size="sm"
                        variant={energyOptimizer.batteryLevel === 85 ? 'default' : 'outline'}
                        onClick={() => setBatteryLevel(85)}
                        className="text-xs h-7 px-2.5 border-emerald-500/40 text-emerald-300"
                      >
                        ۸۵٪ (عادی)
                      </Button>
                      <Button
                        size="sm"
                        variant={energyOptimizer.batteryLevel === 100 ? 'default' : 'outline'}
                        onClick={() => setBatteryLevel(100)}
                        className="text-xs h-7 px-2.5 border-lime-500/40 text-lime-300"
                      >
                        ۱۰۰٪ (کامل)
                      </Button>
                    </div>
                  </div>

                  <div className="text-xs text-slate-400 bg-[#0a0f1c]/90 p-3 rounded-lg border border-white/10">
                    <p>
                      <strong>منطق کاری بهینه‌ساز:</strong> هنگام کاهش شارژ باتری به زیر ۲۰٪، فواصل فراخوانی ورکر پس‌زمینه به صورت خودکار به <span className="text-red-400 font-mono font-bold">۱۸۰ ثانیه</span> افزایش می‌یابد تا از هدررفت منابع CPU و رادیوی دستگاه جلوگیری کند. به محض اتصال به شارژر، فواصل به <span className="text-lime-400 font-mono font-bold">۱۵ ثانیه</span> برای بیشترین دقت سوئیچینگ ارتقا می‌یابد.
                    </p>
                  </div>
                </div>
              </div>
            </TabsContent>
            {/* ────────────────────────────────────────────────────────── */}
            {/* SUB-TAB 7: AI LOAD BALANCER */}
            {/* ────────────────────────────────────────────────────────── */}
            <TabsContent value="loadbalancer" className="space-y-4 pt-2">
              <div className="bg-[#0a0f1c]/70 border border-white/10 rounded-xl p-4 space-y-4">
                <div className="flex items-center justify-between flex-wrap gap-3">
                  <div>
                    <h3 className="text-sm font-bold text-slate-200 flex items-center gap-2">
                      <Waypoints className="w-4 h-4 text-cyan-400" />
                      لودبالانسر هوشمند AI — توزیع تطبیقی ترافیک
                    </h3>
                    <p className="text-xs text-slate-400 mt-1">
                      توزیع خودکار ترافیک روی برترین هسته‌ها، پروتکل‌های ضد DPI، مسیرهای تانل و حالت مستقیم (بدون تانل) با هماهنگی کامل موتور هوش مصنوعی داخلی.
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-slate-400">فعال‌سازی خودکار:</span>
                    <Switch checked={loadBalancer.enabled} onCheckedChange={() => toggleLoadBalancer()} />
                  </div>
                </div>

                {/* Summary cards */}
                <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
                  <div className="bg-slate-950 p-3 rounded-lg border border-white/10 space-y-1">
                    <span className="text-slate-400 block">حالت توزیع:</span>
                    <div className="text-base font-bold font-mono text-cyan-300">
                      {loadBalancer.mode === 'adaptive' ? 'تطبیقی AI' : loadBalancer.mode === 'weighted' ? 'وزنی' : loadBalancer.mode === 'failover' ? 'فیل‌اور' : 'کوانتومی چندمسیره'}
                    </div>
                  </div>
                  <div className="bg-slate-950 p-3 rounded-lg border border-white/10 space-y-1">
                    <span className="text-slate-400 block">تانل / مستقیم:</span>
                    <div className="text-base font-bold font-mono text-slate-100 flex items-baseline gap-1">
                      <span className="text-violet-300">{toPersianNum(loadBalancer.tunnelSharePct)}٪</span>
                      <span className="text-[10px] text-slate-500">/</span>
                      <span className="text-emerald-400">{toPersianNum(loadBalancer.directSharePct)}٪</span>
                    </div>
                  </div>
                  <div className="bg-slate-950 p-3 rounded-lg border border-white/10 space-y-1">
                    <span className="text-slate-400 block">پهنای باند تجمیعی:</span>
                    <div className="text-base font-bold font-mono text-emerald-300 flex items-baseline gap-1">
                      <span>{toPersianNum(loadBalancer.totalBandwidthMbps)}</span>
                      <span className="text-[10px] text-slate-500">Mb/s</span>
                    </div>
                  </div>
                  <div className="bg-slate-950 p-3 rounded-lg border border-white/10 space-y-1">
                    <span className="text-slate-400 block">دفعات توازن:</span>
                    <div className="text-base font-bold font-mono text-amber-300 flex items-baseline gap-1">
                      <span>{toPersianNum(loadBalancer.rebalanceCount)}</span>
                      <span className="text-[10px] text-slate-500">مرتبه</span>
                    </div>
                  </div>
                </div>

                {/* Mode selector + actions */}
                <div className="flex flex-wrap items-center gap-2">
                  <span className="text-xs text-slate-400 ml-1">حالت دلخواه:</span>
                  {(
                    [
                      { id: 'weighted', fa: 'وزنی' },
                      { id: 'adaptive', fa: 'تطبیقی AI' },
                      { id: 'failover', fa: 'فیل‌اور' },
                      { id: 'quantum-multipath', fa: 'کوانتومی چندمسیره' },
                    ] as { id: LoadBalancerMode; fa: string }[]
                  ).map((m) => (
                    <Button
                      key={m.id}
                      size="sm"
                      variant={loadBalancer.mode === m.id ? 'default' : 'outline'}
                      onClick={() => setLoadBalancerMode(m.id)}
                      className="text-xs h-7 px-2.5"
                    >
                      {m.fa}
                    </Button>
                  ))}
                  <div className="flex-1" />
                  <Button size="sm" variant="outline" onClick={() => rebalanceLoad()} className="text-xs h-7 px-2.5 border-cyan-500/40 text-cyan-300">
                    <RefreshCw className="w-3.5 h-3.5 ml-1" />
                    توازن مجدد
                  </Button>
                  <Button size="sm" onClick={() => applyLoadBalancerRoute()} className="text-xs h-7 px-2.5 bg-cyan-600 hover:bg-cyan-500 text-white">
                    <Zap className="w-3.5 h-3.5 ml-1" />
                    اعمال مسیر AI
                  </Button>
                </div>

                {/* AI decision */}
                <div className="bg-slate-950 p-4 rounded-xl border border-cyan-500/20 space-y-2">
                  <div className="flex items-center justify-between flex-wrap gap-2">
                    <span className="text-xs font-bold text-cyan-300 flex items-center gap-2">
                      <Sparkles className="w-4 h-4" />
                      تصمیم موتور AI داخلی
                    </span>
                    <span className="text-[10px] text-slate-500">اطمینان: {toPersianNum(loadBalancer.aiDecision.confidencePct)}٪</span>
                  </div>
                  <div className="grid grid-cols-2 gap-3 text-xs">
                    <div className="bg-[#0a0f1c] p-3 rounded-lg border border-white/10">
                      <span className="text-slate-500 block">هسته پیشنهادی:</span>
                      <span className="text-sm font-bold text-cyan-300 mt-1 block">{loadBalancer.aiDecision.recommendedCoreId || '—'}</span>
                    </div>
                    <div className="bg-[#0a0f1c] p-3 rounded-lg border border-white/10">
                      <span className="text-slate-500 block">پروتکل پیشنهادی:</span>
                      <span className="text-sm font-bold text-violet-300 mt-1 block">{loadBalancer.aiDecision.recommendedProtocolId || '—'}</span>
                    </div>
                  </div>
                  <div className="w-full bg-slate-800 rounded-full h-2 overflow-hidden">
                    <div className="h-2 bg-gradient-to-l from-cyan-400 to-violet-400 rounded-full" style={{ width: loadBalancer.aiDecision.confidencePct + '%' }} />
                  </div>
                </div>

                {/* Core distribution */}
                <div className="space-y-2">
                  <h4 className="text-xs font-bold text-slate-300 flex items-center gap-2">
                    <Cpu className="w-3.5 h-3.5 text-cyan-400" />
                    توزیع ترافیک روی هسته‌ها
                  </h4>
                  {loadBalancer.cores.length === 0 ? (
                    <p className="text-xs text-slate-500">هنوز توزیعی محاسبه نشده — منتظر اولین چرخه AI…</p>
                  ) : (
                    <div className="space-y-2">
                      {loadBalancer.cores.map((c) => (
                        <div key={c.coreId} className="flex items-center gap-3 text-xs">
                          <span className={c.active ? 'w-32 truncate text-slate-200' : 'w-32 truncate text-slate-500'}>
                            {c.coreNameFa || c.coreId}
                          </span>
                          <div className="flex-1 bg-slate-800 rounded-full h-1.5 overflow-hidden">
                            <div className={c.active ? 'h-1.5 rounded-full bg-cyan-400' : 'h-1.5 rounded-full bg-slate-600'} style={{ width: c.trafficSharePct + '%' }} />
                          </div>
                          <span className="w-10 text-left font-mono text-slate-300">{toPersianNum(c.trafficSharePct)}٪</span>
                          {c.active && <Badge className="text-[9px] bg-cyan-500/20 text-cyan-300 border-cyan-500/30">فعال</Badge>}
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                {/* Protocol distribution */}
                <div className="space-y-2">
                  <h4 className="text-xs font-bold text-slate-300 flex items-center gap-2">
                    <Network className="w-3.5 h-3.5 text-violet-400" />
                    توزیع روی پروتکل‌ها و مسیرها
                  </h4>
                  {loadBalancer.protocols.length === 0 ? (
                    <p className="text-xs text-slate-500">—</p>
                  ) : (
                    <div className="space-y-2">
                      {loadBalancer.protocols.map((p) => (
                        <div key={p.protocolId + p.coreId} className="flex items-center gap-3 text-xs">
                          <span className={p.active ? 'w-40 truncate text-slate-200' : 'w-40 truncate text-slate-500'}>
                            {p.protocolNameFa || p.protocolId}
                          </span>
                          <Badge className={p.route === 'direct' ? 'text-[9px] bg-emerald-500/20 text-emerald-300 border-emerald-500/30' : 'text-[9px] bg-violet-500/20 text-violet-300 border-violet-500/30'}>
                            {p.route === 'direct' ? 'بدون تانل' : 'تانل'}
                          </Badge>
                          <div className="flex-1 bg-slate-800 rounded-full h-1.5 overflow-hidden">
                            <div className={p.route === 'direct' ? 'h-1.5 rounded-full bg-emerald-400' : 'h-1.5 rounded-full bg-violet-400'} style={{ width: p.weightPct + '%' }} />
                          </div>
                          <span className="w-10 text-left font-mono text-slate-300">{toPersianNum(p.weightPct)}٪</span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                {/* Reason */}
                <div className="text-xs text-slate-400 bg-[#0a0f1c]/90 p-3 rounded-lg border border-white/10">
                  <span className="text-slate-500 block mb-1">تحلیل AI:</span>
                  {loadBalancer.reasonFa}
                </div>
              </div>
            </TabsContent>
            {/* ────────────────────────────────────────────────────────── */}
            {/* SUB-TAB 8: STEALTH ROTATION (anti-fingerprint switching) */}
            {/* ────────────────────────────────────────────────────────── */}
            <TabsContent value="stealthrotation" className="space-y-4 pt-2">
              <div className="bg-[#0a0f1c]/70 border border-white/10 rounded-xl p-4 space-y-4">
                <div className="flex items-center justify-between flex-wrap gap-3">
                  <div>
                    <h3 className="text-sm font-bold text-slate-200 flex items-center gap-2">
                      <Fingerprint className="w-4 h-4 text-fuchsia-400" />
                      چرخش مخفی AI — تعویض غیرقابل‌ردیابی هسته و پروتکل
                    </h3>
                    <p className="text-xs text-slate-400 mt-1">
                      موتور هوش مصنوعی پیش از آنکه فیلتر امضای اتصال را شناسایی کند، هسته و پروتکل را با جیتر تصادفی تعویض می‌کند تا هیچ الگوی ثابتی قابل انگشت‌نگاری نباشد.
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-slate-400">فعال‌سازی خودکار:</span>
                    <Switch checked={stealthRotation.enabled} onCheckedChange={() => toggleStealthRotation()} />
                  </div>
                </div>

                {/* Risk + counters */}
                <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
                  <div className="bg-slate-950 p-3 rounded-lg border border-white/10 space-y-1">
                    <span className="text-slate-400 block">ریسک اثر انگشت:</span>
                    <div className="text-xl font-bold font-mono text-fuchsia-300">{toPersianNum(stealthRotation.fingerprintRisk)}٪</div>
                    <Progress value={stealthRotation.fingerprintRisk} className="h-1.5 bg-slate-800" />
                  </div>
                  <div className="bg-slate-950 p-3 rounded-lg border border-white/10 space-y-1">
                    <span className="text-slate-400 block">چرخش‌های انجام‌شده:</span>
                    <div className="text-xl font-bold font-mono text-cyan-300">{toPersianNum(stealthRotation.rotationCount)}</div>
                  </div>
                  <div className="bg-slate-950 p-3 rounded-lg border border-white/10 space-y-1">
                    <span className="text-slate-400 block">هسته فعلی:</span>
                    <div className="text-sm font-bold text-slate-100 truncate">{stealthRotation.lastCoreId || orchestrator.activeCoreId || '—'}</div>
                  </div>
                  <div className="bg-slate-950 p-3 rounded-lg border border-white/10 space-y-1">
                    <span className="text-slate-400 block">پروتکل فعلی:</span>
                    <div className="text-sm font-bold text-violet-300 truncate">{stealthRotation.lastProtocolId || '—'}</div>
                  </div>
                </div>

                {/* Technique + actions */}
                <div className="flex flex-wrap items-center gap-2">
                  <Badge className="text-[10px] bg-fuchsia-500/20 text-fuchsia-300 border-fuchsia-500/30">
                    <Shuffle className="w-3 h-3 ml-1" />
                    {stealthRotation.techniqueFa || 'چرخش تصادفی اثر انگشت'}
                  </Badge>
                  <div className="flex-1" />
                  <Button size="sm" onClick={() => rotateStealthNow()} className="text-xs h-7 px-2.5 bg-fuchsia-600 hover:bg-fuchsia-500 text-white">
                    <Radar className="w-3.5 h-3.5 ml-1" />
                    چرخش فوری
                  </Button>
                </div>

                {/* Reason */}
                <div className="text-xs text-slate-400 bg-[#0a0f1c]/90 p-3 rounded-lg border border-white/10">
                  <span className="text-slate-500 block mb-1">تحلیل AI:</span>
                  {stealthRotation.reasonFa}
                </div>

                {/* History */}
                <div className="space-y-2">
                  <h4 className="text-xs font-bold text-slate-300 flex items-center gap-2">
                    <Radar className="w-3.5 h-3.5 text-fuchsia-400" />
                    تاریخچه چرخش‌های مخفی
                  </h4>
                  {stealthRotation.history.length === 0 ? (
                    <p className="text-xs text-slate-500">هنوز چرخشی انجام نشده — اولین چرخش با چرخه AI انجام می‌شود.</p>
                  ) : (
                    <div className="space-y-2">
                      {stealthRotation.history.slice(0, 6).map((h) => (
                        <div key={h.id} className="flex items-center gap-3 text-xs bg-slate-950/60 p-2 rounded-lg border border-white/10">
                          <span className="font-mono text-slate-300 w-24 truncate">{h.fromCoreId} → {h.toCoreId}</span>
                          <span className="font-mono text-violet-300 w-28 truncate">{h.fromProtocolId} → {h.toProtocolId}</span>
                          <Badge className="text-[9px] bg-fuchsia-500/20 text-fuchsia-300 border-fuchsia-500/30">ریسک {toPersianNum(h.fingerprintRisk)}٪</Badge>
                          <span className="text-slate-500 flex-1 truncate">{h.techniqueFa}</span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </TabsContent>
          </Tabs>
        </CardContent>
      </Card>
    </div>
  );
}
