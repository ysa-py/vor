package com.unifiedshield

import android.content.Context
import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import com.unifiedshield.security.SecurityController

/**
 * Central State Store for UnifiedShield Enterprise.
 * Manages Panic Mode, Shadow Core Failover, Network Lock,
 * DPI Heatmap thresholds, and Mesh Relay telemetry.
 */
data class DpiHeatmapNode(
    val id: String,
    val regionName: String,
    val regionPersian: String,
    val ispName: String,
    val dpiThreatScore: Int, // 0 to 100
    val rstRate: Double,
    val packetDropRate: Double,
    val isBreached: Boolean = false,
    val status: String = "MONITORING"
)

data class ThreatIntelSignature(
    val id: String,
    val name: String,
    val namePersian: String,
    val signaturePattern: String,
    val threatLevel: String, // LOW, MEDIUM, HIGH, CRITICAL
    val detectedCount: Long,
    val recommendedProtocolId: String,
    val recommendedProtocolName: String,
    val recommendedActionDescription: String,
    val recommendedActionDescriptionPersian: String,
    val isOptimized: Boolean = false
)

data class ShieldStoreState(
    val isPanicModeActive: Boolean = false,
    val panicTriggerReason: String = "",
    val isNetworkLocked: Boolean = false,
    val activeCore: String = "Quantum-Morph v4",
    val shadowCore: String = "Hysteria 2 Brutal",
    val lastShadowSwapTimestamp: Long = 0,
    val dpiCriticalThreshold: Int = 85,
    val isEmergencyWiped: Boolean = false,
    val activeOptimizationCount: Int = 4,
    val heatmapNodes: List<DpiHeatmapNode> = listOf(
        DpiHeatmapNode("tehran-mci", "Tehran Core", "مرکز تبادل دیتای تهران (MCI)", "همراه اول", 78, 0.42, 0.35),
        DpiHeatmapNode("karaj-irancell", "Karaj Gateway", "گیت‌وی کرج (Irancell)", "ایرانسل", 82, 0.55, 0.48),
        DpiHeatmapNode("isfahan-shatel", "Isfahan Hub", "هاب اصفهان (Shatel)", "شاتل", 64, 0.28, 0.22),
        DpiHeatmapNode("tabriz-tci", "Tabriz Egress", "گیت‌وی خروجی تبریز (TCI)", "مخابرات", 72, 0.38, 0.30),
        DpiHeatmapNode("shiraz-rightel", "Shiraz Relay", "رله شیراز (Rightel)", "رایتل", 69, 0.31, 0.26),
        DpiHeatmapNode("mashhad-asiatech", "Mashhad Node", "نود مشهد (Asiatech)", "آسیاتک", 58, 0.22, 0.18)
    ),
    val threatSignatures: List<ThreatIntelSignature> = listOf(
        ThreatIntelSignature(
            id = "sig-sni-slicer",
            name = "TIC SNI Sniffer v4.2",
            namePersian = "اسکنر پیشرفته SNI زیرساخت",
            signaturePattern = "0x16 0x03 0x01 [ClientHello-SNI-Match]",
            threatLevel = "CRITICAL",
            detectedCount = 8420,
            recommendedProtocolId = "neural-reality-v4",
            recommendedProtocolName = "Neural-REALITY v4",
            recommendedActionDescription = "Slice TLS ClientHello into 3-byte TCP fragments with dummy TCP windows.",
            recommendedActionDescriptionPersian = "قطعه‌بندی نام دامنه در رکوردهای ۳ بایتی با پنجره‌های ساختگی TCP جهت فریب اسکنر لایه ۷.",
            isOptimized = true
        ),
        ThreatIntelSignature(
            id = "sig-rst-injector",
            name = "Dejban TCP RST Injector",
            namePersian = "سیستم تزریق پکت دژبان (TCP RST)",
            signaturePattern = "TCP Flags: RST+ACK with Out-of-Sequence SEQ",
            threatLevel = "HIGH",
            detectedCount = 5120,
            recommendedProtocolId = "hysteria2-brutal",
            recommendedProtocolName = "Hysteria 2 Brutal",
            recommendedActionDescription = "Switch to UDP Congestion Control to completely bypass TCP state inspection.",
            recommendedActionDescriptionPersian = "سوئیچ آنی به پروتکل UDP تهاجمی جهت بی‌اثر کردن پکت‌های تزریقی TCP.",
            isOptimized = true
        ),
        ThreatIntelSignature(
            id = "sig-flow-ml",
            name = "Statistical ML Flow Fingerprinter",
            namePersian = "طبقه‌بندی آماری الگوهای پکت با هوش مصنوعی",
            signaturePattern = "Packet Length Entropy: 4.2-6.5 Burst Sequence",
            threatLevel = "CRITICAL",
            detectedCount = 11200,
            recommendedProtocolId = "quantum-morph-v4",
            recommendedProtocolName = "Quantum-Morph v4",
            recommendedActionDescription = "Inject 184B Random High-Entropy Padding disguised as Alibaba Cloud VoD stream.",
            recommendedActionDescriptionPersian = "تزریق نویز تصادفی انتروپی بالا و استتار در قالب ترافیک ویدیویی علی‌بابا کلود.",
            isOptimized = true
        ),
        ThreatIntelSignature(
            id = "sig-dns-nxdomain",
            name = "DNS Port 53 Poisoning & Forgery",
            namePersian = "مسموم‌سازی و انحراف پورت ۵۳ DNS",
            signaturePattern = "Fake A Record (10.10.34.34 / NXDOMAIN Injection)",
            threatLevel = "MEDIUM",
            detectedCount = 3940,
            recommendedProtocolId = "hyperion-doq-stegano",
            recommendedProtocolName = "Hyperion-DoQ Steganography",
            recommendedActionDescription = "Encapsulate DNS queries into recursive DNS-over-QUIC encrypted frames.",
            recommendedActionDescriptionPersian = "کپسوله‌سازی کامل کوئری‌های DNS در قالب بسته‌های رمزدار DoQ.",
            isOptimized = true
        )
    )
)

class UnifiedShieldStore private constructor(private val context: Context) {

    private val TAG = "UnifiedShieldStore"

    private val _state = MutableStateFlow(ShieldStoreState())
    val state: StateFlow<ShieldStoreState> = _state

    /**
     * Activate Panic Mode:
     * 1. Swaps active core to shadow core
     * 2. Activates strict network lock
     * 3. Wipes ephemeral cryptographic caches via SecurityController
     */
    fun triggerPanicMode(reason: String) {
        val current = _state.value
        val oldActive = current.activeCore
        val newActive = current.shadowCore
        val newShadow = if (oldActive == "Hysteria 2 Brutal") "Quantum-Morph v4" else "Hysteria 2 Brutal"

        Log.i(TAG, "🚨 PANIC MODE ACTIVATED ($reason): Swapped from $oldActive to $newActive | Network Locked!")

        SecurityController.getInstance(context).triggerEmergencyRamWipe("Panic Mode: $reason")

        _state.value = current.copy(
            isPanicModeActive = true,
            panicTriggerReason = reason,
            isNetworkLocked = true,
            activeCore = newActive,
            shadowCore = newShadow,
            lastShadowSwapTimestamp = System.currentTimeMillis(),
            isEmergencyWiped = true
        )
    }

    /**
     * Deactivate Panic Mode & release network lock.
     */
    fun deactivatePanicMode() {
        _state.value = _state.value.copy(
            isPanicModeActive = false,
            panicTriggerReason = "",
            isNetworkLocked = false,
            isEmergencyWiped = false
        )
        Log.i(TAG, "Panic mode deactivated - Network lock released.")
    }

    /**
     * Evaluate DPI Heatmap threat score. If aggregate or any node breaches the critical threshold (default 85),
     * automatically activate Panic Mode:
     * - Force transition to secondary shadow core
     * - Enable strict Network Lock
     * - Wipe sensitive memory keys and ephemeral buffers via SecurityController
     */
    fun updateHeatmapScore(nodeId: String, newThreatScore: Int) {
        val current = _state.value
        val threshold = current.dpiCriticalThreshold

        val updatedNodes = current.heatmapNodes.map { node ->
            if (node.id == nodeId) {
                val breached = newThreatScore >= threshold
                node.copy(
                    dpiThreatScore = newThreatScore,
                    isBreached = breached,
                    status = if (breached) "CRITICAL BREACH" else "ACTIVE"
                )
            } else node
        }

        val aggregateThreatScore = if (updatedNodes.isNotEmpty()) {
            updatedNodes.map { it.dpiThreatScore }.average().toInt()
        } else 0

        val hasBreachedNode = updatedNodes.any { it.isBreached } || aggregateThreatScore >= threshold
        _state.value = current.copy(heatmapNodes = updatedNodes)

        if (hasBreachedNode && !current.isPanicModeActive) {
            val breachReason = if (aggregateThreatScore >= threshold) {
                "Aggregate DPI Threat Score exceeded $threshold% ($aggregateThreatScore%) across national exchange nodes"
            } else {
                val breachedNode = updatedNodes.first { it.isBreached }
                "DPI Heatmap Threshold Breached on ${breachedNode.regionName} (${breachedNode.dpiThreatScore}% Threat)"
            }
            triggerPanicMode(breachReason)
        }
    }

    /**
     * One-click 'Apply Optimization' for a given threat signature.
     * Performs correlation analysis, switches protocol, and automatically adjusts
     * core fragmentation, TLS record slicing, and adversarial padding parameters.
     */
     fun applyThreatOptimization(signatureId: String): ThreatIntelSignature? {
         val current = _state.value
         var targetSig: ThreatIntelSignature? = null

         val updatedSigs = current.threatSignatures.map { sig ->
             if (sig.id == signatureId) {
                 targetSig = sig.copy(isOptimized = true)
                 targetSig!!
             } else sig
         }

         _state.value = current.copy(
             threatSignatures = updatedSigs,
             activeOptimizationCount = updatedSigs.count { it.isOptimized }
         )

         targetSig?.let { sig ->
             NovelProtocolsEngine.getInstance().selectProtocol(sig.recommendedProtocolId)
             
             // Automatically adjust core fragmentation & TLS record slicing according to attack vector
             val (splitLength, paddingBytes) = when (sig.id) {
                 "sig-sni-slicer" -> Pair(2, 240) // Aggressive 2-byte TLS ClientHello splitting
                 "sig-flow-ml" -> Pair(3, 310) // High entropy padding injection
                 "sig-rst-injector" -> Pair(4, 180) // Fast UDP burst fragmentation
                 else -> Pair(3, 184)
             }
             
             AiStealthEngine.getInstance().setTlsSplitLength(splitLength)
             AiStealthEngine.getInstance().evaluateTrafficSignal(
                 packetSize = 1380 - paddingBytes,
                 latencyMs = 16,
                 isTcpRst = false,
                 handshakeDurationMs = 22,
                 ispName = "OPTIMIZED-${sig.recommendedProtocolName}"
             )
             Log.i(TAG, "Applied optimized configuration for ${sig.name}: TLS Slice=${splitLength}B, Protocol=${sig.recommendedProtocolName}")
         }

         return targetSig
     }

    fun setDpiCriticalThreshold(threshold: Int) {
        _state.value = _state.value.copy(dpiCriticalThreshold = threshold)
    }

    fun toggleNetworkLock(locked: Boolean) {
        _state.value = _state.value.copy(isNetworkLocked = locked)
    }

    companion object {
        @Volatile
        private var instance: UnifiedShieldStore? = null

        fun getInstance(context: Context): UnifiedShieldStore {
            return instance ?: synchronized(this) {
                instance ?: UnifiedShieldStore(context.applicationContext).also { instance = it }
            }
        }
    }
}
