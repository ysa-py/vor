'use client';

import React, { useState, useMemo, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Globe, MapPin, Server, Zap, Activity, ArrowRightLeft,
  Signal, TrendingDown, TrendingUp, CheckCircle2, XCircle,
  AlertTriangle, Wifi, ChevronDown, BarChart3, Layers,
  Crosshair, Shield, Radio, Timer,
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import {
  BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip,
  ResponsiveContainer, Cell,
} from 'recharts';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';
import { COUNTRY_SERVERS, type CoreAdapter } from '@/lib/unified-shield-types';
import { GlobalLatencyForceGraph } from '@/components/global-latency-force-graph';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

// Country flag emojis mapping
const COUNTRY_FLAGS: Record<string, string> = {
  DE: '🇩🇪', NL: '🇳🇱', FI: '🇫🇮', SE: '🇸🇪', FR: '🇫🇷',
  US: '🇺🇸', CA: '🇨🇦', GB: '🇬🇧', JP: '🇯🇵', KR: '🇰🇷',
  SG: '🇸🇬', AU: '🇦🇺', BR: '🇧🇷', IN: '🇮🇳', TR: '🇹🇷',
  AE: '🇦🇪', CH: '🇨🇭', NO: '🇳🇴', PL: '🇵🇱', ES: '🇪🇸',
};

// Server data with generated latency and load
interface ServerInfo {
  code: string;
  name: string;
  nameFa: string;
  servers: number;
  latency: number;
  load: number;
  health: 'green' | 'yellow' | 'red';
  active: boolean;
}

function generateServerData(connected: boolean): ServerInfo[] {
  return COUNTRY_SERVERS.map((c, i) => {
    const baseLatency = 30 + Math.abs(i - 2) * 18 + Math.random() * 40;
    const load = Math.round(20 + Math.random() * 70);
    const latency = Math.round(baseLatency);
    const health: 'green' | 'yellow' | 'red' = latency < 80 && load < 60 ? 'green' : latency < 140 && load < 80 ? 'yellow' : 'red';
    return {
      code: c.code,
      name: c.name,
      nameFa: c.nameFa,
      servers: c.servers,
      latency,
      load,
      health,
      active: connected && i === 0,
    };
  });
}

// ──────────────────────────────────────────────
// World Map Grid Visualization
// ──────────────────────────────────────────────
function WorldMapGrid({ servers }: { servers: ServerInfo[] }) {
  // Simplified grid positions for continents
  const gridPositions: Record<string, { row: number; col: number }> = {
    DE: { row: 2, col: 5 }, NL: { row: 2, col: 5 }, FI: { row: 1, col: 6 },
    SE: { row: 1, col: 6 }, FR: { row: 3, col: 4 }, GB: { row: 2, col: 4 },
    CH: { row: 3, col: 5 }, NO: { row: 1, col: 5 }, PL: { row: 2, col: 6 },
    ES: { row: 3, col: 4 },
    US: { row: 2, col: 1 }, CA: { row: 1, col: 1 }, BR: { row: 5, col: 2 },
    JP: { row: 2, col: 9 }, KR: { row: 2, col: 9 }, SG: { row: 5, col: 8 },
    AU: { row: 7, col: 9 }, IN: { row: 4, col: 7 }, TR: { row: 3, col: 6 },
    AE: { row: 4, col: 6 },
  };

  const rows = 9;
  const cols = 11;

  // Group servers by grid position
  const gridMap = useMemo(() => {
    const map: Record<string, ServerInfo[]> = {};
    servers.forEach(s => {
      const pos = gridPositions[s.code];
      if (pos) {
        const key = `${pos.row}-${pos.col}`;
        if (!map[key]) map[key] = [];
        map[key].push(s);
      }
    });
    return map;
  }, [servers]);

  const healthColor = (h: 'green' | 'yellow' | 'red') => {
    switch (h) {
      case 'green': return '#10b981';
      case 'yellow': return '#eab308';
      case 'red': return '#ef4444';
    }
  };

  return (
    <div className="relative">
      <div
        className="grid gap-1 p-4"
        style={{ gridTemplateColumns: `repeat(${cols}, 1fr)`, gridTemplateRows: `repeat(${rows}, 1fr)` }}
      >
        {Array.from({ length: rows * cols }).map((_, idx) => {
          const row = Math.floor(idx / cols) + 1;
          const col = (idx % cols) + 1;
          const key = `${row}-${col}`;
          const cellServers = gridMap[key];

          return (
            <div
              key={idx}
              className="aspect-square rounded-sm flex items-center justify-center relative"
              style={{ backgroundColor: cellServers ? 'rgba(6,182,212,0.05)' : 'rgba(30,41,59,0.3)' }}
            >
              {cellServers && (
                <motion.div
                  initial={{ scale: 0, opacity: 0 }}
                  animate={{ scale: 1, opacity: 1 }}
                  transition={{ delay: (idx % 9) * 0.05, type: 'spring', stiffness: 300 }}
                  className="relative"
                >
                  <div
                    className="w-6 h-6 sm:w-8 sm:h-8 rounded-full flex items-center justify-center text-xs"
                    style={{
                      backgroundColor: `${healthColor(cellServers[0].health)}20`,
                      border: `1.5px solid ${healthColor(cellServers[0].health)}60`,
                      boxShadow: `0 0 12px ${healthColor(cellServers[0].health)}30`,
                    }}
                  >
                    <span className="text-sm">{COUNTRY_FLAGS[cellServers[0].code]}</span>
                  </div>
                  {cellServers[0].active && (
                    <motion.div
                      className="absolute inset-0 rounded-full border-2 border-cyan-400/50"
                      animate={{ scale: [1, 1.6, 1], opacity: [0.6, 0, 0.6] }}
                      transition={{ duration: 2, repeat: Infinity }}
                    />
                  )}
                  <div className="absolute -bottom-4 text-[8px] text-slate-500 whitespace-nowrap">
                    {toPersianNum(cellServers.reduce((sum, s) => sum + s.servers, 0))}
                  </div>
                </motion.div>
              )}
            </div>
          );
        })}
      </div>
      {/* Iran marker */}
      <div className="absolute top-[38%] right-[52%] transform -translate-y-1/2 translate-x-1/2">
        <motion.div
          animate={{ scale: [1, 1.3, 1], opacity: [1, 0.5, 1] }}
          transition={{ duration: 2, repeat: Infinity }}
          className="w-4 h-4 rounded-full bg-red-500 shadow-lg shadow-red-500/50"
        />
        <span className="absolute -bottom-5 right-1/2 transform translate-x-1/2 text-[9px] text-red-400 whitespace-nowrap font-bold">ایران</span>
      </div>
    </div>
  );
}

// ──────────────────────────────────────────────
// Server Country List
// ──────────────────────────────────────────────
function ServerCountryList({ servers }: { servers: ServerInfo[] }) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const { connected } = useUnifiedShieldStore();

  const sortedServers = useMemo(() =>
    [...servers].sort((a, b) => a.latency - b.latency),
    [servers]
  );

  const healthIcon = (h: 'green' | 'yellow' | 'red') => {
    switch (h) {
      case 'green': return <CheckCircle2 className="w-4 h-4 text-emerald-400" />;
      case 'yellow': return <AlertTriangle className="w-4 h-4 text-yellow-400" />;
      case 'red': return <XCircle className="w-4 h-4 text-red-400" />;
    }
  };

  const healthLabel = (h: 'green' | 'yellow' | 'red') => {
    switch (h) {
      case 'green': return 'سالم';
      case 'yellow': return 'متوسط';
      case 'red': return 'ضعیف';
    }
  };

  return (
    <div className="space-y-2 max-h-96 overflow-y-auto custom-scrollbar">
      {sortedServers.map((server, i) => (
        <motion.div
          key={server.code}
          initial={{ opacity: 0, x: 20 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ delay: i * 0.03 }}
          onClick={() => setSelectedId(selectedId === server.code ? null : server.code)}
          className={`rounded-xl border p-3 cursor-pointer transition-all duration-200 ${
            server.active
              ? 'border-cyan-500/40 bg-cyan-500/5 shadow-lg shadow-cyan-500/10'
              : selectedId === server.code
              ? 'border-white/15 shield-tile'
              : 'border-white/10 bg-white/[0.03] hover:bg-slate-800/50'
          }`}
        >
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <span className="text-xl">{COUNTRY_FLAGS[server.code]}</span>
              <div>
                <div className="flex items-center gap-2">
                  <span className="text-sm text-slate-200 font-medium">{server.nameFa}</span>
                  {server.active && (
                    <Badge className="bg-cyan-500/20 text-cyan-400 border-cyan-500/30 text-[9px]">
                      فعال
                    </Badge>
                  )}
                  {i === 0 && !server.active && (
                    <Badge className="bg-emerald-500/10 text-emerald-400 border-emerald-500/30 text-[9px]">
                      ⚡ پیشنهادی
                    </Badge>
                  )}
                </div>
                <p className="text-[10px] text-slate-500">{server.name} — {toPersianNum(server.servers)} سرور</p>
              </div>
            </div>
            <div className="flex items-center gap-3">
              {healthIcon(server.health)}
              <div className="text-left">
                <div className="flex items-center gap-1 text-xs">
                  <Timer className="w-3 h-3 text-slate-500" />
                  <span className={server.latency < 80 ? 'text-emerald-400' : server.latency < 140 ? 'text-yellow-400' : 'text-red-400'}>
                    {toPersianNum(server.latency)} ms
                  </span>
                </div>
              </div>
            </div>
          </div>

          <AnimatePresence>
            {selectedId === server.code && (
              <motion.div
                initial={{ height: 0, opacity: 0 }}
                animate={{ height: 'auto', opacity: 1 }}
                exit={{ height: 0, opacity: 0 }}
                transition={{ duration: 0.2 }}
                className="overflow-hidden"
              >
                <div className="pt-3 mt-3 border-t border-white/10 space-y-2">
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-slate-400">بار سرور</span>
                    <span className={server.load > 80 ? 'text-red-400' : server.load > 50 ? 'text-yellow-400' : 'text-emerald-400'}>
                      {toPersianNum(server.load)}٪
                    </span>
                  </div>
                  <Progress value={server.load} className="h-1.5" />
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-slate-400">وضعیت سلامت</span>
                    <span className={server.health === 'green' ? 'text-emerald-400' : server.health === 'yellow' ? 'text-yellow-400' : 'text-red-400'}>
                      {healthLabel(server.health)}
                    </span>
                  </div>
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-slate-400">تعداد سرور</span>
                    <span className="text-cyan-400">{toPersianNum(server.servers)}</span>
                  </div>
                  {connected && !server.active && (
                    <Button
                      size="sm"
                      className="w-full mt-2 bg-cyan-500/20 text-cyan-400 hover:bg-cyan-500/30 border border-cyan-500/30 text-xs"
                      onClick={(e) => { e.stopPropagation(); }}
                    >
                      <Crosshair className="w-3 h-3 ml-1" />
                      اتصال به {server.nameFa}
                    </Button>
                  )}
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        </motion.div>
      ))}
    </div>
  );
}

// ──────────────────────────────────────────────
// Best Server Recommendation
// ──────────────────────────────────────────────
function BestServerRecommendation({ servers }: { servers: ServerInfo[] }) {
  const { connected } = useUnifiedShieldStore();
  const bestServer = useMemo(() =>
    [...servers].sort((a, b) => {
      const scoreA = (100 - a.latency) * 0.5 + (100 - a.load) * 0.3 + (a.health === 'green' ? 20 : a.health === 'yellow' ? 10 : 0);
      const scoreB = (100 - b.latency) * 0.5 + (100 - b.load) * 0.3 + (b.health === 'green' ? 20 : b.health === 'yellow' ? 10 : 0);
      return scoreB - scoreA;
    })[0],
    [servers]
  );

  if (!bestServer || !connected) return null;

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      className="rounded-xl bg-gradient-to-br from-emerald-500/10 via-cyan-500/5 to-teal-500/10 border border-emerald-500/20 p-4"
    >
      <div className="flex items-center gap-3">
        <div className="w-12 h-12 rounded-xl bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center">
          <Zap className="w-6 h-6 text-emerald-400" />
        </div>
        <div className="flex-1">
          <div className="flex items-center gap-2 mb-1">
            <span className="text-sm font-bold text-emerald-300">بهترین سرور پیشنهادی</span>
            <Badge className="bg-emerald-500/20 text-emerald-400 border-emerald-500/30 text-[9px]">AI</Badge>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xl">{COUNTRY_FLAGS[bestServer.code]}</span>
            <span className="text-slate-200 font-medium">{bestServer.nameFa}</span>
            <span className="text-xs text-slate-500">({bestServer.name})</span>
          </div>
        </div>
        <div className="text-left space-y-1">
          <div className="text-xs text-slate-400">
            تأخیر: <span className="text-emerald-400 font-bold">{toPersianNum(bestServer.latency)} ms</span>
          </div>
          <div className="text-xs text-slate-400">
            بار: <span className="text-cyan-400 font-bold">{toPersianNum(bestServer.load)}٪</span>
          </div>
        </div>
      </div>
    </motion.div>
  );
}

// ──────────────────────────────────────────────
// Latency Comparison Bar Chart
// ──────────────────────────────────────────────
function LatencyComparisonChart({ servers }: { servers: ServerInfo[] }) {
  const chartData = useMemo(() =>
    [...servers]
      .sort((a, b) => a.latency - b.latency)
      .slice(0, 10)
      .map(s => ({
        name: s.nameFa,
        latency: s.latency,
        fill: s.health === 'green' ? '#10b981' : s.health === 'yellow' ? '#eab308' : '#ef4444',
      })),
    [servers]
  );

  return (
    <div className="h-56">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={chartData} layout="vertical">
          <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
          <XAxis type="number" tick={{ fill: '#64748b', fontSize: 10 }} unit=" ms" axisLine={{ stroke: '#334155' }} />
          <YAxis dataKey="name" type="category" tick={{ fill: '#94a3b8', fontSize: 11 }} width={70} axisLine={{ stroke: '#334155' }} />
          <Tooltip contentStyle={{ background: '#0f172a', border: '1px solid #334155', borderRadius: 12 }} />
          <Bar dataKey="latency" radius={[0, 6, 6, 0]} name="تأخیر (ms)" animationDuration={800}>
            {chartData.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={entry.fill} fillOpacity={0.7} />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

// ──────────────────────────────────────────────
// Load Balancing Status
// ──────────────────────────────────────────────
function LoadBalancingStatus({ servers }: { servers: ServerInfo[] }) {
  const { connected, orchestrator, cores } = useUnifiedShieldStore();

  const activeCore = cores.find(c => c.id === orchestrator.activeCoreId);
  const shadowCores = orchestrator.shadowConnections.map(id => cores.find(c => c.id === id)).filter(Boolean) as CoreAdapter[];

  const totalLoad = servers.reduce((sum, s) => sum + s.load, 0) / servers.length;

  return (
    <div className="space-y-3">
      <div className="rounded-xl bg-gradient-to-br from-violet-500/10 via-slate-800/30 to-cyan-500/10 border border-white/10 p-4">
        <div className="flex items-center gap-2 mb-3">
          <Layers className="w-5 h-5 text-violet-400" />
          <span className="text-sm text-slate-200 font-medium">وضعیت تعادل بار</span>
        </div>
        <div className="grid grid-cols-3 gap-3 text-xs mb-3">
          <div className="bg-slate-800/50 rounded-lg p-2 text-center">
            <span className="text-slate-400 block">میانگین بار</span>
            <p className={`font-bold ${totalLoad > 70 ? 'text-red-400' : totalLoad > 50 ? 'text-yellow-400' : 'text-emerald-400'}`}>
              {toPersianNum(Math.round(totalLoad))}٪
            </p>
          </div>
          <div className="bg-slate-800/50 rounded-lg p-2 text-center">
            <span className="text-slate-400 block">سرورهای سالم</span>
            <p className="text-emerald-400 font-bold">
              {toPersianNum(servers.filter(s => s.health === 'green').length)}/{toPersianNum(servers.length)}
            </p>
          </div>
          <div className="bg-slate-800/50 rounded-lg p-2 text-center">
            <span className="text-slate-400 block">هسته فعال</span>
            <p className="text-cyan-400 font-bold">
              {activeCore ? activeCore.nameFa : '—'}
            </p>
          </div>
        </div>
        {connected && (
          <div className="space-y-2">
            <p className="text-[10px] text-slate-500">اتصالات سایه (تعویض فوری):</p>
            <div className="flex gap-2 flex-wrap">
              {shadowCores.map(core => (
                <div key={core.id} className="flex items-center gap-1.5 bg-teal-500/10 border border-teal-500/20 rounded-lg px-2.5 py-1.5">
                  <span className="text-sm">{core.icon}</span>
                  <span className="text-xs text-teal-300">{core.nameFa}</span>
                  <motion.div
                    className="w-1.5 h-1.5 rounded-full bg-teal-400"
                    animate={{ opacity: [1, 0.3, 1] }}
                    transition={{ duration: 2, repeat: Infinity }}
                  />
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// ──────────────────────────────────────────────
// Main Component
// ──────────────────────────────────────────────
export default function GeoRouterPanel() {
  const { connected } = useUnifiedShieldStore();
  const [servers, setServers] = useState<ServerInfo[]>(() => generateServerData(connected));
  const [mapMode, setMapMode] = useState<'force-graph' | 'grid'>('force-graph');

  useEffect(() => {
    setServers(generateServerData(connected));
  }, [connected]);

  return (
    <div className="space-y-4">
      {/* Best Server Recommendation */}
      <BestServerRecommendation servers={servers} />

      {/* Map View Mode Switcher */}
      <div className="flex items-center justify-between bg-[#0a0f1c]/70 p-2 rounded-xl border border-white/10">
        <span className="text-xs font-semibold text-slate-300 pr-2">حالت نمایش نقشه و تأخیر سرورها:</span>
        <div className="flex gap-1">
          <Button
            size="sm"
            variant={mapMode === 'force-graph' ? 'default' : 'ghost'}
            onClick={() => setMapMode('force-graph')}
            className={`text-xs h-7 px-3 ${mapMode === 'force-graph' ? 'bg-cyan-500 hover:bg-cyan-600 text-black font-bold' : 'text-slate-400'}`}
          >
            <Activity className="w-3.5 h-3.5 ml-1" />
            گراف نیرو-محور تأخیر (Force Graph)
          </Button>
          <Button
            size="sm"
            variant={mapMode === 'grid' ? 'default' : 'ghost'}
            onClick={() => setMapMode('grid')}
            className={`text-xs h-7 px-3 ${mapMode === 'grid' ? 'bg-cyan-500 hover:bg-cyan-600 text-black font-bold' : 'text-slate-400'}`}
          >
            <Globe className="w-3.5 h-3.5 ml-1" />
            شبکه جغرافیایی (World Grid)
          </Button>
        </div>
      </div>

      {/* World Map or Force Graph */}
      {mapMode === 'force-graph' ? (
        <motion.div
          key="force-graph"
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
        >
          <GlobalLatencyForceGraph />
        </motion.div>
      ) : (
        <motion.div
          key="grid-map"
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
        >
          <Card className="shield-surface backdrop-blur-sm overflow-hidden relative">
            <div className="absolute inset-0 bg-gradient-to-br from-cyan-500/3 via-transparent to-teal-500/3 pointer-events-none" />
            <CardHeader className="pb-2 relative">
              <CardTitle className="text-slate-200 text-base flex items-center gap-2">
                <Globe className="w-5 h-5 text-cyan-400" />
                نقشه سرورها — {toPersianNum(COUNTRY_SERVERS.length)} کشور
              </CardTitle>
              <CardDescription className="text-slate-500 text-xs">
                موقعیت سرورها با نشانگر سلامت — سبز: سالم، زرد: متوسط، قرمز: ضعیف
              </CardDescription>
            </CardHeader>
            <CardContent className="relative">
              <WorldMapGrid servers={servers} />
              <div className="flex justify-center gap-6 mt-2 text-xs">
                <div className="flex items-center gap-1.5">
                  <div className="w-3 h-3 rounded-full bg-emerald-500/60" />
                  <span className="text-slate-400">سالم</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <div className="w-3 h-3 rounded-full bg-yellow-500/60" />
                  <span className="text-slate-400">متوسط</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <div className="w-3 h-3 rounded-full bg-red-500/60" />
                  <span className="text-slate-400">ضعیف</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <div className="w-3 h-3 rounded-full bg-red-500 shadow-lg shadow-red-500/50" />
                  <span className="text-slate-400">ایران (مبدأ)</span>
                </div>
              </div>
            </CardContent>
          </Card>
        </motion.div>
      )}

      {/* Server List + Latency Chart */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
        >
          <Card className="shield-surface backdrop-blur-sm h-full">
            <CardHeader className="pb-2">
              <CardTitle className="text-slate-200 text-base flex items-center gap-2">
                <Server className="w-5 h-5 text-emerald-400" />
                لیست سرورها
              </CardTitle>
              <CardDescription className="text-slate-500 text-xs">
                مرتب‌سازی بر اساس تأخیر — بهترین سرور در بالا
              </CardDescription>
            </CardHeader>
            <CardContent>
              <ServerCountryList servers={servers} />
            </CardContent>
          </Card>
        </motion.div>

        <div className="space-y-4">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.2 }}
          >
            <Card className="shield-surface backdrop-blur-sm overflow-hidden relative">
              <div className="absolute inset-0 bg-gradient-to-br from-emerald-500/3 via-transparent to-amber-500/3 pointer-events-none" />
              <CardHeader className="pb-2 relative">
                <CardTitle className="text-slate-200 text-base flex items-center gap-2">
                  <BarChart3 className="w-5 h-5 text-amber-400" />
                  مقایسه تأخیر سرورها
                </CardTitle>
              </CardHeader>
              <CardContent className="relative">
                <LatencyComparisonChart servers={servers} />
              </CardContent>
            </Card>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.3 }}
          >
            <LoadBalancingStatus servers={servers} />
          </motion.div>
        </div>
      </div>
    </div>
  );
}
