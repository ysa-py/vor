'use client';

import React, { useState, useEffect, useMemo, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Globe, Zap, Radio, Signal, ArrowUpRight, CheckCircle2,
  RefreshCw, Layers, ShieldCheck, Activity, MapPin, Server
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

export interface ForceNode {
  id: string;
  name: string;
  nameFa: string;
  type: 'client' | 'relay' | 'server';
  x: number;
  y: number;
  vx: number;
  vy: number;
  latencyMs: number;
  jitterMs: number;
  packetLoss: number;
  flag: string;
  load: number;
  active?: boolean;
}

export interface ForceLink {
  source: string;
  target: string;
  latencyMs: number;
  stability: number; // 0-100
}

const INITIAL_NODES: ForceNode[] = [
  // Client Origin
  { id: 'client-tehran', name: 'Tehran Client', nameFa: 'کلاینت مبدأ (تهران)', type: 'client', x: 200, y: 190, vx: 0, vy: 0, latencyMs: 0, jitterMs: 0, packetLoss: 0, flag: '🇮🇷', load: 15, active: true },
  
  // Relay IXPs
  { id: 'relay-asiatech', name: 'Asiatech IXP Relay', nameFa: 'رله آسیاتک برج میلاد', type: 'relay', x: 320, y: 120, vx: 0, vy: 0, latencyMs: 14, jitterMs: 2, packetLoss: 0, flag: '⚡', load: 45 },
  { id: 'relay-afranet', name: 'Afranet Gateway', nameFa: 'دروازه مرزی افرانت', type: 'relay', x: 330, y: 260, vx: 0, vy: 0, latencyMs: 18, jitterMs: 3, packetLoss: 0.2, flag: '⚡', load: 52 },
  { id: 'relay-tic', name: 'TIC International GW', nameFa: 'دروازه بین‌الملل زیرساخت', type: 'relay', x: 440, y: 190, vx: 0, vy: 0, latencyMs: 26, jitterMs: 4, packetLoss: 0.5, flag: '🛡️', load: 70 },

  // Global Servers
  { id: 'srv-istanbul', name: 'Istanbul Edge', nameFa: 'استانبول (ترکیه)', type: 'server', x: 570, y: 100, vx: 0, vy: 0, latencyMs: 38, jitterMs: 3, packetLoss: 0.1, flag: '🇹🇷', load: 35 },
  { id: 'srv-frankfurt', name: 'Frankfurt Core', nameFa: 'فرانکفورت (آلمان)', type: 'server', x: 680, y: 140, vx: 0, vy: 0, latencyMs: 62, jitterMs: 4, packetLoss: 0.2, flag: '🇩🇪', load: 48 },
  { id: 'srv-amsterdam', name: 'Amsterdam IX', nameFa: 'آمستردام (هلند)', type: 'server', x: 670, y: 220, vx: 0, vy: 0, latencyMs: 68, jitterMs: 5, packetLoss: 0.3, flag: '🇳🇱', load: 55 },
  { id: 'srv-helsinki', name: 'Helsinki Fast', nameFa: 'هلسینکی (فنلاند)', type: 'server', x: 740, y: 80, vx: 0, vy: 0, latencyMs: 74, jitterMs: 6, packetLoss: 0.4, flag: '🇫🇮', load: 28 },
  { id: 'srv-dubai', name: 'Dubai Relay', nameFa: 'دبی (امارات)', type: 'server', x: 580, y: 280, vx: 0, vy: 0, latencyMs: 42, jitterMs: 4, packetLoss: 0.2, flag: '🇦🇪', load: 30 },
  { id: 'srv-tokyo', name: 'Tokyo Global', nameFa: 'توکیو (ژاپن)', type: 'server', x: 770, y: 280, vx: 0, vy: 0, latencyMs: 185, jitterMs: 12, packetLoss: 1.8, flag: '🇯🇵', load: 60 },
  { id: 'srv-singapore', name: 'Singapore Hub', nameFa: 'سنگاپور (SG)', type: 'server', x: 730, y: 340, vx: 0, vy: 0, latencyMs: 145, jitterMs: 9, packetLoss: 1.1, flag: '🇸🇬', load: 40 },
];

const INITIAL_LINKS: ForceLink[] = [
  { source: 'client-tehran', target: 'relay-asiatech', latencyMs: 14, stability: 98 },
  { source: 'client-tehran', target: 'relay-afranet', latencyMs: 18, stability: 95 },
  { source: 'relay-asiatech', target: 'relay-tic', latencyMs: 12, stability: 94 },
  { source: 'relay-afranet', target: 'relay-tic', latencyMs: 10, stability: 92 },
  { source: 'relay-tic', target: 'srv-istanbul', latencyMs: 24, stability: 96 },
  { source: 'relay-tic', target: 'srv-dubai', latencyMs: 22, stability: 97 },
  { source: 'srv-istanbul', target: 'srv-frankfurt', latencyMs: 24, stability: 95 },
  { source: 'srv-istanbul', target: 'srv-amsterdam', latencyMs: 30, stability: 94 },
  { source: 'srv-frankfurt', target: 'srv-helsinki', latencyMs: 16, stability: 96 },
  { source: 'srv-dubai', target: 'srv-singapore', latencyMs: 98, stability: 88 },
  { source: 'srv-singapore', target: 'srv-tokyo', latencyMs: 55, stability: 85 },
];

export function GlobalLatencyForceGraph() {
  const [nodes, setNodes] = useState<ForceNode[]>(INITIAL_NODES);
  const [selectedNode, setSelectedNode] = useState<ForceNode | null>(INITIAL_NODES[5]); // Default: Frankfurt
  const [isSimulating, setIsSimulating] = useState(true);
  const [particleOffset, setParticleOffset] = useState(0);

  // Force-simulation and live latency jitter ticker
  useEffect(() => {
    const timer = setInterval(() => {
      setParticleOffset(prev => (prev + 1) % 100);
      
      // Slight natural latency oscillation
      setNodes(prev => prev.map(n => {
        if (n.type === 'client') return n;
        const delta = (Math.random() - 0.5) * 2;
        const newLat = Math.max(10, Math.round(n.latencyMs + delta));
        return { ...n, latencyMs: newLat };
      }));
    }, 1500);

    return () => clearInterval(timer);
  }, []);

  const getNodeColor = (node: ForceNode) => {
    if (node.type === 'client') return '#10b981';
    if (node.type === 'relay') return '#06b6d4';
    if (node.latencyMs < 50) return '#10b981';
    if (node.latencyMs < 90) return '#38bdf8';
    if (node.latencyMs < 140) return '#f59e0b';
    return '#ef4444';
  };

  const getLinkColor = (lat: number) => {
    if (lat < 30) return '#10b981';
    if (lat < 70) return '#06b6d4';
    if (lat < 120) return '#f59e0b';
    return '#ef4444';
  };

  return (
    <Card className="shield-surface-strong backdrop-blur-md overflow-hidden relative">
      <CardHeader className="pb-2 border-b border-white/10">
        <div className="flex items-center justify-between flex-wrap gap-2">
          <div>
            <CardTitle className="text-slate-100 text-sm sm:text-base flex items-center gap-2">
              <Globe className="w-5 h-5 text-cyan-400" />
              <span>نقشه حرارتی و گراف دینامیک تأخیر جهانی (Global Latency Force Graph)</span>
            </CardTitle>
            <CardDescription className="text-slate-400 text-xs mt-0.5">
              شبیه‌ساز توپولوژی ارتباطی کلاینت، رله‌های مرزی و سرورهای مقصد با جریان فوتونی بسته‌ها
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <Badge variant="outline" className="text-xs border-cyan-500/30 text-cyan-300 bg-cyan-500/10">
              <Activity className="w-3 h-3 ml-1 animate-pulse text-cyan-400" />
              تأخیر بلادرنگ (D3 Force Mesh)
            </Badge>
          </div>
        </div>
      </CardHeader>

      <CardContent className="p-3 sm:p-4 space-y-4">
        {/* Force-directed SVG Graph */}
        <div className="relative w-full h-80 sm:h-96 bg-[#070b13]/95 rounded-xl border border-white/10 overflow-hidden">
          <svg viewBox="0 0 880 400" className="w-full h-full select-none">
            <defs>
              <linearGradient id="link-gradient" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" stopColor="#06b6d4" stopOpacity="0.8" />
                <stop offset="100%" stopColor="#10b981" stopOpacity="0.4" />
              </linearGradient>
              <filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
                <feGaussianBlur stdDeviation="3" result="blur" />
                <feComposite in="SourceGraphic" in2="blur" operator="over" />
              </filter>
            </defs>

            {/* Links */}
            {INITIAL_LINKS.map((link, idx) => {
              const src = nodes.find(n => n.id === link.source);
              const tgt = nodes.find(n => n.id === link.target);
              if (!src || !tgt) return null;

              const strokeColor = getLinkColor(link.latencyMs);

              return (
                <g key={`link-${idx}`}>
                  {/* Background link line */}
                  <line
                    x1={src.x}
                    y1={src.y}
                    x2={tgt.x}
                    y2={tgt.y}
                    stroke={strokeColor}
                    strokeWidth="2"
                    strokeOpacity="0.35"
                    strokeDasharray={link.stability < 90 ? '4 4' : undefined}
                  />

                  {/* Animated traveling particle */}
                  <circle
                    r="3.5"
                    fill={strokeColor}
                    filter="url(#glow)"
                  >
                    <animateMotion
                      path={`M ${src.x} ${src.y} L ${tgt.x} ${tgt.y}`}
                      dur={`${Math.max(1.2, link.latencyMs / 40)}s`}
                      repeatCount="indefinite"
                    />
                  </circle>

                  {/* Latency label at link center */}
                  <text
                    x={(src.x + tgt.x) / 2}
                    y={(src.y + tgt.y) / 2 - 4}
                    fill="#94a3b8"
                    fontSize="9"
                    textAnchor="middle"
                    className="font-mono pointer-events-none"
                  >
                    {toPersianNum(link.latencyMs)}ms
                  </text>
                </g>
              );
            })}

            {/* Nodes */}
            {nodes.map((node) => {
              const color = getNodeColor(node);
              const isSelected = selectedNode?.id === node.id;

              return (
                <g
                  key={node.id}
                  transform={`translate(${node.x}, ${node.y})`}
                  onClick={() => setSelectedNode(node)}
                  className="cursor-pointer group"
                >
                  {/* Outer pulse ring for selected/active */}
                  {(isSelected || node.active) && (
                    <circle
                      r="22"
                      fill="none"
                      stroke={color}
                      strokeWidth="1.5"
                      strokeOpacity="0.6"
                      className="animate-ping"
                      style={{ transformOrigin: '0 0' }}
                    />
                  )}

                  {/* Node Circle */}
                  <circle
                    r={node.type === 'client' ? '18' : node.type === 'relay' ? '16' : '15'}
                    fill="#0f172a"
                    stroke={color}
                    strokeWidth={isSelected ? '3' : '2'}
                    filter="url(#glow)"
                  />

                  {/* Flag / Icon inside */}
                  <text
                    x="0"
                    y="4"
                    fontSize={node.type === 'relay' ? '11' : '13'}
                    textAnchor="middle"
                    className="pointer-events-none select-none"
                  >
                    {node.flag}
                  </text>

                  {/* Label */}
                  <text
                    x="0"
                    y="28"
                    fill="#f1f5f9"
                    fontSize="10"
                    fontWeight={isSelected ? 'bold' : 'normal'}
                    textAnchor="middle"
                    className="pointer-events-none select-none"
                  >
                    {node.nameFa}
                  </text>

                  {/* Latency badge */}
                  {node.type !== 'client' && (
                    <text
                      x="0"
                      y="40"
                      fill={color}
                      fontSize="9"
                      fontWeight="bold"
                      textAnchor="middle"
                      className="font-mono pointer-events-none select-none"
                    >
                      {toPersianNum(node.latencyMs)} ms
                    </text>
                  )}
                </g>
              );
            })}
          </svg>
        </div>

        {/* Selected Node Details Card */}
        {selectedNode && (
          <div className="bg-slate-950 p-4 rounded-xl border border-white/10 flex items-center justify-between flex-wrap gap-4">
            <div className="flex items-center gap-3">
              <div className="text-3xl p-2 rounded-xl bg-slate-900 border border-white/10">
                {selectedNode.flag}
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <span className="font-bold text-sm text-slate-100">{selectedNode.nameFa}</span>
                  <Badge variant="outline" className="text-[10px] border-slate-700 text-slate-400">
                    {selectedNode.type === 'client' ? 'مبدأ کاربر' : selectedNode.type === 'relay' ? 'رله واسط' : 'سرور خروجی'}
                  </Badge>
                </div>
                <p className="text-xs text-slate-400 mt-0.5">{selectedNode.name}</p>
              </div>
            </div>

            <div className="flex items-center gap-4 text-xs">
              <div className="text-center">
                <span className="text-slate-500 block text-[10px]">تأخیر زنده</span>
                <span className="font-bold font-mono text-cyan-300 text-sm">
                  {toPersianNum(selectedNode.latencyMs)} ms
                </span>
              </div>
              <div className="text-center">
                <span className="text-slate-500 block text-[10px]">نوسان (Jitter)</span>
                <span className="font-bold font-mono text-amber-300 text-sm">
                  {toPersianNum(selectedNode.jitterMs)} ms
                </span>
              </div>
              <div className="text-center">
                <span className="text-slate-500 block text-[10px]">افت بسته</span>
                <span className="font-bold font-mono text-emerald-300 text-sm">
                  {toPersianNum(selectedNode.packetLoss)}٪
                </span>
              </div>
              <div className="text-center">
                <span className="text-slate-500 block text-[10px]">بار ترافیک</span>
                <span className="font-bold font-mono text-slate-200 text-sm">
                  {toPersianNum(selectedNode.load)}٪
                </span>
              </div>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
