'use client';

import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Shield, ShieldCheck, ShieldX, Ban, Plus, Trash2, Power,
  Brain, Activity, Globe, Lock, Zap, Eye
} from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { useUnifiedShieldStore } from '@/lib/unified-shield-store';
import type { SmartFirewallRule, SmartFirewallState } from '@/lib/unified-shield-types';

function toPersianNum(n: number | string): string {
  const persianDigits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
  return String(n).replace(/\d/g, d => persianDigits[parseInt(d)]);
}

const MODES: { id: SmartFirewallState['mode']; labelFa: string; descFa: string; color: string }[] = [
  { id: 'strict', labelFa: 'سخت‌گیرانه', descFa: 'مسدودسازی کامل — حداکثر امنیت', color: 'text-rose-400' },
  { id: 'balanced', labelFa: 'متعادل', descFa: 'هوشمند — امنیت و کارایی', color: 'text-emerald-400' },
  { id: 'permissive', labelFa: 'سازگار', descFa: 'حداقل محدودیت — حداکثر سرعت', color: 'text-cyan-400' },
];

const ACTION_STYLE: Record<SmartFirewallRule['action'], { labelFa: string; cls: string }> = {
  allow: { labelFa: 'مجاز', cls: 'bg-emerald-500/15 text-emerald-400 border-emerald-500/30' },
  block: { labelFa: 'مسدود', cls: 'bg-rose-500/15 text-rose-400 border-rose-500/30' },
  'dns-only': { labelFa: 'فقط DNS', cls: 'bg-amber-500/15 text-amber-400 border-amber-500/30' },
};

export function SmartFirewallPanel() {
  const {
    firewall, setFirewallMode, toggleFirewallLearning,
    toggleFirewallRule, removeFirewallRule, addFirewallRule, simulateFirewallBlock,
  } = useUnifiedShieldStore();
  const [appName, setAppName] = useState('');
  const [domain, setDomain] = useState('');
  const [action, setAction] = useState<SmartFirewallRule['action']>('block');

  const addRule = () => {
    if (!appName.trim() || !domain.trim()) return;
    addFirewallRule({
      app: appName.trim(),
      appFa: appName.trim(),
      domain: domain.trim(),
      action,
      enabled: true,
    });
    setAppName('');
    setDomain('');
  };

  return (
    <Card className="shield-surface border-0">
      <CardHeader className="pb-2">
        <CardTitle className="text-slate-200 text-base flex items-center gap-2">
          <Shield className="w-5 h-5 text-emerald-400" />
          فایروال هوشمند
          <Badge variant="outline" className="text-[10px] border-emerald-500/30 text-emerald-400">Enterprise</Badge>
        </CardTitle>
        <CardDescription className="text-slate-500 text-xs">
          موتور قوانین مبتنی بر هوش مصنوعی — مسدودسازی نشت، ردیاب‌ها و تلهمتری در لحظه
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Mode selector */}
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
          {MODES.map(m => {
            const active = firewall.mode === m.id;
            return (
              <button
                key={m.id}
                onClick={() => setFirewallMode(m.id)}
                className={`rounded-xl px-3 py-2.5 text-right transition-all border ${
                  active
                    ? 'border-emerald-500/40 bg-emerald-500/10 shadow-lg shadow-emerald-500/10'
                    : 'border-white/10 bg-white/[0.03] hover:bg-white/[0.06]'
                }`}
              >
                <p className={`text-sm font-bold ${active ? m.color : 'text-slate-300'}`}>{m.labelFa}</p>
                <p className="text-[10px] text-slate-500 mt-0.5">{m.descFa}</p>
              </button>
            );
          })}
        </div>

        {/* Stats row */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500 flex items-center gap-1"><Ban className="w-3 h-3 text-rose-400" />مسدودسازی کل</p>
            <p className="text-lg font-bold text-rose-400 tabular">{toPersianNum(firewall.totalBlocked)}</p>
          </div>
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500 flex items-center gap-1"><Eye className="w-3 h-3 text-amber-400" />نشت DNS</p>
            <p className="text-lg font-bold text-amber-400 tabular">{toPersianNum(firewall.dnsLeakBlocked)}</p>
          </div>
          <div className="shield-tile rounded-xl px-3 py-2.5">
            <p className="text-[10px] text-slate-500 flex items-center gap-1"><Globe className="w-3 h-3 text-cyan-400" />قوانین فعال</p>
            <p className="text-lg font-bold text-cyan-400 tabular">{toPersianNum(firewall.rules.filter(r => r.enabled).length)}</p>
          </div>
          <button
            onClick={() => toggleFirewallLearning()}
            className={`rounded-xl px-3 py-2.5 border transition-all text-right ${
              firewall.learningEnabled
                ? 'border-violet-500/40 bg-violet-500/10'
                : 'border-white/10 bg-white/[0.03]'
            }`}
          >
            <p className="text-[10px] text-slate-500 flex items-center gap-1"><Brain className="w-3 h-3 text-violet-400" />یادگیری هوشمند</p>
            <p className={`text-sm font-bold ${firewall.learningEnabled ? 'text-violet-400' : 'text-slate-400'}`}>
              {firewall.learningEnabled ? 'فعال' : 'غیرفعال'}
            </p>
          </button>
        </div>

        {/* Add rule */}
        <div className="shield-inset rounded-xl p-3 flex flex-col sm:flex-row gap-2">
          <input
            value={appName}
            onChange={e => setAppName(e.target.value)}
            placeholder="نام برنامه (مثلاً اینستاگرام)"
            className="flex-1 bg-white/[0.04] border border-white/10 rounded-lg px-3 py-2 text-xs text-slate-200 placeholder:text-slate-600 focus:outline-none focus:border-emerald-500/40"
          />
          <input
            value={domain}
            onChange={e => setDomain(e.target.value)}
            placeholder="دامنه (مثلاً *.instagram.com)"
            dir="ltr"
            className="flex-1 bg-white/[0.04] border border-white/10 rounded-lg px-3 py-2 text-xs text-slate-200 placeholder:text-slate-600 focus:outline-none focus:border-emerald-500/40"
          />
          <select
            value={action}
            onChange={e => setAction(e.target.value as SmartFirewallRule['action'])}
            className="bg-white/[0.04] border border-white/10 rounded-lg px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-emerald-500/40"
          >
            <option value="block">مسدود</option>
            <option value="allow">مجاز</option>
            <option value="dns-only">فقط DNS</option>
          </select>
          <Button onClick={addRule} className="shield-gradient-btn text-white border-0">
            <Plus className="w-4 h-4" /> افزودن قانون
          </Button>
        </div>

        {/* Rules list */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <p className="text-xs text-slate-400 font-semibold flex items-center gap-1"><Activity className="w-3.5 h-3.5 text-emerald-400" />قوانین ({toPersianNum(firewall.rules.length)})</p>
            <Badge variant="outline" className="text-[10px] border-white/10 text-slate-400">
              حالت: {MODES.find(m => m.id === firewall.mode)?.labelFa}
            </Badge>
          </div>
          <AnimatePresence initial={false}>
            {firewall.rules.map(rule => (
              <motion.div
                key={rule.id}
                layout
                initial={{ opacity: 0, y: -8 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, x: 40 }}
                className={`shield-tile rounded-xl px-3 py-2.5 flex items-center gap-3 ${rule.enabled ? '' : 'opacity-50'}`}
              >
                <button
                  onClick={() => toggleFirewallRule(rule.id)}
                  className={`w-9 h-9 rounded-lg flex items-center justify-center transition-all ${
                    rule.enabled ? 'bg-emerald-500/15 text-emerald-400' : 'bg-white/[0.04] text-slate-500'
                  }`}
                  title={rule.enabled ? 'فعال' : 'غیرفعال'}
                >
                  {rule.enabled ? <ShieldCheck className="w-4 h-4" /> : <Power className="w-4 h-4" />}
                </button>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <p className="text-sm text-slate-200 font-semibold truncate">{rule.appFa}</p>
                    <Badge className={`text-[9px] border ${ACTION_STYLE[rule.action].cls}`}>{ACTION_STYLE[rule.action].labelFa}</Badge>
                  </div>
                  <p className="text-[10px] text-slate-500 font-mono truncate" dir="ltr">{rule.domain}</p>
                </div>
                <div className="flex items-center gap-2">
                  <div className="text-left">
                    <p className="text-xs font-bold text-rose-400 tabular">{toPersianNum(rule.blockedAttempts)}</p>
                    <p className="text-[9px] text-slate-600">مسدودیت</p>
                  </div>
                  {rule.action !== 'allow' && (
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => simulateFirewallBlock(rule.id)}
                      className="h-7 px-2 text-[10px] border-rose-500/30 text-rose-400 hover:bg-rose-500/10"
                      title="شبیه‌سازی مسدودسازی"
                    >
                      <Zap className="w-3 h-3" />
                    </Button>
                  )}
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => removeFirewallRule(rule.id)}
                    className="h-7 w-7 p-0 text-slate-500 hover:text-rose-400"
                    title="حذف قانون"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </Button>
                </div>
              </motion.div>
            ))}
          </AnimatePresence>
        </div>

        {/* Status footer */}
        <div className="flex items-center gap-2 text-[10px] text-slate-500 shield-tile rounded-lg px-3 py-2">
          <ShieldX className="w-3.5 h-3.5 text-emerald-400" />
          آخرین مسدودسازی: {firewall.lastBlockTs ? new Date(firewall.lastBlockTs).toLocaleTimeString('fa-IR') : '—'}
          <span className="mx-1 text-slate-700">•</span>
          <Lock className="w-3 h-3 text-cyan-400" />
          فایروال در لایه ۷ (کاربردی) و لایه ۳ (شبکه) فعال است
        </div>
      </CardContent>
    </Card>
  );
}
