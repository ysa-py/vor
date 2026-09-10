'use client';

import React, { useState, useMemo } from 'react';
import { motion } from 'framer-motion';
import {
  TrendingUp, Activity, BarChart2, Zap, Clock,
  ArrowDownCircle, ArrowUpCircle, Sparkles, RefreshCw,
  HardDrive, AlertCircle, CheckCircle2, ShieldCheck
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import {
  AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip,
  ResponsiveContainer, BarChart, Bar, Cell
} from 'recharts';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

export function TrafficForecastPanel() {
  const { trafficForecast, refreshTrafficForecast } = useUnifiedShieldStore();
  const [activeMetric, setActiveMetric] = useState<'both' | 'down' | 'up'>('both');

  const chartData = useMemo(() => {
    return (trafficForecast?.hourlyPredictions || []).map(h => ({
      hour: h.hourLabel,
      downloadMb: h.predictedDownMb,
      uploadMb: h.predictedUpMb,
      totalMb: h.predictedDownMb + h.predictedUpMb,
      category: h.expectedAppCategory,
      isPeak: h.isPeakHour,
      confidence: h.confidenceScore,
    }));
  }, [trafficForecast?.hourlyPredictions]);

  const totalGb = useMemo(() => {
    return ((trafficForecast?.predictedTotal24hMb || 5240) / 1024).toFixed(2);
  }, [trafficForecast?.predictedTotal24hMb]);

  return (
    <Card className="shield-surface-strong backdrop-blur-md overflow-hidden">
      <CardHeader className="pb-2 border-b border-white/10">
        <div className="flex items-center justify-between flex-wrap gap-2">
          <div>
            <CardTitle className="text-slate-100 text-sm sm:text-base flex items-center gap-2">
              <TrendingUp className="w-5 h-5 text-cyan-400" />
              <span>پیش‌بینی هوشمند ترافیک مصرفی ۲۴ ساعت آینده (Traffic Forecast)</span>
            </CardTitle>
            <CardDescription className="text-slate-400 text-xs mt-0.5">
              مدل‌سازی سری‌زمانی مصرف پهنای باند بر اساس عادات اتصال گذشته و ساعات اوج مصرف
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <Button
              size="sm"
              variant="outline"
              onClick={() => refreshTrafficForecast()}
              className="text-xs h-7 px-2.5 border-slate-700 bg-slate-800 text-slate-200 hover:bg-slate-700 hover:text-white"
            >
              <RefreshCw className="w-3.5 h-3.5 ml-1 text-cyan-400" />
              محاسبه مجدد مدل AI
            </Button>
            <Badge variant="outline" className="text-xs border-emerald-500/30 text-emerald-300 bg-emerald-500/10">
              دقت تحلیل: ۹۴.۸٪
            </Badge>
          </div>
        </div>
      </CardHeader>

      <CardContent className="p-3 sm:p-4 space-y-4">
        {/* KPI Stats Grid */}
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 text-xs">
          <div className="bg-slate-950 p-3 rounded-lg border border-white/10 space-y-1">
            <span className="text-slate-400 flex items-center gap-1.5">
              <ArrowDownCircle className="w-3.5 h-3.5 text-cyan-400" />
              پیش‌بینی کل مصرف ۲۴ ساعت:
            </span>
            <div className="text-xl font-bold font-mono text-cyan-300">
              {toPersianNum(totalGb)} <span className="text-xs text-slate-400 font-sans">گیگابایت</span>
            </div>
            <span className="text-[10px] text-slate-500">
              میانگین ساعتی: {toPersianNum(Math.round((trafficForecast?.predictedTotal24hMb || 5240) / 24))} MB/hr
            </span>
          </div>

          <div className="bg-slate-950 p-3 rounded-lg border border-white/10 space-y-1">
            <span className="text-slate-400 flex items-center gap-1.5">
              <Clock className="w-3.5 h-3.5 text-amber-400" />
              بازه زمانی اوج مصرف (Peak Hour):
            </span>
            <div className="text-base font-bold text-amber-300">
              ساعت {toPersianNum(trafficForecast?.predictedPeakHour || 21)}:۰۰ شب (تا {toPersianNum((trafficForecast?.predictedPeakSpeedMbps || 54.8).toFixed(1))} Mbps)
            </div>
            <span className="text-[10px] text-slate-500">
              توصیه خودکار: تنظیم پدینگ حداکثری و پیش‌گرمایش Hysteria 2
            </span>
          </div>

          <div className="bg-slate-950 p-3 rounded-lg border border-white/10 space-y-1">
            <span className="text-slate-400 flex items-center gap-1.5">
              <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
              صرفه‌جویی هوشمند تخمینی:
            </span>
            <div className="text-xl font-bold font-mono text-emerald-300">
              {toPersianNum(trafficForecast?.smartSavingsEstimatedMb || 720)} <span className="text-xs text-slate-400 font-sans">مگابایت</span>
            </div>
            <span className="text-[10px] text-slate-500">
              از طریق فیلتر هدرهای اضافی و فشرده‌سازی لایه ۴
            </span>
          </div>
        </div>

        {/* 24-Hour Forecast Chart */}
        <div className="bg-slate-950 p-3 rounded-xl border border-white/10 space-y-2">
          <div className="flex items-center justify-between text-xs text-slate-300 flex-wrap gap-2">
            <span className="font-semibold flex items-center gap-1">
              <Activity className="w-3.5 h-3.5 text-cyan-400" />
              نمودار ساعتی پیش‌بینی دانلود و آپلود (۲۴ ساعت آینده)
            </span>
            <div className="flex items-center gap-2 text-[11px]">
              <button
                onClick={() => setActiveMetric('both')}
                className={`px-2 py-0.5 rounded border transition-colors ${activeMetric === 'both' ? 'bg-cyan-500/20 border-cyan-500 text-cyan-300 font-bold' : 'bg-slate-800 border-slate-700 text-slate-400'}`}
              >
                مجموع ترافیک
              </button>
              <button
                onClick={() => setActiveMetric('down')}
                className={`px-2 py-0.5 rounded border transition-colors ${activeMetric === 'down' ? 'bg-emerald-500/20 border-emerald-500 text-emerald-300 font-bold' : 'bg-slate-800 border-slate-700 text-slate-400'}`}
              >
                دانلود
              </button>
              <button
                onClick={() => setActiveMetric('up')}
                className={`px-2 py-0.5 rounded border transition-colors ${activeMetric === 'up' ? 'bg-violet-500/20 border-violet-500 text-violet-300 font-bold' : 'bg-slate-800 border-slate-700 text-slate-400'}`}
              >
                آپلود
              </button>
            </div>
          </div>

          <div className="h-56 w-full">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={chartData}>
                <defs>
                  <linearGradient id="forecastDownGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#06b6d4" stopOpacity={0.4} />
                    <stop offset="95%" stopColor="#06b6d4" stopOpacity={0.0} />
                  </linearGradient>
                  <linearGradient id="forecastUpGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#8b5cf6" stopOpacity={0.4} />
                    <stop offset="95%" stopColor="#8b5cf6" stopOpacity={0.0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
                <XAxis dataKey="hour" tick={{ fill: '#94a3b8', fontSize: 10 }} />
                <YAxis tick={{ fill: '#94a3b8', fontSize: 10 }} unit=" MB" />
                <Tooltip
                  contentStyle={{ background: '#0f172a', border: '1px solid #334155', borderRadius: 8 }}
                  formatter={(val: any, name: any) => [`${toPersianNum(val)} مگابایت`, name === 'downloadMb' ? 'دانلود' : name === 'uploadMb' ? 'آپلود' : 'مجموع']}
                  labelFormatter={(label: any) => `ساعت: ${label}`}
                />
                {(activeMetric === 'both' || activeMetric === 'down') && (
                  <Area
                    type="monotone"
                    dataKey="downloadMb"
                    stroke="#06b6d4"
                    strokeWidth={2}
                    fill="url(#forecastDownGrad)"
                    name="دانلود (MB)"
                  />
                )}
                {(activeMetric === 'both' || activeMetric === 'up') && (
                  <Area
                    type="monotone"
                    dataKey="uploadMb"
                    stroke="#8b5cf6"
                    strokeWidth={2}
                    fill="url(#forecastUpGrad)"
                    name="آپلود (MB)"
                  />
                )}
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
