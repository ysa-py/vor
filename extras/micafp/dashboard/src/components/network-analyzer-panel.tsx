'use client';

import React, { useEffect, useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Activity, ArrowUpCircle, ArrowDownCircle, Gauge, Wifi, WifiOff,
  BarChart3, PieChart as PieChartIcon, TrendingUp, TrendingDown,
  Signal, Zap, Network, Server, Timer, Eye, Radio,
  Layers, Hash, Send, Download, RotateCcw,
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Button } from '@/components/ui/button';
import {
  AreaChart, Area, BarChart, Bar, PieChart, Pie, Cell,
  XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend,
} from 'recharts';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';
import { PROTOCOL_LABELS, type ProtocolType } from '@/lib/unified-shield-types';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

// ──────────────────────────────────────────────
// Bandwidth Chart with Gradient
// ──────────────────────────────────────────────
function BandwidthChart() {
  const { connected, networkStats } = useUnifiedShieldStore();
  const history = networkStats.history.map(h => ({
    time: new Date(h.ts).toLocaleTimeString('fa-IR', { hour: '2-digit', minute: '2-digit' }),
    upload: Math.max(0, Math.round(h.uploadMbps * 10) / 10),
    download: Math.max(0, Math.round(h.downloadMbps * 10) / 10),
  })).slice(-24);

  if (!connected) {
    return (
      <div className="h-64 flex items-center justify-center">
        <div className="text-center">
          <WifiOff className="w-12 h-12 mx-auto mb-3 text-slate-600 opacity-50" />
          <p className="text-slate-500">ابتدا متصل شوید</p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-64">
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={history}>
          <defs>
            <linearGradient id="netDlGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#06b6d4" stopOpacity={0.4} />
              <stop offset="50%" stopColor="#06b6d4" stopOpacity={0.15} />
              <stop offset="95%" stopColor="#06b6d4" stopOpacity={0} />
            </linearGradient>
            <linearGradient id="netUlGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#10b981" stopOpacity={0.4} />
              <stop offset="50%" stopColor="#10b981" stopOpacity={0.15} />
              <stop offset="95%" stopColor="#10b981" stopOpacity={0} />
            </linearGradient>
          </defs>
          <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
          <XAxis dataKey="time" tick={{ fill: '#64748b', fontSize: 10 }} axisLine={{ stroke: '#334155' }} />
          <YAxis tick={{ fill: '#64748b', fontSize: 10 }} axisLine={{ stroke: '#334155' }} unit=" Mb" />
          <Tooltip
            contentStyle={{ background: '#0f172a', border: '1px solid #334155', borderRadius: 12, boxShadow: '0 8px 32px rgba(0,0,0,0.4)' }}
            labelStyle={{ color: '#e2e8f0' }}
            itemStyle={{ color: '#94a3b8' }}
          />
          <Area type="monotone" dataKey="download" stroke="#06b6d4" fill="url(#netDlGrad)" strokeWidth={2.5} name="دانلود" dot={false} />
          <Area type="monotone" dataKey="upload" stroke="#10b981" fill="url(#netUlGrad)" strokeWidth={2.5} name="آپلود" dot={false} />
          <Legend wrapperStyle={{ color: '#94a3b8', fontSize: 12 }} />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}

// ──────────────────────────────────────────────
// Connection Quality Gauge (Circular)
// ──────────────────────────────────────────────
function ConnectionQualityGauge() {
  const { cores, orchestrator, connected } = useUnifiedShieldStore();
  const activeCore = cores.find(c => c.id === orchestrator.activeCoreId);

  const quality = useMemo(() => {
    if (!connected || !activeCore) return 0;
    const latencyScore = Math.max(0, 100 - activeCore.health.latency);
    const lossScore = Math.max(0, 100 - activeCore.health.packetLoss * 20);
    const dnsScore = activeCore.health.dnsLeak ? 0 : 100;
    const dpiScore = Math.max(0, 100 - activeCore.health.dpiExposure * 2);
    return Math.round(latencyScore * 0.35 + lossScore * 0.25 + dnsScore * 0.2 + dpiScore * 0.2);
  }, [connected, activeCore]);

  const radius = 70;
  const circumference = 2 * Math.PI * radius;
  const strokeDashoffset = circumference - (quality / 100) * circumference;

  const getColor = (q: number) => {
    if (q >= 80) return '#10b981';
    if (q >= 60) return '#06b6d4';
    if (q >= 40) return '#eab308';
    if (q >= 20) return '#f97316';
    return '#ef4444';
  };

  const getLabel = (q: number) => {
    if (q >= 80) return 'عالی';
    if (q >= 60) return 'خوب';
    if (q >= 40) return 'متوسط';
    if (q >= 20) return 'ضعیف';
    return 'بحرانی';
  };

  const color = getColor(quality);

  return (
    <div className="flex flex-col items-center">
      <div className="relative">
        <svg width="180" height="180" className="transform -rotate-90">
          <circle cx="90" cy="90" r={radius} fill="none" stroke="#1e293b" strokeWidth="10" />
          <motion.circle
            cx="90" cy="90" r={radius} fill="none"
            stroke={color} strokeWidth="10" strokeLinecap="round"
            strokeDasharray={circumference}
            initial={{ strokeDashoffset: circumference }}
            animate={{ strokeDashoffset }}
            transition={{ duration: 1.2, ease: 'easeOut' }}
            style={{ filter: `drop-shadow(0 0 8px ${color}40)` }}
          />
        </svg>
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          <motion.span
            className="text-3xl font-bold"
            style={{ color }}
            key={quality}
            initial={{ scale: 0.5, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            transition={{ type: 'spring', stiffness: 300, damping: 20 }}
          >
            {toPersianNum(quality)}
          </motion.span>
          <span className="text-xs text-slate-400 mt-0.5">از ۱۰۰</span>
        </div>
      </div>
      <Badge
        className="mt-3 text-xs border"
        style={{ borderColor: `${color}40`, color, backgroundColor: `${color}10` }}
      >
        {connected ? getLabel(quality) : 'قطع'}
      </Badge>
    </div>
  );
}

// ──────────────────────────────────────────────
// Network Stability Index
// ──────────────────────────────────────────────
function NetworkStabilityIndex() {
  const { cores, orchestrator, connected } = useUnifiedShieldStore();
  const activeCore = cores.find(c => c.id === orchestrator.activeCoreId);

  const stability = useMemo(() => {
    if (!connected || !activeCore) return 0;
    const uptimeFactor = Math.min(1, activeCore.health.uptime / 86400);
    const lossFactor = Math.max(0, 1 - activeCore.health.packetLoss / 10);
    const blockFactor = activeCore.health.blocked ? 0 : 1;
    const switchFactor = Math.max(0.5, orchestrator.successfulSwitches / Math.max(1, orchestrator.totalSwitches));
    return Math.round((uptimeFactor * 0.3 + lossFactor * 0.3 + blockFactor * 0.25 + switchFactor * 0.15) * 100);
  }, [connected, activeCore, orchestrator]);

  const stabilityColor = stability >= 80 ? 'text-emerald-400' : stability >= 50 ? 'text-yellow-400' : 'text-red-400';
  const stabilityBg = stability >= 80 ? 'from-emerald-500/20 to-emerald-500/5' : stability >= 50 ? 'from-yellow-500/20 to-yellow-500/5' : 'from-red-500/20 to-red-500/5';

  return (
    <div className={`rounded-xl bg-gradient-to-br ${stabilityBg} border border-white/10 p-4`}>
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Signal className="w-5 h-5 text-cyan-400" />
          <span className="text-sm text-slate-300">شاخص پایداری شبکه</span>
        </div>
        <span className={`text-2xl font-bold ${stabilityColor}`}>{toPersianNum(stability)}٪</span>
      </div>
      <Progress value={stability} className="h-2.5 mb-3" />
      <div className="grid grid-cols-2 gap-2 text-xs">
        <div className="bg-slate-800/50 rounded-lg p-2 flex items-center gap-1.5">
          <Timer className="w-3 h-3 text-slate-500" />
          <span className="text-slate-400">آپتایم:</span>
          <span className="text-emerald-400 font-medium">
            {activeCore ? toPersianNum(Math.floor(activeCore.health.uptime / 60)) : '۰'} دقیقه
          </span>
        </div>
        <div className="bg-slate-800/50 rounded-lg p-2 flex items-center gap-1.5">
          <Layers className="w-3 h-3 text-slate-500" />
          <span className="text-slate-400">اتصالات سایه:</span>
          <span className="text-cyan-400 font-medium">{toPersianNum(orchestrator.shadowConnections.length)}</span>
        </div>
      </div>
    </div>
  );
}

// ──────────────────────────────────────────────
// Traffic Breakdown Pie Chart
// ──────────────────────────────────────────────
function TrafficBreakdownPie() {
  const { cores, orchestrator, connected } = useUnifiedShieldStore();

  const protocolData = useMemo(() => {
    if (!connected) return [];
    const protocolMap: Record<string, number> = {};
    cores.forEach(core => {
      if (core.status === 'connected' || core.status === 'standby') {
        core.capabilities.forEach(cap => {
          const label = PROTOCOL_LABELS[cap as ProtocolType];
          if (label) {
            const key = label.nameFa;
            protocolMap[key] = (protocolMap[key] || 0) + (core.id === orchestrator.activeCoreId ? 3 : 1);
          }
        });
      }
    });
    const colors = ['#06b6d4', '#10b981', '#8b5cf6', '#f59e0b', '#ef4444', '#ec4899', '#14b8a6', '#f97316'];
    return Object.entries(protocolMap)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 7)
      .map(([name, value], i) => ({ name, value, fill: colors[i % colors.length] }));
  }, [cores, orchestrator, connected]);

  if (!connected || protocolData.length === 0) {
    return (
      <div className="h-48 flex items-center justify-center">
        <p className="text-slate-500 text-sm">متصل نیست</p>
      </div>
    );
  }

  return (
    <div className="h-56">
      <ResponsiveContainer width="100%" height="100%">
        <PieChart>
          <Pie
            data={protocolData} cx="50%" cy="50%"
            innerRadius={45} outerRadius={80} paddingAngle={3}
            dataKey="value" nameKey="name" animationBegin={0} animationDuration={800}
          >
            {protocolData.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={entry.fill} fillOpacity={0.75} stroke={entry.fill} strokeWidth={1} />
            ))}
          </Pie>
          <Tooltip contentStyle={{ background: '#0f172a', border: '1px solid #334155', borderRadius: 12 }} />
          <Legend wrapperStyle={{ color: '#94a3b8', fontSize: 10 }} />
        </PieChart>
      </ResponsiveContainer>
    </div>
  );
}

// ──────────────────────────────────────────────
// Data Usage Bar Chart
// ──────────────────────────────────────────────
function DataUsageChart() {
  const { stats, connected } = useUnifiedShieldStore();
  const [period, setPeriod] = useState<'daily' | 'weekly' | 'monthly'>('daily');

  const data = useMemo(() => {
    const baseUp = stats.totalDataTransferred.up;
    const baseDown = stats.totalDataTransferred.down;

    if (period === 'daily') {
      return ['شنبه', 'یکشنبه', 'دوشنبه', 'سه‌شنبه', 'چهارشنبه', 'پنجشنبه', 'جمعه'].map((day, i) => ({
        day,
        آپلود: Math.round(baseUp / 7 * (0.5 + Math.random())),
        دانلود: Math.round(baseDown / 7 * (0.5 + Math.random())),
      }));
    }
    if (period === 'weekly') {
      return ['هفته ۱', 'هفته ۲', 'هفته ۳', 'هفته ۴'].map((week) => ({
        day: week,
        آپلود: Math.round(baseUp / 4 * (0.4 + Math.random() * 0.8)),
        دانلود: Math.round(baseDown / 4 * (0.4 + Math.random() * 0.8)),
      }));
    }
    return ['فروردین', 'اردیبهشت', 'خرداد', 'تیر', 'مرداد', 'شهریور'].map((month) => ({
      day: month,
      آپلود: Math.round(baseUp * (0.6 + Math.random())),
      دانلود: Math.round(baseDown * (0.6 + Math.random())),
    }));
  }, [period, stats]);

  return (
    <div>
      <div className="flex gap-2 mb-4">
        {[
          { key: 'daily' as const, label: 'روزانه' },
          { key: 'weekly' as const, label: 'هفتگی' },
          { key: 'monthly' as const, label: 'ماهانه' },
        ].map(p => (
          <button
            key={p.key}
            onClick={() => setPeriod(p.key)}
            className={`rounded-lg border px-3 py-1 text-xs transition-all ${period === p.key ? 'border-cyan-500/50 bg-cyan-500/10 text-cyan-300' : 'border-white/10 bg-white/[0.03] text-slate-400 hover:bg-slate-800/50'}`}
          >
            {p.label}
          </button>
        ))}
      </div>
      <div className="h-48">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
            <XAxis dataKey="day" tick={{ fill: '#64748b', fontSize: 10 }} axisLine={{ stroke: '#334155' }} />
            <YAxis tick={{ fill: '#64748b', fontSize: 10 }} axisLine={{ stroke: '#334155' }} unit=" GB" />
            <Tooltip contentStyle={{ background: '#0f172a', border: '1px solid #334155', borderRadius: 12 }} />
            <Bar dataKey="دانلود" fill="#06b6d4" fillOpacity={0.7} radius={[4, 4, 0, 0]} />
            <Bar dataKey="آپلود" fill="#10b981" fillOpacity={0.7} radius={[4, 4, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

// ──────────────────────────────────────────────
// Packet Statistics
// ──────────────────────────────────────────────
function PacketStats() {
  const { cores, orchestrator, connected, networkStats } = useUnifiedShieldStore();
  const activeCore = cores.find(c => c.id === orchestrator.activeCoreId);

  const packets = useMemo(() => {
    if (!connected || !activeCore) return { sent: 0, received: 0, retransmitted: 0 };
    const bandwidth = activeCore.health.bandwidth;
    const sent = Math.round(bandwidth.up * 125); // approximate packets/sec
    const received = Math.round(bandwidth.down * 125);
    const retransmitted = Math.round(received * activeCore.health.packetLoss / 100);
    return { sent, received, retransmitted };
  }, [connected, activeCore]);

  const statsItems = [
    { icon: Send, label: 'ارسال‌شده', value: packets.sent, color: 'text-emerald-400', bgColor: 'bg-emerald-500/10', unit: 'pkt/s' },
    { icon: Download, label: 'دریافت‌شده', value: packets.received, color: 'text-cyan-400', bgColor: 'bg-cyan-500/10', unit: 'pkt/s' },
    { icon: RotateCcw, label: 'بازارسال‌شده', value: packets.retransmitted, color: 'text-amber-400', bgColor: 'bg-amber-500/10', unit: 'pkt/s' },
  ];

  return (
    <div className="grid grid-cols-3 gap-3">
      {statsItems.map((item, i) => (
        <motion.div
          key={i}
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: i * 0.1 }}
          className={`${item.bgColor} rounded-xl border border-white/10 p-3 text-center`}
        >
          <item.icon className={`w-5 h-5 ${item.color} mx-auto mb-2`} />
          <p className="text-xs text-slate-500 mb-1">{item.label}</p>
          <p className={`text-lg font-bold ${item.color}`}>
            {connected ? toPersianNum(item.value) : '۰'}
          </p>
          <p className="text-[10px] text-slate-600">{item.unit}</p>
        </motion.div>
      ))}
    </div>
  );
}

// ──────────────────────────────────────────────
// Main Component
// ──────────────────────────────────────────────
export default function NetworkAnalyzerPanel() {
  const { connected, stats, networkStats } = useUnifiedShieldStore();

  return (
    <div className="space-y-4">
      {/* Top Row: Bandwidth + Quality Gauge */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="lg:col-span-2"
        >
          <Card className="shield-surface backdrop-blur-sm overflow-hidden relative">
            <div className="absolute inset-0 bg-gradient-to-br from-cyan-500/5 via-transparent to-emerald-500/5 pointer-events-none" />
            <CardHeader className="pb-2 relative">
              <CardTitle className="text-slate-200 text-base flex items-center gap-2">
                <Activity className="w-5 h-5 text-cyan-400" />
                پهنای باند لحظه‌ای
              </CardTitle>
              <CardDescription className="text-slate-500 text-xs">
                نمودار آپلود/دانلود با گرادیان لحظه‌ای
              </CardDescription>
            </CardHeader>
            <CardContent className="relative">
              <div className="flex items-center gap-4 mb-3">
                <div className="flex items-center gap-2 shield-tile rounded-lg px-3 py-1.5">
                  <ArrowUpCircle className="w-4 h-4 text-emerald-400" />
                  <span className="text-xs text-slate-400">آپلود</span>
                  <span className="text-emerald-400 font-bold text-sm">{connected ? (networkStats.uploadMbps !== null ? toPersianNum(Math.round(networkStats.uploadMbps * 10) / 10) : 'نامشخص') : '۰'} Mbps</span>
                </div>
                <div className="flex items-center gap-2 shield-tile rounded-lg px-3 py-1.5">
                  <ArrowDownCircle className="w-4 h-4 text-cyan-400" />
                  <span className="text-xs text-slate-400">دانلود</span>
                  <span className="text-cyan-400 font-bold text-sm">{connected ? (networkStats.downloadMbps !== null ? toPersianNum(Math.round(networkStats.downloadMbps * 10) / 10) : 'نامشخص') : '۰'} Mbps</span>
                </div>
              </div>
              <BandwidthChart />
            </CardContent>
          </Card>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
        >
          <Card className="shield-surface backdrop-blur-sm h-full overflow-hidden relative">
            <div className="absolute inset-0 bg-gradient-to-br from-violet-500/5 via-transparent to-cyan-500/5 pointer-events-none" />
            <CardHeader className="pb-2 relative">
              <CardTitle className="text-slate-200 text-base flex items-center gap-2">
                <Gauge className="w-5 h-5 text-violet-400" />
                کیفیت اتصال
              </CardTitle>
            </CardHeader>
            <CardContent className="relative flex flex-col items-center justify-center">
              <ConnectionQualityGauge />
            </CardContent>
          </Card>
        </motion.div>
      </div>

      {/* Second Row: Stability + Packet Stats */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.2 }}
        >
          <NetworkStabilityIndex />
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.25 }}
        >
          <Card className="shield-surface backdrop-blur-sm h-full">
            <CardHeader className="pb-2">
              <CardTitle className="text-slate-200 text-base flex items-center gap-2">
                <Hash className="w-5 h-5 text-amber-400" />
                آمار بسته‌ها
              </CardTitle>
            </CardHeader>
            <CardContent>
              <PacketStats />
            </CardContent>
          </Card>
        </motion.div>
      </div>

      {/* Third Row: Traffic Pie + Data Usage */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.3 }}
        >
          <Card className="shield-surface backdrop-blur-sm overflow-hidden relative">
            <div className="absolute inset-0 bg-gradient-to-br from-pink-500/5 via-transparent to-violet-500/5 pointer-events-none" />
            <CardHeader className="pb-2 relative">
              <CardTitle className="text-slate-200 text-base flex items-center gap-2">
                <PieChartIcon className="w-5 h-5 text-pink-400" />
                توزیع ترافیک بر اساس پروتکل
              </CardTitle>
            </CardHeader>
            <CardContent className="relative">
              <TrafficBreakdownPie />
            </CardContent>
          </Card>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.35 }}
        >
          <Card className="shield-surface backdrop-blur-sm overflow-hidden relative">
            <div className="absolute inset-0 bg-gradient-to-br from-emerald-500/5 via-transparent to-cyan-500/5 pointer-events-none" />
            <CardHeader className="pb-2 relative">
              <CardTitle className="text-slate-200 text-base flex items-center gap-2">
                <BarChart3 className="w-5 h-5 text-emerald-400" />
                مصرف داده
              </CardTitle>
            </CardHeader>
            <CardContent className="relative">
              <DataUsageChart />
            </CardContent>
          </Card>
        </motion.div>
      </div>
    </div>
  );
}
