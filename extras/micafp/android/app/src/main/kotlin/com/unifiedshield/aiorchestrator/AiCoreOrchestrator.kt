package com.unifiedshield.aiorchestrator

import android.util.Log
import com.unifiedshield.logging.DebugLogger
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlin.math.min
import kotlin.math.pow
import kotlin.random.Random

/**
 * Autonomous AI Core Orchestrator.
 * Implements exponential backoff retry strategy for core switching.
 * Automatically blacklists any core that fails 3 consecutive handshake attempts for 5 minutes (300,000 ms),
 * then automatically shifts traffic to the next highest-scoring available core from the pool without manual intervention.
 */
class AiCoreOrchestrator private constructor() {

    private val TAG = "AiCoreOrchestrator"
    private val logger = DebugLogger.getInstance()
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    private val _coresPool = MutableStateFlow<List<CoreScoreEntry>>(getInitialCoresPool())
    val coresPool: StateFlow<List<CoreScoreEntry>> = _coresPool.asStateFlow()

    private val _activeCoreId = MutableStateFlow("core-vless-reality")
    val activeCoreId: StateFlow<String> = _activeCoreId.asStateFlow()

    private val _orchestratorLogs = MutableStateFlow<List<String>>(listOf(
        "AI Orchestrator initialized. Multi-core pool active.",
        "Scoring matrix calibrated for high-jitter mobile networks."
    ))
    val orchestratorLogs: StateFlow<List<String>> = _orchestratorLogs.asStateFlow()

    private var retryAttempt = 0
    private var isSwitchingCore = false

    private fun getInitialCoresPool(): List<CoreScoreEntry> {
        return listOf(
            CoreScoreEntry("core-vless-reality", "VLESS Reality (XTLS-Vision)", "VLESS / TLS 1.3", 96.5, 14, 0.1, 0.99, 1.8, isActive = true),
            CoreScoreEntry("core-hysteria-2", "Hysteria 2 Brutal", "QUIC / UDP", 98.2, 11, 0.0, 1.00, 1.2),
            CoreScoreEntry("core-tuic-v5", "TUIC v5 Pure BBR", "QUIC / UDP", 94.0, 13, 0.2, 0.98, 2.1),
            CoreScoreEntry("core-storm-dns", "StormDNS Stream", "TCP-over-DNS", 95.8, 15, 0.1, 0.99, 1.6),
            CoreScoreEntry("core-cotten-dns", "CottenDNS Super-FEC", "Multi-Transport DNS", 97.4, 12, 0.0, 1.00, 1.1),
            CoreScoreEntry("core-master-dns", "MasterDns 8-Way ARQ", "DNS Multipath", 93.5, 16, 0.3, 0.97, 2.4),
            CoreScoreEntry("core-shadowsocks", "Shadowsocks 2022 Blake3", "AEAD", 91.0, 15, 0.4, 0.96, 2.8),
            CoreScoreEntry("core-amnezia-wg", "AmneziaWG Junk Obfuscated", "WireGuard UDP", 94.5, 10, 0.2, 0.98, 1.4),
            CoreScoreEntry("core-naive", "NaïveProxy Chromium Stack", "HTTP/2 TLS", 92.8, 21, 0.2, 0.97, 3.1)
        )
    }

    /**
     * Compute exponential backoff in milliseconds: base * 2^attempt + jitter
     */
    fun calculateBackoffMs(attempt: Int): Long {
        val base = 500L
        val maxBackoff = 8000L
        val exp = 2.0.pow(attempt.toDouble()).toLong()
        val calculated = min(maxBackoff, base * exp)
        val jitter = Random.nextLong(100, 400)
        return calculated + jitter
    }

    /**
     * Report a handshake failure for a specific core.
     * If failures reach 3 consecutive attempts, automatically blacklist the core for 5 minutes (300,000 ms)
     * and shift traffic to the next highest-scoring available core.
     */
    fun reportHandshakeFailure(coreId: String, reason: String) {
        scope.launch {
            val now = System.currentTimeMillis()
            val list = _coresPool.value.map { entry ->
                if (entry.coreId == coreId) {
                    val newFailures = entry.consecutiveFailures + 1
                    val shouldBlacklist = newFailures >= 3
                    val blacklistUntil = if (shouldBlacklist) now + (5 * 60 * 1000L) else entry.blacklistedUntilMs

                    if (shouldBlacklist) {
                        logger.warn("AiOrchestrator", "Core [$coreId] failed 3 consecutive handshakes. BLACKLISTED for 5 minutes.")
                        addLog("⛔ Core [${entry.name}] blacklisted for 5 minutes after 3 failed handshakes.")
                    } else {
                        logger.info("AiOrchestrator", "Core [$coreId] handshake failure #$newFailures ($reason)")
                        addLog("⚠️ Core [${entry.name}] handshake attempt #$newFailures failed: $reason")
                    }

                    entry.copy(
                        consecutiveFailures = newFailures,
                        isBlacklisted = shouldBlacklist,
                        blacklistedUntilMs = blacklistUntil,
                        score = (entry.score - 15.0).coerceAtLeast(10.0)
                    )
                } else {
                    entry
                }
            }
            _coresPool.value = list

            // If the failed core is currently active and blacklisted or failing, shift to next highest available core
            if (coreId == _activeCoreId.value) {
                retryAttempt++
                val backoff = calculateBackoffMs(retryAttempt)
                addLog("⏳ Exponential backoff delay ${backoff}ms before shifting core...")
                delay(backoff)
                autoShiftToBestCore("Consecutive failure on $coreId")
            }
        }
    }

    /**
     * Report successful handshake on core, resetting failure count.
     */
    fun reportHandshakeSuccess(coreId: String, latencyMs: Long) {
        scope.launch {
            retryAttempt = 0
            val list = _coresPool.value.map { entry ->
                if (entry.coreId == coreId) {
                    entry.copy(
                        consecutiveFailures = 0,
                        isBlacklisted = false,
                        blacklistedUntilMs = 0L,
                        latencyMs = latencyMs,
                        handshakeSuccessRate = ((entry.handshakeSuccessRate * 4) + 1.0) / 5.0
                    )
                } else entry
            }
            _coresPool.value = list
        }
    }

    /**
     * Automatically shifts traffic to the next highest-scoring available core from the pool.
     */
    fun autoShiftToBestCore(triggerReason: String): CoreScoreEntry? {
        val now = System.currentTimeMillis()
        val currentActive = _activeCoreId.value

        // Filter available cores (unblacklisted or expired blacklist)
        val available = _coresPool.value
            .map { entry ->
                if (entry.isBlacklisted && now >= entry.blacklistedUntilMs) {
                    entry.copy(isBlacklisted = false, consecutiveFailures = 0)
                } else entry
            }
            .filter { it.isAvailable(now) && it.coreId != currentActive }
            .sortedByDescending { it.score }

        val bestCandidate = available.firstOrNull() ?: _coresPool.value.firstOrNull()

        if (bestCandidate != null) {
            _activeCoreId.value = bestCandidate.coreId
            val updatedList = _coresPool.value.map { entry ->
                entry.copy(isActive = entry.coreId == bestCandidate.coreId)
            }
            _coresPool.value = updatedList

            logger.info("AiOrchestrator", "AUTO-SHIFTED traffic to [${bestCandidate.name}] (Score: ${bestCandidate.score}, Latency: ${bestCandidate.latencyMs}ms). Reason: $triggerReason")
            addLog("🚀 Auto-shifted traffic to [${bestCandidate.name}] (Score: ${bestCandidate.score}). Reason: $triggerReason")
        }

        return bestCandidate
    }

    /**
     * Updates the full scoring matrix provided by the Adaptive Network Profiler.
     */
    fun updateScoringMatrix(updatedScores: List<CoreScoreEntry>) {
        val now = System.currentTimeMillis()
        val currentActive = _activeCoreId.value

        // Preserve blacklist state unless expired
        val merged = updatedScores.map { updated ->
            val existing = _coresPool.value.find { it.coreId == updated.coreId }
            if (existing != null && existing.isBlacklisted && now < existing.blacklistedUntilMs) {
                updated.copy(
                    isBlacklisted = true,
                    blacklistedUntilMs = existing.blacklistedUntilMs,
                    consecutiveFailures = existing.consecutiveFailures,
                    isActive = updated.coreId == currentActive
                )
            } else {
                updated.copy(isActive = updated.coreId == currentActive)
            }
        }
        _coresPool.value = merged
    }

    private fun addLog(message: String) {
        val time = java.text.SimpleDateFormat("HH:mm:ss", java.util.Locale.getDefault()).format(java.util.Date())
        val entry = "[$time] $message"
        val current = _orchestratorLogs.value.toMutableList()
        current.add(0, entry)
        if (current.size > 50) current.removeAt(current.size - 1)
        _orchestratorLogs.value = current
    }

    companion object {
        @Volatile
        private var INSTANCE: AiCoreOrchestrator? = null

        fun getInstance(): AiCoreOrchestrator {
            return INSTANCE ?: synchronized(this) {
                INSTANCE ?: AiCoreOrchestrator().also { INSTANCE = it }
            }
        }
    }
}
