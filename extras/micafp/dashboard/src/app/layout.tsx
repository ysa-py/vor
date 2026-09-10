import type { Metadata, Viewport } from "next";
import { Vazirmatn, Geist_Mono } from "next/font/google";
import "./globals.css";
import { Toaster } from "@/components/ui/toaster";
import { AppProviders } from "@/components/app-providers";

const vazirmatn = Vazirmatn({
  variable: "--font-vazirmatn",
  subsets: ["latin", "arabic"],
  display: "swap",
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "یونیفایدشیلد — کنسول فرماندهی امنیت سایبری",
  description:
    "موتور ضد سانسور هوشمند چند هسته‌ای بهینه‌شده برای ایران — ۳۰ هسته، هوش مصنوعی داخلی UCB، تعویض خودکار، حفاظت کوانتومی و قابلیت‌های سازمانی (Enterprise)",
  keywords: [
    "UnifiedShield",
    "MICAFP",
    "ضد سانسور",
    "ایران",
    "VPN",
    "هوش مصنوعی",
    "Hiddify",
    "Xray",
    "sing-box",
    "Psiphon",
    "Enterprise",
    "امنیت سایبری",
  ],
  applicationName: "UnifiedShield Command Console",
  authors: [{ name: "UnifiedShield" }],
  icons: {
    icon: "/logo.svg",
  },
  openGraph: {
    title: "یونیفایدشیلد — کنسول فرماندهی امنیت سایبری",
    description:
      "پلتفرم ضد سانسور سازمانی چند هسته‌ای با هوش مصنوعی داخلی، پروتکل‌های استگانوگرافی و حفاظت کوانتومی",
    type: "website",
    locale: "fa_IR",
  },
};

export const viewport: Viewport = {
  themeColor: "#070b13",
  width: "device-width",
  initialScale: 1,
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="fa"
      dir="rtl"
      suppressHydrationWarning
      className={`dark ${vazirmatn.variable} ${geistMono.variable}`}
    >
      <body className="bg-background text-foreground">
        {/* Apply persisted theme before first paint to avoid flash-of-wrong-theme */}
        <script
          dangerouslySetInnerHTML={{
            __html: `(function(){try{var r=document.documentElement;var t=localStorage.getItem('shield-theme');var d=t==='dark'||((!t||t==='system')&&window.matchMedia('(prefers-color-scheme: dark)').matches);r.classList.toggle('dark',d);r.classList.toggle('light',!d);r.style.colorScheme=d?'dark':'light';var l=localStorage.getItem('shield-locale');if(l==='en'){r.lang='en';r.dir='ltr';}}catch(e){document.documentElement.classList.add('dark');}})();`,
          }}
        />
        {/* Ambient enterprise atmosphere — aurora + precision grid */}
        <div className="shield-atmosphere" aria-hidden="true" />
        <AppProviders>{children}</AppProviders>
        <Toaster />
      </body>
    </html>
  );
}
