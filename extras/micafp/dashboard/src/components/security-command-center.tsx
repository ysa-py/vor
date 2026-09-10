'use client';

import React, { useMemo, useState } from 'react';
import { motion } from 'framer-motion';
import {
  Shield, ShieldCheck, ShieldAlert, ShieldOff, Power,
  Lock, RefreshCw, Eye, Siren, Zap, Activity, CheckCircle2,
  Loader2, XCircle, Info, ChevronDown,
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';
import { useToast } from '@/hooks/use-toast';
import { computeSecurityPosture } from '@/lib/security-posture';
import type { PostureRowStatus } from '@/lib/security-posture';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

const SEVERITY_STYLE: Record<string, { labelFa: string; cls: string }> = {
  critical: { labelFa: 'بحرانی', cls: 'bg-rose-500/15 text-rose-400 border-rose-500/30' },
  high: { labelFa: 'بالا', cls: 'bg-amber-500/15 text-amber-400 border-amber-500/30' },
  medium: { labelFa: 'متوسط', cls: 'bg-cyan-500/15 text-cyan-400 border-cyan-500/30' },
};

const STATUS_META: Record<PostureRowStatus, { labelFa: string; textCls: string; barCls: string; iconCls: string; barWidth: string }> = {
  safe: { labelFa: 'ایمن', textCls: 'text-emerald-400', barCls: 'bg-emerald-500/70', iconCls: 'bg-emerald-500/15 text-emerald-400', barWidth: '100%' },
  warning: { labelFa: 'هشدار', textCls: 'text-amber-400', barCls: 'bg-amber-500/70', iconCls: 'bg-amber-500/15 text-amber-400', barWidth: '60%' },
  severe: { labelFa: 'شدید', textCls: 'text-rose-400', barCls: 'bg-rose-500/70', iconCls: 'bg-rose-500/15 text-rose-400', barWidth: '30%' },
  unknown: { labelFa: 'نامشخص', textCls: 'text-slate-400', barCls: 'bg-slate-500/70', iconCls: 'bg-slate-500/15 text-slate-400', barWidth: '50%' },
};

function StatusIcon({ status }: { status: PostureRowStatus }) {
  if (status === 'safe') return <CheckCircle2 className="w-3 h-3" />;
  if (status === 'unknown') return <XCircle className="w-3 h-3" />;
  return <ShieldAlert className="w-3 h-3" />;
}

export function SecurityCommandCenter() {
  const {
    connected, killSwitch, threatIntel, routing, firewall, iranScanner,
    emergencyActions, runEmergencyAction, toggleKillSwitch, toggleNetworkLock,
    stealthMode, networkResetStatus, toggleStealthMode, runNetworkReset,
  } = useUnifiedShieldStore();
  const { toast } = useToast();
  const [threatExpanded, setThreatExpanded] = useState(false);

  // Live security posture — single shared object derived from real subsystem
  // state. Both the ring and every row read from this one result.
  const posture = useMemo(() => computeSecurityPosture({
    killSwitchEnabled: killSwitch.enabled,
    networkLock: killSwitch.networkLock,
    dnsMode: routing.dnsMode,
    firewallMode: firewall.mode,
    firewallLearningEnabled: firewall.learningEnabled,
    totalBlocked: firewall.totalBlocked,
    connected,
    threats: threatIntel.activeThreats,
    blockedDomainsCount: threatIntel.blockedDomainsCount,
    dpiIntensity: iranScanner.environment.dpiIntensity,
    filterScore: iranScanner.filterScore,
  }), [killSwitch, routing.dnsMode, firewall, threatIntel, connected, iranScanner]);

  const postureColor = posture.score >= 80 ? 'text-emerald-400' : posture.score >= 60 ? 'text-amber-400' : 'text-rose-400';
  const strokeColor = posture.score >= 80 ? '#34d399' : posture.score >= 60 ? '#fbbf24' : '#fb7185';

  const resetRunning = networkResetStatus === 'running';

  const emergencyActionsList = [
    { id: 'kill-switch', labelFa: 'کشتن سوئیچ', descFa: 'قطع کامل ترافیک در نشت', icon: Power, color: 'text-rose-400 border-rose-500/30 hover:bg-rose-500/10', active: killSwitch.enabled, loading: false },
    { id: 'network-lock', labelFa: 'قفل شبکه', descFa: 'جلوگیری از نشت DNS/IP', icon: Lock, color: 'text-amber-400 border-amber-500/30 hover:bg-amber-500/10', active: killSwitch.networkLock, loading: false },
    { id: 'network-reset', labelFa: 'بازنشانی شبکه', descFa: 'بازنشانی TCP/IP سطح پایین', icon: RefreshCw, color: 'text-cyan-400 border-cyan-500/30 hover:bg-cyan-500/10', active: false, loading: resetRunning },
    { id: 'stealth', labelFa: 'حالت مخفی', descFa: 'پنهان‌سازی هویت ترافیک', icon: Eye, color: 'text-violet-400 border-violet-500/30 hover:bg-violet-500/10', active: stealthMode, loading: false },
  ];

  const handleEmergencyAction = async (id: string) => {
    if (id === 'network-reset') {
      if (resetRunning) return; // prevent double-firing
      const result = await runNetworkReset();
      toast({
        title: result === 'success' ? '✅ بازنشانی شبکه کامل شد' : '❌ بازنشانی شبکه ناموفق',
        description: result === 'success'
          ? 'کش DNS پاک شد، نشانی IP تمدید و جدول مسیریابی بازنشانی شد. کشتن سوئیچ و قفل شبکه حفظ شدند.'
          : 'بازنشانی سطح پایین شبکه با خطا مواجه شد. دوباره تلاش کنید.',
      });
      return;
    }
    if (id === 'stealth') {
      const willEnable = !stealthMode;
      toggleStealthMode();
      toast({
        title: willEnable ? '👁 حالت مخفی فعال شد' : '👁 حالت مخفی غیرفعال شد',
        description: willEnable
          ? 'هویت ترافیک پنهان شد — استتار اثر انگشت TLS و امضای بسته‌ها فعال گردید.'
          : 'هویت ترافیک آشکار شد — استتار غیرفعال گردید.',
      });
      return;
    }
    runEmergencyAction(id);
  };

  return (
    <Card className="shield-surface border-0">
      <CardHeader className="pb-2">
        <CardTitle className="text-slate-200 text-base flex items-center gap-2">
          <Shield className="w-5 h-5 text-emerald-400" />
          مرکز فرماندهی امنیت
          <Badge variant="outline" className="text-[10px] border-emerald-500/30 text-emerald-400">Posture Engine</Badge>
        </CardTitle>
        <CardDescription className="text-slate-500 text-xs">
          امتیاز وضعیت امنیتی زنده — محاسبه از وضعیت واقعی زیرسیستم‌ها
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          {/* Posture score ring */}
          <div className="shield-tile rounded-2xl p-4 flex flex-col items-center justify-center">
            <div className="relative w-32 h-32">
              <svg viewBox="0 0 120 120" className="w-full h-full -rotate-90">
                <circle cx="60" cy="60" r="52" fill="none" stroke="rgba(148,163,184,0.12)" strokeWidth="10" />
                <motion.circle
                  cx="60" cy="60" r="52" fill="none"
                  stroke={strokeColor} strokeWidth="10" strokeLinecap="round"
                  strokeDasharray={2 * Math.PI * 52}
                  initial={{ strokeDashoffset: 2 * Math.PI * 52 }}
                  animate={{ strokeDashoffset: 2 * Math.PI * 52 * (1 - posture.score / 100) }}
                  transition={{ duration: 1, ease: 'easeOut' }}
                />
              </svg>
              <div className="absolute inset-0 flex flex-col items-center justify-center">
                <p className={`text-3xl font-extrabold tabular ${postureColor}`}>{toPersianNum(posture.score)}</p>
                <p className="text-[10px] text-slate-500">از ۱۰۰</p>
              </div>
            </div>
            <div className="mt-2 flex items-center gap-2">
              <Badge variant="outline" className={`text-xs ${postureColor} border-current`}>درجه {posture.grade}</Badge>
              <span className="text-[10px] text-slate-500">{toPersianNum(posture.okCount)} از {toPersianNum(posture.checks.length)} سامانه امن</span>
            </div>
          </div>

          {/* Checks breakdown */}
          <div className="lg:col-span-2 space-y-1.5">
            {posture.checks.map((c) => {
              const meta = STATUS_META[c.status];
              const isThreat = c.id === 'threat';
              return (
                <div key={c.id}>
                  <div className="flex items-center gap-2 text-xs" title={isThreat ? c.detailFa : undefined}>
                    <span className={`w-4 h-4 rounded-full flex items-center justify-center ${meta.iconCls}`}>
                      <StatusIcon status={c.status} />
                    </span>
                    <span className="text-slate-300 flex-1 flex items-center gap-1">
                      {c.labelFa}
                      {isThreat && c.status !== 'safe' && (
                        <button
                          onClick={() => setThreatExpanded(v => !v)}
                          className="inline-flex items-center text-slate-500 hover:text-slate-300 transition-colors"
                          title="جزئیات تهدید"
                        >
                          <ChevronDown className={`w-3 h-3 transition-transform ${threatExpanded ? 'rotate-180' : ''}`} />
                        </button>
                      )}
                    </span>
                    <div className="w-24 h-1.5 rounded-full bg-white/[0.05] overflow-hidden">
                      <motion.div
                        className={`h-full rounded-full ${meta.barCls}`}
                        animate={{ width: meta.barWidth }}
                        transition={{ duration: 0.6 }}
                      />
                    </div>
                    <span className={`text-[10px] font-bold w-8 text-left ${meta.textCls}`}>
                      {meta.labelFa}
                    </span>
                  </div>
                  {isThreat && c.status !== 'safe' && threatExpanded && (
                    <div className="mt-1 mr-6 rounded-lg border border-amber-500/20 bg-amber-500/[0.04] px-2.5 py-1.5 flex items-start gap-1.5">
                      <Info className="w-3.5 h-3.5 text-amber-400 shrink-0 mt-0.5" />
                      <p className="text-[10px] text-slate-400 leading-4">{c.detailFa}</p>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>

        {/* Emergency actions */}
        <div>
          <p className="text-xs text-slate-400 font-semibold mb-2 flex items-center gap-1.5">
            <Siren className="w-3.5 h-3.5 text-rose-400" />
            اقدامات اضطراری
          </p>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
            {emergencyActionsList.map(a => {
              const Icon = a.icon;
              return (
                <button
                  key={a.id}
                  onClick={() => handleEmergencyAction(a.id)}
                  disabled={a.loading}
                  className={`rounded-xl border px-3 py-2.5 text-right transition-all bg-white/[0.03] ${a.color} ${a.active ? 'ring-1 ring-current' : ''} ${a.loading ? 'opacity-70 cursor-wait' : ''} disabled:opacity-60`}
                >
                  <span className="flex items-center justify-between mb-1">
                    {a.loading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Icon className="w-4 h-4" />}
                    {a.active && <span className="w-2 h-2 rounded-full bg-current animate-pulse" />}
                  </span>
                  <p className="text-xs font-bold">{a.labelFa}</p>
                  <p className="text-[9px] text-slate-500 mt-0.5">
                    {a.loading ? 'در حال اجرا…' : a.active && a.id === 'stealth' ? 'فعال — هویت پنهان' : a.descFa}
                  </p>
                </button>
              );
            })}
          </div>
        </div>

        {/* Emergency action history */}
        <div className="shield-inset rounded-xl p-3">
          <p className="text-[10px] text-slate-500 font-semibold mb-2 flex items-center gap-1">
            <Activity className="w-3 h-3 text-cyan-400" />
            تاریخچه فرمان‌های اضطراری
          </p>
          {emergencyActions.length === 0 ? (
            <p className="text-xs text-slate-600">هنوز فرمان اضطراری اجرا نشده است.</p>
          ) : (
            <div className="space-y-1.5 max-h-40 overflow-y-auto custom-scrollbar">
              {emergencyActions.map(a => {
                const resultBadge = a.result
                  ? a.result === 'success'
                    ? { labelFa: 'موفق', cls: 'bg-emerald-500/15 text-emerald-400 border-emerald-500/30' }
                    : a.result === 'failed'
                    ? { labelFa: 'ناموفق', cls: 'bg-rose-500/15 text-rose-400 border-rose-500/30' }
                    : { labelFa: 'در حال اجرا', cls: 'bg-cyan-500/15 text-cyan-400 border-cyan-500/30 animate-pulse' }
                  : null;
                const stateBadge = a.newState
                  ? a.newState === 'enabled'
                    ? { labelFa: 'فعال شد', cls: 'bg-violet-500/15 text-violet-400 border-violet-500/30' }
                    : { labelFa: 'غیرفعال شد', cls: 'bg-slate-500/15 text-slate-400 border-slate-500/30' }
                  : null;
                return (
                  <div key={a.id} className="flex items-center gap-2 text-[11px]">
                    <Badge className={`text-[9px] border ${SEVERITY_STYLE[a.severity]?.cls ?? 'border-cyan-500/30 text-cyan-400'}`}>
                      {SEVERITY_STYLE[a.severity]?.labelFa ?? a.severity}
                    </Badge>
                    {resultBadge && (
                      <Badge className={`text-[9px] border ${resultBadge.cls}`}>
                        {resultBadge.labelFa}
                      </Badge>
                    )}
                    {stateBadge && (
                      <Badge className={`text-[9px] border ${stateBadge.cls}`}>
                        {stateBadge.labelFa}
                      </Badge>
                    )}
                    <span className="text-slate-300 flex-1 truncate">{a.descriptionFa}</span>
                    <span className="text-slate-600 tabular">{new Date(a.executedAt).toLocaleTimeString('fa-IR')}</span>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Manual toggles */}
        <div className="grid grid-cols-2 gap-2">
          <button
            onClick={toggleKillSwitch}
            className={`rounded-xl border px-3 py-2.5 text-right transition-all flex items-center gap-2 ${
              killSwitch.enabled ? 'border-rose-500/40 bg-rose-500/10' : 'border-white/10 bg-white/[0.03]'
            }`}
          >
            <ShieldOff className={`w-4 h-4 ${killSwitch.enabled ? 'text-rose-400' : 'text-slate-500'}`} />
            <div>
              <p className={`text-xs font-bold ${killSwitch.enabled ? 'text-rose-400' : 'text-slate-300'}`}>کشتن سوئیچ</p>
              <p className="text-[9px] text-slate-500">{killSwitch.enabled ? 'فعال — ترافیک قطع' : 'غیرفعال'}</p>
            </div>
          </button>
          <button
            onClick={toggleNetworkLock}
            className={`rounded-xl border px-3 py-2.5 text-right transition-all flex items-center gap-2 ${
              killSwitch.networkLock ? 'border-amber-500/40 bg-amber-500/10' : 'border-white/10 bg-white/[0.03]'
            }`}
          >
            <Lock className={`w-4 h-4 ${killSwitch.networkLock ? 'text-amber-400' : 'text-slate-500'}`} />
            <div>
              <p className={`text-xs font-bold ${killSwitch.networkLock ? 'text-amber-400' : 'text-slate-300'}`}>قفل شبکه</p>
              <p className="text-[9px] text-slate-500">{killSwitch.networkLock ? 'فعال — بدون نشت' : 'غیرفعال'}</p>
            </div>
          </button>
        </div>

        <Button
          variant="outline"
          disabled={resetRunning}
          className="w-full border-slate-700 text-slate-400 hover:text-rose-400 hover:border-rose-500/40 disabled:opacity-60"
          onClick={async () => {
            const result = await runNetworkReset();
            toast({
              title: result === 'success' ? '✅ بازنشانی کامل شبکه انجام شد' : '❌ بازنشانی کامل شبکه ناموفق',
              description: result === 'success'
                ? 'همه اتصالات TCP/IP بازنشانی شدند و تمام ترافیک دوباره به داخل تونل هدایت شد.'
                : 'بازنشانی با خطا مواجه شد — دوباره تلاش کنید.',
            });
          }}
        >
          {resetRunning ? <Loader2 className="w-4 h-4 animate-spin" /> : <Zap className="w-4 h-4" />}
          {resetRunning ? 'در حال بازنشانی شبکه…' : 'بازنشانی کامل شبکه (Low-Level Reset)'}
        </Button>
      </CardContent>
    </Card>
  );
}
