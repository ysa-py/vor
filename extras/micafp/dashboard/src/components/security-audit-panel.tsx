'use client';

import React, { useState, useMemo, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Shield, ShieldCheck, ShieldAlert, ShieldOff, Lock, Unlock,
  Eye, EyeOff, Globe, Wifi, WifiOff, AlertTriangle, CheckCircle2,
  XCircle, Fingerprint, Server, Activity, Zap, Key,
  AlertOctagon, Scan, Bug, ShieldHalf, ArrowRight, Info,
  ChevronDown, RefreshCw, Network, Crosshair, Signal,
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';
import { PROTOCOL_LABELS } from '@/lib/unified-shield-types';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

// ──────────────────────────────────────────────
// Privacy Score Circular Gauge
// ──────────────────────────────────────────────
function PrivacyScoreGauge() {
  const { cores, orchestrator, connected, killSwitch, routing, threatIntel } = useUnifiedShieldStore();
  const activeCore = cores.find(c => c.id === orchestrator.activeCoreId);

  const score = useMemo(() => {
    if (!connected || !activeCore) return 0;

    // DNS leak check (0-20)
    const dnsScore = activeCore.health.dnsLeak ? 0 : 20;
    // Kill switch check (0-20)
    const ksScore = killSwitch.enabled ? 20 : 5;
    // Encryption check (0-20) - DoH/DoT gives more points
    const encScore = routing.dnsMode === 'doh' ? 20 : routing.dnsMode === 'dot' ? 17 : 10;
    // DPI exposure (0-20)
    const dpiScore = Math.round(Math.max(0, 20 - activeCore.health.dpiExposure * 0.4));
    // Threat mitigation (0-20)
    const mitigatedRatio = threatIntel.activeThreats.length > 0
      ? threatIntel.activeThreats.filter(t => t.mitigated).length / threatIntel.activeThreats.length
      : 1;
    const threatScore = Math.round(mitigatedRatio * 20);

    return dnsScore + ksScore + encScore + dpiScore + threatScore;
  }, [connected, activeCore, killSwitch, routing, threatIntel]);

  const radius = 80;
  const circumference = 2 * Math.PI * radius;
  const strokeDashoffset = circumference - (score / 100) * circumference;

  const getColor = (s: number) => {
    if (s >= 80) return { main: '#10b981', glow: 'rgba(16,185,129,0.3)' };
    if (s >= 60) return { main: '#06b6d4', glow: 'rgba(6,182,212,0.3)' };
    if (s >= 40) return { main: '#eab308', glow: 'rgba(234,179,8,0.3)' };
    if (s >= 20) return { main: '#f97316', glow: 'rgba(249,115,22,0.3)' };
    return { main: '#ef4444', glow: 'rgba(239,68,68,0.3)' };
  };

  const getLabel = (s: number) => {
    if (s >= 80) return 'حریم خصوصی عالی';
    if (s >= 60) return 'حریم خصوصی خوب';
    if (s >= 40) return 'نیاز به بهبود';
    if (s >= 20) return 'حریم خصوصی ضعیف';
    return 'خطرناک';
  };

  const color = getColor(score);

  return (
    <div className="flex flex-col items-center">
      <div className="relative">
        <svg width="200" height="200" className="transform -rotate-90">
          <defs>
            <linearGradient id="privacyGrad" x1="0%" y1="0%" x2="100%" y2="0%">
              <stop offset="0%" stopColor={color.main} stopOpacity={0.2} />
              <stop offset="100%" stopColor={color.main} />
            </linearGradient>
          </defs>
          <circle cx="100" cy="100" r={radius} fill="none" stroke="#1e293b" strokeWidth="12" />
          <motion.circle
            cx="100" cy="100" r={radius} fill="none"
            stroke="url(#privacyGrad)" strokeWidth="12" strokeLinecap="round"
            strokeDasharray={circumference}
            initial={{ strokeDashoffset: circumference }}
            animate={{ strokeDashoffset }}
            transition={{ duration: 1.5, ease: 'easeOut' }}
            style={{ filter: `drop-shadow(0 0 12px ${color.glow})` }}
          />
        </svg>
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          <motion.span
            className="text-4xl font-bold"
            style={{ color: color.main }}
            key={score}
            initial={{ scale: 0.5, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            transition={{ type: 'spring', stiffness: 300, damping: 20 }}
          >
            {toPersianNum(score)}
          </motion.span>
          <span className="text-xs text-slate-500 mt-1">از ۱۰۰</span>
        </div>
      </div>
      <Badge
        className="mt-3 text-xs"
        style={{ borderColor: `${color.main}40`, color: color.main, backgroundColor: `${color.main}10` }}
      >
        {connected ? getLabel(score) : 'متصل نیست'}
      </Badge>

      {/* Score Breakdown */}
      <div className="w-full mt-4 space-y-2">
        {[
          { label: 'حفاظت DNS', value: connected && activeCore ? (activeCore.health.dnsLeak ? 0 : 20) : 0, max: 20 },
          { label: 'سوئیچ کشت', value: killSwitch.enabled ? 20 : 5, max: 20 },
          { label: 'رمزگذاری DNS', value: routing.dnsMode === 'doh' ? 20 : routing.dnsMode === 'dot' ? 17 : 10, max: 20 },
          { label: 'مخفی‌سازی DPI', value: connected && activeCore ? Math.round(Math.max(0, 20 - activeCore.health.dpiExposure * 0.4)) : 0, max: 20 },
          { label: 'بی‌اثر کردن تهدیدات', value: connected ? Math.round((threatIntel.activeThreats.filter(t => t.mitigated).length / Math.max(1, threatIntel.activeThreats.length)) * 20) : 0, max: 20 },
        ].map((item, i) => (
          <div key={i} className="flex items-center gap-2 text-xs">
            <span className="text-slate-500 w-32 text-right">{item.label}</span>
            <Progress value={(item.value / item.max) * 100} className="h-1.5 flex-1" />
            <span className={`${item.value >= item.max * 0.8 ? 'text-emerald-400' : item.value >= item.max * 0.5 ? 'text-yellow-400' : 'text-red-400'} font-medium w-8`}>
              {toPersianNum(item.value)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ──────────────────────────────────────────────
// DNS Leak Test Card
// ──────────────────────────────────────────────
function DNSLeakTestCard() {
  const { cores, orchestrator, connected, routing } = useUnifiedShieldStore();
  const activeCore = cores.find(c => c.id === orchestrator.activeCoreId);
  const hasLeak = activeCore?.health.dnsLeak ?? false;

  const status = !connected ? 'disconnected' : hasLeak ? 'leak' : 'secure';
  const statusConfig = {
    disconnected: { icon: WifiOff, label: 'متصل نیست', color: 'text-slate-500', bg: 'bg-white/[0.03]', border: 'border-white/10', iconColor: 'text-slate-500' },
    leak: { icon: AlertTriangle, label: 'نشت DNS شناسایی شد!', color: 'text-red-400', bg: 'bg-red-500/5', border: 'border-red-500/30', iconColor: 'text-red-400' },
    secure: { icon: ShieldCheck, label: 'بدون نشت DNS', color: 'text-emerald-400', bg: 'bg-emerald-500/5', border: 'border-emerald-500/30', iconColor: 'text-emerald-400' },
  };

  const cfg = statusConfig[status];
  const Icon = cfg.icon;

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className={`rounded-xl border ${cfg.border} ${cfg.bg} p-4`}
    >
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Globe className="w-4 h-4 text-violet-400" />
          <span className="text-sm text-slate-300 font-medium">تست نشت DNS</span>
        </div>
        <Icon className={`w-5 h-5 ${cfg.iconColor}`} />
      </div>
      <p className={`text-sm font-bold ${cfg.color} mb-2`}>{cfg.label}</p>
      <div className="flex items-center gap-2 text-xs text-slate-500">
        <span>حالت DNS:</span>
        <Badge variant="outline" className={`text-[10px] ${routing.dnsMode === 'doh' ? 'border-emerald-500/30 text-emerald-400' : routing.dnsMode === 'dot' ? 'border-cyan-500/30 text-cyan-400' : 'border-yellow-500/30 text-yellow-400'}`}>
          {routing.dnsMode === 'doh' ? 'DNS over HTTPS' : routing.dnsMode === 'dot' ? 'DNS over TLS' : 'ساده (ناامن)'}
        </Badge>
      </div>
      {connected && (
        <div className="mt-2 text-xs text-slate-500">
          ارائه‌دهنده فعال: <span className="text-cyan-400">{routing.activeDnsProvider}</span>
        </div>
      )}
      {hasLeak && connected && (
        <div className="mt-2 flex items-center gap-1 text-xs bg-red-500/10 rounded-lg px-2 py-1 text-red-400">
          <AlertOctagon className="w-3 h-3" />
          درخواست‌های DNS شما فاش می‌شوند!
        </div>
      )}
    </motion.div>
  );
}

// ──────────────────────────────────────────────
// WebRTC Leak Detection Card
// ──────────────────────────────────────────────
function WebRTCLeakCard() {
  const { connected, killSwitch } = useUnifiedShieldStore();
  // Simulate WebRTC check - network lock prevents WebRTC leaks
  const hasLeak = connected && !killSwitch.networkLock;
  const status = !connected ? 'disconnected' : hasLeak ? 'leak' : 'secure';

  const statusConfig = {
    disconnected: { icon: WifiOff, label: 'متصل نیست', color: 'text-slate-500', bg: 'bg-white/[0.03]', border: 'border-white/10' },
    leak: { icon: AlertTriangle, label: 'نشت WebRTC ممکن است!', color: 'text-yellow-400', bg: 'bg-yellow-500/5', border: 'border-yellow-500/30' },
    secure: { icon: ShieldCheck, label: 'WebRTC محافظت‌شده', color: 'text-emerald-400', bg: 'bg-emerald-500/5', border: 'border-emerald-500/30' },
  };

  const cfg = statusConfig[status as keyof typeof statusConfig];
  const Icon = cfg.icon;

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: 0.1 }}
      className={`rounded-xl border ${cfg.border} ${cfg.bg} p-4`}
    >
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Signal className="w-4 h-4 text-pink-400" />
          <span className="text-sm text-slate-300 font-medium">نشت WebRTC</span>
        </div>
        <Icon className={`w-5 h-5 ${cfg.color}`} />
      </div>
      <p className={`text-sm font-bold ${cfg.color} mb-2`}>{cfg.label}</p>
      <div className="text-xs text-slate-500">
        قفل شبکه: <span className={killSwitch.networkLock ? 'text-emerald-400' : 'text-red-400'}>
          {killSwitch.networkLock ? 'فعال' : 'غیرفعال'}
        </span>
      </div>
      {hasLeak && (
        <div className="mt-2 text-xs text-yellow-400 bg-yellow-500/10 rounded-lg px-2 py-1">
          آی‌پی واقعی شما ممکن است از طریق WebRTC فاش شود
        </div>
      )}
    </motion.div>
  );
}

// ──────────────────────────────────────────────
// IPv6 Leak Detection Card
// ──────────────────────────────────────────────
function IPv6LeakCard() {
  const { connected, routing } = useUnifiedShieldStore();
  const hasLeak = connected && routing.ipv6Enabled;
  const status = !connected ? 'disconnected' : hasLeak ? 'potential' : 'secure';

  const statusConfig = {
    disconnected: { icon: WifiOff, label: 'متصل نیست', color: 'text-slate-500', bg: 'bg-white/[0.03]', border: 'border-white/10' },
    potential: { icon: Info, label: 'IPv6 فعال — ریسک نشت', color: 'text-yellow-400', bg: 'bg-yellow-500/5', border: 'border-yellow-500/30' },
    secure: { icon: ShieldCheck, label: 'بدون نشت IPv6', color: 'text-emerald-400', bg: 'bg-emerald-500/5', border: 'border-emerald-500/30' },
  };

  const cfg = statusConfig[status as keyof typeof statusConfig];
  const Icon = cfg.icon;

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: 0.15 }}
      className={`rounded-xl border ${cfg.border} ${cfg.bg} p-4`}
    >
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Network className="w-4 h-4 text-cyan-400" />
          <span className="text-sm text-slate-300 font-medium">نشت IPv6</span>
        </div>
        <Icon className={`w-5 h-5 ${cfg.color}`} />
      </div>
      <p className={`text-sm font-bold ${cfg.color} mb-2`}>{cfg.label}</p>
      <div className="text-xs text-slate-500">
        IPv6: <span className={routing.ipv6Enabled ? 'text-yellow-400' : 'text-emerald-400'}>
          {routing.ipv6Enabled ? 'فعال' : 'غیرفعال'}
        </span>
      </div>
    </motion.div>
  );
}

// ──────────────────────────────────────────────
// Kill Switch Status Card
// ──────────────────────────────────────────────
function KillSwitchStatusCard() {
  const { killSwitch, connected, toggleKillSwitch, toggleNetworkLock } = useUnifiedShieldStore();

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: 0.2 }}
      className={`rounded-xl border p-4 ${killSwitch.enabled ? 'border-emerald-500/30 bg-emerald-500/5' : 'border-red-500/30 bg-red-500/5'}`}
    >
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          {killSwitch.enabled ? <ShieldCheck className="w-4 h-4 text-emerald-400" /> : <ShieldOff className="w-4 h-4 text-red-400" />}
          <span className="text-sm text-slate-300 font-medium">سوئیچ کشت (Kill Switch)</span>
        </div>
        <Badge className={`text-[10px] ${killSwitch.enabled ? 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30' : 'bg-red-500/20 text-red-400 border-red-500/30'}`}>
          {killSwitch.enabled ? 'فعال' : 'غیرفعال'}
        </Badge>
      </div>

      <p className={`text-sm font-bold ${killSwitch.enabled ? 'text-emerald-400' : 'text-red-400'} mb-3`}>
        {killSwitch.enabled ? 'ترافیک محافظت‌شده — بدون نشت' : 'خطر نشت ترافیک!'}
      </p>

      <div className="space-y-2">
        <div className="flex items-center justify-between text-xs bg-white/[0.04] rounded-lg p-2">
          <div className="flex items-center gap-1.5">
            <Lock className="w-3 h-3 text-slate-500" />
            <span className="text-slate-400">مسدودسازی کامل هنگام قطع</span>
          </div>
          <span className={killSwitch.blockAllOnDisconnect ? 'text-emerald-400' : 'text-red-400'}>
            {killSwitch.blockAllOnDisconnect ? 'فعال' : 'غیرفعال'}
          </span>
        </div>
        <div className="flex items-center justify-between text-xs bg-white/[0.04] rounded-lg p-2">
          <div className="flex items-center gap-1.5">
            <Key className="w-3 h-3 text-slate-500" />
            <span className="text-slate-400">قفل شبکه (Network Lock)</span>
          </div>
          <span className={killSwitch.networkLock ? 'text-emerald-400' : 'text-red-400'}>
            {killSwitch.networkLock ? 'فعال' : 'غیرفعال'}
          </span>
        </div>
      </div>

      {!killSwitch.enabled && (
        <div className="mt-3 flex items-center gap-1 text-xs bg-red-500/10 rounded-lg px-2 py-1.5 text-red-400">
          <AlertOctagon className="w-3 h-3" />
          بدون سوئیچ کشت، ترافیک شما در صورت قطع VPN فاش می‌شود
        </div>
      )}
    </motion.div>
  );
}

// ──────────────────────────────────────────────
// Encryption Strength Indicator
// ──────────────────────────────────────────────
function EncryptionStrengthCard() {
  const { cores, orchestrator, connected, routing } = useUnifiedShieldStore();
  const activeCore = cores.find(c => c.id === orchestrator.activeCoreId);

  const encLevel = useMemo(() => {
    if (!connected || !activeCore) return { level: 0, label: 'متصل نیست', details: [] };
    const capabilities = activeCore.capabilities;
    const hasReality = capabilities.includes('vless-reality-xtls');
    const hasHysteria = capabilities.includes('hysteria2');
    const hasAmneziaWG = capabilities.includes('amneziawg-1.5');
    const isDoH = routing.dnsMode === 'doh';
    const isDoT = routing.dnsMode === 'dot';

    let level = 50;
    const details: string[] = [];

    if (hasReality) { level += 20; details.push('VLESS Reality + XTLS'); }
    if (hasAmneziaWG) { level += 15; details.push('آمنزیاوی‌جی ۱.۵'); }
    if (hasHysteria) { level += 10; details.push('هیستریا۲ — رمزگذاری QUIC'); }
    if (isDoH) { level += 5; details.push('DNS over HTTPS'); }
    else if (isDoT) { level += 3; details.push('DNS over TLS'); }

    level = Math.min(100, level);

    let label = 'رمزگذاری ضعیف';
    if (level >= 90) label = 'رمزگذاری عالی';
    else if (level >= 70) label = 'رمزگذاری قوی';
    else if (level >= 50) label = 'رمزگذاری متوسط';

    return { level, label, details };
  }, [connected, activeCore, routing]);

  const color = encLevel.level >= 80 ? '#10b981' : encLevel.level >= 60 ? '#06b6d4' : encLevel.level >= 40 ? '#eab308' : '#ef4444';

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: 0.25 }}
      className="rounded-xl border border-white/10 bg-white/[0.03] p-4"
    >
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Key className="w-4 h-4 text-amber-400" />
          <span className="text-sm text-slate-300 font-medium">قدرت رمزگذاری</span>
        </div>
        <span className="text-sm font-bold" style={{ color }}>{encLevel.label}</span>
      </div>

      <div className="mb-3">
        <div className="flex items-center justify-between text-xs mb-1">
          <span className="text-slate-500">سطح رمزگذاری</span>
          <span style={{ color }}>{toPersianNum(encLevel.level)}٪</span>
        </div>
        <div className="h-2 bg-slate-700/50 rounded-full overflow-hidden">
          <motion.div
            className="h-full rounded-full"
            style={{ backgroundColor: color }}
            initial={{ width: 0 }}
            animate={{ width: `${encLevel.level}%` }}
            transition={{ duration: 1, ease: 'easeOut' }}
          />
        </div>
      </div>

      {encLevel.details.length > 0 && (
        <div className="flex gap-1 flex-wrap">
          {encLevel.details.map((d, i) => (
            <Badge key={i} variant="outline" className="text-[10px] border-cyan-500/30 text-cyan-400 bg-cyan-500/5">
              {d}
            </Badge>
          ))}
        </div>
      )}

      {connected && activeCore && (
        <div className="mt-3 grid grid-cols-2 gap-2 text-xs">
          <div className="bg-slate-800/50 rounded-lg p-2">
            <span className="text-slate-400 block">پروتکل فعال</span>
            <p className="text-cyan-400 font-bold">{PROTOCOL_LABELS[activeCore.capabilities[0]]?.nameFa ?? activeCore.capabilities[0]}</p>
          </div>
          <div className="bg-slate-800/50 rounded-lg p-2">
            <span className="text-slate-400 block">نویز DPI</span>
            <p className={activeCore.health.dpiExposure < 15 ? 'text-emerald-400' : activeCore.health.dpiExposure < 30 ? 'text-yellow-400' : 'text-red-400'}>
              {toPersianNum(activeCore.health.dpiExposure)}٪
            </p>
          </div>
        </div>
      )}
    </motion.div>
  );
}



// ──────────────────────────────────────────────
// Security Recommendations
// ──────────────────────────────────────────────
function SecurityRecommendations() {
  const { cores, orchestrator, connected, killSwitch, routing, threatIntel } = useUnifiedShieldStore();
  const activeCore = cores.find(c => c.id === orchestrator.activeCoreId);

  const recommendations = useMemo(() => {
    const recs: Array<{ id: string; severity: 'critical' | 'high' | 'medium' | 'low'; text: string; action: string; actionLabel: string }> = [];

    if (!connected) {
      recs.push({ id: 'r-connect', severity: 'critical', text: 'ابتدا به VPN متصل شوید — ترافیک شما محافظت‌نشده است', action: '', actionLabel: '' });
      return recs;
    }

    if (!killSwitch.enabled) {
      recs.push({ id: 'r-ks', severity: 'critical', text: 'سوئیچ کشت غیرفعال است — در صورت قطع VPN، ترافیک شما فاش می‌شود', action: 'toggleKillSwitch', actionLabel: 'فعال‌سازی' });
    }

    if (!killSwitch.networkLock) {
      recs.push({ id: 'r-nl', severity: 'high', text: 'قفل شبکه غیرفعال است — نشت WebRTC ممکن است', action: 'toggleNetworkLock', actionLabel: 'فعال‌سازی' });
    }

    if (activeCore?.health.dnsLeak) {
      recs.push({ id: 'r-dns', severity: 'critical', text: 'نشت DNS شناسایی شد! ارائه‌دهنده DNS خود را تغییر دهید', action: '', actionLabel: '' });
    }

    if (routing.dnsMode === 'plain') {
      recs.push({ id: 'r-doh', severity: 'high', text: 'DNS رمزگذاری‌نشده استفاده می‌شود — به DoH یا DoT تغییر دهید', action: '', actionLabel: '' });
    }

    if (activeCore && activeCore.health.dpiExposure > 30) {
      recs.push({ id: 'r-dpi', severity: 'high', text: `نویز DPI بالا (${toPersianNum(activeCore.health.dpiExposure)}٪) — هسته دیگری انتخاب شود`, action: '', actionLabel: '' });
    }

    const unmitigated = threatIntel.activeThreats.filter(t => !t.mitigated);
    if (unmitigated.length > 0) {
      recs.push({ id: 'r-threat', severity: 'medium', text: `${toPersianNum(unmitigated.length)} تهدید بی‌اثرنشده — اقدام متقابل بررسی شود`, action: '', actionLabel: '' });
    }

    if (activeCore && activeCore.health.latency > 150) {
      recs.push({ id: 'r-latency', severity: 'medium', text: 'تأخیر بالا ممکن است نشان‌دهنده تشخیص DPI باشد', action: '', actionLabel: '' });
    }

    if (routing.ipv6Enabled) {
      recs.push({ id: 'r-ipv6', severity: 'low', text: 'IPv6 فعال است — ممکن است منجر به نشت آدرس شود', action: '', actionLabel: '' });
    }

    return recs;
  }, [connected, killSwitch, routing, activeCore, threatIntel]);

  const severityConfig = {
    critical: { color: 'text-red-400', bg: 'bg-red-500/10', border: 'border-red-500/30', icon: AlertOctagon },
    high: { color: 'text-orange-400', bg: 'bg-orange-500/10', border: 'border-orange-500/30', icon: AlertTriangle },
    medium: { color: 'text-yellow-400', bg: 'bg-yellow-500/10', border: 'border-yellow-500/30', icon: Info },
    low: { color: 'text-cyan-400', bg: 'bg-cyan-500/10', border: 'border-cyan-500/30', icon: Info },
  };

  return (
    <div className="space-y-2 max-h-80 overflow-y-auto custom-scrollbar">
      {recommendations.length === 0 ? (
        <div className="text-center py-6">
          <CheckCircle2 className="w-8 h-8 text-emerald-400 mx-auto mb-2" />
          <p className="text-sm text-emerald-400">همه چیز ایمن است!</p>
          <p className="text-xs text-slate-500">پیشنهاد امنیتی وجود ندارد</p>
        </div>
      ) : (
        recommendations.map((rec, i) => {
          const cfg = severityConfig[rec.severity];
          const Icon = cfg.icon;
          return (
            <motion.div
              key={rec.id}
              initial={{ opacity: 0, x: 10 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: i * 0.05 }}
              className={`rounded-lg border ${cfg.border} ${cfg.bg} p-3`}
            >
              <div className="flex items-start gap-2">
                <Icon className={`w-4 h-4 ${cfg.color} mt-0.5 flex-shrink-0`} />
                <div className="flex-1">
                  <p className="text-xs text-slate-300">{rec.text}</p>
                  {rec.actionLabel && (
                    <Button
                      size="sm"
                      variant="ghost"
                      className="mt-1.5 h-6 text-[10px] text-cyan-400 hover:text-cyan-300 hover:bg-cyan-500/10"
                    >
                      {rec.actionLabel}
                      <ArrowRight className="w-3 h-3 mr-1" />
                    </Button>
                  )}
                </div>
              </div>
            </motion.div>
          );
        })
      )}
    </div>
  );
}

// ──────────────────────────────────────────────
// Real-time Security Status Badges
// ──────────────────────────────────────────────
function SecurityStatusBadges() {
  const { connected, cores, orchestrator, killSwitch, routing, threatIntel } = useUnifiedShieldStore();
  const activeCore = cores.find(c => c.id === orchestrator.activeCoreId);

  const badges = useMemo(() => {
    if (!connected) return [{ label: 'VPN', status: 'disconnected' as const, text: 'قطع' }];
    const items: Array<{ label: string; status: 'secure' | 'warning' | 'danger'; text: string }> = [];

    items.push({
      label: 'VPN',
      status: 'secure',
      text: 'متصل',
    });

    items.push({
      label: 'DNS',
      status: activeCore?.health.dnsLeak ? 'danger' : 'secure',
      text: activeCore?.health.dnsLeak ? 'نشت!' : 'ایمن',
    });

    items.push({
      label: 'Kill Switch',
      status: killSwitch.enabled ? 'secure' : 'danger',
      text: killSwitch.enabled ? 'فعال' : 'خطر',
    });

    items.push({
      label: 'DPI',
      status: (activeCore?.health.dpiExposure ?? 0) > 30 ? 'danger' : (activeCore?.health.dpiExposure ?? 0) > 15 ? 'warning' : 'secure',
      text: `${toPersianNum(Math.round(activeCore?.health.dpiExposure ?? 0))}٪`,
    });

    items.push({
      label: 'رمزگذاری',
      status: routing.dnsMode === 'doh' ? 'secure' : routing.dnsMode === 'dot' ? 'secure' : 'warning',
      text: routing.dnsMode === 'doh' ? 'DoH' : routing.dnsMode === 'dot' ? 'DoT' : 'ساده',
    });

    items.push({
      label: 'تهدید',
      status: threatIntel.threatLevel === 'critical' ? 'danger' : threatIntel.threatLevel === 'high' ? 'warning' : 'secure',
      text: threatIntel.threatLevel === 'critical' ? 'بحرانی' : threatIntel.threatLevel === 'high' ? 'بالا' : threatIntel.threatLevel === 'medium' ? 'متوسط' : 'پایین',
    });

    return items;
  }, [connected, activeCore, killSwitch, routing, threatIntel]);

  const statusColors = {
    secure: { bg: 'bg-emerald-500/10', border: 'border-emerald-500/30', text: 'text-emerald-400' },
    warning: { bg: 'bg-yellow-500/10', border: 'border-yellow-500/30', text: 'text-yellow-400' },
    danger: { bg: 'bg-red-500/10', border: 'border-red-500/30', text: 'text-red-400' },
    disconnected: { bg: 'bg-slate-500/10', border: 'border-slate-500/30', text: 'text-slate-400' },
  };

  return (
    <div className="flex gap-2 flex-wrap">
      {badges.map((badge, i) => {
        const colors = statusColors[badge.status];
        return (
          <motion.div
            key={badge.label}
            initial={{ opacity: 0, scale: 0.8 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ delay: i * 0.05 }}
            className={`rounded-lg border ${colors.border} ${colors.bg} px-2.5 py-1.5 flex items-center gap-1.5`}
          >
            <span className="text-[10px] text-slate-500">{badge.label}</span>
            <span className={`text-xs font-bold ${colors.text}`}>{badge.text}</span>
          </motion.div>
        );
      })}
    </div>
  );
}

// ──────────────────────────────────────────────
// Main Component
// ──────────────────────────────────────────────
export default function SecurityAuditPanel() {
  const { connected } = useUnifiedShieldStore();

  return (
    <div className="space-y-4">
      {/* Real-time Status Badges */}
      <motion.div
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
      >
        <SecurityStatusBadges />
      </motion.div>

      {/* Privacy Score + Recommendations */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
        >
          <Card className="shield-surface backdrop-blur-sm overflow-hidden relative">
            <div className="absolute inset-0 bg-gradient-to-br from-emerald-500/5 via-transparent to-cyan-500/5 pointer-events-none" />
            <CardHeader className="pb-2 relative">
              <CardTitle className="text-slate-200 text-base flex items-center gap-2">
                <Shield className="w-5 h-5 text-emerald-400" />
                امتیاز حریم خصوصی
              </CardTitle>
              <CardDescription className="text-slate-500 text-xs">
                ارزیابی جامع امنیت اتصال شما
              </CardDescription>
            </CardHeader>
            <CardContent className="relative">
              <PrivacyScoreGauge />
            </CardContent>
          </Card>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
        >
          <Card className="shield-surface backdrop-blur-sm h-full">
            <CardHeader className="pb-2">
              <CardTitle className="text-slate-200 text-base flex items-center gap-2">
                <Crosshair className="w-5 h-5 text-amber-400" />
                پیشنهادات امنیتی
              </CardTitle>
              <CardDescription className="text-slate-500 text-xs">
                موارد قابل بهبود برای افزایش امنیت
              </CardDescription>
            </CardHeader>
            <CardContent>
              <SecurityRecommendations />
            </CardContent>
          </Card>
        </motion.div>
      </div>

      {/* Security Cards Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <DNSLeakTestCard />
        <WebRTCLeakCard />
        <IPv6LeakCard />
      </div>

      {/* Kill Switch + Encryption */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <KillSwitchStatusCard />
        <EncryptionStrengthCard />
      </div>
    </div>
  );
}
