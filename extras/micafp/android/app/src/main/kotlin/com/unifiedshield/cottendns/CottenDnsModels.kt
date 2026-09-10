package com.unifiedshield.cottendns

enum class CottenTransportType(val port: Int, val label: String) {
    UDP_53(53, "UDP/53 (Fast Core)"),
    TCP_53(53, "TCP/53 (Reliable Fallback)"),
    DOT_853(853, "DoT/853 (Encrypted TLS)"),
    DOH_443(443, "DoH/443 (HTTPS Web Stream)")
}

enum class CottenFecMode(val label: String, val parityOverheadPct: Int) {
    OFF("FEC Disabled", 0),
    STANDARD_FEC("Standard FEC (Reed-Solomon 10:2)", 20),
    SUPER_FEC("Super-FEC (Fountain Parity 8:4 - Harsh Loss)", 50)
}

enum class CottenRecordRotation(val label: String) {
    TXT_ONLY("TXT Records Only"),
    DYNAMIC_ROTATION("Dynamic (TXT + NULL + CNAME + AAAA + MX)"),
    STEALTH_NULL("NULL Records (Zero DNS Overhead)")
}

data class CottenPathMetric(
    val id: String,
    val resolver: String,
    val transport: CottenTransportType,
    val uploadDeliveryPct: Double,
    val downloadDeliveryPct: Double,
    val directionalRttMs: Int,
    val pathMtu: Int = 1232,
    val confidenceScore: Int = 98, // 0 - 100
    val isPoisonAlertTriggered: Boolean = false,
    val isCurrentlyActive: Boolean = true
)

data class CottenDnsState(
    val isEngineRunning: Boolean = true,
    val autoAdaptiveTransportEnabled: Boolean = true,
    val fecMode: CottenFecMode = CottenFecMode.STANDARD_FEC,
    val recordRotation: CottenRecordRotation = CottenRecordRotation.DYNAMIC_ROTATION,
    val qnameReshapingEnabled: Boolean = true,
    val randomizeTransactionIds: Boolean = true,
    val earlyPoisonRacingEnabled: Boolean = true,
    val equalPathStripingEnabled: Boolean = true,
    val singleCongestionBudgetPct: Int = 25,
    val inFlightFrameReplayCount: Long = 18,
    val poisonAttemptsDefeated: Long = 7,
    val fecFramesRecovered: Long = 142,
    val paths: List<CottenPathMetric> = emptyList()
)
