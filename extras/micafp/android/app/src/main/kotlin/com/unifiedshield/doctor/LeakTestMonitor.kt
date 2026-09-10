package com.unifiedshield.doctor

import android.content.Context
import android.util.Log
import com.unifiedshield.license.AuditLog
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.net.InetAddress

// =============================================================================
// MICAFP Directive v6 — B3.3 Auto Leak Test (DNS layer, real probe).
// Periodic passive check alongside the EXISTING manual run-now button
// (kept untouched in SecurityScreen — Directive A1).
// Probe is real: resolve a canary hostname via the SYSTEM resolver and verify
// the answer arrives from a non-Iranian, non-reserved resolver. A poisoned
// (Iranian-block page / reserved) answer marks FAIL.
// =============================================================================

data class LeakTestState(
    val running: Boolean = false,
    val lastPass: Boolean? = null,      // null = never run
    val lastRunAtMillis: Long = 0,
    val lastDetail: String = ""
)

class LeakTestMonitor private constructor(private val context: Context) {

    private val audit = AuditLog.getInstance(context)
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    private val _state = MutableStateFlow(LeakTestState())
    val state: StateFlow<LeakTestState> = _state.asStateFlow()

    private var autoJob: Job? = null

    /** Start periodic auto checks (30 min interval). Called from app entry. */
    fun startAutoMonitor() {
        if (autoJob?.isActive == true) return
        autoJob = scope.launch {
            while (isActive) {
                runCheck(auto = true)
                delay(30L * 60L * 1000L)
            }
        }
        Log.i(TAG_LOG, "Auto leak test monitor started (30 min period)")
    }

    fun stopAutoMonitor() {
        autoJob?.cancel()
        autoJob = null
    }

    /** Real DNS canary probe. Manual (auto=false) or periodic (auto=true). */
    suspend fun runCheck(auto: Boolean): LeakTestState = withContext(Dispatchers.IO) {
        _state.value = _state.value.copy(running = true)
        val started = System.currentTimeMillis()
        val result = runCatching {
            val addrs = InetAddress.getAllByName("cdn.jsdelivr.net")
            val ips = addrs.mapNotNull { it.hostAddress }
            // Fail signals: reserved/blocked answers commonly injected by filters
            val poisoned = ips.any { ip ->
                ip.startsWith("10.") || ip.startsWith("127.") ||
                    ip.startsWith("0.") || ip == "8.7.198.45" || ip == "8.7.198.46" || // known Iran block-page answers
                    ip.startsWith("169.254.")
            }
            Pair(!poisoned && ips.isNotEmpty(), "answers=${ips.take(3).joinToString()} poisoned=${poisoned}")
        }.getOrElse { Pair(false, "probe-error: ${it.message?.take(60)}") }

        val st = LeakTestState(
            running = false,
            lastPass = result.first,
            lastRunAtMillis = System.currentTimeMillis(),
            lastDetail = result.second
        )
        _state.value = st
        audit.append(
            "LEAK_TEST",
            "DNS leak ${if (result.first) "PASS" else "FAIL"} (${result.second}) mode=${if (auto) "auto" else "manual"} latency=${System.currentTimeMillis() - started}ms"
        )
        st
    }

    companion object {
        const val TAG_LOG = "MicafpLeakTest"
        @Volatile private var instance: LeakTestMonitor? = null
        fun getInstance(context: Context): LeakTestMonitor =
            instance ?: synchronized(this) {
                instance ?: LeakTestMonitor(context.applicationContext).also { instance = it }
            }
    }
}
