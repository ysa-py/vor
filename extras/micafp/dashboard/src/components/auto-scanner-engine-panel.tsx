'use client';

import { useMemo, useState } from 'react';
import {
  LayoutGrid, Signal, Cloud, Zap, Layers, GitBranch, Fingerprint, Wind, CloudCog,
  Ghost, DoorOpen, Waypoints, AtSign, Waves, Terminal, Chrome, ShieldCheck, Radio, Eye, Orbit,
  Brain, Cpu, Download, Copy, RefreshCw, Server, Battery, HeartPulse, Wand2,
  FileText, Check, Sparkles, Activity, ScanSearch, Antenna, Crosshair, Boxes,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';
import {
  IRAN_SCAN_CATEGORIES,
  IRAN_SCAN_MODES,
  IRAN_DISCOVERY_SCALES,
} from '@/lib/auto-scanner-engine';
import type {
  IranScanCategoryId,
  IranScanModeId,
  IranDiscoveryScaleId,
} from '@/lib/auto-scanner-engine';
import { TUNNEL_MODE_LABEL, BLACKOUT_FALLBACK_LABEL } from '@/lib/connectivity-decision';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

const CATEGORY_ICON: Record<IranScanCategoryId, typeof LayoutGrid> = {
  'full-matrix': LayoutGrid,
  'iran-telcos': Signal,
  'white-dns': Cloud,
  'storm-dns': Zap,
  'cotten-dns': Layers,
  'master-dns': GitBranch,
  'vless-reality': Fingerprint,
  'hysteria2-tuic': Wind,
  'cf-clean-warp': CloudCog,
  'shadowtls-mimicry': Ghost,
  'sni-fronting': DoorOpen,
  'ipv4-ipv6-clean': Waypoints,
  'dnstt-edns0': AtSign,
  'vaydns-noizdns': Waves,
  'ssh-wrappers': Terminal,
  'naiveproxy': Chrome,
  'doh-dot': ShieldCheck,
  'iran-intranet-nin': Radio,
  'dpi-rst-inspector': Eye,
  'tor-pluggable': Orbit,
};

const AUTOPILOT_FEATURES: { key: 'pathValidator' | 'autoHealingWatchdog' | 'zeroTouch' | 'blackoutSolver' | 'batterySaver'; labelFa: string; descFa: string; icon: typeof Cpu }[] = [
  { key: 'pathValidator', labelFa: 'اعتبارسنجی بلادرنگ مسیر', descFa: 'پایش مداوم هر ۶ ثانیه روی نود فعال + تزریق RTT به مدل ML', icon: Activity },
  { key: 'autoHealingWatchdog', labelFa: 'پایشگر خودترمیمی پیوسته', descFa: 'پکت‌لاس بالای ۵٪ یا پینگ بالای ۱۲۰ms → جایگزینی خودکار نود', icon: HeartPulse },
  { key: 'zeroTouch', labelFa: 'خلبان خودکار Zero-Touch', descFa: 'افت کامل کیفیت → پویش تازه در پس‌زمینه و اتصال بهترین مسیر', icon: Sparkles },
  { key: 'blackoutSolver', labelFa: 'کلید نجات ۱-کلیکه', descFa: 'اسکن بلادرنگ رله‌های داخلی و کانفیگ عبور از ملی‌شدن شبکه', icon: Crosshair },
  { key: 'batterySaver', labelFa: 'مدیریت بهینه باتری', descFa: 'تنظیم هوشمند فرکانس تپش‌ها با حفظ کامل کارایی تانل', icon: Battery },
];

type ExportKind = 'sing-box' | 'xray' | 'clash' | 'uris';

const EXPORT_TABS: { kind: ExportKind; labelFa: string; descFa: string }[] = [
  { kind: 'sing-box', labelFa: 'Sing-Box', descFa: 'خروجی JSON کامل با Inbound های Tun و Mixed' },
  { kind: 'xray', labelFa: 'Xray-Core', descFa: 'VLESS Reality & Vision با sockopt و TCP MSS' },
  { kind: 'clash', labelFa: 'Clash Meta / Mihomo', descFa: 'YAML استاندارد با url-test و رول‌های GEOIP,IR,DIRECT' },
  { kind: 'uris', labelFa: 'لینک‌های خام (Raw URIs)', descFa: 'vless:// و hysteria2:// و ss:// با ECH و Reality' },
];

export function AutoScannerEngineSections() {
  const {
    autoScannerEngine,
    setScannerCategory,
    runCategoryScan,
    runAllCategoryScan,
    setScanMode,
    setDiscoveryScale,
    autoDetectCarrier,
    runGeminiAnalysis,
    toggleAutoPilotFeature,
    triggerAutoHeal,
    triggerZeroTouch,
    triggerBlackoutSolver,
    regenerateConfigExports,
    exportAutoScannerReport,
    applyTunnelConfig,
    autoApplyBestConfig,
    toggleAutoApply,
    runConnectivityDecision,
    toggleAutoDecide,
    iranScanner,
  } = useUnifiedShieldStore();

  const [activeExport, setActiveExport] = useState<ExportKind>('sing-box');
  const [copied, setCopied] = useState(false);

  const {
    categories, activeCategoryId, scanMode, discoveryScale, discoveredNodes,
    totalCategoryScans, lastCategoryScanTs, carriers, activeCarrierId,
    autoPilot, gemini, localRL, exports, autoApply, appliedTunnel,
    connectivity, autoDecide,
  } = autoScannerEngine;

  const scanEnv = iranScanner.environment;
  const scanDnsOk = iranScanner.probes.find((p) => p.id === 'probe-dns')?.reachable ?? true;

  const activeCarrier = carriers.find((c) => c.id === activeCarrierId) ?? null;
  const activeMode = IRAN_SCAN_MODES.find((m) => m.id === scanMode) ?? IRAN_SCAN_MODES[0];
  const activeScale = IRAN_DISCOVERY_SCALES.find((s) => s.id === discoveryScale) ?? IRAN_DISCOVERY_SCALES[0];

  const exportText = useMemo(() => {
    if (activeExport === 'uris') return exports.rawUris.join('\n');
    if (activeExport === 'xray') return exports.xray;
    if (activeExport === 'clash') return exports.clashMeta;
    return exports.singBox;
  }, [activeExport, exports]);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(exportText);
      setCopied(true);
      setTimeout(() => setCopied(false), 1600);
    } catch {
      // clipboard unavailable — ignore
    }
  };

  const handleDownloadExport = () => {
    const ext = activeExport === 'clash' ? 'yaml' : activeExport === 'uris' ? 'txt' : 'json';
    const blob = new Blob([exportText], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `iran-${activeExport}-${Date.now()}.${ext}`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleDownloadReport = () => {
    const report = exportAutoScannerReport();
    const blob = new Blob([report], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `iran-auto-scanner-report-${Date.now()}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-4">
      {/* Section divider */}
      <div className="flex items-center gap-2 pt-1">
        <Boxes className="w-4 h-4 text-violet-300" />
        <p className="text-xs font-bold text-slate-200">موتور اسکن خودکار — ۲۰ دسته تخصصی + کاوش کوانتومی</p>
        <Badge variant="outline" className="text-[9px] border-violet-500/30 text-violet-300">Enterprise Quantum</Badge>
        <div className="h-px flex-1 bg-white/10" />
        <Button size="sm" variant="outline" onClick={() => runAllCategoryScan()} className="gap-1.5">
          <Boxes className="w-4 h-4 text-cyan-400" />Full Matrix (۲۰ کلاس)
        </Button>
        <Button size="sm" variant="outline" onClick={handleDownloadReport} className="gap-1.5">
          <FileText className="w-4 h-4 text-emerald-400" />گزارش
        </Button>
      </div>

      {/* Engine status strip */}
      <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-2">
        <div className="shield-tile rounded-xl px-3 py-2.5">
          <p className="text-[10px] text-slate-500">حالت اجرایی</p>
          <p className="text-sm font-bold text-cyan-400 mt-0.5 truncate">{activeMode.nameFa}</p>
        </div>
        <div className="shield-tile rounded-xl px-3 py-2.5">
          <p className="text-[10px] text-slate-500">مقیاس کاوش</p>
          <p className="text-sm font-bold text-violet-300 mt-0.5 truncate">{activeScale.nameFa}</p>
        </div>
        <div className="shield-tile rounded-xl px-3 py-2.5">
          <p className="text-[10px] text-slate-500">نودهای کشف‌شده</p>
          <p className="text-sm font-bold text-slate-200 mt-0.5">{toPersianNum(discoveredNodes)}</p>
        </div>
        <div className="shield-tile rounded-xl px-3 py-2.5">
          <p className="text-[10px] text-slate-500">اسکن دسته‌ها</p>
          <p className="text-sm font-bold text-slate-200 mt-0.5">{toPersianNum(totalCategoryScans)}</p>
        </div>
        <div className="shield-tile rounded-xl px-3 py-2.5">
          <p className="text-[10px] text-slate-500">اپراتور فعال</p>
          <p className={`text-sm font-bold mt-0.5 ${activeCarrier ? 'text-emerald-400' : 'text-slate-500'}`}>
            {activeCarrier ? activeCarrier.nameFa : 'نامشخص'}
          </p>
        </div>
        <div className="shield-tile rounded-xl px-3 py-2.5">
          <p className="text-[10px] text-slate-500">آخرین اسکن</p>
          <p className="text-sm font-bold text-slate-400 mt-0.5 font-mono" dir="ltr">
            {lastCategoryScanTs ? new Date(lastCategoryScanTs).toLocaleTimeString('fa-IR') : '—'}
          </p>
        </div>
      </div>

      {/* Execution modes + discovery scales */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <div className="shield-tile rounded-xl p-4">
          <p className="text-xs text-slate-400 mb-2.5 flex items-center gap-1.5">
            <ScanSearch className="w-4 h-4 text-cyan-400" />حالت‌های اجرایی اسکن
          </p>
          <div className="space-y-1.5">
            {IRAN_SCAN_MODES.map((m) => {
              const active = scanMode === m.id;
              return (
                <button
                  key={m.id}
                  onClick={() => setScanMode(m.id)}
                  className={`w-full text-right rounded-lg border px-3 py-2 transition-all ${active ? 'border-cyan-500/40 bg-cyan-500/[0.08]' : 'border-white/10 bg-white/[0.02] hover:bg-white/[0.05]'}`}
                >
                  <div className="flex items-center justify-between">
                    <p className={`text-[11px] font-bold ${active ? 'text-cyan-300' : 'text-slate-300'}`}>{m.nameFa}</p>
                    <span className="text-[9px] text-slate-500 font-mono" dir="ltr">×{toPersianNum(m.parallelism)}</span>
                  </div>
                  <p className="text-[9px] text-slate-500 mt-0.5 leading-4">{m.descFa}</p>
                </button>
              );
            })}
          </div>
        </div>

        <div className="shield-tile rounded-xl p-4">
          <p className="text-xs text-slate-400 mb-2.5 flex items-center gap-1.5">
            <Cpu className="w-4 h-4 text-violet-400" />مقیاس‌های کاوش پویا (Dynamic Discovery)
          </p>
          <div className="space-y-1.5">
            {IRAN_DISCOVERY_SCALES.map((s) => {
              const active = discoveryScale === s.id;
              return (
                <button
                  key={s.id}
                  onClick={() => setDiscoveryScale(s.id)}
                  className={`w-full text-right rounded-lg border px-3 py-2 transition-all ${active ? 'border-violet-500/40 bg-violet-500/[0.08]' : 'border-white/10 bg-white/[0.02] hover:bg-white/[0.05]'}`}
                >
                  <div className="flex items-center justify-between">
                    <p className={`text-[11px] font-bold ${active ? 'text-violet-300' : 'text-slate-300'}`}>{s.nameFa}</p>
                    <span className="text-[9px] text-slate-500 font-mono" dir="ltr">{toPersianNum(s.nodes)} nodes</span>
                  </div>
                  <p className="text-[9px] text-slate-500 mt-0.5">{s.descFa}</p>
                </button>
              );
            })}
          </div>
        </div>
      </div>

      {/* 20 categories */}
      <div className="shield-tile rounded-xl p-4">
        <div className="flex items-center justify-between mb-2.5 flex-wrap gap-2">
          <p className="text-xs text-slate-400 flex items-center gap-1.5">
            <LayoutGrid className="w-4 h-4 text-cyan-400" />۲۰ دسته‌بندی تخصصی پروتکل — برای اسکن اختصاصی کلیک کنید
          </p>
          <Badge variant="outline" className="text-[9px] border-white/10 text-slate-500">
            {categories.filter((c) => c.status === 'ready').length} از {categories.length} اسکن‌شده
          </Badge>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-2">
          {categories.map((c) => {
            const Icon = CATEGORY_ICON[c.id];
            const isActive = activeCategoryId === c.id;
            const isReady = c.status === 'ready';
            return (
              <button
                key={c.id}
                onClick={() => { setScannerCategory(c.id); runCategoryScan(c.id); }}
                className={`text-right rounded-lg border px-2.5 py-2 transition-all ${isActive ? 'border-cyan-500/40 bg-cyan-500/[0.07]' : 'border-white/10 bg-white/[0.02] hover:bg-white/[0.05]'}`}
              >
                <div className="flex items-center gap-2">
                  <span className={`shrink-0 inline-flex items-center justify-center w-7 h-7 rounded-lg ${isReady ? 'bg-emerald-500/10 text-emerald-400' : 'bg-white/[0.05] text-slate-400'}`}>
                    <Icon className="w-4 h-4" />
                  </span>
                  <div className="min-w-0 flex-1">
                    <p className="text-[11px] font-bold text-slate-200 truncate">{c.nameFa}</p>
                    <p className="text-[9px] text-slate-500 truncate" dir="ltr">{c.protocols[0]}{c.protocols.length > 1 ? ` +${c.protocols.length - 1}` : ''}</p>
                  </div>
                </div>
                {isReady && (
                  <div className="flex items-center justify-between mt-1.5 text-[9px]">
                    <span className={c.healthyNodes > c.totalNodes * 0.4 ? 'text-emerald-400 font-mono' : 'text-amber-400 font-mono'}>
                      {toPersianNum(c.healthyNodes)}/{toPersianNum(c.totalNodes)}
                    </span>
                    <span className="text-slate-500 font-mono">{c.bestLatencyMs != null ? toPersianNum(c.bestLatencyMs) + ' ms' : '—'}</span>
                  </div>
                )}
              </button>
            );
          })}
        </div>
      </div>

      {/* Carrier auto-detection */}
      <div className="shield-tile rounded-xl p-4">
        <div className="flex items-center justify-between mb-2.5 flex-wrap gap-2">
          <p className="text-xs text-slate-400 flex items-center gap-1.5">
            <Antenna className="w-4 h-4 text-emerald-400" />تشخیص خودکار اپراتور و شبکه (Carrier Auto-Detection)
          </p>
          <Button size="sm" variant="outline" onClick={() => autoDetectCarrier()} className="gap-1.5">
            <RefreshCw className="w-3.5 h-3.5" />تشخیص اپراتور
          </Button>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-3 lg:grid-cols-5 gap-2">
          {carriers.map((c) => {
            const isActive = activeCarrierId === c.id;
            return (
              <div key={c.id} className={`rounded-lg border px-2.5 py-2 transition-all ${isActive ? 'border-emerald-500/40 bg-emerald-500/[0.08]' : 'border-white/10 bg-white/[0.02]'}`}>
                <div className="flex items-center justify-between">
                  <p className="text-[11px] font-bold text-slate-200 truncate">{c.nameFa}</p>
                  {c.detected && <span className="relative flex h-2 w-2"><span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" /><span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-400" /></span>}
                </div>
                <p className="text-[9px] text-slate-500 font-mono mt-0.5" dir="ltr">{c.asn}</p>
                <div className="flex items-center justify-between mt-1 text-[9px]">
                  <span className="text-slate-500">{c.networkType === 'cellular' ? 'سلولار' : c.networkType === 'dsl' ? 'DSL/VDSL' : 'فیبر'}</span>
                  <span className="text-cyan-400 font-mono">MTU {toPersianNum(c.mtuClamp)}</span>
                </div>
                {c.detected && c.latencyMs != null && (
                  <p className="text-[9px] text-emerald-400 font-mono mt-0.5" dir="ltr">{toPersianNum(c.latencyMs)} ms</p>
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Auto-pilot */}
      <div className="shield-tile rounded-xl p-4">
        <div className="flex items-center justify-between mb-2.5 flex-wrap gap-2">
          <p className="text-xs text-slate-400 flex items-center gap-1.5">
            <Brain className="w-4 h-4 text-violet-400" />خلبان خودکار و خودترمیمی (Automation & Auto-Pilot)
          </p>
          <Badge className="bg-violet-500/15 text-violet-300 border border-violet-500/30 text-[9px]">
            {toPersianNum(autoPilot.healEvents)} رویداد ترمیم | {toPersianNum(autoPilot.validationEvents)} اعتبارسنجی
          </Badge>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-2">
          {AUTOPILOT_FEATURES.map((f) => {
            const Icon = f.icon;
            const on = autoPilot[f.key];
            return (
              <div key={f.key} className={`rounded-lg border px-2.5 py-2 ${on ? 'border-violet-500/25 bg-violet-500/[0.05]' : 'border-white/10 bg-white/[0.02]'}`}>
                <div className="flex items-center justify-between">
                  <p className="text-[10px] font-bold text-slate-200 flex items-center gap-1.5">
                    <Icon className="w-3.5 h-3.5 text-violet-400" />{f.labelFa}
                  </p>
                  <Switch checked={on} onCheckedChange={(v) => toggleAutoPilotFeature(f.key, v)} />
                </div>
                <p className="text-[9px] text-slate-500 mt-1 leading-4">{f.descFa}</p>
              </div>
            );
          })}
        </div>
        <div className="flex flex-wrap gap-2 mt-3">
          <Button size="sm" variant="outline" onClick={() => triggerAutoHeal()} className="gap-1.5">
            <HeartPulse className="w-3.5 h-3.5 text-rose-400" />خودترمیمی فوری
          </Button>
          <Button size="sm" variant="outline" onClick={() => triggerZeroTouch()} className="gap-1.5">
            <Sparkles className="w-3.5 h-3.5 text-cyan-400" />Zero-Touch
          </Button>
          <Button size="sm" variant="outline" onClick={() => triggerBlackoutSolver()} className="gap-1.5">
            <Crosshair className="w-3.5 h-3.5 text-red-400" />کلید نجات اضطراری
          </Button>
        </div>
      </div>

      {/* AI connectivity decision (tunnel / direct / DNS-tunnel on blackout) */}
      <div className="rounded-xl border border-violet-500/25 bg-gradient-to-br from-violet-500/[0.06] to-cyan-500/[0.04] p-4">
        <div className="flex items-center justify-between mb-2 flex-wrap gap-2">
          <p className="text-xs font-bold text-slate-200 flex items-center gap-1.5">
            <Brain className="w-4 h-4 text-violet-400" />تصمیم اتصال هوشمند AI — تانل یا بدون تانل
          </p>
          <div className="flex items-center gap-2 flex-wrap">
            <div className="flex items-center gap-1.5 text-[10px] text-slate-400">
              <span>تصمیم خودکار</span>
              <Switch checked={autoDecide} onCheckedChange={(v) => toggleAutoDecide(v)} />
            </div>
            <Button size="sm" variant="outline" onClick={() => runConnectivityDecision()} className="gap-1.5">
              <RefreshCw className="w-3.5 h-3.5" />تصمیم‌گیری مجدد
            </Button>
          </div>
        </div>

        <div className="grid grid-cols-2 lg:grid-cols-4 gap-2 mb-2">
          <div className="rounded-lg border border-white/10 bg-white/[0.02] px-2.5 py-2">
            <p className="text-[9px] text-slate-500">حالت مسیر</p>
            <p className="text-[11px] font-bold text-slate-200 mt-0.5">{TUNNEL_MODE_LABEL[connectivity.mode]}</p>
          </div>
          <div className="rounded-lg border border-white/10 bg-white/[0.02] px-2.5 py-2">
            <p className="text-[9px] text-slate-500">نیاز به تانل</p>
            <p className={`text-[11px] font-bold mt-0.5 ${connectivity.useTunnel ? 'text-cyan-300' : 'text-emerald-400'}`}>
              {connectivity.useTunnel ? 'بله — تانل فعال' : 'خیر — مستقیم'}
            </p>
          </div>
          <div className="rounded-lg border border-white/10 bg-white/[0.02] px-2.5 py-2">
            <p className="text-[9px] text-slate-500">پروتکل پیشنهادی</p>
            <p className="text-[11px] font-bold text-slate-200 mt-0.5 font-mono truncate" dir="ltr">{connectivity.recommendedProtocolId || '—'}</p>
          </div>
          <div className={`rounded-lg border px-2.5 py-2 ${connectivity.blackoutFallback !== 'none' ? 'border-red-500/25 bg-red-500/[0.05]' : 'border-white/10 bg-white/[0.02]'}`}>
            <p className="text-[9px] text-slate-500">مسیر جایگزین قطعی</p>
            <p className={`text-[11px] font-bold mt-0.5 ${connectivity.blackoutFallback !== 'none' ? 'text-red-300' : 'text-slate-500'}`}>
              {BLACKOUT_FALLBACK_LABEL[connectivity.blackoutFallback]}
            </p>
          </div>
        </div>

        {/* Blackout-specific live indicators */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 mb-2">
          <div className="rounded-lg border border-white/5 bg-white/[0.02] px-2 py-1.5">
            <div className="flex items-center justify-between">
              <span className="text-[9px] text-slate-500">دسترسی بین‌الملل</span>
              <span className={`text-[10px] font-bold ${scanEnv.internationalReach >= 20 ? 'text-emerald-400' : 'text-rose-400'}`}>{toPersianNum(scanEnv.internationalReach)}٪</span>
            </div>
            <div className="h-1 rounded-full bg-white/[0.06] overflow-hidden mt-1">
              <div className={`h-full rounded-full ${scanEnv.internationalReach >= 20 ? 'bg-emerald-400' : 'bg-rose-400'}`} style={{ width: `${scanEnv.internationalReach}%` }} />
            </div>
          </div>
          <div className="rounded-lg border border-white/5 bg-white/[0.02] px-2 py-1.5">
            <div className="flex items-center justify-between">
              <span className="text-[9px] text-slate-500">شبکه ملی</span>
              <span className="text-[10px] font-bold text-emerald-400">{toPersianNum(scanEnv.nationalReach)}٪</span>
            </div>
            <div className="h-1 rounded-full bg-white/[0.06] overflow-hidden mt-1">
              <div className="h-full rounded-full bg-emerald-400" style={{ width: `${scanEnv.nationalReach}%` }} />
            </div>
          </div>
          <div className="rounded-lg border border-white/5 bg-white/[0.02] px-2 py-1.5">
            <div className="flex items-center justify-between">
              <span className="text-[9px] text-slate-500">وضعیت DNS</span>
              <span className={`text-[10px] font-bold ${scanDnsOk ? 'text-emerald-400' : 'text-rose-400'}`}>{scanDnsOk ? 'سالم' : 'مختل'}</span>
            </div>
            <p className="text-[9px] text-slate-500 mt-1">{scanDnsOk ? 'تانلینگ DNS ممکن است' : 'DNS مسموم/قطع'}</p>
          </div>
          <div className="rounded-lg border border-white/5 bg-white/[0.02] px-2 py-1.5">
            <div className="flex items-center justify-between">
              <span className="text-[9px] text-slate-500">شدت DPI</span>
              <span className={`text-[10px] font-bold ${scanEnv.dpiIntensity > 55 ? 'text-rose-400' : 'text-amber-400'}`}>{toPersianNum(scanEnv.dpiIntensity)}٪</span>
            </div>
            <p className="text-[9px] text-slate-500 mt-1">{scanEnv.dpiIntensity > 55 ? 'امضای DPI شناسایی شد' : 'بدون بازرسی شدید'}</p>
          </div>
        </div>

        <p className="text-[11px] text-slate-400 leading-5">{connectivity.reasonFa}</p>

        {/* Fallback protocol specifics when international internet is cut */}
        {connectivity.blackoutFallback !== 'none' && (
          <div className="mt-2 grid grid-cols-1 sm:grid-cols-2 gap-1.5">
            <div className="rounded-lg border border-white/10 bg-white/[0.02] px-2 py-1.5 flex items-center gap-1.5">
              <AtSign className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
              <p className="text-[10px] text-slate-400">تانل DNS: بافر ۴۰۹۶ بایتی + تغییر برچسب QNAME (DNSTT/EDNS0)</p>
            </div>
            <div className="rounded-lg border border-white/10 bg-white/[0.02] px-2 py-1.5 flex items-center gap-1.5">
              <Radio className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
              <p className="text-[10px] text-slate-400">رله معکوس ملی: آپارات / تلوبیون / اسنپ / دیجی‌کالا / ابرآروان</p>
            </div>
          </div>
        )}
      </div>

      {/* Gemini AI + Local RL + exporters */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <div className="rounded-xl border border-cyan-500/25 bg-gradient-to-br from-cyan-500/[0.07] to-violet-500/[0.05] p-4">
          <div className="flex items-center justify-between mb-2">
            <p className="text-xs font-bold text-slate-200 flex items-center gap-1.5">
              <Wand2 className="w-4 h-4 text-cyan-400" />هوش مصنوعی جمینای + یادگیری محلی
            </p>
            <Button size="sm" variant="outline" onClick={() => runGeminiAnalysis()} className="gap-1.5">
              <RefreshCw className="w-3.5 h-3.5" />تحلیل
            </Button>
          </div>
          <div className="flex items-center flex-wrap gap-2">
            <Badge className="bg-cyan-500/15 text-cyan-300 border border-cyan-500/30 text-[10px]">
              طول شکستن TLS: {toPersianNum(gemini.tlsSplitLength)} بایت
            </Badge>
            <Badge variant="outline" className="text-[10px] border-white/10 text-slate-300" dir="ltr">
              {gemini.recommendedProtocolId}
            </Badge>
            <Badge className="bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 text-[10px]">
              اطمینان {toPersianNum(gemini.confidencePct)}٪
            </Badge>
          </div>
          <p className="text-[11px] text-slate-400 mt-2 leading-5">{gemini.reasoningFa}</p>
          <div className="mt-3 space-y-1.5 max-h-32 overflow-y-auto pr-1">
            {localRL.filter((r) => r.attempts > 0).length === 0 && (
              <p className="text-[10px] text-slate-600">سابقه یادگیری محلی خالی است (ثبت ۵۰ اتصال اخیر هر اپراتور، بدون ارسال تله‌متری)</p>
            )}
            {localRL.filter((r) => r.attempts > 0).slice(0, 5).map((r) => (
              <div key={r.carrierId} className="flex items-center justify-between rounded-md border border-white/5 bg-white/[0.02] px-2 py-1">
                <span className="text-[10px] text-slate-400 font-mono" dir="ltr">{r.carrierId}</span>
                <span className="text-[10px] text-slate-500">{toPersianNum(r.successes)}/{toPersianNum(r.attempts)} — {toPersianNum(r.winRatePct)}٪</span>
                <span className="text-[10px] text-slate-500 font-mono" dir="ltr">{toPersianNum(r.avgSetupMs)}ms</span>
              </div>
            ))}
          </div>
        </div>

        {/* Config exporters */}
        <div className="shield-tile rounded-xl p-4">
          <div className="flex items-center justify-between mb-2 flex-wrap gap-2">
            <p className="text-xs text-slate-400 flex items-center gap-1.5">
              <Server className="w-4 h-4 text-amber-400" />سیستم چندگانه خروجی کانفیگ‌ها (Core Exporters)
            </p>
            <div className="flex items-center gap-2 flex-wrap">
              <div className="flex items-center gap-1.5 text-[10px] text-slate-400">
                <span>فعال‌سازی خودکار</span>
                <Switch checked={autoApply} onCheckedChange={(v) => toggleAutoApply(v)} />
              </div>
              <Button size="sm" variant="ghost" onClick={() => regenerateConfigExports()} className="gap-1.5 text-slate-400">
                <RefreshCw className="w-3.5 h-3.5" />
              </Button>
            </div>
          </div>
          {/* Active tunnel — applied to UnifiedShield / MICAFP client */}
          <div className="flex items-center justify-between flex-wrap gap-2 mb-2 rounded-lg border border-emerald-500/20 bg-emerald-500/[0.05] px-2.5 py-2">
            <div className="flex items-center gap-2 min-w-0">
              {appliedTunnel.active ? (
                <span className="relative flex h-2.5 w-2.5 shrink-0"><span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" /><span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-emerald-400" /></span>
              ) : (
                <span className="inline-block w-2.5 h-2.5 rounded-full bg-slate-600 shrink-0" />
              )}
              <div className="min-w-0">
                <p className="text-[11px] font-bold text-slate-200 truncate">
                  {appliedTunnel.active ? appliedTunnel.labelFa : 'تانل فعال: —'}
                </p>
                <p className="text-[9px] text-slate-500 font-mono truncate" dir="ltr">
                  {appliedTunnel.active ? `${appliedTunnel.coreId} / ${appliedTunnel.protocolId}${appliedTunnel.autoApplied ? ' · AUTO' : ' · MANUAL'}` : 'هنوز کانفیگی روی کلاینت اعمال نشده'}
                </p>
              </div>
            </div>
            <div className="flex gap-1.5">
              <Button size="sm" variant="outline" onClick={() => autoApplyBestConfig()} className="gap-1.5">
                <Wand2 className="w-3.5 h-3.5 text-violet-400" />انتخاب خودکار AI
              </Button>
              <Button size="sm" onClick={() => applyTunnelConfig(activeExport)} className="gap-1.5">
                <Zap className="w-3.5 h-3.5" />اعمال تانل
              </Button>
            </div>
          </div>
          <div className="flex flex-wrap gap-1.5 mb-2">
            {EXPORT_TABS.map((t) => {
              const active = activeExport === t.kind;
              return (
                <button
                  key={t.kind}
                  onClick={() => setActiveExport(t.kind)}
                  className={`rounded-lg border px-2.5 py-1.5 text-[11px] font-bold transition-all ${active ? 'border-amber-500/40 bg-amber-500/10 text-amber-300' : 'border-white/10 bg-white/[0.03] text-slate-400 hover:bg-white/[0.06]'}`}
                >
                  {t.labelFa}
                </button>
              );
            })}
          </div>
          <p className="text-[9px] text-slate-500 mb-1.5">{EXPORT_TABS.find((t) => t.kind === activeExport)?.descFa}</p>
          <div className="relative">
            <pre className="rounded-lg border border-white/10 bg-black/40 p-2.5 text-[10px] text-emerald-300/90 overflow-auto max-h-48 font-mono whitespace-pre-wrap break-all" dir="ltr">
              {exportText}
            </pre>
            <div className="absolute top-2 left-2 flex gap-1.5">
              <Button size="sm" variant="outline" onClick={handleCopy} className="gap-1 h-7 px-2 text-[10px]">
                {copied ? <Check className="w-3 h-3 text-emerald-400" /> : <Copy className="w-3 h-3" />}
                {copied ? 'کپی شد' : 'کپی'}
              </Button>
              <Button size="sm" variant="outline" onClick={handleDownloadExport} className="gap-1 h-7 px-2 text-[10px]">
                <Download className="w-3 h-3" />دانلود
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
