'use client';

import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  ShieldCheck, Zap, Lock, Cpu, Globe, Activity,
  CheckCircle2, Sparkles, RefreshCw, AlertTriangle,
  Server, ArrowRightLeft, Radio, Network,
  Layers, ChevronRight, EyeOff, Search, FileText,
  AlertOctagon, Check, X, ShieldAlert, Scale,
  Gauge, TestTube, Bug, Flame, LineChart, Terminal
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { Switch } from '@/components/ui/switch';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';
import type { ProtocolResearchEntry, NoveltyClassification } from '@/lib/unified-shield-types';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

export function CircumventionRDPanel() {
  const {
    circumventionRD,
    toggleExtremeCensorshipMode,
    runNetworkSeparationDiagnostic,
    runAISelfTestAudit,
    runRealValidationLadder,
    benchmarkResearchProtocol,
  } = useUnifiedShieldStore();

  const [activeSubTab, setActiveSubTab] = useState<'registry' | 'probes' | 'validation' | 'speed' | 'ai_test'>('registry');
  const [selectedProtocolId, setSelectedProtocolId] = useState<string>(
    circumventionRD.protocolRegistry[0]?.id || 'qns-v4-rd'
  );
  const [isBenchmarking, setIsBenchmarking] = useState<boolean>(false);
  const [isDiagnosing, setIsDiagnosing] = useState<boolean>(false);

  const selectedProto = circumventionRD.protocolRegistry.find(p => p.id === selectedProtocolId) || circumventionRD.protocolRegistry[0];

  const handleRunDiagnostic = () => {
    setIsDiagnosing(true);
    runNetworkSeparationDiagnostic();
    setTimeout(() => {
      setIsDiagnosing(false);
    }, 1000);
  };

  const handleBenchmark = (id: string) => {
    setIsBenchmarking(true);
    benchmarkResearchProtocol(id);
    setTimeout(() => {
      setIsBenchmarking(false);
    }, 1200);
  };

  const getNoveltyBadge = (classification: NoveltyClassification) => {
    switch (classification) {
      case 'NO_IDENTICAL_IMPLEMENTATION_FOUND':
        return <Badge className="bg-emerald-500/20 text-emerald-300 border-emerald-500/40">عدم وجود نمونه همسان در گیت‌هاب/IETF</Badge>;
      case 'PARTIALLY_NOVEL':
        return <Badge className="bg-amber-500/20 text-amber-300 border-amber-500/40">نوآوری ترکیبی / Partially Novel</Badge>;
      case 'RELATED_TECHNOLOGY':
        return <Badge className="bg-cyan-500/20 text-cyan-300 border-cyan-500/40">تکنولوژی مرتبط / Standard RFC</Badge>;
      case 'ALREADY_EXISTS':
        return <Badge className="bg-slate-700/60 text-slate-300 border-slate-600">پروتکل موجود / Existing Transport</Badge>;
      default:
        return <Badge variant="outline">نامشخص</Badge>;
    }
  };

  return (
    <div className="space-y-6" dir="rtl">
      {/* Header Banner */}
      <Card className="bg-gradient-to-r from-slate-900 via-slate-800 to-indigo-950 border-indigo-500/30">
        <CardHeader className="pb-3">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div className="flex items-center gap-3">
              <div className="p-3 bg-indigo-500/20 rounded-xl border border-indigo-500/40 text-indigo-400">
                <Scale className="w-7 h-7" />
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <CardTitle className="text-xl text-slate-100 font-bold">
                    مرکز تحقیق و توسعه پروتکل‌های مبتنی بر شواهد (Directive v80 R&D)
                  </CardTitle>
                  <Badge variant="outline" className="border-indigo-500/50 text-indigo-300 bg-indigo-950/40">
                    Evidence-Driven
                  </Badge>
                </div>
                <CardDescription className="text-slate-400 text-sm mt-1">
                  پلتفرم ارزیابی دقیق، سنجش پذیری نوآوری‌ها، تشخیص جداشدگی شبکه ملی از بین‌الملل و خودآزمایی هوش مصنوعی
                </CardDescription>
              </div>
            </div>

            {/* Extreme Censorship Mode Switch */}
            <div className="flex items-center gap-3 bg-slate-950/60 p-3 rounded-xl border border-white/10">
              <div className="text-right">
                <p className="text-xs font-semibold text-amber-300 flex items-center gap-1">
                  <AlertOctagon className="w-3.5 h-3.5 text-amber-400" />
                  حالت سانسور شدید (Extreme Censorship Mode)
                </p>
                <p className="text-[11px] text-slate-400">افزایش تنوع پروب‌ها و وزن شواهد تاریخی</p>
              </div>
              <Switch
                checked={circumventionRD.extremeCensorshipMode}
                onCheckedChange={(val) => toggleExtremeCensorshipMode(val)}
              />
            </div>
          </div>
        </CardHeader>

        <CardContent className="pt-0">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3 bg-slate-950/40 p-3 rounded-lg border border-white/10 text-xs">
            <div>
              <span className="text-slate-500 block">وضعیت فعلی شبکه:</span>
              <span className="font-bold text-amber-400 flex items-center gap-1 mt-0.5">
                <Activity className="w-3.5 h-3.5" />
                {circumventionRD.networkSeparation.currentCondition}
              </span>
            </div>
            <div>
              <span className="text-slate-500 block">ارزیابی دور زدن:</span>
              <span className="font-bold text-emerald-400 flex items-center gap-1 mt-0.5">
                <CheckCircle2 className="w-3.5 h-3.5" />
                {circumventionRD.networkSeparation.circumventionFeasibilityFa}
              </span>
            </div>
            <div>
              <span className="text-slate-500 block">دسترسی بین‌الملل:</span>
              <span className="font-bold text-cyan-400 mt-0.5 block">
                %{toPersianNum(circumventionRD.networkSeparation.internationalReachability)}
              </span>
            </div>
            <div>
              <span className="text-slate-500 block">آخرین به‌روزرسانی شواهد:</span>
              <span className="font-bold text-slate-300 mt-0.5 block">
                {toPersianNum(new Date(circumventionRD.lastEvidenceCollectionTs).toLocaleTimeString('fa-IR'))}
              </span>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Sub Tab Selection Bar */}
      <div className="flex flex-wrap items-center gap-2 bg-[#0a0f1c]/85 p-1.5 rounded-xl border border-white/10">
        <Button
          size="sm"
          variant={activeSubTab === 'registry' ? 'default' : 'ghost'}
          onClick={() => setActiveSubTab('registry')}
          className={activeSubTab === 'registry' ? 'bg-indigo-600 text-white' : 'text-slate-400'}
        >
          <FileText className="w-4 h-4 ml-1.5" />
          ثبت‌نام و ارزیابی اصالت پروتکل‌ها
        </Button>
        <Button
          size="sm"
          variant={activeSubTab === 'probes' ? 'default' : 'ghost'}
          onClick={() => setActiveSubTab('probes')}
          className={activeSubTab === 'probes' ? 'bg-indigo-600 text-white' : 'text-slate-400'}
        >
          <Network className="w-4 h-4 ml-1.5" />
          پروب‌های تفکیک شبکه ملی/بین‌الملل
        </Button>
        <Button
          size="sm"
          variant={activeSubTab === 'validation' ? 'default' : 'ghost'}
          onClick={() => setActiveSubTab('validation')}
          className={activeSubTab === 'validation' ? 'bg-indigo-600 text-white' : 'text-slate-400'}
        >
          <TestTube className="w-4 h-4 ml-1.5" />
          نردبان صحت‌سنجی ۸ مرحله‌ای
        </Button>
        <Button
          size="sm"
          variant={activeSubTab === 'speed' ? 'default' : 'ghost'}
          onClick={() => setActiveSubTab('speed')}
          className={activeSubTab === 'speed' ? 'bg-indigo-600 text-white' : 'text-slate-400'}
        >
          <Gauge className="w-4 h-4 ml-1.5" />
          هوش سرعت منطقه‌ای (Speed Intelligence)
        </Button>
        <Button
          size="sm"
          variant={activeSubTab === 'ai_test' ? 'default' : 'ghost'}
          onClick={() => setActiveSubTab('ai_test')}
          className={activeSubTab === 'ai_test' ? 'bg-indigo-600 text-white' : 'text-slate-400'}
        >
          <Bug className="w-4 h-4 ml-1.5" />
          خودآزمایی و ممیزی هوش مصنوعی
        </Button>
      </div>

      {/* Sub Tab 1: Protocol Registry & Novelty Report */}
      {activeSubTab === 'registry' && (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Protocol List */}
          <div className="space-y-3">
            <h3 className="text-sm font-semibold text-slate-300 flex items-center justify-between">
              <span>فهرست پروتکل‌های تحت R&D</span>
              <span className="text-xs text-slate-500">{toPersianNum(circumventionRD.protocolRegistry.length)} پروتکل</span>
            </h3>

            {circumventionRD.protocolRegistry.map((p) => (
              <Card
                key={p.id}
                onClick={() => setSelectedProtocolId(p.id)}
                className={`cursor-pointer transition-all duration-200 border-white/10 ${
                  p.id === selectedProtocolId
                    ? 'bg-indigo-950/40 border-indigo-500/60 shadow-lg shadow-indigo-950/50'
                    : 'bg-slate-900/50 hover:bg-slate-800/50'
                }`}
              >
                <CardContent className="p-4">
                  <div className="flex items-start justify-between gap-2">
                    <div>
                      <p className="font-bold text-slate-200 text-sm">{p.nameFa}</p>
                      <p className="text-xs text-slate-400 font-mono mt-0.5">{p.protocol} v{p.version}</p>
                    </div>
                    <Badge variant="outline" className="text-[10px] border-slate-700 text-slate-300">
                      {p.maturity}
                    </Badge>
                  </div>

                  <div className="mt-3 flex items-center justify-between text-xs">
                    <span className="text-slate-400">امتیاز مقاومت:</span>
                    <span className="text-emerald-400 font-mono font-bold">
                      {toPersianNum(p.censorshipResistanceScore)} / ۱۰۰
                    </span>
                  </div>

                  <div className="mt-2 text-xs">
                    {getNoveltyBadge(p.noveltyClassification)}
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>

          {/* Detailed Protocol Inspection & NOVELTY_REPORT.md View */}
          <div className="lg:col-span-2 space-y-4">
            <Card className="shield-surface-strong">
              <CardHeader className="pb-3 border-b border-white/10">
                <div className="flex items-center justify-between">
                  <div>
                    <CardTitle className="text-lg text-slate-100 flex items-center gap-2">
                      <Terminal className="w-5 h-5 text-indigo-400" />
                      گزارش ارزیابی اصالت و شواهد: {selectedProto.nameFa}
                    </CardTitle>
                    <CardDescription className="text-slate-400 text-xs font-mono mt-1">
                      {selectedProto.protocol} | Source: {selectedProto.source}
                    </CardDescription>
                  </div>

                  <Button
                    size="sm"
                    onClick={() => handleBenchmark(selectedProto.id)}
                    disabled={isBenchmarking}
                    className="bg-indigo-600 hover:bg-indigo-500 text-white"
                  >
                    <RefreshCw className={`w-3.5 h-3.5 ml-1.5 ${isBenchmarking ? 'animate-spin' : ''}`} />
                    بنچ‌مارک مجدد sh
                  </Button>
                </div>
              </CardHeader>

              <CardContent className="p-5 space-y-5">
                {/* Scorecard Matrix */}
                <div>
                  <h4 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">
                    کارت امتیاز سنجش‌پذیری (Protocol Scorecard)
                  </h4>
                  <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
                    <div className="bg-slate-950/60 p-3 rounded-lg border border-white/10">
                      <span className="text-slate-500 text-xs block">عملکرد (Performance):</span>
                      <span className="text-lg font-bold text-cyan-400 font-mono">
                        {toPersianNum(selectedProto.performanceScore)} / ۱۰۰
                      </span>
                      <Progress value={selectedProto.performanceScore} className="h-1 mt-1.5 bg-slate-800" />
                    </div>

                    <div className="bg-slate-950/60 p-3 rounded-lg border border-white/10">
                      <span className="text-slate-500 text-xs block">پایداری (Reliability):</span>
                      <span className="text-lg font-bold text-emerald-400 font-mono">
                        {toPersianNum(selectedProto.reliabilityScore)} / ۱۰۰
                      </span>
                      <Progress value={selectedProto.reliabilityScore} className="h-1 mt-1.5 bg-slate-800" />
                    </div>

                    <div className="bg-slate-950/60 p-3 rounded-lg border border-white/10">
                      <span className="text-slate-500 text-xs block">مقاومت سانسور:</span>
                      <span className="text-lg font-bold text-indigo-400 font-mono">
                        {toPersianNum(selectedProto.censorshipResistanceScore)} / ۱۰۰
                      </span>
                      <Progress value={selectedProto.censorshipResistanceScore} className="h-1 mt-1.5 bg-slate-800" />
                    </div>

                    <div className="bg-slate-950/60 p-3 rounded-lg border border-white/10">
                      <span className="text-slate-500 text-xs block">ریسک شناسایی (Detectability):</span>
                      <span className="text-lg font-bold text-amber-400 font-mono">
                        {toPersianNum(selectedProto.detectabilityRiskScore)} / ۱۰۰
                      </span>
                      <Progress value={selectedProto.detectabilityRiskScore} className="h-1 mt-1.5 bg-slate-800" />
                    </div>

                    <div className="bg-slate-950/60 p-3 rounded-lg border border-white/10">
                      <span className="text-slate-500 text-xs block">کیفیت شواهد (Evidence):</span>
                      <span className="text-lg font-bold text-emerald-300 font-mono">
                        {toPersianNum(selectedProto.evidenceQualityScore)} / ۱۰۰
                      </span>
                      <Progress value={selectedProto.evidenceQualityScore} className="h-1 mt-1.5 bg-slate-800" />
                    </div>

                    <div className="bg-slate-950/60 p-3 rounded-lg border border-white/10">
                      <span className="text-slate-500 text-xs block">تعداد تست‌های مستند:</span>
                      <span className="text-lg font-bold text-slate-200 font-mono">
                        {toPersianNum(selectedProto.evidenceCount)}
                      </span>
                      <span className="text-[10px] text-slate-500 block mt-1">نمونه واقعی ثبت‌شده</span>
                    </div>
                  </div>
                </div>

                {/* NOVELTY_REPORT.md Verdict */}
                <div className="bg-slate-950/80 p-4 rounded-xl border border-indigo-500/20 space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-semibold text-indigo-300 flex items-center gap-1.5">
                      <Sparkles className="w-4 h-4 text-indigo-400" />
                      نتیجه بررسی گیت‌هاب و منابع پژوهشی (NOVELTY_REPORT)
                    </span>
                    {getNoveltyBadge(selectedProto.noveltyClassification)}
                  </div>

                  <p className="text-sm text-slate-200 leading-relaxed pt-1">
                    {selectedProto.noveltyVerdictSummaryFa}
                  </p>
                  <p className="text-xs text-slate-400 font-mono bg-[#0a0f1c]/90 p-2.5 rounded border border-white/10Dir font-sans dir-ltr text-left">
                    {selectedProto.noveltyVerdictSummary}
                  </p>
                </div>

                {/* Known Limitations */}
                <div>
                  <h4 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">
                    محدودیت‌های فنی و عملیاتی شناسایی‌شده (Known Limitations)
                  </h4>
                  <div className="space-y-1.5">
                    {selectedProto.knownLimitationsFa.map((lim, idx) => (
                      <div key={idx} className="flex items-start gap-2 text-xs text-amber-300/90 bg-amber-950/20 p-2 rounded border border-amber-900/30">
                        <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
                        <span>{lim}</span>
                      </div>
                    ))}
                  </div>
                </div>
              </CardContent>
            </Card>
          </div>
        </div>
      )}

      {/* Sub Tab 2: Control Probes & Separation Detector */}
      {activeSubTab === 'probes' && (
        <div className="space-y-6">
          <Card className="shield-surface-strong">
            <CardHeader className="pb-3 flex flex-row items-center justify-between">
              <div>
                <CardTitle className="text-base text-slate-100 flex items-center gap-2">
                  <Network className="w-5 h-5 text-indigo-400" />
                  تحلیل جداشدگی شبکه ملی و تست کنترل پروب‌های مستقل
                </CardTitle>
                <CardDescription className="text-xs text-slate-400 mt-1">
                  عدم خلط فیلترینگ DPI با قطع واقعی مسیریابی بین‌الملل جهت جلوگیری از اعلام نتایج کاذب
                </CardDescription>
              </div>

              <Button
                onClick={handleRunDiagnostic}
                disabled={isDiagnosing}
                className="bg-indigo-600 hover:bg-indigo-500 text-white text-xs"
              >
                <RefreshCw className={`w-3.5 h-3.5 ml-1.5 ${isDiagnosing ? 'animate-spin' : ''}`} />
                سنجش بلادرنگ پروب‌ها
              </Button>
            </CardHeader>

            <CardContent className="space-y-6">
              {/* Probes Grid */}
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {circumventionRD.networkSeparation.activeControlProbes.map((probe, idx) => (
                  <div
                    key={idx}
                    className="bg-slate-950/70 p-4 rounded-xl border border-white/10 space-y-3"
                  >
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-mono font-semibold text-indigo-400">
                        {probe.probeType}
                      </span>
                      <Badge variant="outline" className={probe.reachable ? 'bg-emerald-950/40 border-emerald-500/50 text-emerald-400' : 'bg-red-950/40 border-red-500/50 text-red-400'}>
                        {probe.reachable ? 'برقرار (PASS)' : 'قطع (FAIL)'}
                      </Badge>
                    </div>

                    <div>
                      <p className="text-sm font-bold text-slate-200">{probe.targetFa}</p>
                      <p className="text-xs text-slate-400 font-mono">{probe.target}</p>
                    </div>

                    <div className="flex items-center justify-between text-xs pt-2 border-t border-white/10/80">
                      <span className="text-slate-400">تأخیر: <strong className="text-cyan-400 font-mono">{toPersianNum(probe.latencyMs)}ms</strong></span>
                      <span className="text-slate-400">افت پکت: <strong className="text-amber-400 font-mono">٪{toPersianNum(probe.lossRate)}</strong></span>
                    </div>

                    <p className="text-[11px] text-slate-300 bg-[#0a0f1c]/85 p-2 rounded border border-white/10">
                      {probe.statusTextFa}
                    </p>
                  </div>
                ))}
              </div>

              {/* Probabilities Summary */}
              <div className="bg-slate-950/80 p-4 rounded-xl border border-white/10 space-y-3">
                <h4 className="text-xs font-semibold text-slate-300">محاسبات آماری نرخ دسترس‌پذیری و احتمال مسدودسازی</h4>
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-xs">
                  <div>
                    <span className="text-slate-500 block">دسترس‌پذیری داخلی (Domestic):</span>
                    <span className="text-lg font-bold text-emerald-400 font-mono">
                      ٪{toPersianNum(circumventionRD.networkSeparation.domesticReachability)}
                    </span>
                  </div>
                  <div>
                    <span className="text-slate-500 block">دسترس‌پذیری بین‌الملل (International):</span>
                    <span className="text-lg font-bold text-cyan-400 font-mono">
                      ٪{toPersianNum(circumventionRD.networkSeparation.internationalReachability)}
                    </span>
                  </div>
                  <div>
                    <span className="text-slate-500 block">احتمال سانسور لایه ۷ (Censorship):</span>
                    <span className="text-lg font-bold text-indigo-400 font-mono">
                      ٪{toPersianNum(circumventionRD.networkSeparation.censorshipProbability)}
                    </span>
                  </div>
                  <div>
                    <span className="text-slate-500 block">احتمال اختلال مسیریابی فیزیکی:</span>
                    <span className="text-lg font-bold text-amber-400 font-mono">
                      ٪{toPersianNum(circumventionRD.networkSeparation.routingFailureProbability)}
                    </span>
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {/* Sub Tab 3: 8-Stage Real Validation Ladder */}
      {activeSubTab === 'validation' && (
        <Card className="shield-surface-strong">
          <CardHeader className="pb-3">
            <CardTitle className="text-base text-slate-100 flex items-center gap-2">
              <TestTube className="w-5 h-5 text-indigo-400" />
              نردبان صحت‌سنجی واقعی ۸ مرحله‌ای (Progressive Depth Testing)
            </CardTitle>
            <CardDescription className="text-xs text-slate-400 mt-1">
              عدم اعطای نشان HEALTHY به تست‌های لایه ۴ سطحی؛ تایید کامل تا سطح برنامه کاربردی
            </CardDescription>
          </CardHeader>

          <CardContent className="space-y-4">
            <div className="space-y-3">
              {circumventionRD.realTimeValidationLadder.map((st) => (
                <div
                  key={st.stageNumber}
                  className="bg-slate-950/70 p-3.5 rounded-xl border border-white/10 flex flex-wrap items-center justify-between gap-4"
                >
                  <div className="flex items-center gap-3">
                    <div className="w-8 h-8 rounded-full bg-indigo-950 text-indigo-300 font-bold font-mono text-sm flex items-center justify-center border border-indigo-500/40">
                      {toPersianNum(st.stageNumber)}
                    </div>
                    <div>
                      <p className="font-bold text-slate-200 text-sm">{st.stageNameFa}</p>
                      <p className="text-xs text-slate-400 font-mono">{st.stageName}</p>
                    </div>
                  </div>

                  <div className="flex items-center gap-4 text-xs">
                    <span className="text-slate-400 font-mono">
                      تأخیر: <strong className="text-cyan-400">{toPersianNum(st.latencyMs)}ms</strong>
                    </span>
                    <Badge className="bg-emerald-500/20 text-emerald-300 border-emerald-500/40">
                      <Check className="w-3 h-3 ml-1" />
                      {st.status}
                    </Badge>
                  </div>

                  <div className="w-full text-xs text-slate-400 font-mono bg-[#0a0f1c]/90 p-2 rounded border border-white/10 dir-ltr text-left">
                    Evidence: {st.evidenceSnippet}
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Sub Tab 4: Regional Speed Intelligence */}
      {activeSubTab === 'speed' && (
        <Card className="shield-surface-strong">
          <CardHeader className="pb-3">
            <CardTitle className="text-base text-slate-100 flex items-center gap-2">
              <Gauge className="w-5 h-5 text-indigo-400" />
              هوش سرعت منطقه‌ای واقعی (FASTEST_CURRENTLY_OBSERVED)
            </CardTitle>
            <CardDescription className="text-xs text-slate-400 mt-1">
              عدم ادعای سرعت مطلق کلی؛ ارزیابی تفکیکی بر اساس اپراتور و ISP
            </CardDescription>
          </CardHeader>

          <CardContent>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {circumventionRD.speedIntelligence.map((sp) => (
                <div
                  key={sp.regionKey}
                  className="bg-slate-950/70 p-4 rounded-xl border border-white/10 space-y-3"
                >
                  <div className="flex items-center justify-between">
                    <Badge variant="outline" className="text-xs border-indigo-500/40 text-indigo-300 bg-indigo-950/30">
                      {sp.regionKey}
                    </Badge>
                    <span className="text-[11px] text-slate-500 font-mono">
                      اطمینان: ٪{toPersianNum(sp.confidencePercent)}
                    </span>
                  </div>

                  <h4 className="font-bold text-slate-200 text-sm">{sp.regionNameFa}</h4>

                  <div className="bg-indigo-950/30 p-3 rounded-lg border border-indigo-500/20 space-y-1">
                    <span className="text-xs text-slate-400 block">پروتکل برتر مشاهده‌شده:</span>
                    <span className="text-sm font-bold text-emerald-400">{sp.fastestProtocolFa}</span>
                  </div>

                  <div className="grid grid-cols-2 gap-2 text-xs">
                    <div className="bg-[#0a0f1c]/85 p-2 rounded border border-white/10">
                      <span className="text-slate-500 block">میانگین تأخیر:</span>
                      <span className="text-cyan-400 font-bold font-mono">{toPersianNum(sp.medianLatencyMs)} ms</span>
                    </div>
                    <div className="bg-[#0a0f1c]/85 p-2 rounded border border-white/10">
                      <span className="text-slate-500 block">پهنای باند:</span>
                      <span className="text-emerald-400 font-bold font-mono">{toPersianNum(sp.bandwidthMbps)} Mbps</span>
                    </div>
                  </div>

                  <div className="flex items-center justify-between text-[11px] text-slate-500 pt-1">
                    <span>حجم نمونه: {toPersianNum(sp.sampleSize)} آزمون</span>
                    <span>{toPersianNum(new Date(sp.timestampIso).toLocaleTimeString('fa-IR'))}</span>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Sub Tab 5: AI Self-Test */}
      {activeSubTab === 'ai_test' && (
        <Card className="shield-surface-strong">
          <CardHeader className="pb-3 flex flex-row items-center justify-between">
            <div>
              <CardTitle className="text-base text-slate-100 flex items-center gap-2">
                <Bug className="w-5 h-5 text-indigo-400" />
                ممیزی و خودآزمایی هوش مصنوعی داخلی (AI Self-Test Matrix)
              </CardTitle>
              <CardDescription className="text-xs text-slate-400 mt-1">
                تست هوش مصنوعی با داده‌های متناقض و گمراه‌کننده جهت اطمینان از تفکیک OBSERVED ، INFERRED و PREDICTED
              </CardDescription>
            </div>

            <Button
              onClick={() => runAISelfTestAudit()}
              className="bg-indigo-600 hover:bg-indigo-500 text-white text-xs"
            >
              <RefreshCw className="w-3.5 h-3.5 ml-1.5" />
              اجرای سناریوهای خصمانه AI
            </Button>
          </CardHeader>

          <CardContent className="space-y-4">
            <div className="space-y-3">
              {circumventionRD.aiSelfTestHistory.map((ai) => (
                <div
                  key={ai.adversarialScenarioId}
                  className="bg-slate-950/70 p-4 rounded-xl border border-white/10 space-y-3"
                >
                  <div className="flex items-center justify-between">
                    <span className="font-bold text-slate-200 text-sm flex items-center gap-2">
                      <ShieldAlert className="w-4 h-4 text-indigo-400" />
                      {ai.scenarioNameFa}
                    </span>
                    <Badge className="bg-emerald-500/20 text-emerald-300 border-emerald-500/40">
                      <Check className="w-3 h-3 ml-1" />
                      {ai.verdict} (اطمینان ٪{toPersianNum(ai.confidenceScore)})
                    </Badge>
                  </div>

                  <div className="text-xs text-slate-400 bg-[#0a0f1c]/90 p-2.5 rounded border border-white/10">
                    <span className="text-indigo-300 font-semibold block mb-1">ورودی سناریو (Input Condition):</span>
                    <span className="font-mono">{ai.inputCondition}</span>
                  </div>

                  <div className="text-xs text-slate-200 bg-indigo-950/30 p-2.5 rounded border border-indigo-500/30">
                    <span className="text-emerald-400 font-semibold block mb-1">استنتاج هوش مصنوعی (AI Inference Output):</span>
                    <span className="font-mono">{ai.aiClassification}</span>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
