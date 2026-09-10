package com.unifiedshield.whitedns

enum class WhiteDnsScanType(val label: String, val labelPersian: String) {
    IP_CIDR("IP / CIDR Expansion Scan", "اسکن و انبساط رنج‌های IP و CIDR"),
    DNS_RESOLVER("DNS Resolver Probe (UDP/TCP/DoT/DoH)", "اسکن و تست رِزولورهای DNS با uTLS"),
    SNI_SCANNER("SNI & TLS 1.3/ECH Scanner", "اسکنر SNI و وب‌سایت‌های فرانتینگ"),
    HTTP_PROXY("HTTP/HTTPS Proxy Verifier", "تست و اعتبارسنجی پراکسی‌های HTTP"),
    SOCKS5_PROXY("SOCKS5 Proxy Verifier", "تست و ارزیابی پراکسی‌های SOCKS5"),
    ASN_EXPORT("ASN Clean-IP Extractor", "استخراج IPهای تمیز بر اساس ASN")
}

enum class WhiteDnsScanDepth(val label: String, val labelPersian: String) {
    FAST("Fast (Core A-Integrity + EDNS + TXT)", "سریع (تست‌های اصلی سلامت و تونل TXT)"),
    FULL("Full (All Probes + NXDOMAIN Hijack + Jitter)", "کامل (تمام تست‌ها + بررسی جعل و ربایش)")
}

enum class WhiteDnsDnsProtocol(val port: Int, val protocolName: String) {
    UDP_53(53, "UDP Plain"),
    TCP_53(53, "TCP Plain"),
    DOT_853(853, "DoT (DNS-over-TLS uTLS)"),
    DOH_443(443, "DoH (DNS-over-HTTPS RFC8484)")
}

enum class WhiteDnsExportFormat(val extension: String, val mimeType: String) {
    JSON("json", "application/json"),
    CSV("csv", "text/csv"),
    TXT("txt", "text/plain"),
    XLSX("xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
}

data class WhiteDnsAsnDataset(
    val asn: String,
    val organization: String,
    val country: String,
    val defaultCidrs: List<String>,
    val totalIps: Long
)

data class WhiteDnsScanResult(
    val id: String,
    val target: String,
    val scanType: WhiteDnsScanType,
    val asn: String,
    val org: String,
    val pingLatencyMs: Int,
    val isClean: Boolean,
    val aRecordIntegrity: Boolean,
    val recursionAvailable: Boolean,
    val ednsBufferSize: Int,
    val txtTunnelPassthrough: Boolean,
    val nxDomainHijacked: Boolean,
    val tlsUtlsHandshakeOk: Boolean,
    val httpSocksWorking: Boolean,
    val downloadSpeedMbps: Double,
    val ratingScore: Int, // 0 - 100
    val verifiedAtEpochMs: Long = System.currentTimeMillis()
)

data class WhiteDnsScannerState(
    val isScanning: Boolean = false,
    val isPaused: Boolean = false,
    val selectedScanType: WhiteDnsScanType = WhiteDnsScanType.DNS_RESOLVER,
    val scanDepth: WhiteDnsScanDepth = WhiteDnsScanDepth.FAST,
    val concurrencyWorkers: Int = 16,
    val inputCidrOrDomain: String = "104.16.0.0/16",
    val totalTargets: Int = 256,
    val scannedTargets: Int = 0,
    val cleanTargetsFound: Int = 0,
    val currentScanningTarget: String = "",
    val progressPercentage: Float = 0f,
    val currentSpeedPps: Int = 0,
    val activeProtocol: WhiteDnsDnsProtocol = WhiteDnsDnsProtocol.DOT_853,
    val results: List<WhiteDnsScanResult> = emptyList(),
    val exportStatusMessage: String? = null
)
