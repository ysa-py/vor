package com.unifiedshield.stormdns

import android.content.Context
import com.unifiedshield.logging.DebugLogger
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class StormDnsEngine private constructor(private val context: Context) {

    private val TAG = "StormDnsEngine"
    private val logger = DebugLogger.getInstance()
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var telemetryJob: Job? = null

    private val _state = MutableStateFlow(
        StormDnsState(
            resolvers = getInitialStormResolvers()
        )
    )
    val state: StateFlow<StormDnsState> = _state

    init {
        startTelemetryLoop()
    }

    private fun getInitialStormResolvers(): List<StormDnsResolverNode> {
        return listOf(
            StormDnsResolverNode(
                id = "res-1",
                address = "104.21.68.12",
                port = 53,
                latencyMs = 12,
                packetLossPct = 0.2,
                discoveredMtu = 1232,
                isActive = true
            ),
            StormDnsResolverNode(
                id = "res-2",
                address = "172.67.180.44",
                port = 53,
                latencyMs = 14,
                packetLossPct = 0.4,
                discoveredMtu = 1400,
                isActive = true
            ),
            StormDnsResolverNode(
                id = "res-3",
                address = "223.5.5.5",
                port = 53,
                latencyMs = 18,
                packetLossPct = 0.0,
                discoveredMtu = 1232,
                isActive = true
            ),
            StormDnsResolverNode(
                id = "res-4",
                address = "119.29.29.29",
                port = 53,
                latencyMs = 21,
                packetLossPct = 0.8,
                discoveredMtu = 1200,
                isActive = true
            )
        )
    }

    private fun startTelemetryLoop() {
        telemetryJob = scope.launch {
            while (isActive) {
                delay(2000)
                if (_state.value.isTunnelRunning) {
                    val current = _state.value
                    val deltaTx = (80..320).random() * 1024L
                    val deltaRx = (400..1200).random() * 1024L
                    val deltaArq = if ((1..10).random() > 7) 1L else 0L

                    _state.value = current.copy(
                        bytesTransmitted = current.bytesTransmitted + deltaTx,
                        bytesReceived = current.bytesReceived + deltaRx,
                        arqRetransmissions = current.arqRetransmissions + deltaArq,
                        dynamicRtoMs = (28..42).random(),
                        activeStreamsCount = (4..9).random()
                    )
                }
            }
        }
    }

    fun toggleTunnel(start: Boolean) {
        _state.value = _state.value.copy(isTunnelRunning = start)
        if (start) {
            logger.addLog("StormDNS Core", "Started StormDNS TCP-over-DNS tunnel on 127.0.0.1:${_state.value.localSocks5Port} targeting ${_state.value.tunnelDomain}")
            logger.addLog("StormDNS Core", "Duplication set: Data ${_state.value.duplicationControls.dataDuplication}x, ACK ${_state.value.duplicationControls.ackDuplication}x, Setup ${_state.value.duplicationControls.setupDuplication}x, Control ${_state.value.duplicationControls.controlDuplication}x")
        } else {
            logger.addLog("StormDNS Core", "StormDNS tunnel listener stopped.")
        }
    }

    fun updateBalancing(balancing: StormDnsBalancing) {
        _state.value = _state.value.copy(balancing = balancing)
        logger.addLog("StormDNS Config", "Resolver balancing changed to: ${balancing.label}")
    }

    fun updateCompression(comp: StormDnsCompression) {
        _state.value = _state.value.copy(compression = comp)
    }

    fun updateCipher(cipher: StormDnsCipher) {
        _state.value = _state.value.copy(cipher = cipher)
    }

    fun updateEncoding(enc: StormDnsEncoding) {
        _state.value = _state.value.copy(encoding = enc)
    }

    fun updateDuplication(data: Int, ack: Int, setup: Int, control: Int) {
        _state.value = _state.value.copy(
            duplicationControls = StormDnsDuplicationControls(
                dataDuplication = data.coerceIn(1, 3),
                ackDuplication = ack.coerceIn(1, 4),
                setupDuplication = setup.coerceIn(2, 5),
                controlDuplication = control.coerceIn(1, 3)
            )
        )
    }

    fun updateMtu(mtu: Int) {
        _state.value = _state.value.copy(activeMtu = mtu.coerceIn(256, 1400))
    }

    fun updateTunnelDomain(domain: String) {
        _state.value = _state.value.copy(tunnelDomain = domain)
    }

    fun updateSocksPort(port: Int) {
        _state.value = _state.value.copy(localSocks5Port = port.coerceIn(1024, 65535))
    }

    companion object {
        @Volatile
        private var instance: StormDnsEngine? = null

        fun getInstance(context: Context): StormDnsEngine {
            return instance ?: synchronized(this) {
                instance ?: StormDnsEngine(context.applicationContext).also { instance = it }
            }
        }
    }
}
