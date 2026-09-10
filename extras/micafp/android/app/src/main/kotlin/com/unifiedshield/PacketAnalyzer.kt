package com.unifiedshield

import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlin.math.abs

/**
 * Real-time Outgoing Packet Analyzer for Pre-Emptive DPI Evasion.
 * Inspects TCP/UDP packet dynamics, TCP SYN retransmission rates,
 * and RST injections to trigger proactive core switching BEFORE full blockage.
 */
data class PacketAnomalyState(
    val totalPacketsAnalyzed: Long = 0,
    val tcpRstSpikeDetected: Boolean = false,
    val sniTamperingDetected: Boolean = false,
    val preEmptiveSwitchRequested: Boolean = false,
    val anomalyScore: Double = 0.05,
    val lastDetectedPattern: String = "Normal Flow"
)

class PacketAnalyzer private constructor() {

    private val TAG = "PacketAnalyzer"

    private val _anomalyState = MutableStateFlow(PacketAnomalyState())
    val anomalyState: StateFlow<PacketAnomalyState> = _anomalyState

    private var rstWindow = mutableListOf<Long>()
    private var synRetransmitWindow = mutableListOf<Long>()
    private var totalPackets = 0L

    /**
     * Inspect outgoing packet bytes, latency, and packet loss dynamics in real-time.
     */
    fun analyzeOutgoingPacket(
        packetSize: Int,
        isSynRetransmit: Boolean,
        isRstReceived: Boolean,
        latencyMs: Long = 30L,
        packetLossRate: Double = 0.05,
        onPreEmptiveSwitchNeeded: (reason: String) -> Unit
    ) {
        totalPackets++
        val currentTime = System.currentTimeMillis()

        if (isRstReceived) {
            rstWindow.add(currentTime)
        }
        if (isSynRetransmit) {
            synRetransmitWindow.add(currentTime)
        }

        // Clean window older than 10 seconds
        val cutoff = currentTime - 10000
        rstWindow.removeAll { it < cutoff }
        synRetransmitWindow.removeAll { it < cutoff }

        val rstRatePerSec = rstWindow.size / 10.0
        val synRetransmitRate = synRetransmitWindow.size / 10.0

        // High latency or high packet loss indicator (> 25% loss or > 450ms latency)
        val isLossHigh = packetLossRate > 0.25 || latencyMs > 450L

        // Calculate Anomaly Score (0.0 to 1.0)
        val score = ((rstRatePerSec * 0.25) + (synRetransmitRate * 0.35) + (if (isLossHigh) 0.30 else 0.0)).coerceIn(0.0, 1.0)
        val isSpike = score > 0.55
        val isSniTampered = isSynRetransmit && synRetransmitWindow.size > 3

        val pattern = when {
            isSniTampered -> "DPI Active SNI Filtering / RST Injection"
            isLossHigh -> "Pre-Emptive: High Packet Loss ($packetLossRate) / Latency Spike (${latencyMs}ms)"
            isSpike -> "High Retransmission Spike (Carrier Blockage)"
            else -> "Clean Outgoing Stream"
        }

        val state = PacketAnomalyState(
            totalPacketsAnalyzed = totalPackets,
            tcpRstSpikeDetected = rstRatePerSec > 1.5,
            sniTamperingDetected = isSniTampered,
            preEmptiveSwitchRequested = isSpike,
            anomalyScore = score,
            lastDetectedPattern = pattern
        )
        _anomalyState.value = state

        if (isSpike) {
            Log.w(TAG, "PROACTIVE ANALYZER TRIGGERED: DPI Anomaly score=$score pattern=$pattern. Hot-swapping to shadow core...")
            onPreEmptiveSwitchNeeded(pattern)
        }
    }

    fun reset() {
        rstWindow.clear()
        synRetransmitWindow.clear()
        totalPackets = 0
        _anomalyState.value = PacketAnomalyState()
    }

    companion object {
        @Volatile
        private var instance: PacketAnalyzer? = null

        fun getInstance(): PacketAnalyzer {
            return instance ?: synchronized(this) {
                instance ?: PacketAnalyzer().also { instance = it }
            }
        }
    }
}
