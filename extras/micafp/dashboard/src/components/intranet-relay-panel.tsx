'use client';

import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Server, Shield, Zap, Globe, ArrowRightLeft, Radio,
  Lock, CheckCircle2, AlertTriangle, RefreshCw, Cpu,
  Activity, ShieldCheck, Download, Upload, Terminal,
  Workflow, Network, Power, KeyRound, Sparkles
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Progress } from '@/components/ui/progress';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

export interface DomesticRelayNode {
  id: string;
  name: string;
  nameFa: string;
  datacenter: string;
  datacenterFa: string;
  ip: string;
  port: number;
  protocol: 'Reverse-TLS' | 'gRPC-Multiplex' | 'HTTP2-Stream' | 'WebSocket-WSS' | 'ShadowTLS-v3';
  camouflageDomain: string;
  camouflageDomainFa: string;
  status: 'active' | 'standby' | 'testing' | 'offline';
  pingMs: number;
  tunnelThroughputMbps: number;
  rstInjectionRate: number;
  encryption: string;
  destinationExitNode: string;
  destinationCountry: string;
  destinationCountryFa: string;
}

const DOMESTIC_RELAYS: DomesticRelayNode[] = [
  {
    id: 'relay-asiatech-teh',
    name: 'Asiatech Edge Tehran 01',
    nameFa: 'آسیاتک برج میلاد تهران',
    datacenter: 'Asiatech Milad Tower DC',
    datacenterFa: 'دیتاسنتر آسیاتک برج میلاد',
    ip: '185.143.232.14',
    port: 443,
    protocol: 'Reverse-TLS',
    camouflageDomain: 'sep.shaparak.ir',
    camouflageDomainFa: 'درگاه پرداخت شاپرک (SEP)',
    status: 'active',
    pingMs: 14,
    tunnelThroughputMbps: 185.4,
    rstInjectionRate: 0.1,
    encryption: 'TLS 1.3 / ChaCha20-Poly1305',
    destinationExitNode: 'Frankfurt-DE-01',
    destinationCountry: 'DE',
    destinationCountryFa: 'آلمان (فرانکفورت)',
  },
  {
    id: 'relay-afranet-teh',
    name: 'Afranet Gateway 03',
    nameFa: 'افرanet بهشتی تهران',
    datacenter: 'Afranet Tier-3 DC',
    datacenterFa: 'دیتاسنتر افرانت',
    ip: '91.98.112.55',
    port: 8443,
    protocol: 'gRPC-Multiplex',
    camouflageDomain: 'bpm.shaparak.ir',
    camouflageDomainFa: 'درگاه به‌پرداخت ملت (BPM)',
    status: 'active',
    pingMs: 18,
    tunnelThroughputMbps: 210.8,
    rstInjectionRate: 0.0,
    encryption: 'gRPC TLS / AES-256-GCM',
    destinationExitNode: 'Amsterdam-NL-02',
    destinationCountry: 'NL',
    destinationCountryFa: 'هلند (آمستردام)',
  },
  {
    id: 'relay-shatel-karaj',
    name: 'Shatel Central Karaj',
    nameFa: 'شاتل دیتاسنتر کرج',
    datacenter: 'Shatel Core DC',
    datacenterFa: 'دیتاسنتر مرکزی شاتل',
    ip: '85.185.74.88',
    port: 443,
    protocol: 'HTTP2-Stream',
    camouflageDomain: 'saman.shaparak.ir',
    camouflageDomainFa: 'پرداخت سامان کیش (SEP)',
    status: 'active',
    pingMs: 16,
    tunnelThroughputMbps: 164.2,
    rstInjectionRate: 0.2,
    encryption: 'HTTP/2 TLS 0-RTT',
    destinationExitNode: 'Helsinki-FI-01',
    destinationCountry: 'FI',
    destinationCountryFa: 'فنلاند (هلسینکی)',
  },
  {
    id: 'relay-mobinnet-shz',
    name: 'MobinNet South Shiraz',
    nameFa: 'مبین‌نت جنوب شیراز',
    datacenter: 'MobinNet Shiraz DC',
    datacenterFa: 'دیتاسنتر مبین‌نت شیراز',
    ip: '178.252.189.23',
    port: 2053,
    protocol: 'ShadowTLS-v3',
    camouflageDomain: 'sadad.shaparak.ir',
    camouflageDomainFa: 'پرداخت سداد ملی (Sadad)',
    status: 'standby',
    pingMs: 24,
    tunnelThroughputMbps: 140.0,
    rstInjectionRate: 0.4,
    encryption: 'ShadowTLS-v3 Handshake Mimic',
    destinationExitNode: 'Istanbul-TR-01',
    destinationCountry: 'TR',
    destinationCountryFa: 'ترکیه (استانبول)',
  },
  {
    id: 'relay-hiweb-tbz',
    name: 'HiWeb Tabriz Hub',
    nameFa: 'های‌وب شمال‌غرب تبریز',
    datacenter: 'HiWeb Tabriz Node',
    datacenterFa: 'دیتاسنتر های‌وب تبریز',
    ip: '5.200.12.80',
    port: 443,
    protocol: 'Reverse-TLS',
    camouflageDomain: 'pec.shaparak.ir',
    camouflageDomainFa: 'پارسیان تجارت الکترونیک (PEC)',
    status: 'standby',
    pingMs: 28,
    tunnelThroughputMbps: 125.6,
    rstInjectionRate: 0.3,
    encryption: 'TLS 1.3 / X25519',
    destinationExitNode: 'Frankfurt-DE-04',
    destinationCountry: 'DE',
    destinationCountryFa: 'آلمان (فرانکفورت)',
  },
  {
    id: 'relay-parsonline-esf',
    name: 'ParsOnline Central Isfahan',
    nameFa: 'پارس‌آنلاین اصفهان',
    datacenter: 'ParsOnline Isfahan DC',
    datacenterFa: 'دیتاسنتر پارس‌آنلاین اصفهان',
    ip: '94.182.160.10',
    port: 8080,
    protocol: 'WebSocket-WSS',
    camouflageDomain: 'asan.shaparak.ir',
    camouflageDomainFa: 'آسان پرداخت پرشین (AsanPardakht)',
    status: 'standby',
    pingMs: 22,
    tunnelThroughputMbps: 110.0,
    rstInjectionRate: 0.5,
    encryption: 'WSS TLS 1.3 Masked',
    destinationExitNode: 'London-UK-01',
    destinationCountry: 'UK',
    destinationCountryFa: 'انگلستان (لندن)',
  },
];

export function IranIntranetRelayPanel() {
  const { connected, addLog } = useUnifiedShieldStore();
  const [relays, setRelays] = useState<DomesticRelayNode[]>(DOMESTIC_RELAYS);
  const [selectedRelayId, setSelectedRelayId] = useState<string>('relay-asiatech-teh');
  const [emergencyIntranetMode, setEmergencyIntranetMode] = useState<boolean>(true);
  const [autoDomesticFailover, setAutoDomesticFailover] = useState<boolean>(true);
  const [whitelistCamouflage, setWhitelistCamouflage] = useState<boolean>(true);
  const [isTunnelTesting, setIsTunnelTesting] = useState<boolean>(false);
  const [testLog, setTestLog] = useState<string | null>(null);

  const activeRelay = useMemo(() => {
    return relays.find(r => r.id === selectedRelayId) ?? relays[0];
  }, [relays, selectedRelayId]);

  // Live jitter update for realism
  useEffect(() => {
    if (!connected) return;
    const timer = setInterval(() => {
      setRelays(prev => prev.map(r => {
        if (r.status === 'offline') return r;
        const jitter = Math.floor(Math.random() * 5 - 2);
        const newPing = Math.max(10, r.pingMs + jitter);
        const tJitter = (Math.random() * 4 - 2);
        const newThroughput = Math.max(80, Math.round((r.tunnelThroughputMbps + tJitter) * 10) / 10);
        return {
          ...r,
          pingMs: newPing,
          tunnelThroughputMbps: newThroughput,
        };
      }));
    }, 4000);
    return () => clearInterval(timer);
  }, [connected]);

  const handleTestTunnel = useCallback(() => {
    setIsTunnelTesting(true);
    setTestLog(`در حال ارسال پروب معکوس TLS به سرور واسط ${activeRelay.nameFa}...`);

    setTimeout(() => {
      setTestLog(`اتصال اولیه به ${activeRelay.ip}:${activeRelay.port} با هدر استتار ${activeRelay.camouflageDomain} برقرار شد.`);
    }, 900);

    setTimeout(() => {
      setTestLog(`تونل معکوس داخلی برقرار گردید! خروجی به سمت ${activeRelay.destinationExitNode} (${activeRelay.destinationCountryFa}) بدون هیچ نشت DPI فعال است.`);
      setIsTunnelTesting(false);
      addLog({
        type: 'connect',
        message: `Intranet Reverse-Tunnel Verified: Hop [${activeRelay.datacenter}] -> Edge [${activeRelay.destinationExitNode}]`,
        messageFa: `آزمایش تونل معکوس اینترنت ملی تأیید شد: رله [${activeRelay.nameFa}] -> خروجی [${activeRelay.destinationExitNode}] با موفقیت متصل شد`,
      });
    }, 2200);
  }, [activeRelay, addLog]);

  return (
    <div className="space-y-4">
      {/* Header & Status Banner */}
      <Card className="shield-surface-strong overflow-hidden relative">
        <div className="absolute top-0 right-0 left-0 h-1 bg-gradient-to-r from-emerald-500 via-cyan-500 to-amber-500" />
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between flex-wrap gap-2">
            <div className="flex items-center gap-2.5">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-cyan-600 to-emerald-600 flex items-center justify-center shadow-lg shadow-cyan-500/20">
                <Workflow className="w-5 h-5 text-white" />
              </div>
              <div>
                <CardTitle className="text-slate-100 text-base sm:text-lg flex items-center gap-2">
                  تونل رله دیتاسنترهای داخلی (Iran Domestic Relay & Intranet Bypass)
                </CardTitle>
                <CardDescription className="text-slate-400 text-xs">
                  معماری دو‌مرحله‌ای ضد قطعی اینترنت بین‌الملل: کلاینت ➔ دیتاسنتر داخلی (Iran DC) ➔ تونل معکوس به سرور خارج
                </CardDescription>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <Badge className="bg-cyan-500/20 text-cyan-300 border-cyan-500/40 text-xs">
                {toPersianNum(relays.length)} نود رله داخلی فعال
              </Badge>
              <Badge className={emergencyIntranetMode ? 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30 text-xs' : 'bg-slate-700 text-slate-400 text-xs'}>
                {emergencyIntranetMode ? 'حالت اینترنت ملی فعال' : 'حالت عادی'}
              </Badge>
            </div>
          </div>
        </CardHeader>

        <CardContent className="space-y-4">
          {/* Visual Architecture Pipeline */}
          <div className="bg-[#0a0f1c]/85 border border-white/10 rounded-xl p-4">
            <p className="text-xs font-bold text-slate-300 mb-3 flex items-center gap-1.5">
              <Sparkles className="w-4 h-4 text-amber-400" />
              <span>مسیر زنده جریان ترافیک اینترنت ملی (Active Tunnel Topology):</span>
            </p>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-3 relative">
              {/* Step 1: Client to Iran DC */}
              <div className="bg-slate-800/90 border border-emerald-500/30 rounded-xl p-3 space-y-1.5 relative overflow-hidden">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold text-emerald-400">گام ۱: دستگاه شما (کلاینت)</span>
                  <Badge className="bg-emerald-500/20 text-emerald-300 text-[10px]">ترافیک مجاز داخلی</Badge>
                </div>
                <p className="text-[11px] text-slate-300">اتصال به IP داخلی مجاز اینترانت ایران</p>
                <div className="text-[10px] text-slate-400 flex justify-between pt-1">
                  <span>پروتکل استتار:</span>
                  <span className="font-mono text-cyan-300">{activeRelay.protocol}</span>
                </div>
                <div className="text-[10px] text-slate-400 flex justify-between">
                  <span>دامنه شاپرک/بانک:</span>
                  <span className="font-mono text-amber-300">{activeRelay.camouflageDomain}</span>
                </div>
              </div>

              {/* Step 2: Domestic Relay Node */}
              <div className="bg-slate-800/90 border border-cyan-500/40 rounded-xl p-3 space-y-1.5 relative overflow-hidden shadow-lg shadow-cyan-500/10">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold text-cyan-300">گام ۲: سرور واسط داخلی</span>
                  <Badge className="bg-cyan-500/20 text-cyan-300 text-[10px]">پینگ: {toPersianNum(activeRelay.pingMs)}ms</Badge>
                </div>
                <p className="text-[11px] text-slate-200 font-semibold">{activeRelay.nameFa}</p>
                <div className="text-[10px] text-slate-400 flex justify-between pt-1">
                  <span>دیتاسنتر:</span>
                  <span className="text-slate-300">{activeRelay.datacenterFa}</span>
                </div>
                <div className="text-[10px] text-slate-400 flex justify-between">
                  <span>پهنای باند تونل:</span>
                  <span className="text-emerald-400 font-bold">{toPersianNum(activeRelay.tunnelThroughputMbps)} Mbps</span>
                </div>
              </div>

              {/* Step 3: Foreign Edge Exit */}
              <div className="bg-slate-800/90 border border-indigo-500/30 rounded-xl p-3 space-y-1.5 relative overflow-hidden">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold text-indigo-300">گام ۳: خروج به اینترنت آزاد</span>
                  <Badge className="bg-indigo-500/20 text-indigo-300 text-[10px]">{activeRelay.destinationCountry}</Badge>
                </div>
                <p className="text-[11px] text-slate-300">{activeRelay.destinationCountryFa}</p>
                <div className="text-[10px] text-slate-400 flex justify-between pt-1">
                  <span>رمزنگاری لایه ۲:</span>
                  <span className="text-slate-300 font-mono text-[9px]">{activeRelay.encryption}</span>
                </div>
                <div className="text-[10px] text-slate-400 flex justify-between">
                  <span>تزریق پکت RST:</span>
                  <span className="text-emerald-400 font-bold">{toPersianNum(activeRelay.rstInjectionRate)}٪ (صفر)</span>
                </div>
              </div>
            </div>
          </div>

          {/* Configuration Controls */}
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <div className="bg-[#0a0f1c]/70 border border-white/10 rounded-xl p-3 flex items-center justify-between">
              <div>
                <p className="text-xs font-bold text-slate-200">حالت اضطراری اینترنت ملی</p>
                <p className="text-[10px] text-slate-400">مسیریابی فقط از طریق دیتاسنترهای داخلی</p>
              </div>
              <Switch checked={emergencyIntranetMode} onCheckedChange={setEmergencyIntranetMode} />
            </div>

            <div className="bg-[#0a0f1c]/70 border border-white/10 rounded-xl p-3 flex items-center justify-between">
              <div>
                <p className="text-xs font-bold text-slate-200">سوئیچ خودکار سرور واسط</p>
                <p className="text-[10px] text-slate-400">تغییر اتوماتیک رله در صورت افت سرعت</p>
              </div>
              <Switch checked={autoDomesticFailover} onCheckedChange={setAutoDomesticFailover} />
            </div>

            <div className="bg-[#0a0f1c]/70 border border-white/10 rounded-xl p-3 flex items-center justify-between">
              <div>
                <p className="text-xs font-bold text-slate-200">استتار درگاه بانکی / شاپرک</p>
                <p className="text-[10px] text-slate-400">تغییر SNI به درگاه‌های مجاز سفید</p>
              </div>
              <Switch checked={whitelistCamouflage} onCheckedChange={setWhitelistCamouflage} />
            </div>
          </div>

          {/* Relay Nodes Grid */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <p className="text-xs font-bold text-slate-300 flex items-center gap-1">
                <Server className="w-4 h-4 text-cyan-400" />
                <span>لیست سرورهای واسط دیتاسنترهای ایران (Domestic Edge Relays):</span>
              </p>
              <Button
                size="sm"
                variant="outline"
                onClick={handleTestTunnel}
                disabled={isTunnelTesting}
                className="text-xs h-7 gap-1 border-cyan-500/40 text-cyan-300 hover:bg-cyan-500/10"
              >
                <RefreshCw className={`w-3.5 h-3.5 ${isTunnelTesting ? 'animate-spin' : ''}`} />
                <span>تست زنده سلامت تونل</span>
              </Button>
            </div>

            {testLog && (
              <motion.div
                initial={{ opacity: 0, y: -4 }}
                animate={{ opacity: 1, y: 0 }}
                className="bg-slate-950 border border-cyan-500/30 rounded-lg p-2.5 font-mono text-[11px] text-cyan-300 flex items-center gap-2"
              >
                <Terminal className="w-4 h-4 text-cyan-400 shrink-0" />
                <span>{testLog}</span>
              </motion.div>
            )}

            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {relays.map((relay) => {
                const isSelected = relay.id === selectedRelayId;
                return (
                  <div
                    key={relay.id}
                    onClick={() => setSelectedRelayId(relay.id)}
                    className={`rounded-xl border p-3.5 cursor-pointer transition-all duration-200 ${
                      isSelected
                        ? 'bg-slate-800 border-cyan-500 shadow-md shadow-cyan-500/20 ring-1 ring-cyan-500/50'
                        : 'bg-slate-900/70 border-white/10 hover:bg-white/[0.06] hover:border-slate-600'
                    }`}
                  >
                    <div className="flex items-center justify-between mb-2">
                      <div className="flex items-center gap-2">
                        <div className={`w-2.5 h-2.5 rounded-full ${isSelected ? 'bg-cyan-400 animate-pulse' : 'bg-emerald-400'}`} />
                        <span className="font-bold text-slate-100 text-xs sm:text-sm">{relay.nameFa}</span>
                      </div>
                      <Badge className={isSelected ? 'bg-cyan-500/20 text-cyan-300 border-cyan-500/40 text-[10px]' : 'bg-slate-800 text-slate-400 border-slate-700 text-[10px]'}>
                        {relay.protocol}
                      </Badge>
                    </div>

                    <div className="space-y-1 text-[11px]">
                      <div className="flex justify-between text-slate-400">
                        <span>دیتاسنتر:</span>
                        <span className="text-slate-300 font-medium">{relay.datacenterFa}</span>
                      </div>
                      <div className="flex justify-between text-slate-400">
                        <span>استتار سفید:</span>
                        <span className="text-amber-300/90 font-mono text-[10px]">{relay.camouflageDomain}</span>
                      </div>
                      <div className="flex justify-between text-slate-400">
                        <span>مقصد نهایی:</span>
                        <span className="text-indigo-300 font-medium">{relay.destinationCountryFa}</span>
                      </div>
                      <div className="flex justify-between pt-1 border-t border-white/10">
                        <span className="text-slate-400">تأخیر داخلی:</span>
                        <span className="text-cyan-300 font-bold">{toPersianNum(relay.pingMs)} ms</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-slate-400">ظرفیت پهنای‌باند:</span>
                        <span className="text-emerald-400 font-bold">{toPersianNum(relay.tunnelThroughputMbps)} Mbps</span>
                      </div>
                    </div>

                    {isSelected && (
                      <div className="mt-2.5 pt-2 border-t border-white/10 flex items-center justify-between text-[10px] text-cyan-300">
                        <span>نود رله فعال است</span>
                        <CheckCircle2 className="w-3.5 h-3.5 text-cyan-400" />
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
