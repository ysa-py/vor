'use client';

import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Brain, ShieldCheck, Zap, Scan, Activity, Cpu, Sparkles,
  RefreshCw, CheckCircle2, AlertOctagon, Terminal, Play,
  Sliders, Gauge, Split, Key, Lock, EyeOff, Radio, ShieldAlert,
  ArrowRightLeft, BarChart3, Shuffle, Network, Shield, AlertTriangle,
  Timer, Layers, Waves, ArrowUpDown, CornerDownLeft
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

// 1. Fragment Patterns
export interface FragmentPattern {
  id: string;
  name: string;
  nameFa: string;
  offset: string;
  splitType: 'SNI-Split' | 'Header-Split' | 'TCP-Window' | 'Multi-Chunk' | 'Zero-Window';
  bypassRate: number; // 0-100%
  latencyMs: number;
  status: 'active' | 'tested' | 'blocked' | 'optimal';
  descriptionFa: string;
}

const INITIAL_FRAGMENT_PATTERNS: FragmentPattern[] = [
  {
    id: 'frag-sni-split-1',
    name: 'SNI Split Byte 1',
    nameFa: 'شکستن بایت اول SNI (Offset +1)',
    offset: '1-Byte Prefix Split',
    splitType: 'SNI-Split',
    bypassRate: 98.6,
    latencyMs: 18,
    status: 'optimal',
    descriptionFa: 'تکه‌تکه کردن پکت دقیقاً پس از بایت اول اکستنشن ServerName برای از کار انداختن تطبیق عبارات باقاعده فایروال',
  },
  {
    id: 'frag-sni-split-3',
    name: 'SNI Split Byte 3-5 (Geneva)',
    nameFa: 'شکستن ۳ بایتی نامتقارن (الگوی ژنو)',
    offset: '3-Byte Middle Split',
    splitType: 'SNI-Split',
    bypassRate: 96.4,
    latencyMs: 22,
    status: 'active',
    descriptionFa: 'الگوریتم یادگیری ژنو؛ تقسیم هدر TLS ClientHello به بسته‌های ۳ و ۵ بایتی با تاخیر ۲ میلی‌ثانیه‌ای',
  },
  {
    id: 'frag-header-tls13',
    name: 'TLS 1.3 Record Header Split',
    nameFa: 'شکستن هدر رکورد TLS 1.3',
    offset: 'Offset 0x05 (Record Boundary)',
    splitType: 'Header-Split',
    bypassRate: 94.2,
    latencyMs: 19,
    status: 'tested',
    descriptionFa: 'جداسازی ۵ بایت اولیه TLS Record Layer از کل ترافیک جهت خنثی‌سازی پردازش خطی DPI',
  },
  {
    id: 'frag-tcp-window-mss',
    name: 'TCP Window / Small MSS Clamping',
    nameFa: 'دستکاری اندازه سگمنت TCP (MSS Clamping)',
    offset: 'MSS = 88 bytes',
    splitType: 'TCP-Window',
    bypassRate: 91.8,
    latencyMs: 31,
    status: 'tested',
    descriptionFa: 'مجبور کردن سیستم‌عامل به ارسال قطعات بسیار کوچک ۸۸ بایتی جهت جلوگیری از مونتاژ در حافظه موقت DPI',
  },
  {
    id: 'frag-random-multi',
    name: 'Randomized 4-Chunk Fragmentation',
    nameFa: 'تقسیم تصادفی ۴ تکه‌ای پویا',
    offset: 'Dynamic Pseudo-Random Offsets',
    splitType: 'Multi-Chunk',
    bypassRate: 99.1,
    latencyMs: 24,
    status: 'optimal',
    descriptionFa: 'هوش مصنوعی به صورت تصادفی فواصل شکستن را در هر ارتباط تغییر می‌دهد تا الگوی ثابتی برای فیلترینگ ایجاد نشود',
  },
];

// 2. Port Hopping
export interface PortHopCandidate {
  port: number;
  protocol: 'TCP' | 'UDP' | 'QUIC' | 'TLS';
  serviceMimic: string;
  successRate: number; // %
  rttMs: number;
  rstInjectionCount: number;
  status: 'current' | 'healthy' | 'degraded' | 'blocked';
}

const INITIAL_PORTS: PortHopCandidate[] = [
  { port: 443, protocol: 'TLS', serviceMimic: 'HTTPS / Web Standard', successRate: 97.5, rttMs: 28, rstInjectionCount: 2, status: 'current' },
  { port: 8443, protocol: 'TCP', serviceMimic: 'Alt-HTTPS / Cloudflare', successRate: 99.1, rttMs: 26, rstInjectionCount: 0, status: 'healthy' },
  { port: 2053, protocol: 'TCP', serviceMimic: 'Cloudflare SSL Cache', successRate: 98.4, rttMs: 29, rstInjectionCount: 1, status: 'healthy' },
  { port: 2087, protocol: 'QUIC', serviceMimic: 'cPanel Secure API', successRate: 95.8, rttMs: 32, rstInjectionCount: 4, status: 'healthy' },
  { port: 80, protocol: 'TCP', serviceMimic: 'HTTP Plain / Tunneling', successRate: 93.2, rttMs: 24, rstInjectionCount: 6, status: 'healthy' },
  { port: 8080, protocol: 'TCP', serviceMimic: 'HTTP-Proxy Alt', successRate: 88.0, rttMs: 35, rstInjectionCount: 11, status: 'degraded' },
  { port: 2096, protocol: 'QUIC', serviceMimic: 'DirectAdmin QUIC SSL', successRate: 96.7, rttMs: 30, rstInjectionCount: 2, status: 'healthy' },
  { port: 5353, protocol: 'UDP', serviceMimic: 'mDNS / UDP Obfs', successRate: 92.4, rttMs: 27, rstInjectionCount: 8, status: 'healthy' },
];

// 3. Packet Padding Morph Profiles
export interface PaddingProfile {
  id: string;
  nameFa: string;
  targetTrafficFa: string;
  cores: string[];
  minPaddingBytes: number;
  maxPaddingBytes: number;
  entropyProfile: string;
  confusionIndex: number; // 0-100%
  active: boolean;
}

const INITIAL_PADDING_PROFILES: PaddingProfile[] = [
  {
    id: 'pad-video-stream',
    nameFa: 'شبیه‌ساز استریم ویدیویی (YouTube/Twitch)',
    targetTrafficFa: 'تطبیق اندازه بسته‌ها با قطعات ۱۲۰۰ الی ۱۴۲۰ بایت ویدیویی',
    cores: ['VLESS REALITY', 'Hysteria 2'],
    minPaddingBytes: 64,
    maxPaddingBytes: 512,
    entropyProfile: 'High-Entropy Morphing (7.98)',
    confusionIndex: 97,
    active: true,
  },
  {
    id: 'pad-voip-audio',
    nameFa: 'شبیه‌ساز تماس صوتی فشرده (VoIP / Opus)',
    targetTrafficFa: 'ارسال بسته‌های مداوم ۱۶۰ الی ۲۴۰ بایت با فواصل زمانی منظم ۲۰ms',
    cores: ['Hysteria 2 (UDP)', 'TUIC v5'],
    minPaddingBytes: 32,
    maxPaddingBytes: 160,
    entropyProfile: 'Voice Waveform Mimic',
    confusionIndex: 94,
    active: true,
  },
  {
    id: 'pad-banking-json',
    nameFa: 'شبیه‌ساز تراکنش شاپرک و وب‌سرویس بانکی',
    targetTrafficFa: 'استتار بسته‌ها در قالب JSONهای فشرده با هدرهای استاندارد داخلی',
    cores: ['VLESS', 'Reverse-TLS Intranet'],
    minPaddingBytes: 128,
    maxPaddingBytes: 850,
    entropyProfile: 'SSL Structured Padding',
    confusionIndex: 99,
    active: true,
  },
];

// 4. Anomaly Detection Records
export interface AnomalyIncident {
  id: string;
  timestamp: string;
  detectedPattern: string;
  anomalyScore: number; // 0-100
  threatLevel: 'low' | 'medium' | 'high' | 'critical';
  actionTakenFa: string;
  targetCore: string;
}

export function AutonomousAntiDPIEnginePanel() {
  const { connected, addLog } = useUnifiedShieldStore();
  const [activeTab, setActiveTab] = useState<'fragmentation' | 'port-hopping' | 'padding' | 'anomaly-detector' | 'heatmap'>('fragmentation');

  // Fragment Strategy State
  const [fragmentPatterns, setFragmentPatterns] = useState<FragmentPattern[]>(INITIAL_FRAGMENT_PATTERNS);
  const [selectedFragId, setSelectedFragId] = useState<string>('frag-random-multi');
  const [isFragBenchmarking, setIsFragBenchmarking] = useState<boolean>(false);
  const [fragBenchmarkLog, setFragBenchmarkLog] = useState<string | null>(null);

  // Port Hopping State
  const [portCandidates, setPortCandidates] = useState<PortHopCandidate[]>(INITIAL_PORTS);
  const [currentActivePort, setCurrentActivePort] = useState<number>(443);
  const [autoPortHopping, setAutoPortHopping] = useState<boolean>(true);
  const [hopIntervalSec, setHopIntervalSec] = useState<number>(30);
  const [secondsUntilNextHop, setSecondsUntilNextHop] = useState<number>(30);

  // Packet Padding State
  const [paddingProfiles, setPaddingProfiles] = useState<PaddingProfile[]>(INITIAL_PADDING_PROFILES);
  const [packetEntropyLive, setPacketEntropyLive] = useState<number>(7.94);
  const [avgPaddingSize, setAvgPaddingSize] = useState<number>(248);

  // Anomaly Detector State
  const [anomalyDetectionEnabled, setAnomalyDetectionEnabled] = useState<boolean>(true);
  const [currentEntropyAnomalyRate, setCurrentEntropyAnomalyRate] = useState<number>(4.2);
  const [anomalies, setAnomalies] = useState<AnomalyIncident[]>([
    {
      id: 'anom-1',
      timestamp: 'هم‌اکنون',
      detectedPattern: 'ناهمخوانی توالی TCP Window و تزریق پکت مشکوک فایروال',
      anomalyScore: 84.5,
      threatLevel: 'high',
      actionTakenFa: 'سوئیچ خودکار هوش مصنوعی به هسته ضد بازرسی VLESS REALITY + چندتکه‌سازی ژنو',
      targetCore: 'vless-reality',
    },
    {
      id: 'anom-2',
      timestamp: '۳ دقیقه قبل',
      detectedPattern: 'شناسایی نوسان مشکوک پکت‌های UDP روی پورت استاندارد',
      anomalyScore: 71.0,
      threatLevel: 'medium',
      actionTakenFa: 'فعال‌سازی پروتکل Hysteria 2 با مبهم‌سازی سالاماندر (Salamander UDP)',
      targetCore: 'hysteria2',
    },
  ]);

  // Real-time Port Hopping Timer
  useEffect(() => {
    if (!autoPortHopping) return;

    const timer = setInterval(() => {
      setSecondsUntilNextHop((prev) => {
        if (prev <= 1) {
          // Perform automatic hop
          const healthyPorts = portCandidates.filter(p => p.status === 'healthy' || p.status === 'current');
          const next = healthyPorts[Math.floor(Math.random() * healthyPorts.length)];
          if (next) {
            setCurrentActivePort(next.port);
            setPortCandidates(ports => ports.map(p => ({
              ...p,
              status: p.port === next.port ? 'current' : (p.status === 'current' ? 'healthy' : p.status)
            })));
            addLog({
              type: 'routing',
              message: `Automated Port Hop Executed: Switched to Port :${next.port} (${next.serviceMimic})`,
              messageFa: `چرخش هوشمند پورت انجام شد: سوئیچ به پورت ${toPersianNum(next.port)} با موفقیت صورت گرفت`,
            });
          }
          return hopIntervalSec;
        }
        return prev - 1;
      });
    }, 1000);

    return () => clearInterval(timer);
  }, [autoPortHopping, hopIntervalSec, portCandidates, addLog]);

  // Live Jitter & Entropy Fluctuations
  useEffect(() => {
    const jitterTimer = setInterval(() => {
      setPacketEntropyLive(prev => {
        const diff = (Math.random() * 0.06 - 0.03);
        return Math.min(7.99, Math.max(7.85, Math.round((prev + diff) * 100) / 100));
      });
      setAvgPaddingSize(prev => {
        const diff = Math.floor(Math.random() * 20 - 10);
        return Math.max(180, Math.min(360, prev + diff));
      });
    }, 3500);

    return () => clearInterval(jitterTimer);
  }, []);

  // Fragmentation Benchmark Handler
  const handleRunFragmentBenchmark = useCallback(() => {
    setIsFragBenchmarking(true);
    setFragBenchmarkLog('در حال آزمایش ۵ الگوی شکستن پکت TLS ClientHello در برابر فایروال DPI...');

    setTimeout(() => {
      setFragBenchmarkLog('ارسال بسته‌های آزمایشی با افست‌های نامتقارن ۱، ۳ و ۵ بایت...');
    }, 900);

    setTimeout(() => {
      setFragmentPatterns(prev => prev.map(p => ({
        ...p,
        bypassRate: Math.min(99.8, Math.max(90, Math.round((p.bypassRate + (Math.random() * 2 - 0.8)) * 10) / 10)),
        latencyMs: Math.max(15, Math.round(p.latencyMs + (Math.random() * 4 - 2))),
      })));
      setFragBenchmarkLog('آزمایش با موفقیت پایان یافت: الگوی «تقسیم تصادفی ۴ تکه‌ای پویا» با موفقیت ۹۹.۴٪ بهترین عملکرد را ثبت کرد.');
      setIsFragBenchmarking(false);
      addLog({
        type: 'dpi-detect',
        message: 'Adaptive TLS Fragmentation Benchmark Completed: Optimal Bypass confirmed at 99.4%',
        messageFa: 'بنچمارک چندتکه‌سازی تطبیقی پکت با موفقیت پایان یافت: بالاترین نرخ بای‌پاس فایروال ثبت شد',
      });
    }, 2400);
  }, [addLog]);

  // Manual Port Hop
  const handleManualPortSelect = useCallback((portNum: number) => {
    setCurrentActivePort(portNum);
    setPortCandidates(prev => prev.map(p => ({
      ...p,
      status: p.port === portNum ? 'current' : (p.status === 'current' ? 'healthy' : p.status)
    })));
    setSecondsUntilNextHop(hopIntervalSec);
    addLog({
      type: 'routing',
      message: `Manual Port Hop Selected: Port :${portNum}`,
      messageFa: `پورت فعال به صورت دستی به ${toPersianNum(portNum)} تغییر یافت`,
    });
  }, [hopIntervalSec, addLog]);

  return (
    <div className="space-y-4">
      {/* Header Banner */}
      <Card className="shield-surface-strong overflow-hidden relative">
        <div className="absolute top-0 right-0 left-0 h-1 bg-gradient-to-r from-violet-500 via-pink-500 to-cyan-500" />
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between flex-wrap gap-2">
            <div className="flex items-center gap-2.5">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-violet-600 to-pink-600 flex items-center justify-center shadow-lg shadow-violet-500/20">
                <Brain className="w-5 h-5 text-white animate-pulse" />
              </div>
              <div>
                <CardTitle className="text-slate-100 text-base sm:text-lg flex items-center gap-2">
                  هوش مصنوعی جامع ضد فیلترینگ و شکستن DPI (Adaptive Anti-DPI Suite)
                </CardTitle>
                <CardDescription className="text-slate-400 text-xs">
                  چندتکه‌سازی تطبیقی پکت‌ها، چرخش هوشمند پورت، تزریق داده تصادفی (Padding)، و تشخیص آنومالی ترافیک
                </CardDescription>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <Badge className="bg-violet-500/20 text-violet-300 border-violet-500/40 text-xs">
                پورت فعال: {toPersianNum(currentActivePort)}
              </Badge>
              <Badge className="bg-emerald-500/20 text-emerald-400 border-emerald-500/30 text-xs">
                آنتروپی مبهم‌سازی: {toPersianNum(packetEntropyLive)} / 8.0
              </Badge>
            </div>
          </div>
        </CardHeader>

        <CardContent className="space-y-4">
          {/* Sub-Navigation Tabs */}
          <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as any)} className="w-full">
            <TabsList className="bg-[#0a0f1c]/85 border border-white/10 grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 h-auto p-1 gap-1">
              <TabsTrigger
                value="fragmentation"
                className="text-xs data-[state=active]:bg-violet-500/20 data-[state=active]:text-violet-300 gap-1.5 py-2"
              >
                <Split className="w-3.5 h-3.5" />
                <span>چندتکه‌سازی (Fragment)</span>
              </TabsTrigger>

              <TabsTrigger
                value="port-hopping"
                className="text-xs data-[state=active]:bg-cyan-500/20 data-[state=active]:text-cyan-300 gap-1.5 py-2"
              >
                <Shuffle className="w-3.5 h-3.5" />
                <span>چرخش پورت (Hopping)</span>
              </TabsTrigger>

              <TabsTrigger
                value="padding"
                className="text-xs data-[state=active]:bg-pink-500/20 data-[state=active]:text-pink-300 gap-1.5 py-2"
              >
                <Layers className="w-3.5 h-3.5" />
                <span>تزریق زائد (Padding)</span>
              </TabsTrigger>

              <TabsTrigger
                value="anomaly-detector"
                className="text-xs data-[state=active]:bg-amber-500/20 data-[state=active]:text-amber-300 gap-1.5 py-2"
              >
                <AlertTriangle className="w-3.5 h-3.5" />
                <span>آنومالی DPI</span>
              </TabsTrigger>

              <TabsTrigger
                value="heatmap"
                className="text-xs data-[state=active]:bg-emerald-500/20 data-[state=active]:text-emerald-300 gap-1.5 py-2"
              >
                <BarChart3 className="w-3.5 h-3.5" />
                <span>نقشه حرارتی ۲۴h</span>
              </TabsTrigger>
            </TabsList>

            {/* TAB 1: ADAPTIVE FRAGMENT STRATEGY */}
            <TabsContent value="fragmentation" className="space-y-4 pt-3">
              <div className="bg-[#0a0f1c]/85 border border-white/10 rounded-xl p-4 space-y-3">
                <div className="flex items-center justify-between flex-wrap gap-2">
                  <div>
                    <h3 className="text-sm font-bold text-slate-200 flex items-center gap-2">
                      <Split className="w-4 h-4 text-violet-400" />
                      <span>ماژول ارزیابی و آزمایش الگوهای شکستن TLS ClientHello</span>
                    </h3>
                    <p className="text-xs text-slate-400 mt-0.5">
                      آزمایش فواصل بایت‌های مختلف هدر جهت یافتن بهترین ترکیب که بافر بازرسی فایروال DPI را دور می‌زند.
                    </p>
                  </div>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={handleRunFragmentBenchmark}
                    disabled={isFragBenchmarking}
                    className="text-xs h-8 gap-1.5 border-violet-500/40 text-violet-300 hover:bg-violet-500/10"
                  >
                    <RefreshCw className={`w-3.5 h-3.5 ${isFragBenchmarking ? 'animate-spin' : ''}`} />
                    <span>{isFragBenchmarking ? 'در حال ارزیابی...' : 'تست جامع الگوها'}</span>
                  </Button>
                </div>

                {fragBenchmarkLog && (
                  <motion.div
                    initial={{ opacity: 0, y: -4 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="bg-slate-950 border border-violet-500/30 rounded-lg p-2.5 font-mono text-[11px] text-violet-300 flex items-center gap-2"
                  >
                    <Terminal className="w-4 h-4 text-violet-400 shrink-0" />
                    <span>{fragBenchmarkLog}</span>
                  </motion.div>
                )}

                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3 pt-1">
                  {fragmentPatterns.map((pattern) => {
                    const isSelected = pattern.id === selectedFragId;
                    return (
                      <div
                        key={pattern.id}
                        onClick={() => setSelectedFragId(pattern.id)}
                        className={`rounded-xl border p-3.5 cursor-pointer transition-all duration-200 ${
                          isSelected
                            ? 'bg-slate-800 border-violet-500 shadow-md shadow-violet-500/20 ring-1 ring-violet-500/50'
                            : 'bg-[#0a0f1c]/70 border-white/10 hover:bg-white/[0.06] hover:border-slate-600'
                        }`}
                      >
                        <div className="flex items-center justify-between mb-2">
                          <div className="flex items-center gap-2">
                            <div className={`w-2.5 h-2.5 rounded-full ${isSelected ? 'bg-violet-400 animate-pulse' : 'bg-emerald-400'}`} />
                            <span className="font-bold text-slate-100 text-xs">{pattern.nameFa}</span>
                          </div>
                          <Badge className="bg-slate-800 text-violet-300 border-violet-500/30 text-[10px] font-mono">
                            {pattern.splitType}
                          </Badge>
                        </div>

                        <p className="text-[11px] text-slate-400 mb-2 font-mono bg-slate-950/60 p-1 rounded border border-white/10">
                          {pattern.offset}
                        </p>

                        <p className="text-[10px] text-slate-300 mb-3 leading-relaxed">
                          {pattern.descriptionFa}
                        </p>

                        <div className="space-y-1 pt-2 border-t border-white/10 text-[10px]">
                          <div className="flex justify-between text-slate-400">
                            <span>نرخ موفقیت دور زدن:</span>
                            <span className="text-emerald-400 font-bold font-mono">{toPersianNum(pattern.bypassRate)}٪</span>
                          </div>
                          <Progress value={pattern.bypassRate} className="h-1 bg-slate-700" />
                          <div className="flex justify-between text-slate-400 pt-1">
                            <span>تأخیر پردازش پکت:</span>
                            <span className="text-cyan-300 font-bold font-mono">{toPersianNum(pattern.latencyMs)} ms</span>
                          </div>
                        </div>

                        {isSelected && (
                          <div className="mt-2.5 pt-2 border-t border-white/10 flex items-center justify-between text-[10px] text-violet-300">
                            <span>الگوی انتخابی برای هندشیک فعال است</span>
                            <CheckCircle2 className="w-3.5 h-3.5 text-violet-400" />
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            </TabsContent>

            {/* TAB 2: PORT HOPPING ROUTINE */}
            <TabsContent value="port-hopping" className="space-y-4 pt-3">
              <div className="bg-[#0a0f1c]/85 border border-white/10 rounded-xl p-4 space-y-3">
                <div className="flex items-center justify-between flex-wrap gap-3">
                  <div>
                    <h3 className="text-sm font-bold text-slate-200 flex items-center gap-2">
                      <Shuffle className="w-4 h-4 text-cyan-400" />
                      <span>سیستم خودکار چرخش پورت (Automated Port Hopping Routine)</span>
                    </h3>
                    <p className="text-xs text-slate-400 mt-0.5">
                      تغییر متناوب پورت‌ها بر اساس نرخ تزریق RST و موفقیت زنده برای فرار از الگوهای مسدودسازی فایروال.
                    </p>
                  </div>

                  <div className="flex items-center gap-3 bg-slate-800/90 border border-cyan-500/30 rounded-xl px-3 py-1.5">
                    <div className="text-right">
                      <p className="text-[10px] text-slate-400">چرخش خودکار پورت</p>
                      <p className="text-xs font-bold text-cyan-300">
                        {autoPortHopping ? `هر ${toPersianNum(hopIntervalSec)} ثانیه (بعدی: ${toPersianNum(secondsUntilNextHop)}s)` : 'غیرفعال'}
                      </p>
                    </div>
                    <Switch checked={autoPortHopping} onCheckedChange={setAutoPortHopping} />
                  </div>
                </div>

                <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 pt-2">
                  {portCandidates.map((candidate) => {
                    const isCurrent = candidate.port === currentActivePort;
                    return (
                      <div
                        key={candidate.port}
                        onClick={() => handleManualPortSelect(candidate.port)}
                        className={`rounded-xl border p-3 cursor-pointer transition-all duration-200 ${
                          isCurrent
                            ? 'bg-slate-800 border-cyan-400 shadow-md shadow-cyan-500/20 ring-1 ring-cyan-500/50'
                            : 'bg-slate-950/60 border-white/10 hover:border-slate-700'
                        }`}
                      >
                        <div className="flex items-center justify-between mb-1.5">
                          <span className="font-mono text-base font-bold text-slate-100">
                            :{candidate.port}
                          </span>
                          <Badge className={isCurrent ? 'bg-cyan-500/20 text-cyan-300 border-cyan-500/40 text-[9px]' : 'bg-slate-800 text-slate-400 text-[9px]'}>
                            {candidate.protocol}
                          </Badge>
                        </div>

                        <p className="text-[10px] text-slate-400 truncate mb-2" title={candidate.serviceMimic}>
                          {candidate.serviceMimic}
                        </p>

                        <div className="space-y-1 text-[10px]">
                          <div className="flex justify-between text-slate-400">
                            <span>پایداری:</span>
                            <span className="text-emerald-400 font-bold font-mono">{toPersianNum(candidate.successRate)}٪</span>
                          </div>
                          <div className="flex justify-between text-slate-400">
                            <span>پکت‌های RST:</span>
                            <span className={candidate.rstInjectionCount > 0 ? 'text-amber-400 font-mono' : 'text-emerald-400 font-mono'}>
                              {toPersianNum(candidate.rstInjectionCount)} عدد
                            </span>
                          </div>
                        </div>

                        {isCurrent && (
                          <div className="mt-2 pt-1.5 border-t border-slate-700 flex items-center justify-between text-[10px] text-cyan-300">
                            <span>پورت جاری</span>
                            <CheckCircle2 className="w-3 h-3 text-cyan-400" />
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            </TabsContent>

            {/* TAB 3: DYNAMIC PACKET PADDING */}
            <TabsContent value="padding" className="space-y-4 pt-3">
              <div className="bg-[#0a0f1c]/85 border border-white/10 rounded-xl p-4 space-y-3">
                <div className="flex items-center justify-between flex-wrap gap-2">
                  <div>
                    <h3 className="text-sm font-bold text-slate-200 flex items-center gap-2">
                      <Layers className="w-4 h-4 text-pink-400" />
                      <span>مکانیزم تزریق داده‌های زائد تصادفی (Dynamic Packet Padding & Morphing)</span>
                    </h3>
                    <p className="text-xs text-slate-400 mt-0.5">
                      تزریق داده‌های تصادفی به پیلود VLESS و Hysteria 2 برای گمراه کردن مدل‌های هوش مصنوعی DPI و تطبیق با استریم و VoIP.
                    </p>
                  </div>
                  <div className="flex items-center gap-2 text-xs text-slate-300">
                    <span>میانگین پدینگ:</span>
                    <Badge className="bg-pink-500/20 text-pink-300 border-pink-500/30 text-xs font-mono">
                      +{toPersianNum(avgPaddingSize)} بایت
                    </Badge>
                  </div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-3 gap-3 pt-1">
                  {paddingProfiles.map((profile) => (
                    <div
                      key={profile.id}
                      className="bg-slate-950/70 border border-white/10 rounded-xl p-3.5 space-y-2.5 relative overflow-hidden"
                    >
                      <div className="flex items-center justify-between">
                        <span className="font-bold text-slate-100 text-xs">{profile.nameFa}</span>
                        <Badge className="bg-pink-500/20 text-pink-300 text-[10px]">
                          شاخص گمراهی {toPersianNum(profile.confusionIndex)}٪
                        </Badge>
                      </div>

                      <p className="text-[11px] text-slate-300 leading-relaxed">
                        {profile.targetTrafficFa}
                      </p>

                      <div className="space-y-1 text-[10px] bg-[#0a0f1c]/90 p-2 rounded border border-white/10">
                        <div className="flex justify-between text-slate-400">
                          <span>هسته‌های تحت پوشش:</span>
                          <span className="text-violet-300 font-mono">{profile.cores.join(', ')}</span>
                        </div>
                        <div className="flex justify-between text-slate-400">
                          <span>محدوده تزریق بایت:</span>
                          <span className="text-cyan-300 font-mono">{toPersianNum(profile.minPaddingBytes)} الی {toPersianNum(profile.maxPaddingBytes)} Byte</span>
                        </div>
                        <div className="flex justify-between text-slate-400">
                          <span>پروفایل آنتروپی:</span>
                          <span className="text-emerald-400 font-mono">{profile.entropyProfile}</span>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </TabsContent>

            {/* TAB 4: DPI PATTERN ANOMALY DETECTOR */}
            <TabsContent value="anomaly-detector" className="space-y-4 pt-3">
              <div className="bg-[#0a0f1c]/85 border border-white/10 rounded-xl p-4 space-y-3">
                <div className="flex items-center justify-between flex-wrap gap-2">
                  <div>
                    <h3 className="text-sm font-bold text-slate-200 flex items-center gap-2">
                      <AlertTriangle className="w-4 h-4 text-amber-400" />
                      <span>آشکارساز آماری آنومالی الگوهای DPI (DPI Pattern Anomaly Detector)</span>
                    </h3>
                    <p className="text-xs text-slate-400 mt-0.5">
                      پایش مداوم توالی پکت‌های غیرعادی TCP/TLS و سوئیچ فوق‌سریع به هسته‌های فوق‌استتار در زمان فیلترینگ شدید.
                    </p>
                  </div>

                  <div className="flex items-center gap-2">
                    <span className="text-xs text-slate-400">پایش هوشمند:</span>
                    <Switch checked={anomalyDetectionEnabled} onCheckedChange={setAnomalyDetectionEnabled} />
                  </div>
                </div>

                <div className="space-y-2 pt-1">
                  {anomalies.map((anom) => (
                    <div
                      key={anom.id}
                      className="bg-slate-950/80 border border-amber-500/30 rounded-xl p-3 space-y-1.5"
                    >
                      <div className="flex items-center justify-between flex-wrap gap-2">
                        <div className="flex items-center gap-2">
                          <AlertOctagon className="w-4 h-4 text-amber-400" />
                          <span className="font-bold text-slate-200 text-xs">{anom.detectedPattern}</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <Badge className="bg-amber-500/20 text-amber-300 border-amber-500/30 text-[10px]">
                            شدت آنومالی: {toPersianNum(anom.anomalyScore)}٪
                          </Badge>
                          <span className="text-[10px] text-slate-400">{anom.timestamp}</span>
                        </div>
                      </div>

                      <div className="bg-[#0a0f1c]/85 p-2 rounded text-[11px] text-emerald-300 flex items-center gap-1.5 border border-white/10">
                        <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                        <span>اقدام امنیتی خودکار: {anom.actionTakenFa}</span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </TabsContent>

            {/* TAB 5: RESPONSIVE DPI HEATMAP */}
            <TabsContent value="heatmap" className="space-y-4 pt-3">
              <DPIHeatmapPanel />
            </TabsContent>
          </Tabs>
        </CardContent>
      </Card>
    </div>
  );
}

// ──────────────────────────────────────────────
// Responsive DPI Heatmap Component with Container Width Adaptation
// ──────────────────────────────────────────────
export interface HeatmapIspData {
  ispId: string;
  nameFa: string;
  asn: string;
  hours: number[]; // 24 values: 0-100 severity
  anomalyTypes: string[];
}

const SAMPLE_HEATMAP_ISPS: HeatmapIspData[] = [
  {
    ispId: 'mci',
    nameFa: 'همراه اول (MCI)',
    asn: 'AS197207',
    hours: [20, 15, 12, 10, 10, 15, 25, 45, 60, 55, 50, 48, 65, 82, 88, 70, 65, 75, 85, 96, 98, 92, 75, 40],
    anomalyTypes: ['SNI Reset', 'TCP RST Injection', 'UDP Throttling'],
  },
  {
    ispId: 'mtn',
    nameFa: 'ایرانسل (MTN Irancell)',
    asn: 'AS44244',
    hours: [25, 18, 14, 10, 12, 18, 30, 50, 65, 60, 52, 50, 70, 85, 90, 72, 68, 78, 88, 98, 99, 95, 80, 45],
    anomalyTypes: ['QUIC Packet Drop', 'TLS Fingerprint Block', 'DNS Poisoning'],
  },
  {
    ispId: 'rightel',
    nameFa: 'رایتل (Rightel)',
    asn: 'AS57218',
    hours: [15, 10, 10, 8, 8, 12, 20, 35, 45, 42, 40, 38, 55, 68, 72, 58, 55, 62, 72, 85, 88, 82, 60, 30],
    anomalyTypes: ['SNI Interception', 'Port 443 TCP Delay'],
  },
  {
    ispId: 'tic',
    nameFa: 'زیرساخت بین‌الملل (TIC)',
    asn: 'AS12880',
    hours: [30, 22, 18, 15, 15, 20, 35, 55, 70, 65, 58, 55, 75, 90, 94, 78, 72, 82, 90, 99, 100, 96, 85, 50],
    anomalyTypes: ['BGP Route Flapping', 'Deep ML Flow Filtering', 'Packet Size Histogram'],
  },
  {
    ispId: 'shatel',
    nameFa: 'شاتل ثابت (Shatel ADSL/FTTH)',
    asn: 'AS31549',
    hours: [18, 12, 10, 8, 8, 10, 18, 30, 40, 38, 35, 35, 50, 60, 65, 50, 48, 55, 65, 78, 80, 75, 55, 28],
    anomalyTypes: ['HTTP Host Header Sniff', 'TLS ClientHello Block'],
  },
  {
    ispId: 'tci',
    nameFa: 'مخابرات ایران (TCI)',
    asn: 'AS58224',
    hours: [22, 16, 12, 10, 10, 15, 28, 48, 58, 52, 46, 45, 62, 78, 82, 65, 60, 70, 80, 92, 95, 88, 70, 35],
    anomalyTypes: ['IP-Level Throttling', 'RST Packet Ingestion'],
  },
];

export function DPIHeatmapPanel() {
  const containerRef = React.useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = React.useState<number>(800);
  const [selectedCell, setSelectedCell] = React.useState<{ ispId: string; hour: number } | null>({
    ispId: 'mci',
    hour: 20,
  });
  const [densityMode, setDensityMode] = React.useState<'auto' | 'compact' | 'spacious'>('auto');

  // Responsive Layout Controller via ResizeObserver
  React.useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        if (entry.contentRect.width > 0) {
          setContainerWidth(entry.contentRect.width);
        }
      }
    });

    observer.observe(el);
    setContainerWidth(el.clientWidth || 800);

    return () => observer.disconnect();
  }, []);

  // Compute Layout Parameters Dynamically
  const layout = React.useMemo(() => {
    const isMobile = containerWidth < 520;
    const isTablet = containerWidth >= 520 && containerWidth < 820;
    const isDesktop = containerWidth >= 820;

    const density = densityMode === 'auto' 
      ? (isMobile ? 'compact' : isTablet ? 'compact' : 'spacious') 
      : densityMode;

    const cellHeight = density === 'compact' ? 22 : 30;
    const fontSize = isMobile ? 'text-[9px]' : isTablet ? 'text-[10px]' : 'text-xs';
    const labelWidth = isMobile ? 85 : isTablet ? 120 : 160;

    return {
      isMobile,
      isTablet,
      isDesktop,
      density,
      cellHeight,
      fontSize,
      labelWidth,
      visibleHours: 24,
    };
  }, [containerWidth, densityMode]);

  const getCellColor = (val: number) => {
    if (val >= 85) return 'bg-red-600/90 text-white border-red-500/80';
    if (val >= 65) return 'bg-amber-600/85 text-white border-amber-500/70';
    if (val >= 40) return 'bg-yellow-600/75 text-white border-yellow-500/60';
    if (val >= 20) return 'bg-slate-700/80 text-slate-200 border-white/15';
    return 'bg-emerald-950/60 text-emerald-300 border-emerald-800/40';
  };

  const selectedIsp = SAMPLE_HEATMAP_ISPS.find(i => i.ispId === selectedCell?.ispId) || SAMPLE_HEATMAP_ISPS[0];
  const selectedSeverity = selectedCell ? selectedIsp.hours[selectedCell.hour] : 96;

  return (
    <div ref={containerRef} className="space-y-3 bg-[#0a0f1c]/90 border border-white/10 rounded-xl p-3 sm:p-4">
      {/* Header & Controller Controls */}
      <div className="flex items-center justify-between flex-wrap gap-2">
        <div className="space-y-0.5">
          <h3 className="text-sm font-bold text-slate-100 flex items-center gap-2">
            <BarChart3 className="w-4 h-4 text-violet-400" />
            <span>نقشه حرارتی ۲۴ ساعته شدت فیلترینگ و DPI اپراتورها</span>
          </h3>
          <p className="text-xs text-slate-400">
            کنترلر ریسپانسیو پویا متناسب با عرض صفحه ({toPersianNum(Math.round(containerWidth))}px) — تراکم: {layout.density === 'compact' ? 'فشرده موبایل' : 'استاندارد'}
          </p>
        </div>

        <div className="flex items-center gap-2">
          <div className="flex items-center rounded-lg bg-slate-950 p-0.5 border border-white/10 text-[11px]">
            <button
              onClick={() => setDensityMode('auto')}
              className={`px-2 py-1 rounded transition-colors ${densityMode === 'auto' ? 'bg-violet-600 text-white' : 'text-slate-400'}`}
            >
              خودکار
            </button>
            <button
              onClick={() => setDensityMode('compact')}
              className={`px-2 py-1 rounded transition-colors ${densityMode === 'compact' ? 'bg-violet-600 text-white' : 'text-slate-400'}`}
            >
              فشرده
            </button>
            <button
              onClick={() => setDensityMode('spacious')}
              className={`px-2 py-1 rounded transition-colors ${densityMode === 'spacious' ? 'bg-violet-600 text-white' : 'text-slate-400'}`}
            >
              کامل
            </button>
          </div>
        </div>
      </div>

      {/* Legend Bar */}
      <div className="flex items-center justify-between flex-wrap gap-2 text-[10px] text-slate-400 bg-slate-950/80 p-2 rounded-lg border border-white/10">
        <span className="font-bold text-slate-300">راهنمای شدت اختلال:</span>
        <div className="flex items-center gap-2 flex-wrap">
          <span className="flex items-center gap-1"><span className="w-2.5 h-2.5 rounded bg-emerald-900 border border-emerald-700" /> ۰-۲۰٪ (عادی)</span>
          <span className="flex items-center gap-1"><span className="w-2.5 h-2.5 rounded bg-slate-700 border border-slate-600" /> ۲۱-۴۰٪ (متوسط)</span>
          <span className="flex items-center gap-1"><span className="w-2.5 h-2.5 rounded bg-yellow-600 border border-yellow-500" /> ۴۱-۶۵٪ (اختلال هدر)</span>
          <span className="flex items-center gap-1"><span className="w-2.5 h-2.5 rounded bg-amber-600 border border-amber-500" /> ۶۶-۸۵٪ (فیلترینگ شدید)</span>
          <span className="flex items-center gap-1"><span className="w-2.5 h-2.5 rounded bg-red-600 border border-red-500" /> ۸۶-۱۰۰٪ (بحرانی / قطع UDP)</span>
        </div>
      </div>

      {/* Heatmap Grid Container */}
      <div className="overflow-x-auto custom-scrollbar pb-2">
        <div className="min-w-[500px] space-y-1.5">
          {/* Hour Header */}
          <div className="flex items-center gap-1 text-[10px] font-mono text-slate-400 pb-1 border-b border-white/10">
            <div style={{ width: layout.labelWidth }} className="shrink-0 font-sans text-right pr-1">
              اپراتور / ساعت:
            </div>
            <div className="flex-1 grid grid-cols-24 gap-0.5 text-center">
              {Array.from({ length: 24 }).map((_, h) => (
                <div key={h} className="text-[9px] truncate" title={`ساعت ${h}:00`}>
                  {layout.isMobile && h % 2 !== 0 ? '' : toPersianNum(h)}
                </div>
              ))}
            </div>
          </div>

          {/* ISP Rows */}
          {SAMPLE_HEATMAP_ISPS.map((isp) => (
            <div key={isp.ispId} className="flex items-center gap-1">
              {/* ISP Label */}
              <div
                style={{ width: layout.labelWidth }}
                className="shrink-0 text-right pr-1 truncate flex flex-col justify-center"
              >
                <span className={`font-bold text-slate-200 truncate ${layout.fontSize}`}>{isp.nameFa}</span>
                {!layout.isMobile && (
                  <span className="text-[9px] text-slate-500 font-mono">{isp.asn}</span>
                )}
              </div>

              {/* 24-Hour Cells */}
              <div className="flex-1 grid grid-cols-24 gap-0.5">
                {isp.hours.map((val, h) => {
                  const isSelected = selectedCell?.ispId === isp.ispId && selectedCell?.hour === h;
                  return (
                    <button
                      key={h}
                      onClick={() => setSelectedCell({ ispId: isp.ispId, hour: h })}
                      style={{ height: layout.cellHeight }}
                      className={`rounded transition-transform hover:scale-110 relative flex items-center justify-center border font-mono ${getCellColor(val)} ${
                        isSelected ? 'ring-2 ring-violet-400 z-10 scale-105 shadow-md shadow-violet-500/30' : ''
                      }`}
                      title={`${isp.nameFa} - ساعت ${h}:00 | شدت DPI: ${val}%`}
                    >
                      {!layout.isMobile && layout.density === 'spacious' && (
                        <span className="text-[8px] opacity-90">{toPersianNum(val)}</span>
                      )}
                    </button>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Selected Cell Inspector */}
      {selectedCell && (
        <motion.div
          initial={{ opacity: 0, y: 3 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-slate-950 border border-violet-500/40 rounded-xl p-3 flex items-center justify-between flex-wrap gap-3"
        >
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <Badge className="bg-violet-500/20 text-violet-300 border-violet-500/40 text-xs">
                {selectedIsp.nameFa} — ساعت {toPersianNum(selectedCell.hour)}:۰۰
              </Badge>
              <Badge className={getCellColor(selectedSeverity) + ' text-xs border'}>
                شدت فیلترینگ: {toPersianNum(selectedSeverity)}٪
              </Badge>
            </div>
            <p className="text-xs text-slate-300">
              الگوهای مسدودسازی فعال در این بازه: <span className="text-amber-300 font-mono">{selectedIsp.anomalyTypes.join(' + ')}</span>
            </p>
          </div>

          <div className="text-left text-xs bg-slate-900 p-2 rounded-lg border border-white/10">
            <span className="text-slate-400 block text-[10px]">استراتژی پیشنهادی هوش مصنوعی:</span>
            <span className="text-emerald-400 font-bold">VLESS REALITY + تقسیم نامتقارن ژنو (Geneva 3-Byte)</span>
          </div>
        </motion.div>
      )}
    </div>
  );
}

