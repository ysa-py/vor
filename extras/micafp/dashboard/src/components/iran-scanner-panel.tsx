'use client';

import React, { useEffect, useMemo, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Radar, Scan, Brain, ShieldAlert, Globe, Radio, Network, Cpu,
  CheckCircle2, XCircle, Activity, Zap, Play, Square, RefreshCw,
  Wifi, WifiOff, Gauge, Sparkles, Eye, Lock, ArrowRight, Ban,
  ShieldCheck, Waypoints, DoorOpen, Server, TrendingUp, Download, Atom,
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';
import type { IranFilterLevel, IranProbeResult } from '@/lib/unified-shield-types';
import { AutoScannerEngineSections } from '@/components/auto-scanner-engine-panel';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

const LEVEL_STYLE: Record<IranFilterLevel, { labelFa: string; color: string; bg: string; bar: string; icon: 'ok' | 'warn' | 'bad' }> = {
  'open': { labelFa: 'اینترنت باز', color: 'text-emerald-400', bg: 'bg-emerald-500/15 border-emerald-500/30', bar: 'bg-emerald-400', icon: 'ok' },
  'light': { labelFa: 'فیلترینگ سبک', color: 'text-cyan-400', bg: 'bg-cyan-500/15 border-cyan-500/30', bar: 'bg-cyan-400', icon: 'ok' },
  'dpi': { labelFa: 'بازرسی عمیق بسته', color: 'text-amber-400', bg: 'bg-amber-500/15 border-amber-500/30', bar: 'bg-amber-400', icon: 'warn' },
  'sni': { labelFa: 'فیلترینگ SNI گسترده', color: 'text-orange-400', bg: 'bg-orange-500/15 border-orange-500/30', bar: 'bg-orange-400', icon: 'warn' },
  'ip-block': { labelFa: 'مسدودسازی سطح IP', color: 'text-rose-400', bg: 'bg-rose-500/15 border-rose-500/30', bar: 'bg-rose-400', icon: 'warn' },
  'extreme': { labelFa: 'فیلترینگ چندلایه فوق‌سخت', color: 'text-fuchsia-400', bg: 'bg-fuchsia-500/15 border-fuchsia-500/30', bar: 'bg-fuchsia-400', icon: 'bad' },
  'international-cutoff': { labelFa: 'قطع اینترنت بین‌الملل', color: 'text-red-400', bg: 'bg-red-500/15 border-red-500/30', bar: 'bg-red-400', icon: 'bad' },
  'national-only': { labelFa: 'فقط شبکه ملی', color: 'text-violet-400', bg: 'bg-violet-500/15 border-violet-500/30', bar: 'bg-violet-400', icon: 'bad' },
};

const PROBE_ICON: Record<IranProbeResult['type'], typeof Globe> = {
  dns: Globe,
  tcp: Network,
  tls: Lock,
  quic: Zap,
  egress: Wifi,
  national: Radio,
  doh: ShieldCheck,
  ipv6: Waypoints,
  port: DoorOpen,
  throttle: Gauge,
};

const SIM_LEVELS: { level: IranFilterLevel; labelFa: string }[] = [
  { level: 'open', labelFa: 'آزاد' },
  { level: 'light', labelFa: 'سبک' },
  { level: 'dpi', labelFa: 'DPI' },
  { level: 'sni', labelFa: 'SNI' },
  { level: 'ip-block', labelFa: 'IP' },
  { level: 'extreme', labelFa: 'فوق‌سخت' },
  { level: 'international-cutoff', labelFa: 'قطع بین‌الملل' },
  { level: 'national-only', labelFa: 'ملی' },
];

export function IranScannerPanel() {
  const {
    iranScanner,
    connected,
    orchestrator,
    nationalBlackoutShield,
    startIranScanner,
    stopIranScanner,
    toggleScannerAutoPilot,
    setScannerInterval,
    runScannerCycle,
    simulateIranFilterLevel,
    runBurstScan,
    toggleScannerAdaptive,
    toggleQuantumMultiPath,
    discoverBestServer,
    exportScannerReport,
  } = useUnifiedShieldStore();

  const { scanning, autoPilot, intervalSec, nextScanInSec, filterLevel, filterScore, filterLevelFa, filterLevelDescFa, adaptiveInterval, probesExecutedTotal, scansToday, hardLayers, servers, activeServerId, learning, ispCoverage, forecast, multiPath } = iranScanner;
  const activeServer = servers.find((sv) => sv.id === activeServerId) ?? null;

  const handleExport = () => {
    const report = exportScannerReport();
    const blob = new Blob([report], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `iran-scanner-report-${Date.now()}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  // Live countdown + auto cycle while scanning
  useEffect(() => {
    if (!scanning) return;
    const t = setInterval(() => {
      const s = useUnifiedShieldStore.getState().iranScanner;
      if (s.nextScanInSec <= 1) {
        useUnifiedShieldStore.getState().runScannerCycle();
      } else {
        useUnifiedShieldStore.setState(state => ({
          iranScanner: { ...state.iranScanner, nextScanInSec: state.iranScanner.nextScanInSec - 1 },
        }));
      }
    }, 1000);
    return () => clearInterval(t);
  }, [scanning]);

  const levelStyle = LEVEL_STYLE[filterLevel];

  // Sparkline points from filterScoreHistory
  const spark = useMemo(() => {
    const hist = iranScanner.filterScoreHistory;
    if (hist.length < 2) return '';
    const w = 220, h = 48;
    const min = Math.min(...hist), max = Math.max(...hist);
    const range = Math.max(1, max - min);
    const pts = hist.map((v, i) => {
      const x = (i / (hist.length - 1)) * w;
      const y = h - ((v - min) / range) * (h - 6) - 3;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    });
    return pts.join(' ');
  }, [iranScanner.filterScoreHistory]);

  const env = iranScanner.environment;
  const envBars = [
    { labelFa: 'شدت DPI', value: env.dpiIntensity, color: 'bg-amber-400' },
    { labelFa: 'پوشش SNI', value: env.sniCoverage, color: 'bg-orange-400' },
    { labelFa: 'مسدودسازی IP', value: env.ipBlockCoverage, color: 'bg-rose-400' },
    { labelFa: 'دسترسی بین‌الملل', value: env.internationalReach, color: 'bg-cyan-400' },
    { labelFa: 'دسترسی شبکه ملی', value: env.nationalReach, color: 'bg-emerald-400' },
  ];

  return (
    <Card className="shield-surface border-0">
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between flex-wrap gap-2">
          <div>
            <CardTitle className="text-slate-200 text-base flex items-center gap-2">
              <Radar className="w-5 h-5 text-cyan-400" />
              اسکنر هوشمند ضد فیلترینگ ایران
              <Badge variant="outline" className="text-[10px] border-cyan-500/30 text-cyan-400">AI Engine</Badge>
              <Badge variant="outline" className="text-[10px] border-amber-500/30 text-amber-400">Enterprise $99.9B</Badge>
            </CardTitle>
            <CardDescription className="text-slate-500 text-xs">
              اسکن داینامیک و پیوسته سطح فیلترینگ ایران — تشخیص خودکار قطعی بین‌الملل و اتصال از قوی‌ترین مسیر، بدون دخالت کاربر
            </CardDescription>
          </div>
          {/* Engine controls */}
          <div className="flex items-center gap-3 flex-wrap">
            <div className="flex items-center gap-2 text-xs text-slate-400">
              <span>خلبان خودکار</span>
              <Switch checked={autoPilot} onCheckedChange={(v) => toggleScannerAutoPilot(v)} />
            </div>
            <select
              value={intervalSec}
              onChange={(e) => setScannerInterval(Number(e.target.value))}
              className="bg-white/[0.04] border border-white/10 rounded-lg px-2 py-1.5 text-xs text-slate-300 focus:outline-none"
            >
              <option value={6}>هر ۶ ثانیه</option>
              <option value={12}>هر ۱۲ ثانیه</option>
              <option value={30}>هر ۳۰ ثانیه</option>
              <option value={60}>هر ۶۰ ثانیه</option>
            </select>
            <Button
              size="sm"
              variant={scanning ? 'destructive' : 'default'}
              onClick={() => (scanning ? stopIranScanner() : startIranScanner())}
              className="gap-1.5"
            >
              {scanning ? <><Square className="w-4 h-4" />توقف اسکن</> : <><Play className="w-4 h-4" />شروع اسکن پیوسته</>}
            </Button>
            <Button size="sm" variant="outline" onClick={() => runScannerCycle()} className="gap-1.5">
              <RefreshCw className="w-4 h-4" />اسکن فوری
            </Button>
            <Button size="sm" variant="outline" onClick={() => runBurstScan()} className="gap-1.5" title="۳ چرخه سریع پیاپی">
              <Zap className="w-4 h-4 text-amber-400" />اسکن انفجاری
            </Button>
            <Button size="sm" variant="outline" onClick={handleExport} className="gap-1.5" title="دانلود گزارش تشخیصی کامل">
              <Download className="w-4 h-4 text-cyan-400" />گزارش
            </Button>
          </div>
        </div>
      </CardHeader>

      <CardContent className="space-y-4">
        {/* Status strip */}
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-2">
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500">وضعیت موتور</p>
            <p className="text-sm font-bold flex items-center gap-1.5 mt-0.5">
              {scanning ? (
                <span className="inline-flex items-center gap-1.5 text-emerald-400">
                  <span className="relative flex h-2.5 w-2.5">
                    <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
                    <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-emerald-400" />
                  </span>
                  اسکن فعال
                </span>
              ) : (
                <span className="inline-flex items-center gap-1.5 text-slate-500">
                  <span className="inline-flex rounded-full h-2.5 w-2.5 bg-slate-500" />
                  متوقف
                </span>
              )}
            </p>
          </div>
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500">چرخه‌های کامل</p>
            <p className="text-sm font-bold text-cyan-400 mt-0.5">{toPersianNum(iranScanner.scanCycleCount)}</p>
          </div>
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500">اسکن بعدی</p>
            <p className="text-sm font-bold text-slate-200 mt-0.5">{toPersianNum(nextScanInSec)} ثانیه</p>
          </div>
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500">اتصال</p>
            <p className={`text-sm font-bold mt-0.5 ${connected ? 'text-emerald-400' : 'text-slate-400'}`}>
              {connected ? 'متصل' : 'قطع'} {connected && orchestrator.activeCoreId && `— ${orchestrator.activeCoreId}`}
            </p>
          </div>
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500">اسکن‌های امروز</p>
            <p className="text-sm font-bold text-cyan-400 mt-0.5">{toPersianNum(scansToday)}</p>
          </div>
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500">پروب‌های اجراشده</p>
            <p className="text-sm font-bold text-slate-200 mt-0.5">{toPersianNum(probesExecutedTotal)}</p>
          </div>
        </div>

        {/* Live classification + probes */}
        <div className="grid grid-cols-1 lg:grid-cols-5 gap-4">
          {/* Left: live filter-level gauge */}
          <div className="lg:col-span-2 space-y-3">
            <div className="shield-tile rounded-xl p-4">
              <div className="flex items-center justify-between">
                <p className="text-xs text-slate-400 flex items-center gap-1.5"><Gauge className="w-4 h-4 text-cyan-400" />سطح فیلترینگ زنده</p>
                <Badge className={`${levelStyle.bg} ${levelStyle.color} border`}>{levelStyle.labelFa}</Badge>
              </div>
              <div className="flex items-center gap-4 mt-3">
                <div className="relative w-28 h-28 shrink-0">
                  <svg viewBox="0 0 100 100" className="w-full h-full -rotate-90">
                    <circle cx="50" cy="50" r="42" fill="none" stroke="rgba(255,255,255,0.06)" strokeWidth="9" />
                    <motion.circle
                      cx="50" cy="50" r="42" fill="none"
                      stroke={filterScore > 75 ? '#f43f5e' : filterScore > 45 ? '#f59e0b' : '#34d399'}
                      strokeWidth="9" strokeLinecap="round"
                      strokeDasharray={2 * Math.PI * 42}
                      initial={false}
                      animate={{ strokeDashoffset: 2 * Math.PI * 42 * (1 - filterScore / 100) }}
                      transition={{ duration: 0.8, ease: 'easeOut' }}
                    />
                  </svg>
                  <div className="absolute inset-0 flex flex-col items-center justify-center">
                    <motion.span key={filterScore} initial={{ opacity: 0, y: 4 }} animate={{ opacity: 1, y: 0 }} className="text-2xl font-black text-white">
                      {toPersianNum(filterScore)}
                    </motion.span>
                    <span className="text-[9px] text-slate-500">از ۱۰۰</span>
                  </div>
                </div>
                <div className="space-y-1.5 min-w-0">
                  <p className="text-sm font-bold text-slate-200">{filterLevelFa}</p>
                  <p className="text-[11px] text-slate-500 leading-5">{filterLevelDescFa}</p>
                  <div className="flex items-center gap-1.5 text-[11px]">
                    {autoPilot ? (
                      <Badge className="bg-emerald-500/15 text-emerald-400 border border-emerald-500/30"><Sparkles className="w-3 h-3 ml-1" />خلبان خودکار فعال</Badge>
                    ) : (
                      <Badge className="bg-slate-500/15 text-slate-400 border border-white/10">خلبان خودکار غیرفعال</Badge>
                    )}
                    {nationalBlackoutShield.emergencyModeActive && (
                      <Badge className="bg-red-500/15 text-red-400 border border-red-500/30"><ShieldAlert className="w-3 h-3 ml-1" />سپر ملی فعال</Badge>
                    )}
                  </div>
                </div>
              </div>
              {/* Score sparkline */}
              <div className="mt-3">
                <p className="text-[10px] text-slate-500 mb-1">روند شدت فیلترینگ (چرخه‌های اخیر)</p>
                <svg viewBox="0 0 220 48" className="w-full h-12" preserveAspectRatio="none">
                  <polyline points={spark} fill="none" stroke={filterScore > 75 ? '#f43f5e' : '#f59e0b'} strokeWidth="2" strokeLinejoin="round" strokeLinecap="round" />
                </svg>
              </div>
            </div>

            {/* Environment bars */}
            <div className="shield-tile rounded-xl p-4 space-y-2.5">
              <p className="text-xs text-slate-400 flex items-center gap-1.5"><Activity className="w-4 h-4 text-cyan-400" />پارامترهای محیط زنده</p>
              {envBars.map(b => (
                <div key={b.labelFa} className="space-y-1">
                  <div className="flex justify-between text-[10px]">
                    <span className="text-slate-400">{b.labelFa}</span>
                    <span className="text-slate-300 font-mono">{toPersianNum(b.value)}٪</span>
                  </div>
                  <div className="h-1.5 rounded-full bg-white/[0.06] overflow-hidden">
                    <motion.div
                      className={`h-full rounded-full ${b.color}`}
                      initial={false}
                      animate={{ width: `${b.value}%` }}
                      transition={{ duration: 0.7, ease: 'easeOut' }}
                    />
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Right: probes + AI decision */}
          <div className="lg:col-span-3 space-y-3">
            <div className="shield-tile rounded-xl p-4">
              <div className="flex items-center justify-between mb-2.5">
                <p className="text-xs text-slate-400 flex items-center gap-1.5"><Scan className="w-4 h-4 text-cyan-400" />پروب‌های چرخه آخر</p>
                <span className="text-[10px] text-slate-500 font-mono">
                  {iranScanner.probes.filter(p => p.reachable).length}/{iranScanner.probes.length} موفق
                </span>
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                <AnimatePresence initial={false}>
                  {iranScanner.probes.map((p) => {
                    const Icon = PROBE_ICON[p.type];
                    return (
                      <motion.div
                        key={p.id}
                        layout
                        initial={{ opacity: 0, y: 6 }}
                        animate={{ opacity: 1, y: 0 }}
                        exit={{ opacity: 0 }}
                        transition={{ duration: 0.25 }}
                        className={`rounded-lg border px-2.5 py-2 ${p.reachable ? 'border-emerald-500/20 bg-emerald-500/[0.04]' : 'border-rose-500/25 bg-rose-500/[0.05]'}`}
                      >
                        <div className="flex items-center gap-2">
                          <Icon className={`w-4 h-4 shrink-0 ${p.reachable ? 'text-emerald-400' : 'text-rose-400'}`} />
                          <div className="min-w-0 flex-1">
                            <p className="text-[11px] font-bold text-slate-200 truncate">{p.nameFa}</p>
                            <p className="text-[9px] text-slate-500 truncate">{p.targetFa}</p>
                          </div>
                          {p.reachable
                            ? <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
                            : <XCircle className="w-4 h-4 text-rose-400 shrink-0" />}
                        </div>
                        <div className="flex items-center justify-between mt-1.5 text-[9px]">
                          <span className={p.reachable ? 'text-emerald-500/80' : 'text-rose-500/80'}>{p.detailFa}</span>
                          {p.reachable && <span className="text-slate-400 font-mono">{toPersianNum(p.latencyMs)} ms</span>}
                        </div>
                      </motion.div>
                    );
                  })}
                </AnimatePresence>
              </div>
            </div>

            {/* AI decision */}
            <div className="rounded-xl border border-cyan-500/25 bg-gradient-to-br from-cyan-500/[0.07] to-violet-500/[0.05] p-4">
              <div className="flex items-center gap-2 mb-2">
                <Brain className="w-4 h-4 text-cyan-400" />
                <p className="text-xs font-bold text-slate-200">تصمیم هوش مصنوعی — قوی‌ترین مسیر</p>
              </div>
              <div className="flex items-center flex-wrap gap-2">
                <Badge className="bg-cyan-500/15 text-cyan-300 border border-cyan-500/30">{iranScanner.activeStrategyFa}</Badge>
                <Badge variant="outline" className="text-[10px] border-white/10 text-slate-400">
                  هسته: {iranScanner.routeCoreNameFa}
                </Badge>
                {nationalBlackoutShield.activeNovelProtocolId && nationalBlackoutShield.emergencyModeActive && (
                  <Badge variant="outline" className="text-[10px] border-violet-500/30 text-violet-300">
                    پروتکل: {nationalBlackoutShield.activeNovelProtocolId}
                  </Badge>
                )}
              </div>
              <p className="text-[11px] text-slate-400 mt-2 leading-5">{iranScanner.aiReasonFa}</p>
              <div className="flex items-center gap-2 mt-2 text-[10px] text-slate-500">
                <ArrowRight className="w-3 h-3" />
                <span>مسیر: {iranScanner.routeCoreNameFa} → {nationalBlackoutShield.activeRelayHop}</span>
              </div>
            </div>

            {/* History */}
            <div className="shield-tile rounded-xl p-4">
              <p className="text-xs text-slate-400 mb-2.5 flex items-center gap-1.5"><Activity className="w-4 h-4 text-cyan-400" />تاریخچه چرخه‌های اسکن</p>
              <div className="space-y-1.5 max-h-56 overflow-y-auto pr-1">
                {iranScanner.history.length === 0 && (
                  <p className="text-[11px] text-slate-600">هنوز چرخه‌ای اجرا نشده — اسکن را شروع کنید</p>
                )}
                <AnimatePresence initial={false}>
                  {iranScanner.history.slice(0, 12).map((h) => {
                    const ls = LEVEL_STYLE[h.filterLevel];
                    const time = new Date(h.timestamp).toLocaleTimeString('fa-IR');
                    return (
                      <motion.div
                        key={h.id}
                        layout
                        initial={{ opacity: 0, x: -8 }}
                        animate={{ opacity: 1, x: 0 }}
                        className="flex items-center gap-2 rounded-lg border border-white/5 bg-white/[0.02] px-2.5 py-1.5"
                      >
                        <span className="text-[10px] text-slate-500 font-mono w-16 shrink-0">{time}</span>
                        <span className={`text-[11px] font-bold ${ls.color} shrink-0`}>{ls.labelFa}</span>
                        <span className="text-[10px] text-slate-500 truncate flex-1">{h.bestStrategyFa}</span>
                        <span className="text-[9px] text-slate-600 font-mono shrink-0">{toPersianNum(h.durationMs)}ms</span>
                        {h.autoConnected && <Badge className="bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 text-[9px] shrink-0">اتصال خودکار</Badge>}
                      </motion.div>
                    );
                  })}
                </AnimatePresence>
              </div>
            </div>
          </div>
        </div>

        {/* Level simulator (diagnostics) */}
        <div className="shield-tile rounded-xl p-3">
          <p className="text-[10px] text-slate-500 mb-2 flex items-center gap-1.5"><Ban className="w-3 h-3 text-amber-400" />شبیه‌ساز سطح فیلترینگ (تشخیصی) — برای آزمایش رفتار خودکار در هر سطح</p>
          <div className="flex flex-wrap gap-1.5">
            {SIM_LEVELS.map(s => {
              const active = filterLevel === s.level;
              const ls = LEVEL_STYLE[s.level];
              return (
                <button
                  key={s.level}
                  onClick={() => simulateIranFilterLevel(s.level)}
                  className={`rounded-lg border px-2.5 py-1.5 text-[11px] font-bold transition-all ${
                    active ? `${ls.bg} ${ls.color}` : 'border-white/10 bg-white/[0.03] text-slate-400 hover:bg-white/[0.06]'
                  }`}
                >
                  {s.labelFa}
                </button>
              );
            })}
          </div>
        </div>

        {/* v10 — engine control row */}
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
          <div className="shield-tile rounded-xl px-3 py-2.5 flex items-center justify-between">
            <div>
              <p className="text-[11px] font-bold text-slate-200 flex items-center gap-1.5"><Gauge className="w-3.5 h-3.5 text-cyan-400" />فاصله تطبیقی</p>
              <p className="text-[9px] text-slate-500 mt-0.5">در فیلترینگ سخت خودکار سریع‌تر اسکن می‌کند</p>
            </div>
            <Switch checked={adaptiveInterval} onCheckedChange={(v) => toggleScannerAdaptive(v)} />
          </div>
          <div className="shield-tile rounded-xl px-3 py-2.5 flex items-center justify-between">
            <div>
              <p className="text-[11px] font-bold text-slate-200 flex items-center gap-1.5"><Atom className="w-3.5 h-3.5 text-violet-400" />مسیریابی کوانتومی چندمسیره</p>
              <p className="text-[9px] text-slate-500 mt-0.5">تکه‌سازی پکت در ۳ مسیر موازی و بازسازی در مقصد</p>
            </div>
            <Switch checked={multiPath.enabled} onCheckedChange={(v) => toggleQuantumMultiPath(v)} />
          </div>
          <div className="shield-tile rounded-xl px-3 py-2.5 flex items-center justify-between">
            <div>
              <p className="text-[11px] font-bold text-slate-200 flex items-center gap-1.5"><Server className="w-3.5 h-3.5 text-emerald-400" />کشف سرور AI</p>
              <p className="text-[9px] text-slate-500 mt-0.5">بهترین سرور هر سطح را جستجو و انتخاب می‌کند</p>
            </div>
            <Button size="sm" variant="outline" onClick={() => discoverBestServer()} className="gap-1">
              <RefreshCw className="w-3.5 h-3.5" />کشف
            </Button>
          </div>
        </div>

        {/* v10 — hard filter layers */}
        <div className="shield-tile rounded-xl p-4">
          <div className="flex items-center justify-between mb-2.5">
            <p className="text-xs text-slate-400 flex items-center gap-1.5"><ShieldAlert className="w-4 h-4 text-fuchsia-400" />لایه‌های فیلتر سخت (اندازه‌گیری هم‌زمان)</p>
            <Badge className="bg-fuchsia-500/15 text-fuchsia-300 border border-fuchsia-500/30 text-[9px]">
              {hardLayers.filter((h) => h.active).length} از {hardLayers.length} لایه فعال
            </Badge>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-2">
            {hardLayers.map((h) => (
              <div key={h.id} className={`rounded-lg border px-2.5 py-2 ${h.active ? 'border-rose-500/30 bg-rose-500/[0.06]' : 'border-white/10 bg-white/[0.02]'}`}>
                <div className="flex items-center justify-between">
                  <p className="text-[10px] font-bold text-slate-300">{h.nameFa}</p>
                  {h.active ? <span className="animate-pulse inline-block w-1.5 h-1.5 rounded-full bg-rose-400" /> : <span className="inline-block w-1.5 h-1.5 rounded-full bg-emerald-400" />}
                </div>
                <div className="h-1.5 rounded-full bg-white/[0.06] overflow-hidden mt-1.5">
                  <motion.div
                    className={`h-full rounded-full ${h.active ? 'bg-rose-400' : 'bg-emerald-400'}`}
                    initial={false}
                    animate={{ width: `${h.value}%` }}
                    transition={{ duration: 0.6, ease: 'easeOut' }}
                  />
                </div>
                <p className="text-[9px] text-slate-500 mt-1 leading-4">{h.detailFa}</p>
              </div>
            ))}
          </div>
        </div>

        {/* v10 — AI server discovery */}
        <div className="shield-tile rounded-xl p-4">
          <div className="flex items-center justify-between mb-2.5">
            <p className="text-xs text-slate-400 flex items-center gap-1.5"><Server className="w-4 h-4 text-emerald-400" />کشف خودکار سرور — AI قوی‌ترین گره هر سطح را انتخاب می‌کند</p>
            {activeServer && (
              <Badge className="bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 text-[9px]">
                ★ سرور فعال: {activeServer.nameFa} ({toPersianNum(activeServer.score)})
              </Badge>
            )}
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-2">
            {servers.map((sv) => {
              const isActive = sv.id === activeServerId;
              return (
                <div
                  key={sv.id}
                  className={`rounded-lg border px-2.5 py-2 transition-all ${isActive ? 'border-emerald-500/40 bg-emerald-500/[0.08]' : sv.reachable ? 'border-white/10 bg-white/[0.02]' : 'border-rose-500/25 bg-rose-500/[0.05] opacity-70'}`}
                >
                  <div className="flex items-center justify-between">
                    <p className="text-[11px] font-bold text-slate-200 flex items-center gap-1.5">
                      {isActive && <span className="relative flex h-2 w-2"><span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" /><span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-400" /></span>}
                      {sv.nameFa}
                    </p>
                    <span className="text-[9px] text-slate-500">{sv.regionFa}</span>
                  </div>
                  <p className="text-[9px] text-slate-500 mt-0.5 font-mono" dir="ltr">{sv.protocolId}</p>
                  <div className="flex items-center justify-between mt-1.5 text-[9px]">
                    <span className={sv.reachable ? 'text-emerald-400 font-mono' : 'text-rose-400'}>{sv.reachable ? toPersianNum(sv.latencyMs) + ' ms' : 'غیرقابل دسترس'}</span>
                    <span className="text-slate-400">بار {toPersianNum(sv.loadPct)}٪</span>
                    <span className={`font-bold font-mono ${sv.score > 75 ? 'text-emerald-400' : sv.score > 55 ? 'text-amber-400' : 'text-slate-500'}`}>{toPersianNum(sv.score)}</span>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* v10 — strategy learning + forecast */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div className="shield-tile rounded-xl p-4">
            <p className="text-xs text-slate-400 mb-2.5 flex items-center gap-1.5"><TrendingUp className="w-4 h-4 text-cyan-400" />یادگیری هوش مصنوعی — نرخ موفقیت استراتژی‌ها</p>
            <div className="space-y-2">
              {learning.slice().sort((a, b) => b.winRatePct - a.winRatePct).slice(0, 6).map((l) => (
                <div key={l.strategyId} className="space-y-0.5">
                  <div className="flex justify-between text-[10px]">
                    <span className="text-slate-300 font-mono" dir="ltr">{l.strategyId}</span>
                    <span className="text-slate-400">{toPersianNum(l.successes)}/{toPersianNum(l.attempts)} — {toPersianNum(l.winRatePct)}٪ (وزن {l.weight.toFixed(2)})</span>
                  </div>
                  <div className="h-1.5 rounded-full bg-white/[0.06] overflow-hidden">
                    <motion.div
                      className={`h-full rounded-full ${l.winRatePct > 75 ? 'bg-emerald-400' : l.winRatePct > 55 ? 'bg-amber-400' : 'bg-rose-400'}`}
                      initial={false}
                      animate={{ width: `${l.winRatePct}%` }}
                      transition={{ duration: 0.6, ease: 'easeOut' }}
                    />
                  </div>
                </div>
              ))}
            </div>
          </div>

          <div className="shield-tile rounded-xl p-4">
            <p className="text-xs text-slate-400 mb-2.5 flex items-center gap-1.5"><Activity className="w-4 h-4 text-violet-400" />پیش‌بینی شدت فیلترینگ — ۶ ساعت آینده</p>
            <div className="flex items-end gap-2 h-28">
              {forecast.map((f) => (
                <div key={f.hour} className="flex-1 flex flex-col items-center gap-1 min-w-0">
                  <span className={`text-[9px] font-bold ${f.band === 'critical' ? 'text-rose-400' : f.band === 'high' ? 'text-amber-400' : f.band === 'medium' ? 'text-cyan-400' : 'text-emerald-400'}`}>{toPersianNum(f.expectedScore)}</span>
                  <div className="w-full rounded-t-md bg-white/[0.05] overflow-hidden flex items-end" style={{ height: '4.5rem' }}>
                    <motion.div
                      className={`w-full rounded-t-md ${f.band === 'critical' ? 'bg-rose-500/80' : f.band === 'high' ? 'bg-amber-500/80' : f.band === 'medium' ? 'bg-cyan-500/70' : 'bg-emerald-500/70'}`}
                      initial={false}
                      animate={{ height: `${(f.expectedScore / 100) * 100}%` }}
                      transition={{ duration: 0.6, ease: 'easeOut' }}
                    />
                  </div>
                  <span className="text-[9px] text-slate-500 font-mono">{toPersianNum(f.hour)}:00</span>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* v10 — ISP coverage + multi-path */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div className="shield-tile rounded-xl p-4">
            <p className="text-xs text-slate-400 mb-2.5 flex items-center gap-1.5"><Wifi className="w-4 h-4 text-emerald-400" />پوشش اپراتورهای ایران (شبکه ملی)</p>
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-1.5">
              {ispCoverage.map((isp) => (
                <div key={isp.id} className={`rounded-lg border px-2 py-1.5 ${isp.reachable ? 'border-emerald-500/20 bg-emerald-500/[0.04]' : 'border-rose-500/25 bg-rose-500/[0.05]'}`}>
                  <div className="flex items-center gap-1.5">
                    {isp.reachable ? <CheckCircle2 className="w-3 h-3 text-emerald-400 shrink-0" /> : <XCircle className="w-3 h-3 text-rose-400 shrink-0" />}
                    <p className="text-[10px] font-bold text-slate-200 truncate">{isp.nameFa}</p>
                  </div>
                  <p className="text-[9px] text-slate-500 mt-0.5">{isp.reachable ? toPersianNum(isp.latencyMs) + ' ms' : isp.detailFa}</p>
                </div>
              ))}
            </div>
          </div>

          <div className="shield-tile rounded-xl p-4">
            <div className="flex items-center justify-between mb-2.5">
              <p className="text-xs text-slate-400 flex items-center gap-1.5"><Atom className="w-4 h-4 text-violet-400" />مسیرهای کوانتومی فعال</p>
              <Badge className={multiPath.enabled ? 'bg-violet-500/15 text-violet-300 border border-violet-500/30 text-[9px]' : 'bg-white/5 text-slate-500 border border-white/10 text-[9px]'}>
                {multiPath.enabled ? '⚛ چندمسیره فعال' : 'خاموش'}
              </Badge>
            </div>
            <div className="space-y-2">
              {multiPath.activePaths.map((mp) => (
                <div key={mp.id} className={`rounded-lg border px-2.5 py-2 ${mp.active ? 'border-violet-500/30 bg-violet-500/[0.06]' : 'border-white/10 bg-white/[0.02] opacity-60'}`}>
                  <div className="flex items-center justify-between">
                    <p className="text-[10px] font-bold text-slate-200 flex items-center gap-1.5">
                      {mp.active && <span className="relative flex h-2 w-2"><span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-violet-400 opacity-75" /><span className="relative inline-flex rounded-full h-2 w-2 bg-violet-400" /></span>}
                      {mp.pathFa}
                    </p>
                    <span className="text-[9px] font-mono text-slate-400">{mp.active ? toPersianNum(mp.latencyMs) + ' ms' : 'غیرفعال'}</span>
                  </div>
                  <div className="h-1 rounded-full bg-white/[0.06] overflow-hidden mt-1.5">
                    <motion.div className="h-full rounded-full bg-violet-400" initial={false} animate={{ width: `${mp.bytesPct}%` }} transition={{ duration: 0.6 }} />
                  </div>
                  <p className="text-[9px] text-slate-500 mt-1">سهم بار: {toPersianNum(mp.bytesPct)}٪</p>
                </div>
              ))}
            </div>
          </div>
        </div>
        <AutoScannerEngineSections />
      </CardContent>
    </Card>
  );
}
