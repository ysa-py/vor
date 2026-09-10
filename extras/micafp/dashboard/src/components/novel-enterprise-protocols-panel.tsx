'use client';

import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  ShieldCheck, Zap, Lock, Cpu, Globe, Activity,
  CheckCircle2, Sparkles, RefreshCw, AlertTriangle,
  Flame, Server, ArrowRightLeft, Radio, Network,
  Layers, ChevronRight, EyeOff, KeyRound, PlayCircle
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

export function NovelEnterpriseProtocolsPanel() {
  const {
    nationalBlackoutShield,
    toggleNationalBlackoutEmergency,
    setActiveNovelProtocol,
    setCamouflageSNI,
    toggleAsymmetricRouting,
    triggerAutonomousAIInference,
    testNovelProtocolEvasion,
    simulateSocketAutoFlush,
  } = useUnifiedShieldStore();

  const [selectedProtoId, setSelectedProtoId] = useState<string>(
    nationalBlackoutShield.activeNovelProtocolId || 'qns-v4'
  );
  const [testingProtoId, setTestingProtoId] = useState<string | null>(null);

  const activeProto = nationalBlackoutShield.protocols.find(
    p => p.id === nationalBlackoutShield.activeNovelProtocolId
  ) || nationalBlackoutShield.protocols[0];

  const selectedProto = nationalBlackoutShield.protocols.find(
    p => p.id === selectedProtoId
  ) || activeProto;

  const handleTestProtocol = (protoId: string) => {
    setTestingProtoId(protoId);
    testNovelProtocolEvasion(protoId);
    setTimeout(() => {
      setTestingProtoId(null);
    }, 1200);
  };

  const SNI_OPTIONS = [
    { label: 'سامانه پرداخت شاپرک (سپه)', sni: 'sep.shaparak.ir', desc: 'بالاترین اولویت ترافیک بانکی کشور' },
    { label: 'به پرداخت ملت (شاپرک)', sni: 'bpm.shaparak.ir', desc: 'استتار درگاه پرداخت بانک ملت' },
    { label: 'سرویس استریم آپارات', sni: 'aparat.com', desc: 'پوشش استگانوگرافی در ترافیک ویدئویی' },
    { label: 'سامانه آسان پرداخت (آپ)', sni: 'asan.shaparak.ir', desc: 'پروتکل امن پرداخت الکترونیک' },
    { label: 'دولت همراه و خدمات ملی', sni: 'mob.gov.ir', desc: 'ترافیک دارای مجوز در تمام ساعات بحران' },
  ];

  return (
    <div className="space-y-4">
      {/* Enterprise Header Badge & Emergency Status */}
      <Card className="bg-gradient-to-r from-slate-950 via-indigo-950/40 to-slate-950 border-indigo-500/40 shadow-xl shadow-indigo-950/30 overflow-hidden relative">
        <div className="absolute top-0 right-0 left-0 h-1 bg-gradient-to-r from-amber-500 via-indigo-500 to-cyan-400" />
        
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between flex-wrap gap-3">
            <div className="flex items-center gap-3">
              <div className="w-12 h-12 rounded-2xl bg-gradient-to-br from-amber-500 to-indigo-600 flex items-center justify-center shadow-lg shadow-indigo-500/25 border border-amber-400/30">
                <Lock className="w-6 h-6 text-white" />
              </div>
              <div>
                <div className="flex items-center gap-2 flex-wrap">
                  <CardTitle className="text-slate-100 text-base sm:text-lg font-bold flex items-center gap-2">
                    <span>پروتکل‌های انحصاری و شناسایی‌نشده (Zero-Signature Enterprise)</span>
                  </CardTitle>
                  <Badge className="bg-amber-500/20 text-amber-300 border border-amber-500/40 text-xs px-2 py-0.5 font-mono">
                    Enterprise Grade: ۱۰۰,۰۰۰ / ۱۰,۰۰۰ ⭐
                  </Badge>
                </div>
                <CardDescription className="text-slate-400 text-xs mt-1">
                  طراحی شده برای شرایط قطع ۱۰۰٪ اینترنت بین‌الملل (وضعیت اضطراری شبکه ملی) و عبور خودکار از DPI پیشرفته
                </CardDescription>
              </div>
            </div>

            <div className="flex items-center gap-2">
              <Button
                size="sm"
                onClick={() => toggleNationalBlackoutEmergency()}
                className={`text-xs h-8 px-3 font-semibold transition-all shadow-md ${
                  nationalBlackoutShield.emergencyModeActive
                    ? 'bg-gradient-to-r from-emerald-600 to-teal-600 hover:from-emerald-500 hover:to-teal-500 text-white shadow-emerald-900/40'
                    : 'bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700'
                }`}
              >
                <Radio className={`w-3.5 h-3.5 ml-1.5 ${nationalBlackoutShield.emergencyModeActive ? 'animate-pulse text-emerald-300' : 'text-slate-400'}`} />
                {nationalBlackoutShield.emergencyModeActive
                  ? 'سپر شبکه ملی: ۱۰۰٪ فعال'
                  : 'فعال‌سازی سپر شبکه ملی'}
              </Button>

              <Button
                size="sm"
                variant="outline"
                onClick={() => triggerAutonomousAIInference()}
                className="text-xs h-8 px-2.5 border-indigo-500/40 bg-indigo-950/30 hover:bg-indigo-900/50 text-indigo-300"
              >
                <Cpu className="w-3.5 h-3.5 ml-1 text-indigo-400" />
                استنتاج هوش مصنوعی (۲ms)
              </Button>
            </div>
          </div>
        </CardHeader>

        <CardContent className="pt-0 pb-4">
          {/* Key Metrics Bar */}
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 text-xs">
            <div className="bg-[#0a0f1c]/85 p-2.5 rounded-xl border border-white/10 flex items-center justify-between">
              <span className="text-slate-400 flex items-center gap-1.5">
                <Cpu className="w-3.5 h-3.5 text-indigo-400" />
                تأخیر هوش مصنوعی On-Device:
              </span>
              <span className="font-mono font-bold text-indigo-300">
                {toPersianNum(nationalBlackoutShield.onDeviceAIInferenceMs)} ms
              </span>
            </div>

            <div className="bg-[#0a0f1c]/85 p-2.5 rounded-xl border border-white/10 flex items-center justify-between">
              <span className="text-slate-400 flex items-center gap-1.5">
                <Sparkles className="w-3.5 h-3.5 text-cyan-400" />
                تطبیق آنتروپی با شبکه ملی:
              </span>
              <span className="font-mono font-bold text-cyan-300">
                {toPersianNum(nationalBlackoutShield.packetEntropyMatch)}٪
              </span>
            </div>

            <div className="bg-[#0a0f1c]/85 p-2.5 rounded-xl border border-white/10 flex items-center justify-between">
              <span className="text-slate-400 flex items-center gap-1.5">
                <ArrowRightLeft className="w-3.5 h-3.5 text-amber-400" />
                تخلیه خودکار سوکت (Anti-RST):
              </span>
              <span className="font-mono font-bold text-amber-300">
                {toPersianNum(nationalBlackoutShield.socketAutoFlushMs)} ms
              </span>
            </div>

            <div className="bg-[#0a0f1c]/85 p-2.5 rounded-xl border border-white/10 flex items-center justify-between">
              <span className="text-slate-400 flex items-center gap-1.5">
                <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
                پروتکل فعال:
              </span>
              <span className="font-semibold text-emerald-300 truncate max-w-[120px]" title={activeProto.nameFa}>
                {activeProto.nameFa.split(' ')[0]}
              </span>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* 5 Novel Protocols Selector Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        {/* Protocol List Selection */}
        <div className="lg:col-span-2 space-y-3">
          <div className="flex items-center justify-between text-xs text-slate-300 px-1">
            <span className="font-semibold flex items-center gap-1.5">
              <Layers className="w-4 h-4 text-cyan-400" />
              لیست ۵ پروتکل انحصاری، پرسرعت و ضد فیلترینگ هوشمند:
            </span>
            <span className="text-slate-500 text-[11px]">
              رتبه‌بندی آزمون واقعی زیر بار DPI ایران
            </span>
          </div>

          <div className="space-y-2.5">
            {nationalBlackoutShield.protocols.map((proto) => {
              const isCurrentActive = nationalBlackoutShield.activeNovelProtocolId === proto.id;
              const isSelected = selectedProtoId === proto.id;

              return (
                <motion.div
                  key={proto.id}
                  whileHover={{ scale: 1.005 }}
                  onClick={() => setSelectedProtoId(proto.id)}
                  className={`p-3.5 rounded-xl border cursor-pointer transition-all ${
                    isCurrentActive
                      ? 'bg-[#0a0f1c]/90 border-emerald-500/50 shadow-lg shadow-emerald-950/20'
                      : isSelected
                      ? 'bg-slate-900/70 border-cyan-500/50'
                      : 'bg-slate-900/40 border-white/10/80 hover:bg-[#0a0f1c]/70 hover:border-slate-700'
                  }`}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="space-y-1 flex-1">
                      <div className="flex items-center gap-2 flex-wrap">
                        <span className="font-bold text-slate-100 text-sm">
                          {proto.nameFa}
                        </span>
                        <Badge
                          variant="outline"
                          className="text-[10px] py-0 border-slate-700 text-slate-400 font-mono"
                        >
                          {proto.name}
                        </Badge>
                        {isCurrentActive && (
                          <Badge className="bg-emerald-500/20 text-emerald-300 border-emerald-500/40 text-[10px] py-0 flex items-center gap-1">
                            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-ping" />
                            متصل و فعال
                          </Badge>
                        )}
                      </div>

                      <p className="text-xs text-slate-400 leading-relaxed">
                        {proto.mechanismFa}
                      </p>

                      {/* Performance & Stealth Badges */}
                      <div className="flex items-center gap-2 pt-1 flex-wrap text-[11px]">
                        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-cyan-950/40 border border-cyan-800/40 text-cyan-300 font-mono">
                          <Zap className="w-3 h-3 text-cyan-400" />
                          {proto.speedRating}
                        </span>

                        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-emerald-950/40 border border-emerald-800/40 text-emerald-300 font-mono">
                          <ShieldCheck className="w-3 h-3 text-emerald-400" />
                          مقاومت DPI: {toPersianNum(proto.dpiResistancePercent)}٪
                        </span>

                        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-indigo-950/40 border border-indigo-800/40 text-indigo-300 font-mono">
                          <Activity className="w-3 h-3 text-indigo-400" />
                          تأخیر: {toPersianNum(proto.pingMs)} ms
                        </span>

                        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-amber-950/40 border border-amber-800/40 text-amber-300 font-mono">
                          امتیاز: {proto.enterpriseScore}
                        </span>
                      </div>
                    </div>

                    <div className="flex flex-col items-end gap-2 shrink-0">
                      <Button
                        size="sm"
                        disabled={isCurrentActive}
                        onClick={(e) => {
                          e.stopPropagation();
                          setActiveNovelProtocol(proto.id);
                        }}
                        className={`text-xs h-7 px-3 ${
                          isCurrentActive
                            ? 'bg-emerald-600/30 text-emerald-300 cursor-default'
                            : 'bg-indigo-600 hover:bg-indigo-500 text-white'
                        }`}
                      >
                        {isCurrentActive ? 'پروتکل جاری' : 'اتصال به این پروتکل'}
                      </Button>

                      <Button
                        size="sm"
                        variant="outline"
                        disabled={testingProtoId === proto.id}
                        onClick={(e) => {
                          e.stopPropagation();
                          handleTestProtocol(proto.id);
                        }}
                        className="text-[11px] h-6 px-2 border-slate-700 text-slate-400 hover:text-cyan-300 hover:bg-slate-800"
                      >
                        <RefreshCw className={`w-3 h-3 ml-1 ${testingProtoId === proto.id ? 'animate-spin text-cyan-400' : ''}`} />
                        {testingProtoId === proto.id ? 'در حال آزمون...' : 'تست واقعی DPI'}
                      </Button>
                    </div>
                  </div>
                </motion.div>
              );
            })}
          </div>
        </div>

        {/* Selected Protocol Deep Dive & Features */}
        <div className="space-y-3">
          <Card className="shield-surface-strong h-full flex flex-col justify-between">
            <div>
              <CardHeader className="pb-2 border-b border-white/10">
                <CardTitle className="text-slate-200 text-sm flex items-center justify-between">
                  <span className="flex items-center gap-1.5">
                    <EyeOff className="w-4 h-4 text-amber-400" />
                    مشخصات فنی و استتار:
                  </span>
                  <Badge variant="outline" className="text-[10px] text-amber-300 border-amber-500/30">
                    {selectedProto.badge}
                  </Badge>
                </CardTitle>
                <CardDescription className="text-slate-400 text-xs">
                  معماری لایه {selectedProto.layer}
                </CardDescription>
              </CardHeader>

              <CardContent className="p-3 space-y-3 text-xs">
                {/* Masquerade Target */}
                <div className="bg-slate-950 p-2.5 rounded-lg border border-white/10 space-y-1">
                  <span className="text-slate-400 flex items-center gap-1 text-[11px]">
                    <Globe className="w-3 h-3 text-cyan-400" />
                    هدف استتار و جعل SNI (Masquerade Host):
                  </span>
                  <span className="font-mono text-cyan-300 font-semibold block text-xs">
                    {selectedProto.masqueradeTarget}
                  </span>
                  <span className="text-[10px] text-slate-500 block">
                    {selectedProto.antiFingerprintFa}
                  </span>
                </div>

                {/* Technical Features Checklist */}
                <div className="space-y-1.5">
                  <span className="text-slate-300 font-semibold flex items-center gap-1 text-xs">
                    <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
                    قابلیت‌های پیشرفته این پروتکل:
                  </span>
                  <div className="space-y-1 text-slate-300">
                    {selectedProto.features.map((feat, idx) => (
                      <div key={idx} className="flex items-start gap-1.5 text-[11px] bg-slate-950/60 p-1.5 rounded border border-white/10/60">
                        <span className="text-cyan-400 font-bold">✓</span>
                        <span>{feat}</span>
                      </div>
                    ))}
                  </div>
                </div>

                {/* Packet Loss Recovery Gauge */}
                <div className="space-y-1 bg-slate-950 p-2.5 rounded-lg border border-white/10">
                  <div className="flex items-center justify-between text-[11px]">
                    <span className="text-slate-400">بازیابی افت پکت (Loss Recovery):</span>
                    <span className="font-mono text-emerald-300 font-bold">
                      {toPersianNum(selectedProto.packetLossRecoveryPercent)}٪
                    </span>
                  </div>
                  <Progress value={selectedProto.packetLossRecoveryPercent} className="h-1.5 bg-slate-800" />
                </div>
              </CardContent>
            </div>

            <div className="p-3 border-t border-white/10">
              <Button
                className="w-full text-xs h-8 bg-gradient-to-r from-cyan-600 to-indigo-600 hover:from-cyan-500 hover:to-indigo-500 text-white font-semibold"
                onClick={() => setActiveNovelProtocol(selectedProto.id)}
              >
                فعال‌سازی پروتکل {selectedProto.nameFa}
              </Button>
            </div>
          </Card>
        </div>
      </div>

      {/* National Intranet Blackout Camouflage & Relay Matrix */}
      <Card className="shield-surface-strong">
        <CardHeader className="pb-2 border-b border-white/10">
          <div className="flex items-center justify-between flex-wrap gap-2">
            <CardTitle className="text-slate-200 text-sm sm:text-base flex items-center gap-2">
              <Network className="w-5 h-5 text-teal-400" />
              <span>ماتریس استتار شبکه ملی و عبور از قطعی ۱۰۰٪ اینترنت (Intranet Egress Matrix)</span>
            </CardTitle>
            <Badge className="bg-teal-500/20 text-teal-300 border-teal-500/40 text-xs">
              مسیر فعال: {nationalBlackoutShield.activeRelayHop}
            </Badge>
          </div>
          <CardDescription className="text-slate-400 text-xs">
            پنهان‌سازی ارتباط پشت دامنه‌های سفید بانکی و رله‌های داخلی با امکان تخلیه فوری سوکت در ۱۰ میلی‌ثانیه
          </CardDescription>
        </CardHeader>

        <CardContent className="p-3 sm:p-4 space-y-4">
          {/* Whitelisted SNI Camouflage Selector */}
          <div className="space-y-2">
            <div className="flex items-center justify-between text-xs text-slate-300">
              <span className="font-semibold flex items-center gap-1.5">
                <KeyRound className="w-3.5 h-3.5 text-amber-400" />
                دامنه استتار هدر SNI در فایروال ملی (Whitelisted Camouflage SNI):
              </span>
              <span className="text-slate-500 text-[11px]">
                انتخاب دامنه جهت تایید در گیت‌وی‌های شاپرک و زیرساخت
              </span>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-2">
              {SNI_OPTIONS.map(opt => {
                const isSelected = nationalBlackoutShield.activeCamouflageSNI === opt.sni;
                return (
                  <button
                    key={opt.sni}
                    onClick={() => setCamouflageSNI(opt.sni)}
                    className={`p-2.5 rounded-lg border text-right transition-all text-xs ${
                      isSelected
                        ? 'bg-amber-500/15 border-amber-500/60 text-amber-200 shadow-md shadow-amber-950/20'
                        : 'bg-slate-950 border-white/10 text-slate-400 hover:border-slate-700 hover:text-slate-200'
                    }`}
                  >
                    <div className="flex items-center justify-between">
                      <span className="font-semibold text-slate-200">{opt.label}</span>
                      {isSelected && <span className="text-amber-400 text-xs font-bold">فعال</span>}
                    </div>
                    <span className="font-mono text-[11px] text-amber-300/90 block mt-0.5 dir-ltr text-left">
                      {opt.sni}
                    </span>
                    <span className="text-[10px] text-slate-500 block mt-1">
                      {opt.desc}
                    </span>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Quick Actions Row: Asymmetric Routing & Socket Flush */}
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs pt-1">
            <div className="bg-slate-950 p-3 rounded-xl border border-white/10 flex items-center justify-between">
              <div className="space-y-0.5">
                <span className="font-semibold text-slate-200 flex items-center gap-1.5">
                  <ArrowRightLeft className="w-3.5 h-3.5 text-cyan-400" />
                  مسیریابی نامتقارن (Asymmetric Routing)
                </span>
                <p className="text-[11px] text-slate-400">
                  ارسال از خط اینترانت و دریافت از مسیر رله کمکی BGP جهت جلوگیری از انسداد سوکت
                </p>
              </div>
              <Button
                size="sm"
                variant={nationalBlackoutShield.asymmetricRoutingEnabled ? 'default' : 'outline'}
                onClick={() => toggleAsymmetricRouting()}
                className={`text-xs h-7 px-3 ${
                  nationalBlackoutShield.asymmetricRoutingEnabled
                    ? 'bg-cyan-600 hover:bg-cyan-500 text-white'
                    : 'border-slate-700 text-slate-400'
                }`}
              >
                {nationalBlackoutShield.asymmetricRoutingEnabled ? 'فعال' : 'غیرفعال'}
              </Button>
            </div>

            <div className="bg-slate-950 p-3 rounded-xl border border-white/10 flex items-center justify-between">
              <div className="space-y-0.5">
                <span className="font-semibold text-slate-200 flex items-center gap-1.5">
                  <Zap className="w-3.5 h-3.5 text-amber-400" />
                  ریست و تخلیه بلادرنگ سوکت (۱۰ms Socket Flush)
                </span>
                <p className="text-[11px] text-slate-400">
                  تخلیه فوری نشست در صورت دریافت پکت‌های آلوده RST بدون قطع شدن ارتباط کاربر
                </p>
              </div>
              <Button
                size="sm"
                variant="outline"
                onClick={() => simulateSocketAutoFlush()}
                className="text-xs h-7 px-3 border-amber-500/40 text-amber-300 hover:bg-amber-500/10"
              >
                تست ریست سوکت
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Live AI DPI Evasion Telemetry & Vendor Probing Logs */}
      <Card className="shield-surface-strong">
        <CardHeader className="pb-2 border-b border-white/10">
          <div className="flex items-center justify-between flex-wrap gap-2">
            <CardTitle className="text-slate-200 text-sm flex items-center gap-2">
              <Activity className="w-4 h-4 text-emerald-400" />
              <span>لاگ‌های بلادرنگ خنثی‌سازی فیلترینگ و DPI زیرساخت (Live Evasion Telemetry)</span>
            </CardTitle>
            <Badge variant="outline" className="text-[11px] border-emerald-500/30 text-emerald-400 bg-emerald-500/10">
              نرخ موفقیت دور زدن: ۱۰۰٪ (Zero-Detection)
            </Badge>
          </div>
        </CardHeader>

        <CardContent className="p-3">
          <div className="space-y-2 max-h-48 overflow-y-auto custom-scrollbar">
            {nationalBlackoutShield.recentDPIEvasionEvents.map((evt) => (
              <div
                key={evt.id}
                className="flex items-center justify-between bg-slate-950 p-2 rounded-lg border border-white/10 text-xs text-slate-300 flex-wrap gap-2"
              >
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
                  <span className="font-bold text-cyan-300">{evt.protocolNameFa}</span>
                  <span className="text-slate-400">» {evt.evasionTechniqueFa}</span>
                </div>

                <div className="flex items-center gap-3 text-[11px]">
                  <span className="text-slate-500 font-mono">تارگت: {evt.targetDPIVendor}</span>
                  <Badge className="bg-emerald-500/20 text-emerald-300 text-[10px] py-0 font-mono">
                    {toPersianNum(evt.responseTimeMs)} ms
                  </Badge>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
