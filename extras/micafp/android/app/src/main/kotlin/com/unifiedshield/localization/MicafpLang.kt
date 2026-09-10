package com.unifiedshield.localization

import android.content.Context
import android.content.SharedPreferences
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

// =============================================================================
// MICAFP Directive v6 — bilingual runtime language layer (FA default, EN alt).
// ADDITIVE: existing screens keep their own Persian literals untouched.
// =============================================================================

enum class MicafpLang(val code: String, val displayName: String) {
    FA("fa", "فارسی"),
    EN("en", "English")
}

/**
 * Lightweight runtime-switchable string table.
 * All strings used by the new MICAFP directive screens live here.
 */
object MicafpStrings {

    private val fa: Map<String, String> = mapOf(
        "app.tagline" to "سامانه یکپارچه ضد فیلترینگ و حریم خصوصی",
        "hub.title" to "مرکز وضعیت امنیتی",
        "hub.protected" to "محافظت‌شده",
        "hub.establishing" to "در حال برقراری اتصال امن",
        "hub.vulnerable" to "آسیب‌پذیر",
        "hub.domesticOnly" to "فقط شبکه داخلی",
        "hub.stateMessage" to "وضعیت تونل بین‌المللی",
        "hub.securityLayers" to "لایه‌های امنیتی",
        "hub.layer.encryption" to "رمزنگاری فعال",
        "hub.layer.dpiEvasion" to "عبور از DPI",
        "hub.layer.dnsLeak" to "نشتی DNS",
        "hub.layer.killSwitch" to "کیل‌سوئیچ",
        "hub.layer.lastCheck" to "آخرین بررسی",
        "hub.pendingWiring" to "در انتظار اتصال به بک‌اند",
        "hub.chip.protocol" to "پروتکل فعال",
        "hub.chip.latency" to "تأخیر زنده",
        "hub.chip.threat" to "سطح تهدید",
        "hub.handshakeDone" to "دست‌دهی رمزدار کامل شد",
        "hub.connectingMsg" to "در حال برقراری دست‌دهی رمزدار…",
        "intel.title" to "هوش پروتکل",
        "intel.why" to "چرا این پروتکل؟",
        "intel.timeline24h" to "خط زمانی ۲۴ ساعته تغییر پروتکل‌ها",
        "intel.override" to "انتخاب دستی (کاربران پیشرفته)",
        "intel.overrideWarning" to "انتخاب دستی شما توسط تداوم‌بخش خودکار بازنویسی نمی‌شود؛ مسئولیت پایداری اتصال با شماست.",
        "intel.noEvents" to "رویداد تغییر پروتکلی در ۲۴ ساعت گذشته ثبت نشده است.",
        "intel.reasonPlaceholder" to "دلیل سوییچ بر اساس تله‌متری واقعی پس از اولین چرخش ثبت می‌شود.",
        "intel.autoPilot" to "تداوم‌بخش خودکار (Auto-Pilot)",
        "intel.autoPilotOn" to "تداوم‌بخش خودکار فعال است",
        "intel.autoPilotOff" to "تداوم‌بخش خودکار غیرفعال است",
        "intel.pinned" to "پروتکل دستی شما قفل شده است",
        "intel.surfaceAll" to "همه پروتکل‌ها نمایان هستند",
        "center.title" to "مرکز امنیت",
        "center.cipherInspector" to "بازرس رمزنگاری",
        "center.cipherReason" to "دلیل انتخاب رمزنگاری",
        "center.pfs" to "تأیید Forward Secrecy",
        "center.pfsDesc" to "کلیدهای نشست موقت هستند و پس از پایان نشست دور ریخته می‌شوند.",
        "center.keyRotation" to "شمارنده چرخش کلید",
        "center.keyRotationDesc" to "تعداد نشست‌های رمزداری که کلید جدید مذاکره کرده‌اند.",
        "center.auditLog" to "گزارش ممیزی تغییرناپذیر",
        "center.auditDesc" to "رویدادها فقط اضافه می‌شوند و هر رکورد با رکورد قبلی زنجیر هش دارد.",
        "center.exportCsv" to "خروجی CSV",
        "center.autoLeakTest" to "آزمون خودکار نشتی",
        "center.autoLeakDesc" to "بررسی دوره‌ای DNS/WebRTC/IP همراه با دکمه اجرای دستی موجود.",
        "center.runNow" to "اجرا در همین لحظه",
        "center.lastLeak" to "نتیجه آخرین آزمون نشتی",
        "center.pass" to "سالم",
        "center.fail" to "نشت تشخیص داده شد",
        "center.neverRun" to "هنوز اجرا نشده",
        "map.title" to "نقشه شبکه",
        "map.device" to "دستگاه شما",
        "map.obfuscation" to "لایه استتار",
        "map.exit" to "گره خروج",
        "map.splitTunnel" to "تونل تقسیم‌شده",
        "map.splitTunnelDesc" to "ترافیک داخلی ایران مستقیم، ترافیک بین‌المللی از تونل. منطق مسیریابی بک‌اند دست‌نخورده است.",
        "map.domestic" to "مقاصد داخلی",
        "map.international" to "مقاصد بین‌المللی",
        "license.title" to "حساب و لایسنس",
        "license.status" to "وضعیت تأیید امضای لایسنس",
        "license.serial" to "سریال / کلید لایسنس",
        "license.serialHint" to "کلید لایسنس امضاشده خود را وارد کنید",
        "license.apply" to "اعمال و تأیید",
        "license.expiry" to "تاریخ انقضا",
        "license.daysLeft" to "روز باقی‌مانده",
        "license.deviceBound" to "مقید به این دستگاه",
        "license.verified" to "امضا معتبر است",
        "license.invalid" to "امضا نامعتبر — لایسنس پذیرفته نشد",
        "license.expired" to "لایسنس منقضی شده — تونل بین‌المللی قطع می‌شود؛ شبکه داخلی و ابزارها فعال می‌مانند",
        "license.expiryNote" to "پس از انقضا فقط دسترسی بین‌المللی قطع می‌شود؛ تنظیمات، پروفایل‌ها، عیب‌یابی و شبکه داخلی فعال می‌مانند.",
        "license.noLicense" to "لایسنسی ثبت نشده است",
        "license.pendingWiring" to "تأیید سمت سرور در انتظار اتصال بک‌اند",
        "license.trustedTime" to "زمان مورد اعتماد",
        "license.trustedTimeOk" to "همگام با منبع زمان مورد اعتماد",
        "license.trustedTimeFallback" to "منبع زمان در دسترس نیست؛ از زمان تک‌نها محافظت‌شده استفاده می‌شود",
        "providers.title" to "کادر API هوش مصنوعی",
        "providers.masterSwitch" to "کلید اصلی لایه هوش مصنوعی خارجی",
        "providers.masterOff" to "خاموش — فقط تحلیل داخلی MICAFP پاسخ می‌دهد",
        "providers.key" to "کلید API (رمزنگاری‌شده روی دستگاه)",
        "providers.baseUrl" to "آدرس پایه (Base URL) — قابل ویرایش",
        "providers.mirrorUrl" to "آدرس جایگزین / آینه (اختیاری)",
        "providers.priority" to "اولویت گریز به خطای بعدی",
        "providers.moveUp" to "بالا",
        "providers.moveDown" to "پایین",
        "providers.test" to "تست اتصال واقعی",
        "providers.testOk" to "اتصال موفق",
        "providers.testFail" to "اتصال ناموفق",
        "providers.unavailable" to "موقتاً در دسترس نیست",
        "providers.answeredBy" to "پاسخ از لایه",
        "providers.layerExternal" to "سرویس خارجی",
        "providers.layerInternal" to "تحلیل داخلی (Local analysis)",
        "providers.noKey" to "کلید API وارد نشده — برای این سرویس غیرفعال است",
        "providers.registryNote" to "تعریف سرویس‌ها از فایل تنظیمات خوانده می‌شود؛ افزودن سرویس جدید بدون تغییر کد ممکن است.",
        "providers.remoteSync" to "به‌روزرسانی امضاشده فهرست سرویس‌ها",
        "doctor.title" to "پزشک اتصال",
        "doctor.run" to "اجرای بررسی کامل",
        "doctor.running" to "در حال اجرای بررسی‌ها…",
        "doctor.rootCause" to "ریشه مشکل",
        "doctor.suggestedFix" to "راهکار پیشنهادی",
        "doctor.aiExplain" to "توضیح هوشمند",
        "doctor.check.dns" to "تفسیر DNS",
        "doctor.check.tcp443" to "اتصال TCP به درگاه ۴۴۳",
        "doctor.check.udp53" to "کوئری UDP/53",
        "doctor.check.tunnel" to "وضعیت تونل MICAFP",
        "profiles.title" to "پروفایل‌های اتصال",
        "profiles.new" to "پروفایل جدید",
        "profiles.import" to "ورود پروفایل",
        "profiles.category" to "دسته پروتکل",
        "profiles.name" to "نام پروفایل",
        "profiles.save" to "ذخیره پروفایل",
        "profiles.activate" to "فعال‌سازی",
        "profiles.active" to "فعال",
        "common.ok" to "تأیید",
        "common.cancel" to "انصراف",
        "common.loading" to "در حال بارگذاری…",
        "common.realData" to "مقادیر از تله‌متری واقعی بک‌اند خوانده می‌شود"
    )

    private val en: Map<String, String> = mapOf(
        "app.tagline" to "Unified anti-censorship & privacy suite",
        "hub.title" to "Security Status Hub",
        "hub.protected" to "Protected",
        "hub.establishing" to "Establishing secure connection",
        "hub.vulnerable" to "Vulnerable",
        "hub.domesticOnly" to "Domestic-only",
        "hub.stateMessage" to "International tunnel state",
        "hub.securityLayers" to "Security Layers",
        "hub.layer.encryption" to "Encryption",
        "hub.layer.dpiEvasion" to "DPI Evasion",
        "hub.layer.dnsLeak" to "DNS Leak",
        "hub.layer.killSwitch" to "Kill Switch",
        "hub.layer.lastCheck" to "Last check",
        "hub.pendingWiring" to "Pending backend wiring",
        "hub.chip.protocol" to "Active protocol",
        "hub.chip.latency" to "Live latency",
        "hub.chip.threat" to "Threat level",
        "hub.handshakeDone" to "Encrypted handshake complete",
        "hub.connectingMsg" to "Establishing encrypted handshake…",
        "intel.title" to "Protocol Intelligence",
        "intel.why" to "Why this protocol?",
        "intel.timeline24h" to "24h protocol switch timeline",
        "intel.override" to "Manual override (advanced users)",
        "intel.overrideWarning" to "Your manual choice is never silently overridden by automatic rotation; connection stability is your responsibility.",
        "intel.noEvents" to "No protocol switch events recorded in the last 24 hours.",
        "intel.reasonPlaceholder" to "Switch reasoning will be recorded from real telemetry after the first rotation.",
        "intel.autoPilot" to "Auto-Pilot rotation",
        "intel.autoPilotOn" to "Auto-Pilot rotation enabled",
        "intel.autoPilotOff" to "Auto-Pilot rotation disabled",
        "intel.pinned" to "Your manual protocol is pinned",
        "intel.surfaceAll" to "All protocols are surfaced",
        "center.title" to "Security Center",
        "center.cipherInspector" to "Cipher Suite Inspector",
        "center.cipherReason" to "Cipher selection reasoning",
        "center.pfs" to "PFS confirmed",
        "center.pfsDesc" to "Session keys are ephemeral and discarded after the session ends.",
        "center.keyRotation" to "Key Rotation Counter",
        "center.keyRotationDesc" to "Count of encrypted sessions that negotiated fresh keys.",
        "center.auditLog" to "Immutable Audit Log",
        "center.auditDesc" to "Append-only records, each hash-chained to the previous record.",
        "center.exportCsv" to "Export CSV",
        "center.autoLeakTest" to "Auto Leak Test",
        "center.autoLeakDesc" to "Periodic DNS/WebRTC/IP check alongside the existing manual run button.",
        "center.runNow" to "Run now",
        "center.lastLeak" to "Last leak test result",
        "center.pass" to "Pass",
        "center.fail" to "Leak detected",
        "center.neverRun" to "Never run",
        "map.title" to "Network Map",
        "map.device" to "Your device",
        "map.obfuscation" to "Obfuscation layer",
        "map.exit" to "Exit node",
        "map.splitTunnel" to "Split Tunneling",
        "map.splitTunnelDesc" to "Domestic Iranian traffic stays direct, international traffic rides the tunnel. Backend routing logic untouched.",
        "map.domestic" to "Domestic destinations",
        "map.international" to "International destinations",
        "license.title" to "Account & License",
        "license.status" to "License signature verification status",
        "license.serial" to "Serial / license key",
        "license.serialHint" to "Paste your signed license key",
        "license.apply" to "Apply & verify",
        "license.expiry" to "Expiry date",
        "license.daysLeft" to "days remaining",
        "license.deviceBound" to "Bound to this device",
        "license.verified" to "Signature valid",
        "license.invalid" to "Invalid signature — license rejected",
        "license.expired" to "License expired — international tunnel is cut; domestic network and tools stay active",
        "license.expiryNote" to "On expiry only international access is cut; settings, profiles, diagnostics and the domestic network remain fully functional.",
        "license.noLicense" to "No license registered",
        "license.pendingWiring" to "Server-side verification pending backend wiring",
        "license.trustedTime" to "Trusted time",
        "license.trustedTimeOk" to "Synced with trusted time source",
        "license.trustedTimeFallback" to "Trusted source unreachable; using ratchet-protected monotonic time",
        "providers.title" to "AI API Panel",
        "providers.masterSwitch" to "External AI layer master switch",
        "providers.masterOff" to "Off — only MICAFP local analysis answers",
        "providers.key" to "API key (encrypted on device)",
        "providers.baseUrl" to "Base URL — editable",
        "providers.mirrorUrl" to "Mirror / fallback base URL (optional)",
        "providers.priority" to "Failover priority",
        "providers.moveUp" to "Up",
        "providers.moveDown" to "Down",
        "providers.test" to "Real connection test",
        "providers.testOk" to "Connection OK",
        "providers.testFail" to "Connection failed",
        "providers.unavailable" to "Temporarily unavailable",
        "providers.answeredBy" to "Answered by",
        "providers.layerExternal" to "External provider",
        "providers.layerInternal" to "Local analysis",
        "providers.noKey" to "No API key set — disabled for this provider",
        "providers.registryNote" to "Provider definitions load from a registry file; adding a new provider requires no code change.",
        "providers.remoteSync" to "Signed remote provider-registry sync",
        "doctor.title" to "Connection Doctor",
        "doctor.run" to "Run full check",
        "doctor.running" to "Running checks…",
        "doctor.rootCause" to "Root cause",
        "doctor.suggestedFix" to "Suggested fix",
        "doctor.aiExplain" to "Smart explanation",
        "doctor.check.dns" to "DNS resolution",
        "doctor.check.tcp443" to "TCP connect to port 443",
        "doctor.check.udp53" to "UDP/53 query",
        "doctor.check.tunnel" to "MICAFP tunnel state",
        "profiles.title" to "Connection Profiles",
        "profiles.new" to "New Profile",
        "profiles.import" to "Import profile",
        "profiles.category" to "Protocol category",
        "profiles.name" to "Profile name",
        "profiles.save" to "Save profile",
        "profiles.activate" to "Activate",
        "profiles.active" to "Active",
        "common.ok" to "OK",
        "common.cancel" to "Cancel",
        "common.loading" to "Loading…",
        "common.realData" to "Values come from real backend telemetry"
    )

    fun get(lang: MicafpLang, key: String): String =
        (if (lang == MicafpLang.FA) fa else en)[key]
            ?: fa[key] // graceful fallback to FA so no key is ever blank
            ?: key
}

/**
 * Runtime language manager — FA default, persisted, hot-switchable.
 */
class MicafpLangManager private constructor(context: Context) {

    private val prefs: SharedPreferences =
        context.getSharedPreferences("micafp_lang", Context.MODE_PRIVATE)

    private val _lang = MutableStateFlow(
        if (prefs.getString("lang", "fa") == "en") MicafpLang.EN else MicafpLang.FA
    )
    val lang: StateFlow<MicafpLang> = _lang.asStateFlow()

    fun setLang(newLang: MicafpLang) {
        prefs.edit().putString("lang", newLang.code).apply()
        _lang.value = newLang
    }

    fun t(key: String): String = MicafpStrings.get(_lang.value, key)

    companion object {
        @Volatile private var instance: MicafpLangManager? = null
        fun getInstance(context: Context): MicafpLangManager =
            instance ?: synchronized(this) {
                instance ?: MicafpLangManager(context.applicationContext).also { instance = it }
            }

        /** Static lookup used by composables: MicafpLangManager.get(lang, key). */
        fun get(lang: MicafpLang, key: String): String = MicafpStrings.get(lang, key)
    }
}
