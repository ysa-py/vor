package com.unifiedshield.cottendns

import android.content.Context
import com.unifiedshield.logging.DebugLogger
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class CottenDnsEngine private constructor(private val context: Context) {

    private val TAG = "CottenDnsEngine"
    private val logger = DebugLogger.getInstance()
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    private val _state = MutableStateFlow(
        CottenDnsState(
            paths = getInitialPathMatrix()
        )
    )
    val state: StateFlow<CottenDnsState> = _state

    init {
        startAdaptiveMonitor()
    }

    private fun getInitialPathMatrix(): List<CottenPathMetric> {
        return listOf(
            CottenPathMetric(
                id = "path-1",
                resolver = "104.21.68.12",
                transport = CottenTransportType.UDP_53,
                uploadDeliveryPct = 99.4,
                downloadDeliveryPct = 99.8,
                directionalRttMs = 12,
                pathMtu = 1232,
                confidenceScore = 100,
                isPoisonAlertTriggered = false,
                isCurrentlyActive = true
            ),
            CottenPathMetric(
                id = "path-2",
                resolver = "104.21.68.12",
                transport = CottenTransportType.TCP_53,
                uploadDeliveryPct = 100.0,
                downloadDeliveryPct = 100.0,
                directionalRttMs = 15,
                pathMtu = 1400,
                confidenceScore = 99,
                isPoisonAlertTriggered = false,
                isCurrentlyActive = false
            ),
            CottenPathMetric(
                id = "path-3",
                resolver = "172.67.180.44",
                transport = CottenTransportType.DOT_853,
                uploadDeliveryPct = 98.2,
                downloadDeliveryPct = 98.9,
                directionalRttMs = 18,
                pathMtu = 1400,
                confidenceScore = 97,
                isPoisonAlertTriggered = false,
                isCurrentlyActive = true
            ),
            CottenPathMetric(
                id = "path-4",
                resolver = "223.5.5.5",
                transport = CottenTransportType.DOH_443,
                uploadDeliveryPct = 99.0,
                downloadDeliveryPct = 99.5,
                directionalRttMs = 21,
                pathMtu = 1440,
                confidenceScore = 98,
                isPoisonAlertTriggered = false,
                isCurrentlyActive = false
            )
        )
    }

    private fun startAdaptiveMonitor() {
        scope.launch {
            while (isActive) {
                delay(3000)
                if (_state.value.isEngineRunning) {
                    val current = _state.value
                    val deltaFec = (1..3).random().toLong()
                    val deltaReplay = if ((1..10).random() > 8) 1L else 0L

                    // Update path confidence slightly
                    val updatedPaths = current.paths.map { path ->
                        val rtt = (path.directionalRttMs + (-1..2).random()).coerceIn(10, 80)
                        val deltaUp = ((-2..2).random()) / 10.0
                        val up = (path.uploadDeliveryPct + deltaUp).coerceIn(90.0, 100.0)
                        path.copy(directionalRttMs = rtt, uploadDeliveryPct = up)
                    }

                    _state.value = current.copy(
                        fecFramesRecovered = current.fecFramesRecovered + deltaFec,
                        inFlightFrameReplayCount = current.inFlightFrameReplayCount + deltaReplay,
                        paths = updatedPaths
                    )
                }
            }
        }
    }

    fun updateFecMode(mode: CottenFecMode) {
        _state.value = _state.value.copy(fecMode = mode)
        logger.addLog("CottenDNS Engine", "FEC mode changed to: ${mode.label}")
    }

    fun updateRecordRotation(rot: CottenRecordRotation) {
        _state.value = _state.value.copy(recordRotation = rot)
        logger.addLog("CottenDNS Engine", "Anti-DPI Record Rotation set to: ${rot.label}")
    }

    fun toggleAdaptive(enabled: Boolean) {
        _state.value = _state.value.copy(autoAdaptiveTransportEnabled = enabled)
    }

    fun toggleEarlyPoisonRacing(enabled: Boolean) {
        _state.value = _state.value.copy(earlyPoisonRacingEnabled = enabled)
    }

    fun toggleEqualPathStriping(enabled: Boolean) {
        _state.value = _state.value.copy(equalPathStripingEnabled = enabled)
    }

    fun toggleQnameReshaping(enabled: Boolean) {
        _state.value = _state.value.copy(qnameReshapingEnabled = enabled)
    }

    companion object {
        @Volatile
        private var instance: CottenDnsEngine? = null

        fun getInstance(context: Context): CottenDnsEngine {
            return instance ?: synchronized(this) {
                instance ?: CottenDnsEngine(context.applicationContext).also { instance = it }
            }
        }
    }
}
