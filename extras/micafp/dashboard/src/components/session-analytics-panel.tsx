'use client';

import React, { useMemo } from 'react';
import { motion } from 'framer-motion';
import {
  Activity, Download, Upload, Timer, History, BarChart3,
  TrendingUp, FileDown, Gauge, ArrowRightLeft, ShieldAlert, CheckCircle2
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer
} from 'recharts';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

const REASON_LABEL: Record<string, { labelFa: string; cls: string }> = {
  manual: { labelFa: 'دستی', cls: 'bg-slate-500/15 text-slate-400 border-slate-500/30' },
  dropped: { labelFa: 'قطعی', cls: 'bg-rose-500/15 text-rose-400 border-rose-500/30' },
  switched: { labelFa: 'تعویض', cls: 'bg-amber-500/15 text-amber-400 border-amber-500/30' },
  error: { labelFa: 'خطا', cls: 'bg-orange-500/15 text-orange-400 border-orange-500/30' },
};

export function SessionAnalyticsPanel() {
  const { liveConnection, connected, exportDiagnosticReport } = useUnifiedShieldStore();

  const chartData = useMemo(() => {
    const hist = liveConnection.throughputHistory;
    if (hist.length < 2) {
      // Seed a couple of placeholder points so the chart renders before the first tick
      const now = Date.now();
      return [
        { time: '—', down: 0, up: 0 },
        { time: '—', down: 0, up: 0 },
      ];
    }
    return hist.map((s, i) => ({
      time: `${toPersianNum(i + 1)}s`,
      down: s.downMbps,
      up: s.upMbps,
    }));
  }, [liveConnection.throughputHistory]);

  const avgLatency = useMemo(() => {
    if (liveConnection.latencySamples.length === 0) return 0;
    return Math.round(
      liveConnection.latencySamples.reduce((a, b) => a + b, 0) / liveConnection.latencySamples.length,
    );
  }, [liveConnection.latencySamples]);

  const totalDownGb = liveConnection.dataBytesDown / 1e9;
  const totalUpGb = liveConnection.dataBytesUp / 1e9;
  const protocolList = Object.values(liveConnection.protocolStats).filter(p => p.attempts > 0);

  const downloadReport = () => {
    const report = exportDiagnosticReport();
    const blob = new Blob([report], { type: 'text/plain;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `unifiedshield-diagnostic-${new Date().toISOString().slice(0, 10)}.txt`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  return (
    <Card className="shield-surface border-0">
      <CardHeader className="pb-2">
        <CardTitle className="text-slate-200 text-base flex items-center gap-2">
          <Activity className="w-5 h-5 text-cyan-400" />
          تله‌متری زنده جلسه
          <Badge variant="outline" className="text-[10px] border-cyan-500/30 text-cyan-400">Real-time</Badge>
          {connected && (
            <motion.span className="w-2 h-2 rounded-full bg-emerald-400" animate={{ opacity: [1, 0.3, 1] }} transition={{ duration: 1.4, repeat: Infinity }} />
          )}
        </CardTitle>
        <CardDescription className="text-slate-500 text-xs">
          پهنای باند لحظه‌ای، تأخیر زنده، تاریخچه جلسات و نرخ موفقیت پروتکل‌ها
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Throughput sparkline */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <p className="text-xs text-slate-400 font-semibold flex items-center gap-1.5">
              <TrendingUp className="w-3.5 h-3.5 text-emerald-400" />
              توان عملیاتی لحظه‌ای (Mb/s)
            </p>
            <div className="flex items-center gap-3 text-[10px] text-slate-500">
              <span className="flex items-center gap-1"><span className="w-2 h-2 rounded-full bg-cyan-400" />دانلود</span>
              <span className="flex items-center gap-1"><span className="w-2 h-2 rounded-full bg-violet-400" />آپلود</span>
            </div>
          </div>
          <div className="h-36 shield-inset rounded-xl p-1">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={chartData} margin={{ top: 6, right: 6, left: -22, bottom: 0 }}>
                <defs>
                  <linearGradient id="gradDown" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#22d3ee" stopOpacity={0.5} />
                    <stop offset="100%" stopColor="#22d3ee" stopOpacity={0} />
                  </linearGradient>
                  <linearGradient id="gradUp" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#a78bfa" stopOpacity={0.4} />
                    <stop offset="100%" stopColor="#a78bfa" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(148,163,184,0.08)" />
                <XAxis dataKey="time" tick={{ fill: '#64748b', fontSize: 9 }} axisLine={false} tickLine={false} />
                <YAxis tick={{ fill: '#64748b', fontSize: 9 }} axisLine={false} tickLine={false} />
                <Tooltip contentStyle={{ background: '#0f172a', border: '1px solid rgba(148,163,184,0.2)', borderRadius: 10, fontSize: 11, direction: 'rtl' }} />
                <Area type="monotone" dataKey="down" name="دانلود" stroke="#22d3ee" strokeWidth={2} fill="url(#gradDown)" />
                <Area type="monotone" dataKey="up" name="آپلود" stroke="#a78bfa" strokeWidth={2} fill="url(#gradUp)" />
              </AreaChart>
            </ResponsiveContainer>
          </div>
          <div className="flex items-center justify-between mt-2 text-[10px] text-slate-500">
            <span className="flex items-center gap-1"><Download className="w-3 h-3 text-cyan-400" />مجموع: {toPersianNum(totalDownGb.toFixed(2))} GB</span>
            <span className="flex items-center gap-1"><Upload className="w-3 h-3 text-violet-400" />مجموع: {toPersianNum(totalUpGb.toFixed(2))} GB</span>
          </div>
        </div>

        {/* Latency + live metrics */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500 flex items-center gap-1"><Timer className="w-3 h-3 text-emerald-400" />تأخیر فعلی</p>
            <p className="text-lg font-bold text-emerald-400 tabular">{toPersianNum(liveConnection.currentLatencyMs)} <span className="text-[9px] text-slate-500">ms</span></p>
          </div>
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500 flex items-center gap-1"><Gauge className="w-3 h-3 text-cyan-400" />میانگین تأخیر</p>
            <p className="text-lg font-bold text-cyan-400 tabular">{toPersianNum(avgLatency)} <span className="text-[9px] text-slate-500">ms</span></p>
          </div>
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500 flex items-center gap-1"><ArrowRightLeft className="w-3 h-3 text-violet-400" />تعویض هسته</p>
            <p className="text-lg font-bold text-violet-400 tabular">{toPersianNum(liveConnection.switchesPerformed)}</p>
          </div>
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500 flex items-center gap-1"><ShieldAlert className="w-3 h-3 text-amber-400" />مسدودیت دفع‌شده</p>
            <p className="text-lg font-bold text-amber-400 tabular">{toPersianNum(liveConnection.blockEventsAvoided)}</p>
          </div>
        </div>

        {/* Protocol success rates */}
        <div>
          <p className="text-xs text-slate-400 font-semibold mb-2 flex items-center gap-1.5">
            <BarChart3 className="w-3.5 h-3.5 text-indigo-400" />
            نرخ موفقیت پروتکل‌ها (پنجره متحرک)
          </p>
          {protocolList.length === 0 ? (
            <p className="text-[11px] text-slate-600 shield-tile rounded-lg px-3 py-2.5">
              هنوز آزمون یا تعویض پروتکلی ثبت نشده. با «تست DPI واقعی» یا تعویض هسته، شمارنده‌ها فعال می‌شوند.
            </p>
          ) : (
            <div className="space-y-1.5">
              {protocolList.map(p => (
                <div key={p.id} className="shield-tile rounded-lg px-3 py-2">
                  <div className="flex items-center justify-between text-xs mb-1">
                    <span className="text-slate-300 truncate">{p.nameFa}</span>
                    <span className={`font-bold tabular ${p.rollingRate >= 90 ? 'text-emerald-400' : p.rollingRate >= 70 ? 'text-amber-400' : 'text-rose-400'}`}>
                      {toPersianNum(p.rollingRate)}٪
                      <span className="text-[9px] text-slate-500 mr-1">({toPersianNum(p.successes)}/{toPersianNum(p.attempts)})</span>
                    </span>
                  </div>
                  <div className="h-1.5 rounded-full bg-white/[0.05] overflow-hidden">
                    <motion.div
                      className={`h-full rounded-full ${p.rollingRate >= 90 ? 'bg-gradient-to-l from-emerald-500 to-teal-400' : p.rollingRate >= 70 ? 'bg-gradient-to-l from-amber-500 to-orange-400' : 'bg-gradient-to-l from-rose-500 to-red-400'}`}
                      animate={{ width: `${p.rollingRate}%` }}
                      transition={{ duration: 0.6 }}
                    />
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Session history */}
        <div>
          <p className="text-xs text-slate-400 font-semibold mb-2 flex items-center gap-1.5">
            <History className="w-3.5 h-3.5 text-teal-400" />
            تاریخچه جلسات اتصال
          </p>
          {liveConnection.sessions.length === 0 ? (
            <p className="text-[11px] text-slate-600 shield-tile rounded-lg px-3 py-2.5">
              هنوز جلسه‌ای ثبت نشده. پس از اتصال و قطع، جلسات اینجا ذخیره می‌شوند.
            </p>
          ) : (
            <div className="space-y-1.5 max-h-48 overflow-y-auto custom-scrollbar">
              {liveConnection.sessions.map(s => {
                const reason = REASON_LABEL[s.disconnectReason] ?? REASON_LABEL.manual;
                const d = Math.floor(s.durationSec / 86400);
                const h = Math.floor((s.durationSec % 86400) / 3600);
                const m = Math.floor((s.durationSec % 3600) / 60);
                const durStr = d > 0 ? `${toPersianNum(d)} روز ${toPersianNum(h)} ساعت` : h > 0 ? `${toPersianNum(h)} ساعت ${toPersianNum(m)} دقیقه` : `${toPersianNum(m)} دقیقه`;
                return (
                  <div key={s.id} className="shield-tile rounded-lg px-3 py-2 flex items-center gap-2 text-[11px]">
                    <div className="flex-1 min-w-0">
                      <p className="text-slate-300 truncate">
                        {new Date(s.startTs).toLocaleString('fa-IR', { hour: '2-digit', minute: '2-digit' })} — {durStr}
                      </p>
                      <p className="text-[9px] text-slate-600 truncate font-mono" dir="ltr">
                        {s.protocolId} • {(s.dataDownBytes / 1e9).toFixed(2)} GB ↓ / {(s.dataUpBytes / 1e9).toFixed(2)} GB ↑
                      </p>
                    </div>
                    <Badge className={`text-[9px] border ${reason.cls}`}>{reason.labelFa}</Badge>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Export report */}
        <Button onClick={downloadReport} className="w-full shield-gradient-btn text-white border-0">
          <FileDown className="w-4 h-4" />
          دانلود گزارش تشخیصی (Diagnostic Report)
        </Button>
        <div className="flex items-center gap-2 text-[10px] text-slate-500 shield-tile rounded-lg px-3 py-2">
          <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
          گزارش شامل: زمان اتصال، داده انتقالی، تعویض هسته، مسدودیت‌ها، تأخیر و تاریخچه جلسات
        </div>
      </CardContent>
    </Card>
  );
}
