package com.unifiedshield.stormdns

enum class StormDnsBalancing(val label: String, val labelPersian: String) {
    LOWEST_LATENCY("Lowest Latency (Fastest RTT)", "کمترین زمان رفت و برگشت (سریع‌ترین RTT)"),
    LEAST_LOSS("Least Loss (Lowest Drop Rate)", "کمترین افت بسته (حداکثر پایداری)"),
    ROUND_ROBIN("Round Robin (Uniform Load)", "چرخشی مساوی میان رِزولورها"),
    RANDOM("Random Selection", "انتخاب تصادفی")
}

enum class StormDnsCompression(val label: String) {
    ZSTD("ZSTD (High Ratio)"),
    LZ4("LZ4 (Ultra Fast)"),
    ZLIB("ZLIB (Standard)"),
    NONE("None (Raw Packets)")
}

enum class StormDnsCipher(val label: String) {
    AES_128_GCM("AES-128-GCM (Hardware Accelerated)"),
    AES_192_GCM("AES-192-GCM"),
    AES_256_GCM("AES-256-GCM (Military Grade)"),
    CHACHA20("ChaCha20-Poly1305 (Mobile Friendly)"),
    XOR("XOR (Zero Overhead)"),
    NONE("None (No Encryption)")
}

enum class StormDnsEncoding(val label: String) {
    BASE32("Base32 (Case Insensitive DNS Safe)"),
    BASE64("Base64 (High Density)"),
    HEX("Hexadecimal (Compatible)")
}

data class StormDnsResolverNode(
    val id: String,
    val address: String,
    val port: Int = 53,
    val latencyMs: Int,
    val packetLossPct: Double,
    val discoveredMtu: Int = 1232,
    val isActive: Boolean = true,
    val requestsSent: Long = 1420,
    val responsesReceived: Long = 1390
)

data class StormDnsDuplicationControls(
    val dataDuplication: Int = 1,     // 1x - 3x
    val ackDuplication: Int = 2,      // 1x - 4x (uplink bottleneck protection)
    val setupDuplication: Int = 3,    // 2x - 5x (handshake survival)
    val controlDuplication: Int = 2   // 1x - 3x (keepalive/ping)
)

data class StormDnsState(
    val isTunnelRunning: Boolean = false,
    val localSocks5Port: Int = 10853,
    val localListenAddress: String = "127.0.0.1",
    val tunnelDomain: String = "st.unifiedshield.net",
    val encryptionKey: String = "k8f9a2b7c4d1e6f03948572019a8b7c6",
    val balancing: StormDnsBalancing = StormDnsBalancing.LEAST_LOSS,
    val compression: StormDnsCompression = StormDnsCompression.ZSTD,
    val cipher: StormDnsCipher = StormDnsCipher.AES_128_GCM,
    val encoding: StormDnsEncoding = StormDnsEncoding.BASE32,
    val activeMtu: Int = 1232,
    val isAutoMtuDiscoveryEnabled: Boolean = true,
    val duplicationControls: StormDnsDuplicationControls = StormDnsDuplicationControls(),
    val logBasedStartupEnabled: Boolean = true,
    val localDnsCacheEnabled: Boolean = true,
    val bytesTransmitted: Long = 842000,
    val bytesReceived: Long = 4920000,
    val arqRetransmissions: Long = 48,
    val dynamicRtoMs: Int = 32,
    val activeStreamsCount: Int = 6,
    val resolvers: List<StormDnsResolverNode> = emptyList()
)
