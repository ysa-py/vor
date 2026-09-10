package com.unifiedshield

import android.util.Log
import kotlinx.coroutines.*
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.math.min

/**
 * DPI (Deep Packet Inspection) detector using TLS handshake analysis
 * and a sliding window algorithm to detect interference patterns.
 *
 * When the DPI score exceeds 0.72, triggers a core switch to evade
 * detection. Uses sliding window of recent TLS handshake measurements.
 */
class DpiDetector {

    private val TAG = "DpiDector"
    private val monitoringActive = AtomicBoolean(false)
    private var monitorJob: Job? = null
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    // Sliding window for TLS handshake timing analysis
    private val windowSize = 50
    private val handshakeTimings = ConcurrentLinkedQueue<Long>()
    private val tlsFingerprintScores = ConcurrentLinkedQueue<Double>()

    // DPI indicators
    private var connectionResets = 0
    private var tlsHandshakeFailures = 0
    private var totalHandshakes = 0
    private var suspiciousPatternCount = 0

    // Callback when DPI score exceeds threshold
    private var onDpiDetected: ((score: Double) -> Unit)? = null

    companion object {
        const val DPI_THRESHOLD = 0.72
        const val MONITOR_INTERVAL_MS = 2000L
        const val RESET_WINDOW_MS = 30000L
    }

    /**
     * Start monitoring for DPI patterns.
     * @param callback Called with DPI score when it exceeds threshold
     */
    fun startMonitoring(callback: (score: Double) -> Unit) {
        if (monitoringActive.getAndSet(true)) return
        onDpiDetected = callback

        monitorJob = scope.launch {
            while (isActive && monitoringActive.get()) {
                val score = calculateDpiScore()
                if (score > DPI_THRESHOLD) {
                    Log.w(TAG, "DPI detected! Score: $score (threshold: $DPI_THRESHOLD)")
                    withContext(Dispatchers.Main) {
                        onDpiDetected?.invoke(score)
                    }
                }
                delay(MONITOR_INTERVAL_MS)
            }
        }
    }

    /**
     * Stop monitoring for DPI patterns.
     */
    fun stopMonitoring() {
        monitoringActive.set(false)
        monitorJob?.cancel()
        handshakeTimings.clear()
        tlsFingerprintScores.clear()
        connectionResets = 0
        tlsHandshakeFailures = 0
        totalHandshakes = 0
        suspiciousPatternCount = 0
    }

    /**
     * Record a TLS handshake timing measurement.
     */
    fun recordHandshake(timingMs: Long, success: Boolean) {
        totalHandshakes++
        handshakeTimings.add(timingMs)
        if (!success) {
            tlsHandshakeFailures++
        }

        // Trim window
        while (handshakeTimings.size > windowSize) {
            handshakeTimings.poll()
        }
    }

    /**
     * Record a connection reset (RST packet).
     */
    fun recordConnectionReset() {
        connectionResets++
        suspiciousPatternCount++
    }

    /**
     * Record a TLS fingerprint anomaly score.
     */
    fun recordTlsFingerprintScore(score: Double) {
        tlsFingerprintScores.add(score)
        while (tlsFingerprintScores.size > windowSize) {
            tlsFingerprintScores.poll()
        }
    }

    /**
     * Calculate the DPI score using a sliding window algorithm.
     *
     * Factors:
     * 1. TLS handshake failure rate (weight: 0.35)
     * 2. Connection reset frequency (weight: 0.30)
     * 3. TLS fingerprint anomaly (weight: 0.20)
     * 4. Timing pattern deviation (weight: 0.15)
     *
     * Score range: 0.0 (no DPI) to 1.0 (confirmed DPI)
     */
    private fun calculateDpiScore(): Double {
        if (totalHandshakes < 5) return 0.0

        // Factor 1: TLS handshake failure rate
        val failureRate = tlsHandshakeFailures.toDouble() / totalHandshakes.toDouble()
        val failureScore = min(1.0, failureRate * 3.0) // Scale: 33% failure = score 1.0

        // Factor 2: Connection reset frequency
        val resetRate = connectionResets.toDouble() / maxOf(1, totalHandshakes).toDouble()
        val resetScore = min(1.0, resetRate * 5.0) // Scale: 20% resets = score 1.0

        // Factor 3: TLS fingerprint anomaly average
        val fingerprintScore = if (tlsFingerprintScores.isNotEmpty()) {
            tlsFingerprintScores.average()
        } else {
            0.0
        }

        // Factor 4: Timing pattern deviation (abnormal variance suggests DPI)
        val timingScore = if (handshakeTimings.size >= 10) {
            val timings = handshakeTimings.toList()
            val mean = timings.average()
            val variance = timings.map { (it - mean) * (it - mean) }.average()
            // High variance with bimodal distribution suggests selective interference
            val cv = Math.sqrt(variance) / maxOf(1.0, mean) // coefficient of variation
            min(1.0, cv * 2.0)
        } else {
            0.0
        }

        // Weighted combination
        val compositeScore = (
            failureScore * 0.35 +
            resetScore * 0.30 +
            fingerprintScore * 0.20 +
            timingScore * 0.15
        )

        return compositeScore.coerceIn(0.0, 1.0)
    }

    /**
     * Reset counters (called after core switch).
     */
    fun reset() {
        connectionResets = 0
        tlsHandshakeFailures = 0
        totalHandshakes = 0
        suspiciousPatternCount = 0
        handshakeTimings.clear()
        tlsFingerprintScores.clear()
    }

    /**
     * Get current DPI score without triggering callbacks.
     */
    fun getCurrentScore(): Double = calculateDpiScore()
}
