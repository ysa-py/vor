package com.unifiedshield.tunnel

enum class TunnelType(
    val id: String,
    val title: String,
    val titlePersian: String,
    val protocol: String,
    val description: String,
    val descriptionPersian: String,
    val isDpiResistant: Boolean,
    val isDefaultRecommended: Boolean = false,
    val hasSshChaining: Boolean = false
) {
    MASTER_DNS(
        id = "master_dns",
        title = "MasterDnsVPN",
        titlePersian = "مستر دی‌ان‌اس (پروتکل اختصاصی + ARQ)",
        protocol = "Custom ARQ-5B (AES/ChaCha/XOR)",
        description = "Ultra-fast DNS tunnel with custom 5-7B ARQ protocol, 8 multi-resolver balancing modes, up to 9x faster than DNSTT, and 0.270s handshakes.",
        descriptionPersian = "پیشرفته‌ترین تونل DNS با پروتکل اختصاصی و هدر فوق‌سبک ARQ (فقط ۵ تا ۷ بایت)، تا ۹ برابر سریع‌تر از DNSTT و ۳.۶ برابر سریع‌تر از Slipstream، ۸ حالت متعادل‌سازی چند سروری و پشتیبانی از MTUهای بسیار کوچک.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    MASTER_DNS_SSH(
        id = "master_dns_ssh",
        title = "MasterDnsVPN + SSH",
        titlePersian = "مستر دی‌ان‌اس + اس‌اس‌اچ (چند مسیره)",
        protocol = "Custom ARQ + SSH Multi-Hop",
        description = "MasterDnsVPN combined with SSH chaining for impenetrable zero-leak DNS defense and multi-carrier TCP transport.",
        descriptionPersian = "ترکیب MasterDnsVPN با زنجیره چندمسیره SSH برای امنیت دولایه، حمل ترافیک‌های TCP (Shadowsocks/VLESS) و عبور پایدار از شدیدترین فیلترینگ.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = true
    ),
    DNSTT(
        id = "dnstt",
        title = "DNSTT",
        titlePersian = "دی‌ان‌اس‌تی‌تی (پیش‌فرض)",
        protocol = "KCP + Noise",
        description = "Stable and reliable DNS tunneling. Default and recommended tunnel type for most users.",
        descriptionPersian = "تونل‌سازی پایدار و قابل اعتماد DNS با پروتکل KCP و رمزنگاری Noise. گزینه پیش‌فرض و پیشنهادی.",
        isDpiResistant = true,
        isDefaultRecommended = true,
        hasSshChaining = false
    ),
    DNSTT_SSH(
        id = "dnstt_ssh",
        title = "DNSTT + SSH",
        titlePersian = "دی‌ان‌اس‌تی‌تی + اس‌اس‌اچ (زنجیره‌ای)",
        protocol = "KCP + Noise + SSH",
        description = "DNSTT with SSH chaining for zero DNS leaks and dual layer cryptographic encapsulation.",
        descriptionPersian = "تونل DNSTT زنجیره‌شده با SSH جهت جلوگیری ۱۰۰٪ از نشت DNS و رمزنگاری دولایه.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = true
    ),
    NOIZ_DNS(
        id = "noizdns",
        title = "NoizDNS",
        titlePersian = "نویز دی‌ان‌اس (مقاوم در برابر فیلترینگ)",
        protocol = "KCP + Noise",
        description = "DPI-resistant DNS tunneling with optional stealth mode and dynamic entropy modulation.",
        descriptionPersian = "تونل DNS بسیار مقاوم در برابر سامانه‌های هوشمند DPI همراه با حالت استتار ویژه شبکه‌های مسدود.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    NOIZ_DNS_SSH(
        id = "noizdns_ssh",
        title = "NoizDNS + SSH",
        titlePersian = "نویز دی‌ان‌اس + اس‌اس‌اچ",
        protocol = "KCP + Noise + SSH",
        description = "NoizDNS with SSH chaining for extra security layer against targeted packet inspection.",
        descriptionPersian = "ترکیب NoizDNS با تونل SSH برای استتار حداکثری و امنیت بالا در شرایط سانسور شدید.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = true
    ),
    VAY_DNS(
        id = "vaydns",
        title = "VayDNS",
        titlePersian = "وای دی‌ان‌اس (قالب بهینه‌سازی شده)",
        protocol = "KCP + Noise",
        description = "Optimized DNS tunneling with configurable wire format, record types, QNAME lengths, and rate limiting.",
        descriptionPersian = "تونل DNS بهینه‌سازی‌شده با قابلیت تنظیم قالب سیم (Wire Format)، انواع رکورد (TXT/NULL/CNAME)، طول QNAME و کنترل ترافیک.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    VAY_DNS_SSH(
        id = "vaydns_ssh",
        title = "VayDNS + SSH",
        titlePersian = "وای دی‌ان‌اس + اس‌اس‌اچ",
        protocol = "KCP + Noise + SSH",
        description = "VayDNS with SSH chaining for maximum throughput and zero DNS leakage.",
        descriptionPersian = "تونل VayDNS به همراه زنجیره SSH جهت دریافت بالاترین سرعت و جلوگیری از نشت ترافیک.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = true
    ),
    SLIPSTREAM(
        id = "slipstream",
        title = "Slipstream",
        titlePersian = "اسلیپ‌استریم (QUIC توربو)",
        protocol = "QUIC",
        description = "High-performance QUIC tunneling with 0-RTT handshakes and multiplexed UDP transport.",
        descriptionPersian = "تونل کوئیک با کارایی بالا، اتصال صفر-میلی‌ثانیه‌ای (0-RTT) و چندگانه‌سازی روی پروتکل UDP.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    SLIPSTREAM_SSH(
        id = "slipstream_ssh",
        title = "Slipstream + SSH",
        titlePersian = "اسلیپ‌استریم + اس‌اس‌اچ",
        protocol = "QUIC + SSH",
        description = "Slipstream QUIC transport with SSH encapsulation for impenetrable privacy.",
        descriptionPersian = "بستر پرسرعت Slipstream QUIC با کپسوله‌سازی SSH برای حفظ حریم خصوصی کامل.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = true
    ),
    SSH_STANDALONE(
        id = "ssh_standalone",
        title = "SSH",
        titlePersian = "اس‌اس‌اچ مستقیم (SSH Standalone)",
        protocol = "SSH",
        description = "Standalone SSH tunnel (no DNS tunneling) with TLS, WebSocket, or Payload Injection wrappers.",
        descriptionPersian = "تونل مستقل SSH با قابلیت بسته‌بندی در TLS (فرانتینگ دامنه)، وب‌سوکت CDN و تزریق هدرهای فریبنده.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = true
    ),
    NAIVE_PROXY(
        id = "naiveproxy",
        title = "NaiveProxy",
        titlePersian = "نایو پروکسی (کروم JA4/JA3)",
        protocol = "HTTPS (Chromium)",
        description = "HTTPS tunnel with authentic Chrome TLS fingerprinting (JA4) to completely evade DPI classifiers.",
        descriptionPersian = "تونل امن HTTPS با اثرانگشت واقعی مرورگر کروم و لایه کرومیوم برای عبور بی‌نقص از اسکنرهای هوشمند.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    NAIVE_PROXY_SSH(
        id = "naiveproxy_ssh",
        title = "NaiveProxy + SSH",
        titlePersian = "نایو پروکسی + اس‌اس‌اچ",
        protocol = "HTTPS + SSH",
        description = "NaiveProxy with SSH chaining for extra encryption over authentic Chrome TLS.",
        descriptionPersian = "ترکیب NaiveProxy با تونل امن SSH برای رمزنگاری فوق‌العاده قوی بر بستر ترافیک کروم.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = true
    ),
    DOH(
        id = "doh",
        title = "DOH",
        titlePersian = "دی‌ان‌اس روی پروتکل امن (DoH RFC 8484)",
        protocol = "DNS over HTTPS",
        description = "DNS-only encryption via HTTPS (RFC 8484) to eliminate DNS tampering without proxying other traffic.",
        descriptionPersian = "رمزنگاری اختصاصی کوئری‌های DNS از طریق HTTPS بدون دستکاری سایر ترافیک‌ها.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    TOR(
        id = "tor",
        title = "Tor",
        titlePersian = "شبکه تور (Tor Snowflake / obfs4 / Meek)",
        protocol = "Tor Network",
        description = "Connect via Tor with Snowflake WebRTC brokers, obfs4 obfuscation, Meek Azure, or custom bridges.",
        descriptionPersian = "اتصال به شبکه تور از طریق پل‌های Snowflake، مبدل‌های obfs4، تکنیک Meek و پل‌های سفارشی.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    STORM_DNS(
        id = "storm_dns",
        title = "StormDNS",
        titlePersian = "استورم دی‌ان‌اس (TCP در DNS + تکثیر جهت‌دار)",
        protocol = "TCP-over-DNS + ARQ + ZSTD",
        description = "Moves TCP streams through DNS queries/responses over UDP/53 with SOCKS5 listener, directional duplication (ACK/Setup), and ZSTD/LZ4 compression.",
        descriptionPersian = "انتقال ترافیک TCP در بستر کوئری‌های DNS با شنود محلی SOCKS5، تکثیر هوشمند جهتی بسته‌ها برای شکست بن‌بست آپلود، پنجره‌های ARQ و فشرده‌سازی ZSTD.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    COTTEN_DNS(
        id = "cotten_dns",
        title = "CottenDNS",
        titlePersian = "کاتن دی‌ان‌اس (تطبیقی چندمسیره + Super-FEC)",
        protocol = "Adaptive Multi-Transport + Super-FEC",
        description = "Adaptive independent scoring per (resolver, transport), Super-FEC parity recovery, early poison racing, and anti-DPI record rotation.",
        descriptionPersian = "سامانه تطبیقی با ارزیابی مستقل هر مسیر (UDP/TCP/DoT/DoH)، کشف پویای MTU، تصحیح خطای پیش‌رو (Super-FEC)، مسابقه سریع با جعل DNS و چرخش رکوردها.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    VLESS_REALITY(
        id = "vless_reality",
        title = "VLESS Reality",
        titlePersian = "وی‌لس ریالیتی (Vision / XTLS)",
        protocol = "VLESS + Reality + Vision",
        description = "Zero-certificate direct TLS 1.3 masquerading borrowing legitimate server certificates without SNI detection.",
        descriptionPersian = "پروتکل VLESS Reality با استتار مستقیم روی سرورهای معتبر خارجی بدون نیاز به دامنه شخصی و فاقد هرگونه نشانه فیلترینگ.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    VMESS_AEAD(
        id = "vmess_aead",
        title = "VMess AEAD",
        titlePersian = "وی‌مس AEAD (وب‌سوکت CDN)",
        protocol = "VMess + AEAD + WebSocket",
        description = "AEAD-encrypted VMess stream over Cloudflare/Fastly CDN WebSockets with TLS domain fronting.",
        descriptionPersian = "پروتکل امن VMess AEAD بر بستر وب‌سوکت‌های توزیع‌شده CDN همراه با فرانتینگ دامنه.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    TROJAN_TLS(
        id = "trojan_tls",
        title = "Trojan",
        titlePersian = "تروجان (gRPC / TLS 1.3)",
        protocol = "Trojan + TLS 1.3 + gRPC",
        description = "Authentic HTTPS traffic mimicry with valid certificates, ALPN negotiation, and gRPC multiplexing.",
        descriptionPersian = "شبیه‌سازی کامل وب‌سایت‌های عادی HTTPS با هندشیک واقعی TLS 1.3 و چندگانه‌سازی چندمسیره gRPC.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    SHADOWSOCKS_2022(
        id = "shadowsocks_2022",
        title = "Shadowsocks 2022",
        titlePersian = "شادوساکس ۲۰۲۲ (Blake3 / AEAD)",
        protocol = "Shadowsocks 2022-blake3",
        description = "Next-generation replay-resistant AEAD cipher protocol with blake3 session subkeys and UDP sessions.",
        descriptionPersian = "شادوساکس ۲۰۲۲ با الگوریتم Blake3 و کلیدهای چرخشی برای جلوگیری قطعی از حملات بازپخش (Replay Attacks).",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    HYSTERIA_2(
        id = "hysteria_2",
        title = "Hysteria 2",
        titlePersian = "هیستریا ۲ (Brutal UDP Congestion)",
        protocol = "Hysteria 2 + Salamander",
        description = "Ultra-fast custom UDP congestion control overcoming 80%+ packet loss with Salamander SNI obfuscation.",
        descriptionPersian = "کنترل ازدحام تهاجمی UDP با غلبه بر افت بسته بالای ۸۰٪ در اختلالات شدید شبکه همراه با استتار سالاماندر.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    TUIC_V5(
        id = "tuic_v5",
        title = "TUIC v5",
        titlePersian = "توئیک نسخه ۵ (BBR QUIC 0-RTT)",
        protocol = "TUIC v5 + QUIC",
        description = "BBR-driven zero-RTT transport directly over QUIC with concurrent multi-stream multiplexing.",
        descriptionPersian = "پروتکل پرسرعت توئیک بر بستر QUIC با الگوریتم BBR و اتصال صفر میلی‌ثانیه‌ای.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    ANY_TLS(
        id = "any_tls",
        title = "AnyTLS",
        titlePersian = "انی تی‌ال‌اس (پلی‌مورفیک ClientHello)",
        protocol = "Polymorphic AnyTLS",
        description = "Dynamic ClientHello polymorphism defeating statistical AI and signature-based DPI classifiers.",
        descriptionPersian = "پروتکل پلی‌مورفیک با تغییر شکل مداوم بایت‌های هندشیک TLS جهت خنثی‌سازی یادگیری ماشین فیلترینگ.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    WIREGUARD_AMNEZIA(
        id = "amnezia_wg",
        title = "AmneziaWG",
        titlePersian = "امنزیا وایرگارد (تزریق بایت‌های فریبنده)",
        protocol = "AmneziaWG (Junk Packet Obf)",
        description = "WireGuard kernel protocol with randomized header signatures and junk packet padding.",
        descriptionPersian = "پروتکل کرنل وایرگارد با تزریق بسته‌های فریبنده و هدرهای تصادفی جهت عبور بدون مسدودی از فیلتر UDP.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    SHADOW_TLS(
        id = "shadow_tls",
        title = "ShadowTLS v3",
        titlePersian = "شدو تی‌ال‌اس ۳ (شبیه‌ساز دقیق TLS)",
        protocol = "ShadowTLS v3 + SNI Mask",
        description = "Authentic TLS 1.3 handshake forwarding and mimicry to whitelisted cloud edge endpoints.",
        descriptionPersian = "ارسال هندشیک واقعی TLS 1.3 به سایت‌های مجاز برای عبور بدون کوچکترین شانس تشخیص توسط DPI.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    BROOK(
        id = "brook",
        title = "Brook",
        titlePersian = "بروک (پروتکل سبک ضد فیلتر)",
        protocol = "Brook Stream",
        description = "Ultra-lightweight zero-fingerprint tunnel designed for robust anti-censorship.",
        descriptionPersian = "پروتکل سبک، سریع و بدون امضا جهت برقراری تونل پایدار در شبکه‌های محدود شده.",
        isDpiResistant = true,
        isDefaultRecommended = false,
        hasSshChaining = false
    ),
    // ── MICAFP Master Directive v6 / Section B3.2 — additive protocol entry (A1: zero deletion, additive only) ──
    SOCKS5_PROXY(
        id = "socks5_proxy",
        title = "SOCKS5 Proxy",
        titlePersian = "ساکس‌فایو (پروکسی مستقیم)",
        protocol = "SOCKS5 (RFC 1928)",
        description = "Direct SOCKS5 proxy transport with optional remote DNS resolution; useful as diagnostic baseline and as carrier for chained transports.",
        descriptionPersian = "پروکسی مستقیم SOCKS5 با امکان رزولوشن DNS از سمت سرور؛ مناسب به‌عنوان مبنای تشخیصی و حامل پروتکل‌های زنجیره‌ای.",
        isDpiResistant = false,
        isDefaultRecommended = false,
        hasSshChaining = false
    )
}

enum class DnsTransport(val label: String, val port: Int) {
    UDP("UDP", 53),
    DOT("DNS-over-TLS (DoT)", 853),
    DOH("DNS-over-HTTPS (DoH)", 443)
}

enum class SshCipher(val label: String, val id: String) {
    AES_128_GCM("AES-128-GCM (Recommended)", "aes128-gcm@openssh.com"),
    CHACHA20_POLY1305("ChaCha20-Poly1305", "chacha20-poly1305@openssh.com"),
    AES_128_CTR("AES-128-CTR", "aes128-ctr")
}

enum class SshWrapperType(val label: String, val labelPersian: String) {
    DIRECT("Direct TCP", "اتصال مستقیم TCP"),
    TLS("SSH over TLS (Custom SNI)", "پوشش در TLS با فرانتینگ دامنه"),
    WEBSOCKET("SSH over WebSocket (ws/wss)", "پوشش در وب‌سوکت روی CDN"),
    HTTP_CONNECT("SSH over HTTP CONNECT", "پراکسی HTTP CONNECT با هدر دلخواه"),
    PAYLOAD_INJECTION("SSH Payload Injection", "تزریق بایت‌های فریبنده قبل از هندشیک")
}

enum class VayDnsRecordType(val label: String) {
    TXT("TXT (Standard)"),
    NULL("NULL (Low Overhead)"),
    CNAME("CNAME (Stealth)"),
    AAAA("AAAA (IPv6 Alias)"),
    MX("MX (Mail Exchange)")
}

enum class TorBridgeType(val label: String, val labelPersian: String) {
    SNOWFLAKE("Snowflake (WebRTC Broker)", "پل اسنوفلیک بر بستر وب‌آر‌تی‌سی"),
    OBFS4("obfs4 (Pluggable Transport)", "پل مبدل obfs4"),
    MEEK_AZURE("Meek Azure (Domain Fronting)", "پل میک با فرانتینگ مایکروسافت آژور"),
    CUSTOM("Custom Bridge", "پل اختصاصی کاربر")
}

data class SshConfig(
    val host: String = "104.21.68.12",
    val port: Int = 22,
    val username: String = "shield_user",
    val password: String = "auto_ephemeral_key",
    val cipher: SshCipher = SshCipher.AES_128_GCM,
    val wrapperType: SshWrapperType = SshWrapperType.TLS,
    val customSni: String = "speedtest.net",
    val wsPath: String = "/unified-ssh",
    val httpProxyHost: String = "104.16.132.229",
    val httpProxyPort: Int = 8080,
    val customHostHeader: String = "cdn.cloudflare.net",
    val rawPayloadTemplate: String = "GET / HTTP/1.1\r\nHost: [host]\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n"
)

data class DnsConfig(
    val transport: DnsTransport = DnsTransport.UDP,
    val resolverIpOrUrl: String = "223.5.5.5",
    val serverDomain: String = "t.unifiedshield.net",
    val noisePublicKey: String = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    val ednsSubnetEnabled: Boolean = false,
    val ednsBufferSize: Int = 1232
)

data class VayDnsConfig(
    val recordType: VayDnsRecordType = VayDnsRecordType.TXT,
    val qnameLength: Int = 64,
    val rateLimitPps: Int = 60,
    val customPaddingSeed: String = "VAY-ENTROPY-SEED"
)

data class NoizDnsConfig(
    val stealthModeEnabled: Boolean = true,
    val noisePattern: String = "Noise_IK_25519_ChaChaPoly_BLAKE2s",
    val jitterMs: Int = 15
)

data class NaiveProxyConfig(
    val host: String = "naive.unifiedshield.net",
    val port: Int = 443,
    val username: String = "user",
    val password: String = "pass",
    val ja4Fingerprint: String = "t13d1516h2_8daaf6152771_0271d4a82a09",
    val paddingEnabled: Boolean = true
)

data class TorConfig(
    val bridgeType: TorBridgeType = TorBridgeType.SNOWFLAKE,
    val snowflakeBrokerUrl: String = "https://snowflake-broker.torproject.net/",
    val snowflakeFrontDomain: String = "cdn.sstatic.net",
    val obfs4BridgeLine: String = "obfs4 192.0.2.1:443 1234567890ABCDEF1234567890ABCDEF12345678 cert=xyz iat-mode=0"
)

data class LocalProxyConfig(
    val socks5Port: Int = 10808,
    val httpPort: Int = 10809,
    val listenAddress: String = "127.0.0.1",
    val allowLan: Boolean = false
)

data class TunnelProfile(
    val id: String,
    val name: String,
    val namePersian: String,
    val tunnelType: TunnelType,
    val isActive: Boolean = false,
    val pingMs: Int = 22,
    val throughputRating: String = "92.4 MB/s",
    val isAutoDiscovered: Boolean = false,
    val sshConfig: SshConfig = SshConfig(),
    val dnsConfig: DnsConfig = DnsConfig(),
    val vayDnsConfig: VayDnsConfig = VayDnsConfig(),
    val noizDnsConfig: NoizDnsConfig = NoizDnsConfig(),
    val naiveProxyConfig: NaiveProxyConfig = NaiveProxyConfig(),
    val torConfig: TorConfig = TorConfig(),
    val masterDnsConfig: MasterDnsConfig = MasterDnsConfig()
)
