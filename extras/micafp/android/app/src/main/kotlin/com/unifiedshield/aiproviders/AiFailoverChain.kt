package com.unifiedshield.aiproviders

import android.content.Context
import android.util.Log
import com.unifiedshield.TunnelManager
import com.unifiedshield.license.AuditLog

// =============================================================================
// MICAFP Directive v6 — Addendum B.9 + B.10 + C3 failover chain.
//
// Resolution order for a request (B.10, exactly):
//   (a) user-supplied mirror/alternate base URL of that provider
//   (b) the provider's primary base URL (tunnel-routed wiring is the
//       human-merge-gated commit; direct today, honestly reported)
//   (c) next provider in the USER-defined priority list
//   (d) internal MICAFP AI layer (B.9 guaranteed fallback) — always
//       available, no API key, reuses the EXISTING daemon telemetry
//       (TunnelManager/DpiDetector telemetry) with explainable output.
//
// Rules honoured here:
//  * No forked parallel AI system — internal layer reads existing signals.
//  * UI must show which layer answered (attribution is never faked).
//  * External layer is fully user-controllable via the master switch.
// =============================================================================

enum class AiAnswerLayer { EXTERNAL_PROVIDER, INTERNAL_LOCAL_ANALYSIS }

data class AiAnswer(
    val layer: AiAnswerLayer,
    val providerId: String?,       // null for internal layer
    val providerName: String,      // display name or "Local analysis" / "تحلیل داخلی"
    val text: String,
    val viaBaseUrl: String? = null,
    val latencyMs: Long = 0,
    val degradedNote: String? = null
)

class AiFailoverChain private constructor(private val context: Context) {

    private val TAG = "MicafpAiChain"
    private val store = AiProviderStore.getInstance(context)
    private val audit = AuditLog.getInstance(context)

    /**
     * Ask with full failover. Never throws; never fabricates an external
     * answer. If everything external fails and internal layer produces a
     * grounded summary, attribution is INTERNAL_LOCAL_ANALYSIS.
     */
    suspend fun ask(systemPrompt: String, userPrompt: String): AiAnswer {
        val registry = AiProviderRegistry.load(context)

        if (!store.masterSwitch) {
            return internalAnswer(systemPrompt, userPrompt, "master-switch-off")
        }

        // Ordered candidate list: user priority first, registry order for the rest
        val userOrder = store.getPriorityOrder()
        val ordered = (userOrder + registry.providers.map { it.id }.filter { it !in userOrder })
            .distinct()
            .mapNotNull { id -> registry.providers.firstOrNull { it.id == id } }
            .filter { store.getConfig(it.id).enabled && store.getConfig(it.id).apiKey.isNotBlank() }

        var lastReason = "no-provider-configured"
        for (def in ordered) {
            val cfg = store.getConfig(def.id)

            // (a) user mirror, (b) primary — B.10 order; then next provider (c)
            val urls = listOf(cfg.mirrorUrl, cfg.baseUrlOverride, def.baseUrlDefault)
                .filter { it.isNotBlank() }
                .distinct()

            for (url in urls) {
                val result = AiProviderClient.complete(def, cfg, url, systemPrompt, userPrompt)
                when (result) {
                    is AiCallResult.Success -> {
                        audit.append(
                            "AI_FAILOVER",
                            "Provider ${def.id} answered via $url in ${result.latencyMs}ms"
                        )
                        return AiAnswer(
                            layer = AiAnswerLayer.EXTERNAL_PROVIDER,
                            providerId = def.id,
                            providerName = def.displayName,
                            text = result.text,
                            viaBaseUrl = result.viaBaseUrl,
                            latencyMs = result.latencyMs
                        )
                    }
                    is AiCallResult.Failure -> {
                        lastReason = "${def.id}@${url}: ${result.reason}"
                        Log.w(TAG, "Failover step failed: $lastReason")
                    }
                }
            }
        }

        audit.append("AI_FAILOVER", "All external providers failed ($lastReason) → internal local analysis answered")
        return internalAnswer(systemPrompt, userPrompt, lastReason)
    }

    /**
     * B.9 guaranteed fallback: grounded, explainable summary produced from
     * the EXISTING daemon telemetry — no parallel AI stack, no invention.
     */
    private fun internalAnswer(systemPrompt: String, userPrompt: String, degradeReason: String): AiAnswer {
        val tm = TunnelManager.getInstance(context)
        val s = tm.stats.value
        val text = buildString {
            appendLine("MICAFP local analysis (no external provider used):")
            appendLine("• Tunnel: ${if (s.connected) "connected" else "disconnected"} via ${s.currentCore} core")
            appendLine("• Live throughput: ↓${s.downloadSpeedKbps} KB/s ↑${s.uploadSpeedKbps} KB/s")
            appendLine("• DPI pressure score: ${"%.2f".format(s.dpiScore)} (evasion active)")
            appendLine("• Packet loss: ${"%.1f".format(s.packetLossRate)}% — ISP ${s.activeIsp}")
            if (degradeReason != "master-switch-off") {
                appendLine("• External layer unreachable: $degradeReason")
            }
        }
        return AiAnswer(
            layer = AiAnswerLayer.INTERNAL_LOCAL_ANALYSIS,
            providerId = null,
            providerName = "Local analysis",
            text = text,
            degradedNote = degradeReason
        )
    }

    companion object {
        @Volatile private var instance: AiFailoverChain? = null
        fun getInstance(context: Context): AiFailoverChain =
            instance ?: synchronized(this) {
                instance ?: AiFailoverChain(context.applicationContext).also { instance = it }
            }
    }
}
