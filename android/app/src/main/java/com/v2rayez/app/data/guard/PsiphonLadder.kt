package com.v2rayez.app.data.guard

/**
 * Psiphon strategy ladder — the MSN-GUARD five-rung escalation with
 * per-device winner memory ("winningStrategyKey"), wrapped around the
 * existing Psiphon engine.
 *
 * Upstream ladder (MsnGuardVpnService): each rung gets a timeout budget;
 * successes are attributed via the active-tunnel notice and remembered per
 * device so the next connect starts at the winning rung.
 */
object PsiphonLadder {

    /** One ladder rung. */
    data class Rung(
        val id: Int,
        val label: String,
        /** Rung timeout budget (ms) before escalating. */
        val timeoutMs: Long,
        /** Protocol families allowed on this rung (Psiphon LimitTunnelProtocols). */
        val allowedProtocols: List<String>,
    )

    /**
     * The five-rung ladder (fronted -> direct escalation, mirroring
     * MSN-GUARD's strategy sequence; region phase budget: 25 s upstream).
     */
    val REGION_PHASE_BUDGET_MS: Long = 25_000

    val LADDER: List<Rung> = listOf(
        Rung(0, "Fronted HTTPS", 15_000, listOf("FRONTED-MEEK-HTTP", "FRONTED-MEEK-HTTPS")),
        Rung(1, "Fronted HTTP", 12_000, listOf("FRONTED-MEEK-HTTP")),
        Rung(2, "Direct TLS", 10_000, listOf("TLS", "HTTPS")),
        Rung(3, "Direct QUIC", 10_000, listOf("QUIC")),
        Rung(4, "Chainable unfronted", 8_000, listOf("UNFRONTED-HTTP", "TCP")),
    )

    /**
     * Starting rung for a device given its remembered winner (null when
     * unknown). Upstream behavior: the remembered rung is tried first, and
     * on failure the ladder continues from the NEXT rung (not from zero).
     */
    fun startingRung(rememberedWinner: Int?): Rung {
        if (rememberedWinner == null || rememberedWinner !in LADDER.indices) {
            return LADDER.first()
        }
        return LADDER[rememberedWinner]
    }

    /** Next rung after a failure at [currentId] (null = ladder exhausted). */
    fun nextRung(currentId: Int): Rung? {
        val next = LADDER.firstOrNull { it.id > currentId }
        return next
    }
}
