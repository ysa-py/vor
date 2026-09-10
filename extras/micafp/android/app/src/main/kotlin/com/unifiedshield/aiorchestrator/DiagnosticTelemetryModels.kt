package com.unifiedshield.aiorchestrator

data class CoreScoreEntry(
    val coreId: String,
    val name: String,
    val protocolType: String,
    val score: Double, // 0.0 - 100.0
    val latencyMs: Long,
    val packetLossPct: Double,
    val handshakeSuccessRate: Double, // 0.0 - 1.0
    val jitterMs: Double,
    val consecutiveFailures: Int = 0,
    val isBlacklisted: Boolean = false,
    val blacklistedUntilMs: Long = 0L,
    val isActive: Boolean = false
) {
    fun isAvailable(currentTimeMs: Long): Boolean {
        return !isBlacklisted || currentTimeMs >= blacklistedUntilMs
    }
}

data class TelemetryDataPoint(
    val timestamp: Long,
    val rttMs: Float,
    val packetLossPct: Float,
    val censorshipPressure: Float, // 0 - 100
    val dpiBlocksDetected: Int
)

data class AnomalyDetectionEvent(
    val id: String = java.util.UUID.randomUUID().toString().substring(0, 8),
    val timestamp: String,
    val signatureName: String,
    val confidence: Float,
    val interceptedHeader: String,
    val triggeredCoreSwitch: Boolean,
    val switchedToCore: String? = null
)
