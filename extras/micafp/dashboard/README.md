# MICAFP-UnifiedShield-vip-ultra-c-x-m

## موتور ضد سانسور هوشمند چند هسته‌ای — بهینه‌شده برای ایران

---

### معرفی

**یونیفایدشیلد (UnifiedShield)** یک پلتفرم ضد سانسور چند هسته‌ای هوشمند است که مخصوص عبور از فیلترینگ ایران طراحی شده است. این سیستم با استفاده از ۹ هسته VPN مستقل، موتور هوش مصنوعی مبتنی بر الگوریتم UCB (Multi-Armed Bandit)، و قوانین ISP اختصاصی ایران، بالاترین سطح دسترسی آزاد به اینترنت را تضمین می‌کند.

---

### ۹ هسته VPN

| # | هسته | نسخه | پروتکل‌ها | نقش |
|---|------|-------|----------|------|
| ۱ | **hiddify-core** | v4.1.0 | VLESS Reality, VMess, Trojan, Hysteria2, TUIC v5, ShadowTLS v3, NaiveProxy | هسته هماهنگ‌سازی اصلی |
| ۲ | **GFW-knocker/Xray-core** | v25.8.3-mahsa-r1 | VLESS Fragment, MVLESS, WireGuard Noise, FakeHost | موتور تخصصی عبور از فیلترینگ ایران |
| ۳ | **sing-box** | v1.14.0-alpha.25 | Hysteria2, TUIC v5, ShadowTLS v3, NaiveProxy | مدیریت پروتکل‌ها (داخلی هیدیفای) |
| ۴ | **AmneziaVPN (awg-go)** | 4.8.15.4 | AmneziaWG 1.5 | پروتکل آمنزیاوی‌جی با هدرهای جونک |
| ۵ | **DefyxVPN** | v5.2.8 | VLESS Reality, AmneziaWG 1.5 | عبور پرسرعت با P2P |
| ۶ | **MoaV** | v1.7.7 | MoaV Tunnel | تونل تطبیقی با چرخش کلید پویا |
| ۷ | **Lantern** | v7.9.0 | Domain Fronting, Pluggable Transports | فرانتینگ دامنه |
| ۸ | **MahsaNG core** | v26.3.31-mahsa-r1 | MVLESS, WireGuard Noise, VLESS Fragment | موتور بهینه‌شده برای ایران |
| ۹ | **Psiphon (GFW-knocker fork)** | latest | SSH+Obfs, CDN Fronting | بکاپ آخرین مرحله (بدون سرور) |

---

### ویژگی‌های کلیدی

#### هوش مصنوعی داخلی (on-device)
- **الگوریتم UCB (Upper Confidence Bound)**: یادگیری روی دستگاه بدون وابستگی ابری
- **بهره‌برداری vs اکتشاف**: تعادل بین استفاده از بهترین هسته و آزمایش هسته‌های جدید
- **پیش‌بینی مسدودیت**: شناسایی زودهنگام خطر مسدودیت و تعویض پیش‌دستانه
- **وزن‌های یادگیری تقویتی**: به‌روزرسانی خودکار بر اساس عملکرد هر هسته

#### قوانین ISP اختصاصی ایران
- **همراه اول (MCI)**: هسته‌های ترجیحی مهساان‌جی و آمنزیاوی‌پی‌ان
- **ایرانسل**: هسته‌های ترجیحی هیدیفای و دیفیکس
- **شتل**: هسته‌های ترجیحی آمنزیاوی‌پی‌ان و سایفون
- **آسیاتک**: هسته‌های ترجیحی مهساان‌جی و هیدیفای
- **رایتل**: هسته‌های ترجیحی دیفیکس و هیدیفای

#### قابلیت‌های پیشرفته
- **تحلیلگر شبکه**: نظارت لحظه‌ای پهنای باند، کیفیت اتصال، شاخص پایداری، تحلیل ترافیک
- **مسیریاب جغرافیایی**: انتخاب خودکار بهترین سرور بر اساس ISP، تعادل بار
- **ممیزی امنیتی**: تست نشت DNS، تشخیص نشت WebRTC، ارزیابی رمزنگاری، امتیاز حریم خصوصی
- **نوار وضعیت لحظه‌ای**: نمایش لحظه‌ای وضعیت VPN، سرعت، ISP، سطح تهدید DPI
- **اتصالات سایه**: تعویض فوری هسته بدون قطعی در کمتر از ۲ ثانیه
- **Kill Switch**: قفل شبکه هنگام قطع اتصال
- **اتصال مجدد خودکار**: با Backoff نمایی
- **تونل تقسیم**: bypass خودکار آی‌پی‌های ایران
- **DNS امن**: DoH/DoT با چرخش ارائه‌دهنده
- **به‌روزرسانی OTA**: GitHub Releases API، وصله دلتا، تأیید SHA256
- **شناسایی DPI**: تشخیص امضاهای فیلترینگ ایران (TLS Reset, 403, Null Route, SNI Filter, DNS Poison)
- **هوش تهدید**: شناسایی و مقابله با تهدیدات DPI

#### پلتفرم‌ها
| پلتفرم | نوع تونل | نیاز به روت |
|---------|---------|-------------|
| اندروید ۵+ | VpnService | ❌ بدون روت |
| ویندوز ۷+ | Wintun/TAP | ❌ |
| لینوکس Kernel 4.x+ | tun/tap | ❌ |
| آی‌اواس ۱۵+ | NEPacketTunnelProvider | ❌ بدون جیلبریک |
| اوپن‌دبلیوآرتی ۲۱.۰۲+ | netifd/tun | ❌ |
| مک‌اواس ۱۲+ | NEPacketTunnelProvider | ❌ |

---

### ساختار پروژه

```
MICAFP-UnifiedShield-vip-ultra-c-x-m/
├── package.json
├── next.config.ts
├── tsconfig.json
├── tailwind.config.ts
├── postcss.config.mjs
├── eslint.config.mjs
├── components.json
├── Caddyfile
├── .env
├── .gitignore
├── prisma/
├── db/
├── public/
├── src/
│   ├── app/
│   │   ├── layout.tsx          — RTL فارسی، تم تاریک
│   │   ├── page.tsx            — داشبورد اصلی
│   │   ├── globals.css         — استایل‌های سفارشی
│   │   └── api/
│   │       ├── route.ts        — API ریشه
│   │       ├── ai-engine/      — موتور هوش مصنوعی
│   │       ├── auto-reconnect/ — اتصال مجدد خودکار
│   │       ├── cores/          — مدیریت هسته‌ها
│   │       ├── dpi-test/       — تست DPI
│   │       ├── geo-router/     — مسیریابی جغرافیایی
│   │       ├── health/         — بررسی سلامت
│   │       ├── kill-switch/    — کلید کشتن
│   │       ├── network-analyzer/ — تحلیل شبکه
│   │       ├── orchestrator/   — هماهنگ‌کننده
│   │       ├── ota/            — به‌روزرسانی OTA
│   │       ├── security-audit/ — ممیزی امنیتی
│   │       └── threat-intel/   — هوش تهدید
│   ├── components/
│   │   ├── network-analyzer-panel.tsx  — پنل تحلیل شبکه
│   │   ├── geo-router-panel.tsx       — پنل مسیریابی جغرافیایی
│   │   ├── security-audit-panel.tsx    — پنل ممیزی امنیتی
│   │   ├── realtime-status-bar.tsx     — نوار وضعیت لحظه‌ای
│   │   └── ui/                         — shadcn/ui components
│   ├── lib/
│   │   ├── unified-shield-types.ts    — تمام نوع‌ها و ثابت‌ها
│   │   ├── unified-shield-store.ts    — Zustand store
│   │   ├── network-analyzer.ts        — تحلیلگر شبکه
│   │   ├── geo-router.ts             — مسیریاب جغرافیایی
│   │   ├── security-audit.ts         — ممیزی امنیتی
│   │   ├── utils.ts                  — ابزارها
│   │   └── db.ts                     — دیتابیس
│   └── hooks/
├── docs/
└── scripts/
```

---

### نصب و اجرا

```bash
# نصب وابستگی‌ها
bun install

# اجرای محیط توسعه
bun dev

# بیلد پروداکشن
bun run build

# اجرای پروداکشن
bun start
```

---

### تکنولوژی‌ها

- **Next.js 16** — فریمورک وب
- **TypeScript** — نوع‌گذاری ایستا
- **Tailwind CSS 4** — استایل‌دهی
- **shadcn/ui** — کامپوننت‌های UI
- **Zustand** — مدیریت وضعیت
- **Recharts** — نمودارهای تعاملی
- **Framer Motion** — انیمیشن‌ها
- **Radix UI** — دسترسی‌پذیری

---

### امنیت

- رمزنگاری AES-256-GCM
- تأیید SHA256 باینری‌ها
- محافظت نشت DNS
- شناسایی امضای DPI ایران
- بازگشت خودکار نسخه (Rollback)
- AI محلی بدون وابستگی ابری
- بدون روت / بدون جیلبریک
- بدون نیاز به سرور (VPS)

---

### مجوز

این پروژه برای استفاده آزاد منتشر شده است.

**یونیفایدشیلد — آزادی اینترنت حق همه است** ✊
