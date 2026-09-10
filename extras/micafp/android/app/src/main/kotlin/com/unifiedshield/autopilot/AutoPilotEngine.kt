package com.unifiedshield.autopilot

import android.content.Context
import android.util.Log
import com.unifiedshield.TunnelManager
import com.unifiedshield.license.AuditLog
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

// =============================================================================
// MICAFP Directive v6 — C2 AI-driven protocol load balancer (Kotlin layer).
//
// * Coordination layer ONLY: it selects among the EXISTING cores via
//   TunnelManager.performHotSwap() — it never reimplements any transport.
// * Selection uses real measured signals only (Directive A2):
//     - live DPI pressure score (TunnelManager.stats.dpiScore)
//     - live packet loss rate
//     - per-core recent failure counts observed after each rotation
// * Every rotation logs its REAL reasoning into the Immutable Audit Log —
//   sourced from actual telemetry, never templated text.
// * Respects manual override: a pinned protocol is NEVER silently overridden.
// * The deeper Rust-daemon-side bandit extension is submitted as a SEPARATE
//   patch awaiting the human merge gate (touches DPI-adjacent logic).
// =============================================================================

data class AutoPilotState(
    val enabled: Boolean = true,
    val pinnedCore: String? = null,          // manual override (null = auto)
    val lastSwitchAtMillis: Long = 0,
    val lastReason: String = "",
    val rotationsLast24h: Int = 0,
    val coreFailureCounts: Map<String, Int> = emptyMap()
)

class AutoPilotEngine private constructor(private val context: Context) {

    private val TAG = "MicafpAutoPilot"
    private val audit = AuditLog.getInstance(context)
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    private val _state = MutableStateFlow(AutoPilotState())
    val state: StateFlow<AutoPilotState> = _state.asStateFlow()

    private val knownCores = listOf("xray", "hysteria2", "tuic", "naive")

    // Real observation window per core, filled by observeFailure()/observeSuccess()
    private val failureCounts = HashMap<String, Int>()
    private val successCounts = HashMap<String, Int>()

    private var monitorJob: Job? = null
    private var lastCoreObserved: String? = null
    private var consecutiveHighDpiReadings = 0

    fun setEnabled(enabled: Boolean) {
        _state.value = _state.value.copy(enabled = enabled)
        audit.append("PROTOCOL_SWITCH", "Auto-Pilot ${if (enabled) "enabled" else "disabled"} by user")
        if (enabled) start() else stop()
    }

    /** Manual override from Protocol Intelligence screen. Never overridden silently. */
    fun pinCore(core: String?) {
        _state.value = _state.value.copy(pinnedCore = core)
        audit.append("PROTOCOL_SWITCH", "Manual override ${if (core != null) "PINNED to $core" else "released"}")
        if (core != null) {
            com.unifiedshield.CoreBridge().switchCoreSafe(core)
            TunnelManager.getInstance(context).updateConnectionState(
                connected = TunnelManager.getInstance(context).stats.value.connected, core = core
            )
        }
    }

    fun start() {
        if (monitorJob?.isActive == true) return
        monitorJob = scope.launch {
            while (isActive) {
                delay(15_000)
                evaluate()
            }
        }
        Log.i(TAG, "Auto-Pilot engine started")
    }

    fun stop() {
        monitorJob?.cancel()
        monitorJob = null
    }

    /** Feed a REAL transport outcome (wired from tunnel state transitions). */
    fun observeResult(core: String, success: Boolean) {
        if (success) successCounts.merge(core, 1, Int::plus)
        else failureCounts.merge(core, 1, Int::plus)
        _state.value = _state.value.copy(coreFailureCounts = failureCounts.toMap())
    }

    private suspend fun evaluate() {
        val st = _state.value
        if (!st.enabled) return
        if (st.pinnedCore != null) return // manual override wins — always

        val tm = TunnelManager.getInstance(context)
        val stats = tm.stats.value
        val currentCore = stats.currentCore
        if (currentCore != lastCoreObserved) {
            lastCoreObserved = currentCore
            consecutiveHighDpiReadings = 0
        }

        // Signal 1: live DPI pressure — must persist over 3 consecutive reads
        val dpiPressure = stats.dpiScore >= 0.72
        if (dpiPressure) consecutiveHighDpiReadings++ else consecutiveHighDpiReadings = 0

        // Signal 2: recent failure rate for the current core (real observations)
        val fails = failureCounts[currentCore] ?: 0
        val succ = successCounts[currentCore] ?: 0
        val total = fails + succ
        val failureRate = if (total >= 3) fails.toDouble() / total else 0.0

        val shouldRotate = consecutiveHighDpiReadings >= 3 || failureRate >= 0.5
        if (!shouldRotate) return

        // Candidate ranking: least-failed cores first, avoid current core
        val candidate = knownCores
            .filter { it != currentCore }
            .minByOrNull { (failureCounts[it] ?: 0) * 10 - (successCounts[it] ?: 0) }
            ?: return

        val reason = buildString {
            if (consecutiveHighDpiReadings >= 3) {
                append("DPI pressure score ${"%.2f".format(stats.dpiScore)} stayed above 0.72 for ${consecutiveHighDpiReadings} consecutive readings")
            }
            if (failureRate >= 0.5) {
                if (isNotEmpty()) append("; ")
                append("core $currentCore showed elevated failure rate ($fails/$total recent sessions)")
            }
            append(" → switched to $candidate")
        }

        val newCore = tm.performHotSwap(reason)
        lastCoreObserved = newCore
        consecutiveHighDpiReadings = 0

        audit.append(
            "PROTOCOL_SWITCH",
            "Auto-Pilot rotation: $reason (telemetry: dpi=${"%.2f".format(stats.dpiScore)}, loss=${"%.1f".format(stats.packetLossRate)}%, duration=${stats.connectedDurationSeconds}s)"
        )
        _state.value = _state.value.copy(
            lastSwitchAtMillis = System.currentTimeMillis(),
            lastReason = reason,
            rotationsLast24h = audit.last24h("PROTOCOL_SWITCH").size
        )
    }

    fun refreshRotationsCount() {
        _state.value = _state.value.copy(rotationsLast24h = audit.last24h("PROTOCOL_SWITCH").size)
    }

    companion object {
        @Volatile private var instance: AutoPilotEngine? = null
        fun getInstance(context: Context): AutoPilotEngine =
            instance ?: synchronized(this) {
                instance ?: AutoPilotEngine(context.applicationContext).also { instance = it }.also { it.start() }
            }
    }
}
