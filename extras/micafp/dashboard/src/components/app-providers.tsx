'use client';

import { ThemeProvider } from '@/lib/theme';
import { LanguageProvider } from '@/lib/i18n';

export function AppProviders({ children }: { children: React.ReactNode }) {
  return (
    <ThemeProvider>
      <LanguageProvider>{children}</LanguageProvider>
    </ThemeProvider>
  );
}
