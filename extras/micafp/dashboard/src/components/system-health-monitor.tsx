'use client';

import React, { useEffect } from 'react';
import { motion } from 'framer-motion';
import {
  Cpu, MemoryStick, ArrowDownCircle, ArrowUpCircle, Timer,
  Gauge, Thermometer, Battery, RefreshCw, Wifi, Layers
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

function MeterBar({
  label, icon, value, max, unit, color, hint,
}: {
  label: string; icon: React.ReactNode; value: number; max: number; unit: string; color: string; hint: string;
}) {
  const pct = Math.min(100, (value / max) * 100);
  return (
    <div className="shield-tile rounded-xl px-3 py-2.5">
      <div className="flex items-center justify-between mb-1.5">
        <span className="text-[11px] text-slate-400 flex items-center gap-1.5">
          {icon}
          {label}
        </span>
        <span className={`text-xs font-bold tabular ${color}`}>
          {toPersianNum(value)} <span className="text-[9px] text-slate-500">{unit}</span>
        </span>
      </div>
      <div className="h-1.5 rounded-full bg-white/[0.05] overflow-hidden">
        <motion.div
          className={`h-full rounded-full ${color === 'text-emerald-400' ? 'bg-gradient-to-l from-emerald-500 to-teal-400' : color === 'text-cyan-400' ? 'bg-gradient-to-l from-cyan-500 to-sky-400' : color === 'text-amber-400' ? 'bg-gradient-to-l from-amber-500 to-orange-400' : color === 'text-rose-400' ? 'bg-gradient-to-l from-rose-500 to-red-400' : 'bg-gradient-to-l from-violet-500 to-indigo-400'}`}
          animate={{ width: `${pct}%` }}
          transition={{ duration: 0.8, ease: 'easeOut' }}
        />
      </div>
      <p className="text-[9px] text-slate-600 mt-1">{hint}</p>
    </div>
  );
}

export function SystemHealthMonitor() {
  const { systemHealth, refreshSystemHealth, connected } = useUnifiedShieldStore();

  useEffect(() => {
    refreshSystemHealth();
    const t = setInterval(() => refreshSystemHealth(), 2500);
    return () => clearInterval(t);
  }, [refreshSystemHealth]);

  const tempColor = systemHealth.tempC > 60 ? 'text-rose-400' : systemHealth.tempC > 48 ? 'text-amber-400' : 'text-emerald-400';
  const batteryColor = systemHealth.battery > 20 ? 'text-emerald-400' : 'text-rose-400';

  return (
    <Card className="shield-surface border-0">
      <CardHeader className="pb-2">
        <CardTitle className="text-slate-200 text-base flex items-center gap-2">
          <ActivityIcon />
          مانیتور سلامت سیستم
          <Badge variant="outline" className="text-[10px] border-cyan-500/30 text-cyan-400">زنده</Badge>
          <motion.span
            className="w-2 h-2 rounded-full bg-cyan-400"
            animate={{ opacity: [1, 0.3, 1] }}
            transition={{ duration: 1.4, repeat: Infinity }}
          />
        </CardTitle>
        <CardDescription className="text-slate-500 text-xs">
          تلهمتری بلادرنگ منابع دستگاه — تجدید هر ۲.۵ ثانیه
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
          <MeterBar label="پردازنده (CPU)" icon={<Cpu className="w-3.5 h-3.5 text-emerald-400" />} value={systemHealth.cpu} max={100} unit="٪" color="text-emerald-400" hint="بار پردازش هسته رمزنگاری و هوش مصنوعی" />
          <MeterBar label="حافظه (RAM)" icon={<MemoryStick className="w-3.5 h-3.5 text-violet-400" />} value={systemHealth.ram} max={100} unit="٪" color="text-violet-400" hint="حافظه کش مسیرهای فعال" />
          <MeterBar label="دانلود" icon={<ArrowDownCircle className="w-3.5 h-3.5 text-cyan-400" />} value={systemHealth.netInMbps} max={200} unit="Mb/s" color="text-cyan-400" hint="پهنای باند ورودی تونل" />
          <MeterBar label="آپلود" icon={<ArrowUpCircle className="w-3.5 h-3.5 text-amber-400" />} value={systemHealth.netOutMbps} max={100} unit="Mb/s" color="text-amber-400" hint="پهنای باند خروجی تونل" />
        </div>

        <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500 flex items-center gap-1"><Timer className="w-3 h-3 text-cyan-400" />تأخیر DNS</p>
            <p className="text-base font-bold text-cyan-400 tabular">{toPersianNum(systemHealth.dnsLatencyMs)} <span className="text-[9px] text-slate-500">ms</span></p>
          </div>
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500 flex items-center gap-1"><Gauge className="w-3 h-3 text-indigo-400" />فرایندها</p>
            <p className="text-base font-bold text-indigo-400 tabular">{toPersianNum(systemHealth.processCount)}</p>
          </div>
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500 flex items-center gap-1"><Thermometer className="w-3 h-3 text-rose-400" />دما</p>
            <p className={`text-base font-bold tabular ${tempColor}`}>{toPersianNum(systemHealth.tempC)}°</p>
          </div>
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500 flex items-center gap-1"><Battery className="w-3 h-3 text-emerald-400" />باتری</p>
            <p className={`text-base font-bold tabular ${batteryColor}`}>
              {toPersianNum(systemHealth.battery)}٪ {systemHealth.charging && <span className="text-[9px] text-emerald-500">(شارژ)</span>}
            </p>
          </div>
        </div>

        <div className="flex items-center justify-between text-[10px] text-slate-500 shield-tile rounded-lg px-3 py-2">
          <div className="flex items-center gap-2">
            <Wifi className="w-3.5 h-3.5 text-emerald-400" />
            وضعیت تونل: <span className={connected ? 'text-emerald-400 font-bold' : 'text-rose-400 font-bold'}>{connected ? 'فعال' : 'قطع'}</span>
            <span className="mx-1 text-slate-700">•</span>
            <Layers className="w-3 h-3 text-cyan-400" />
            لایه: ۳ + ۴ + ۷
          </div>
          <span className="flex items-center gap-1">
            <RefreshCw className="w-3 h-3" />
            آخرین بهروزرسانی: {new Date(systemHealth.lastUpdateTs).toLocaleTimeString('fa-IR')}
          </span>
        </div>
      </CardContent>
    </Card>
  );
}

function ActivityIcon() {
  return <Cpu className="w-5 h-5 text-cyan-400" />;
}
