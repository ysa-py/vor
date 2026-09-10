package com.unifiedshield.tunnel

enum class MasterDnsEncryption(val label: String, val labelPersian: String, val overheadBytes: Int, val securityLevel: String) {
    AES_128_GCM("AES-128-GCM", "رمزنگاری سخت‌افزاری AES-128-GCM", 16, "بسیار بالا"),
    CHACHA20_POLY1305("ChaCha20-Poly1305", "رمزنگاری بهینه موبایل ChaCha20", 16, "بسیار بالا"),
    XOR_STREAM("XOR Stream (Zero Overhead)", "جریان سبک XOR (بدون سربار بایت اضافی و پردازش فوق‌سریع)", 0, "سبک و سریع")
}

enum class MasterDnsBalancingMode(
    val modeName: String,
    val titlePersian: String,
    val descriptionPersian: String
) {
    LATENCY_BASED(
        modeName = "Latency-Based (RTT Weighted)",
        titlePersian = "بر مبنای کمترین تاخیر (Latency-Based)",
        descriptionPersian = "ارسال پویا به رِزولورهایی که کمترین زمان پاسخ (RTT) و پینگ را دارند."
    ),
    PACKET_LOSS_WEIGHTED(
        modeName = "Packet Loss-Weighted",
        titlePersian = "بر مبنای نرخ افت بسته (Loss-Weighted)",
        descriptionPersian = "تعدیل وزن و ارسال ترافیک با توجه به نرخ بسته‌های از دست رفته در هر مسیر."
    ),
    ROUND_ROBIN(
        modeName = "Round Robin",
        titlePersian = "توزیع چرخشی ساده (Round Robin)",
        descriptionPersian = "پخش مساوی و متوالی بسته‌ها میان تمامی سرورهای رِزولور فعال."
    ),
    PRIORITY_FAILOVER(
        modeName = "Priority Failover",
        titlePersian = "سرور اصلی و جایگزین اضطراری (Failover)",
        descriptionPersian = "تمرکز ترافیک بر سریع‌ترین سرور و تعویض زیرثانیه‌ای به پشتیبان در صورت مسدودی."
    ),
    DUPLICATE_BROADCAST(
        modeName = "Duplicate Broadcast (0% Loss)",
        titlePersian = "تکثیر و ارسال همزمان بسته (Redundancy Duplication)",
        descriptionPersian = "ارسال همزمان کپی بسته‌ها به چند رِزولور برای تضمین صفر درصد افت بسته در بدترین شرایط قطعی."
    ),
    DYNAMIC_ENTROPY_RATIO(
        modeName = "Dynamic Entropy Ratio",
        titlePersian = "توزیع آنتروپی متغیر (Anti-DPI Entropy)",
        descriptionPersian = "تغییر پیوسته نسبت بسته‌ها جهت خنثی‌سازی الگوریتم‌های یادگیری ماشین سامانه‌های فیلترینگ."
    ),
    LEAST_LOADED(
        modeName = "Least Loaded",
        titlePersian = "کم‌ترافیک‌ترین رِزولور (Least Loaded)",
        descriptionPersian = "هدایت کوئری‌های DNS به خلوت‌ترین سرور با کمترین بار پردازشی و صف."
    ),
    CONSISTENT_HASH_RING(
        modeName = "Consistent Hash Ring",
        titlePersian = "حلقه هش پایدار (Hash Ring)",
        descriptionPersian = "حفظ ماندگاری نشست‌ها (Session Stickiness) و توزیع متوازن ترافیک TCP."
    )
}

enum class MasterDnsCoreEngine(val label: String, val language: String) {
    GO_NATIVE("Go Core (High Performance - Main Version)", "Go"),
    LEGACY_PYTHON("Legacy Python Core (Fallback Engine)", "Python"),
    KOTLIN_ARQ("Native Kotlin/C ARQ Multipath", "Kotlin/Native")
}

enum class MasterDnsTcpCarrier(val protocolName: String, val titlePersian: String) {
    DIRECT_TUN("Direct RAW TCP", "انتقال مستقیم TCP روی DNS"),
    SHADOWSOCKS_CARRIER("Shadowsocks Carrier (2022-blake3)", "حمل ترافیک شدوساکس داخل تونل"),
    VLESS_VMESS_CARRIER("VLESS / VMess Protocol Carrier", "حمل ترافیک VLESS/VMess روی بسته‌های DNS"),
    TROJAN_CARRIER("Trojan gRPC Carrier", "حمل ترافیک تروجان با استتار TLS"),
    OPENVPN_CARRIER("OpenVPN UDP Carrier", "حمل بسته‌های اوپن‌وی‌پی‌ان با هدر ARQ")
}

data class MasterDnsResolverNode(
    val id: String,
    val name: String,
    val address: String,
    val port: Int = 53,
    val transport: DnsTransport = DnsTransport.UDP,
    val pingMs: Int = 18,
    val packetLossPct: Double = 0.0,
    val weight: Int = 100,
    val isAlive: Boolean = true,
    val queriesSent: Long = 0,
    val queriesAnswered: Long = 0
)

data class MasterDnsConfig(
    val encryption: MasterDnsEncryption = MasterDnsEncryption.AES_128_GCM,
    val headerOverheadBytes: Int = 5, // ARQ-5-7B = 88% lower than DNSTT, 71% lower than SlipStream
    val arqWindowSize: Int = 128,
    val arqTimeoutMs: Int = 35,
    val customMtu: Int = 512, // Works even with very small MTU
    val balancingMode: MasterDnsBalancingMode = MasterDnsBalancingMode.LATENCY_BASED,
    val enablePacketDuplication: Boolean = false,
    val duplicationFactor: Int = 2,
    val coreEngine: MasterDnsCoreEngine = MasterDnsCoreEngine.GO_NATIVE,
    val enableSocksOptimization: Boolean = true, // Reduced SOCKS overhead
    val enableTcpForwarding: Boolean = true, // TCPForwarding mode can carry Shadowsocks, VLESS, etc.
    val tcpCarrier: MasterDnsTcpCarrier = MasterDnsTcpCarrier.SHADOWSOCKS_CARRIER,
    val enableStrongDnsCache: Boolean = true, // 0.270s handshake
    val cachedHandshakeLatencyMs: Int = 270, // 0.270s vs 1.746s standard
    val rawXorKey: String = "MASTER-DNS-KEY-7X9B",
    val serverDomain: String = "mdns.unifiedshield.net",
    val resolvers: List<MasterDnsResolverNode> = listOf(
        MasterDnsResolverNode("r1", "Alibaba Anycast Resolver", "223.5.5.5", 53, DnsTransport.UDP, 14, 0.0, 100, true),
        MasterDnsResolverNode("r2", "Tencent Clean DNS", "119.29.29.29", 53, DnsTransport.UDP, 18, 0.0, 95, true),
        MasterDnsResolverNode("r3", "Alibaba DoH Ultra-Fast", "https://dns.alidns.com/dns-query", 443, DnsTransport.DOH, 19, 0.0, 90, true),
        MasterDnsResolverNode("r4", "Cloudflare Low-Jitter DNS", "1.1.1.1", 53, DnsTransport.UDP, 22, 0.5, 80, true),
        MasterDnsResolverNode("r5", "Baidu Public DNS", "180.76.76.76", 53, DnsTransport.UDP, 25, 0.0, 75, true),
        MasterDnsResolverNode("r6", "Quad9 Privacy Resolver", "9.9.9.9", 53, DnsTransport.UDP, 28, 1.0, 70, true),
        MasterDnsResolverNode("r7", "Clean DNS Edge Backup", "223.6.6.6", 53, DnsTransport.UDP, 16, 0.0, 92, true),
        MasterDnsResolverNode("r8", "Tencent DoT Secondary", "1.12.12.12", 853, DnsTransport.DOT, 20, 0.0, 85, true)
    )
)
