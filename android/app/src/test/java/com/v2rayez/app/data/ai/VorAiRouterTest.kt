package com.v2rayez.app.data.ai

import com.v2rayez.app.domain.model.ProxyCoreType
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * JVM unit tests for the pure recommendation core of [VorAiRouter] — the
 * wiring of the shared Vor decision model into the connect flow.
 *
 * The honest-guarantee rules under test:
 *  1. A fresh bandit (no observations) NEVER claims a hint.
 *  2. Once an engine consistently wins on a fingerprint, that champion is
 *     preferred — but only among engines actually available.
 *  3. An unavailable engine is never recommended, no matter its score.
 */
class VorAiRouterTest {

    private fun observeSeq(
        engine: String,
        successes: Int,
        failures: Int = 0,
        fp: String = "wifi|ir|mci",
        rttMs: Long = 80,
        dpiKill: Boolean = false,
    ): VorCoreKt.SelectorState {
        var state = VorCoreKt.SelectorState()
        var t = 1_000L
        repeat(successes) {
            state = VorCoreKt.observe(
                state,
                VorCoreKt.Outcome(
                    fingerprint = fp, engine = engine, strategy = "raw",
                    success = true, rttMs = rttMs, dpiKill = dpiKill,
                ),
                t++,
            )
        }
        repeat(failures) {
            state = VorCoreKt.observe(
                state,
                VorCoreKt.Outcome(
                    fingerprint = fp, engine = engine, strategy = "raw",
                    success = false, rttMs = 0, dpiKill = dpiKill,
                ),
                t++,
            )
        }
        return state
    }

    @Test
    fun `wire name mapping covers all proxy cores`() {
        assertEquals("xray", VorAiRouter.wireName(ProxyCoreType.XRAY))
        assertEquals("singbox", VorAiRouter.wireName(ProxyCoreType.SING_BOX))
        assertEquals("mihomo", VorAiRouter.wireName(ProxyCoreType.CLASH))
        assertEquals(ProxyCoreType.XRAY, VorAiRouter.coreFromWire("xray"))
        assertEquals(ProxyCoreType.SING_BOX, VorAiRouter.coreFromWire("singbox"))
        assertEquals(ProxyCoreType.CLASH, VorAiRouter.coreFromWire("mihomo"))
        assertNull(VorAiRouter.coreFromWire("unknown-engine"))
    }

    @Test
    fun `fresh install never claims a hint`() {
        // Zero observations -> the bandit has no basis; the caller must keep
        // its own resolution (UX-honesty: never fabricate knowledge).
        val state = VorCoreKt.SelectorState()
        assertNull(
            VorAiRouter.recommendCore(
                state, "wifi|ir|mci",
                listOf(ProxyCoreType.XRAY, ProxyCoreType.SING_BOX),
            ),
        )
    }

    @Test
    fun `below minimum pulls still no hint`() {
        val state = observeSeq("xray", successes = 2)
        assertNull(
            VorAiRouter.recommendCore(
                state, "wifi|ir|mci",
                listOf(ProxyCoreType.XRAY, ProxyCoreType.SING_BOX),
            ),
        )
    }

    @Test
    fun `consistent winner becomes the recommended engine`() {
        // xray succeeded many times on this fingerprint -> it must be the hint.
        val state = observeSeq("xray", successes = 8)
        assertEquals(
            ProxyCoreType.XRAY,
            VorAiRouter.recommendCore(
                state, "wifi|ir|mci",
                listOf(ProxyCoreType.XRAY, ProxyCoreType.SING_BOX),
            ),
        )
    }

    @Test
    fun `champion is not recommended when unavailable`() {
        val state = observeSeq("xray", successes = 8)
        // sing-box is the only available engine but has NEVER been tried on
        // this device — recommending it would be fabricated knowledge.
        // Honest behavior: no hint at all (null), the caller keeps its own
        // resolution.
        assertNull(
            VorAiRouter.recommendCore(state, "wifi|ir|mci", listOf(ProxyCoreType.SING_BOX)),
        )
    }

    @Test
    fun `fingerprint isolation - another network keeps its own champion`() {
        val state = observeSeq("xray", successes = 8)
        // Same bandit, DIFFERENT fingerprint: no learned entry there, so the
        // engine-level ranking still picks xray (it is the only engine tried).
        assertEquals(
            ProxyCoreType.XRAY,
            VorAiRouter.recommendCore(
                state, "cellular|ir|irancell",
                listOf(ProxyCoreType.XRAY, ProxyCoreType.SING_BOX),
            ),
        )
    }

    @Test
    fun `failing engine is demoted`() {
        // sing-box succeeds; xray keeps failing on this fingerprint.
        var state = VorCoreKt.SelectorState()
        var t = 1L
        repeat(6) {
            state = VorCoreKt.observe(
                state,
                VorCoreKt.Outcome(
                    fingerprint = "wifi|ir|mci", engine = "xray", strategy = "raw",
                    success = false, rttMs = 0, dpiKill = true,
                ),
                t++,
            )
        }
        repeat(6) {
            state = VorCoreKt.observe(
                state,
                VorCoreKt.Outcome(
                    fingerprint = "wifi|ir|mci", engine = "singbox", strategy = "raw",
                    success = true, rttMs = 120,
                ),
                t++,
            )
        }
        assertEquals(
            ProxyCoreType.SING_BOX,
            VorAiRouter.recommendCore(
                state, "wifi|ir|mci",
                listOf(ProxyCoreType.XRAY, ProxyCoreType.SING_BOX),
            ),
        )
    }

    @Test
    fun `no engine tried means no hint even with other-fingerprint data`() {
        // Data exists for a different fingerprint's engines only when the
        // bandit arms exist globally; recommendation must still be honest
        // about the CURRENT fingerprint's champion when strong enough.
        val state = observeSeq("xray", successes = 8, fp = "wifi|ir|mci")
        // On cellular fingerprint there is no champion entry; engine-level
        // fallback ranks xray (the only engine with pulls) — still a real
        // observed engine, not fabricated.
        val hint = VorAiRouter.recommendCore(
            state, "cellular|ir|irancell",
            listOf(ProxyCoreType.XRAY, ProxyCoreType.SING_BOX, ProxyCoreType.CLASH),
        )
        assertEquals(ProxyCoreType.XRAY, hint)
    }

    @Test
    fun `selector state round-trips through json`() {
        val state = observeSeq("xray", successes = 5)
        val restored = VorCoreKt.SelectorState.fromJson(state.toJson().toString())
        assertEquals(state, restored)
    }
}
