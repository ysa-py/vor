'use client';

import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react';

export type Locale = 'fa' | 'en';
export type Direction = 'rtl' | 'ltr';

const STORAGE_KEY = 'shield-locale';

function safeRead(key: string): string | null {
  try {
    return typeof window !== 'undefined' ? window.localStorage.getItem(key) : null;
  } catch {
    return null;
  }
}

function safeWrite(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // storage disabled — locale still applies for this session
  }
}

function detectBrowserLocale(): Locale {
  try {
    const nav = navigator.language || (navigator as { userLanguage?: string }).userLanguage || '';
    return nav.toLowerCase().startsWith('fa') ? 'fa' : 'en';
  } catch {
    return 'fa';
  }
}

type Messages = Record<string, string>;

const messages: Record<Locale, Messages> = {
  fa: {
    'app.title': 'یونیفایدشیلد – MICAFP',
    'app.subtitle': 'موتور ضد سانسور هوشمند چند هسته‌ای – بهینه ایران',
    'settings.general': 'عمومی',
    'settings.language': 'زبان',
    'settings.languageDesc': 'زبان رابط کاربری',
    'settings.theme': 'تم',
    'settings.themeDesc': 'ظاهر برنامه',
    'settings.behavior': 'رفتار',
    'settings.boot': 'اجرا هنگام بوت',
    'settings.bootDesc': 'شروع خودکار با سیستم‌عامل',
    'settings.autoconnect': 'اتصال خودکار',
    'settings.autoconnectDesc': 'اتصال هنگام باز کردن برنامه',
    'settings.notifications': 'اعلان‌ها',
    'settings.notificationsDesc': 'اعلان تغییر وضعیت و هشدارها',
    'settings.stealth': 'پنهان‌سازی و دیباگ',
    'settings.stealthMode': 'حالت مخفی (Stealth Mode)',
    'settings.stealthModeDesc': 'کاهش ردپای VPN — پنهان‌سازی پیشرفته',
    'settings.debug': 'حالت دیباگ',
    'settings.debugDesc': 'ثبت لاگ‌های دقیق برای عیب‌یابی',
    'settings.network': 'شبکه',
    'settings.timeout': 'مهلت اتصال (ms)',
    'settings.mtu': 'اندازه MTU',
    'settings.reset': 'بازنشانی به پیش‌فرض',
    'theme.dark': 'تاریک',
    'theme.light': 'روشن',
    'theme.system': 'سیستم',
    'language.fa': 'فارسی',
    'language.en': 'English',
    'app.connectPrompt': 'اتصال با یک لمس',
    'app.connected': 'متصل و ایمن',
    'app.connecting': 'در حال اتصال…',
    'app.connectHint': 'برای اتصال خودکار لمس کنید',
    'app.protectedHint': 'یونیفایدشیلد از شما محافظت می‌کند',
    'app.loading': 'در حال بارگذاری سپر یکپارچه…',
    'app.footerTagline': 'موتور ضد سانسور هوشمند چند هسته‌ای (بهینه ایران)',
    'header.connected': 'متصل',
    'header.disconnected': 'قطع',
  },
  en: {
    'app.title': 'UnifiedShield – MICAFP',
    'app.subtitle': 'Multi-core intelligent anti-censorship engine — optimized for Iran',
    'settings.general': 'General',
    'settings.language': 'Language',
    'settings.languageDesc': 'User interface language',
    'settings.theme': 'Theme',
    'settings.themeDesc': 'App appearance',
    'settings.behavior': 'Behavior',
    'settings.boot': 'Run at boot',
    'settings.bootDesc': 'Start automatically with the OS',
    'settings.autoconnect': 'Auto connect',
    'settings.autoconnectDesc': 'Connect when the app opens',
    'settings.notifications': 'Notifications',
    'settings.notificationsDesc': 'Status changes and alerts',
    'settings.stealth': 'Stealth & Diagnostics',
    'settings.stealthMode': 'Stealth Mode',
    'settings.stealthModeDesc': 'Reduce VPN footprint — advanced obfuscation',
    'settings.debug': 'Debug mode',
    'settings.debugDesc': 'Detailed logs for troubleshooting',
    'settings.network': 'Network',
    'settings.timeout': 'Connection timeout (ms)',
    'settings.mtu': 'MTU size',
    'settings.reset': 'Reset to defaults',
    'theme.dark': 'Dark',
    'theme.light': 'Light',
    'theme.system': 'System',
    'language.fa': 'فارسی',
    'language.en': 'English',
    'app.connectPrompt': 'Connect with one tap',
    'app.connected': 'Connected & secure',
    'app.connecting': 'Connecting…',
    'app.connectHint': 'Tap to connect automatically',
    'app.protectedHint': 'UnifiedShield is protecting you',
    'app.loading': 'Loading UnifiedShield…',
    'app.footerTagline': 'Multi-core intelligent anti-censorship engine (optimized for Iran)',
    'header.connected': 'Connected',
    'header.disconnected': 'Disconnected',
  },
};

interface LanguageContextValue {
  locale: Locale;
  dir: Direction;
  setLocale: (locale: Locale) => void;
  t: (key: string) => string;
}

const LanguageContext = createContext<LanguageContextValue | null>(null);

function isValidLocale(value: string | null): value is Locale {
  return value === 'fa' || value === 'en';
}

export function LanguageProvider({ children }: { children: React.ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>('fa');

  // Restore persisted choice, falling back to the browser language.
  useEffect(() => {
    const saved = safeRead(STORAGE_KEY);
    setLocaleState(isValidLocale(saved) ? saved : detectBrowserLocale());
  }, []);

  const setLocale = useCallback((next: Locale) => {
    setLocaleState(next);
    safeWrite(STORAGE_KEY, next);
  }, []);

  // Keep <html lang> and <html dir> in sync with the active locale.
  useEffect(() => {
    const root = document.documentElement;
    root.lang = locale;
    root.dir = locale === 'fa' ? 'rtl' : 'ltr';
  }, [locale]);

  // Translation lookup — falls back to the key name, never "undefined".
  const t = useCallback(
    (key: string): string => {
      const active = messages[locale] ?? messages.fa;
      return active[key] ?? messages.fa[key] ?? key;
    },
    [locale],
  );

  const value = useMemo<LanguageContextValue>(
    () => ({ locale, dir: locale === 'fa' ? 'rtl' : 'ltr', setLocale, t }),
    [locale, setLocale, t],
  );

  return <LanguageContext.Provider value={value}>{children}</LanguageContext.Provider>;
}

export function useLanguage(): LanguageContextValue {
  const ctx = useContext(LanguageContext);
  if (!ctx) throw new Error('useLanguage must be used within a LanguageProvider');
  return ctx;
}
