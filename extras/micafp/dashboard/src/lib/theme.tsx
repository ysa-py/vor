'use client';

import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react';

export type Theme = 'dark' | 'light' | 'system';
export type ResolvedTheme = 'dark' | 'light';

const STORAGE_KEY = 'shield-theme';

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
    // storage disabled (private browsing) — ignore, theme still applies for this session
  }
}

function systemPrefersDark(): boolean {
  try {
    return window.matchMedia('(prefers-color-scheme: dark)').matches;
  } catch {
    return true;
  }
}

interface ThemeContextValue {
  theme: Theme;
  resolvedTheme: ResolvedTheme;
  setTheme: (theme: Theme) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

function isValidTheme(value: string | null): value is Theme {
  return value === 'dark' || value === 'light' || value === 'system';
}

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, setThemeState] = useState<Theme>('system');
  const [resolvedTheme, setResolvedTheme] = useState<ResolvedTheme>('dark');

  // Restore persisted choice on mount (client only — avoids SSR hydration mismatch).
  useEffect(() => {
    const saved = safeRead(STORAGE_KEY);
    if (isValidTheme(saved)) setThemeState(saved);
  }, []);

  const setTheme = useCallback((next: Theme) => {
    setThemeState(next);
    safeWrite(STORAGE_KEY, next);
  }, []);

  // Resolve system preference and reflect changes reactively on <html>.
  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');

    const apply = () => {
      const resolved: ResolvedTheme =
        theme === 'system' ? (media.matches ? 'dark' : 'light') : theme;

      setResolvedTheme(resolved);
      const root = document.documentElement;
      root.classList.toggle('dark', resolved === 'dark');
      root.classList.toggle('light', resolved === 'light');
      root.style.colorScheme = resolved;
    };

    apply();

    if (theme === 'system') {
      const onChange = () => apply();
      media.addEventListener('change', onChange);
      return () => media.removeEventListener('change', onChange);
    }
  }, [theme]);

  const value = useMemo<ThemeContextValue>(
    () => ({ theme, resolvedTheme, setTheme }),
    [theme, resolvedTheme, setTheme],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error('useTheme must be used within a ThemeProvider');
  return ctx;
}
