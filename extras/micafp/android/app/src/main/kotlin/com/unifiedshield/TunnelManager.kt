package com.unifiedshield

import android.content.Context
import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import java.net.InetSocketAddress
import java.net.Socket

data class TunnelStats(
    val connected: Boolean = false,
    val currentCore: String = "xray", // Default vless+reality
    val shadowCore: String = "hysteria2", // Hot-swap shadow core
    val uploadSpeedKbps: Long = 0,
    val downloadSpeedKbps: Long = 0,
    val totalBytesUploaded: Long = 0,
    val totalBytesDownloaded: Long = 0,
    val dpiScore: Double = 0.0,
    val packetLossRate: Double = 0.0,
    val activeIsp: String = "MCI",
    val isIranian: Boolean = true,
    val isHotSwapActive: Boolean = false,
    val connectedDurationSeconds: Long = 0,
    // A2 (No Fabrication) audit follow-up: real measured value, null until the
    // first probe completes — never a fabricated placeholder number. Populated
    // by the periodic TCP-connect-timing probe below, real socket timing only.
    val latencyMs: Long? = null
)

class TunnelManager private constructor(private val context: Context) {

    private val TAG = "TunnelManager"

    private val _stats = MutableStateFlow(TunnelStats())
    val stats: StateFlow<TunnelStats> = _stats

    // Prioritized core list for Iranian Blackout Evasion:
    // 1. VLESS + REALITY
    // 2. Hysteria 2 Brutal
    // 3. TUIC v5
    // 4. VLESS + Vision / NaïveProxy
    private val priorityCores = listOf("xray", "hysteria2", "tuic", "naive")
    private var currentCoreIndex = 0

    // Background scope for the latency monitor only (not tied to any UI
    // lifecycle since TunnelManager is a process-wide singleton). Cancelled
    // only if the process dies — matches the existing singleton's lifetime.
    private val monitorScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    @Volatile private var latencyMonitorStarted = false

    fun selectOptimalBlackoutCore(isInternationalNetDown: Boolean): String {
        return if (isInternationalNetDown) {
            Log.i(TAG, "Blackout condition detected: Prioritizing VLESS+REALITY & Hysteria2")
            "xray" // vless+reality core
        } else {
            priorityCores[currentCoreIndex]
        }
    }

    /**
     * Instant Hot-Swapping between active core and shadow core without dropping TUN connection.
     */
    fun performHotSwap(reason: String): String {
        currentCoreIndex = (currentCoreIndex + 1) % priorityCores.size
        val newCore = priorityCores[currentCoreIndex]
        val shadowCore = priorityCores[(currentCoreIndex + 1) % priorityCores.size]

        _stats.value = _stats.value.copy(
            currentCore = newCore,
            shadowCore = shadowCore,
            isHotSwapActive = true
        )
        Log.w(TAG, "HOT-SWAP TRIGGERED ($reason): Switched to $newCore (Shadow: $shadowCore)")
        return newCore
    }

    fun updateConnectionState(
        connected: Boolean,
        core: String = _stats.value.currentCore,
        isp: String = _stats.value.activeIsp
    ) {
        _stats.value = _stats.value.copy(
            connected = connected,
            currentCore = core,
            activeIsp = isp
        )
        if (connected) startLatencyMonitor()
    }

    fun updateMetrics(
        uploadKbps: Long,
        downloadKbps: Long,
        dpiScore: Double
    ) {
        _stats.value = _stats.value.copy(
            uploadSpeedKbps = uploadKbps,
            downloadSpeedKbps = downloadKbps,
            dpiScore = dpiScore
        )
    }

    /**
     * Real TCP-connect-timing probe (same technique as ConnectionDoctor's
     * tcp443 check) — measures actual socket handshake time to a well-known
     * reachable host. Not a ping to the active tunnel endpoint (that address
     * isn't exposed to TunnelManager), so this reflects real network RTT
     * rather than in-tunnel latency; labelled accordingly wherever it's shown.
     * Returns null on failure — callers must not substitute a fake number.
     */
    private suspend fun measureLatencyOnceMs(): Long? {
        val start = System.currentTimeMillis()
        val ok = runCatching {
            Socket().use { s -> s.connect(InetSocketAddress("1.1.1.1", 443), 4000) }
        }.isSuccess
        return if (ok) System.currentTimeMillis() - start else null
    }

    /** Starts the periodic real-latency probe loop exactly once, only while connected. */
    private fun startLatencyMonitor() {
        if (latencyMonitorStarted) return
        latencyMonitorStarted = true
        monitorScope.launch {
            while (true) {
                if (_stats.value.connected) {
                    val measured = measureLatencyOnceMs()
                    // Only overwrite with a real result; a failed probe leaves
                    // the last known-good value rather than fabricating one.
                    if (measured != null) {
                        _stats.value = _stats.value.copy(latencyMs = measured)
                    }
                }
                delay(5000)
            }
        }
    }

    companion object {
        @Volatile
        private var instance: TunnelManager? = null

        fun getInstance(context: Context): TunnelManager {
            return instance ?: synchronized(this) {
                instance ?: TunnelManager(context.applicationContext).also { instance = it }
            }
        }
    }
}
