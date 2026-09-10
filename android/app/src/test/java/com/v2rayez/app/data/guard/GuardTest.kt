package com.v2rayez.app.data.guard

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** Vor Guard (MSN-GUARD ports) unit tests. */
class GuardTest {

    @Test
    fun verified_connect_requires_real_rx_bytes() {
        val gate = VerifiedConnectGate()
        assertFalse(gate.isVerified())
        gate.onRx(1024)
        assertFalse(gate.isVerified()) // 1 KiB is not enough
        gate.onRx(3072)
        assertTrue(gate.isVerified()) // exactly 4 KiB
    }

    @Test
    fun verified_connect_resets_between_attempts() {
        val gate = VerifiedConnectGate()
        gate.onRx(50_000)
        assertTrue(gate.isVerified())
        gate.reset()
        assertFalse(gate.isVerified())
    }

    @Test
    fun verified_connect_progress_is_bounded() {
        val gate = VerifiedConnectGate()
        assertEquals(0f, gate.progress(), 0.001f)
        gate.onRx(2048)
        assertEquals(0.5f, gate.progress(), 0.001f)
        gate.onRx(999_999)
        assertEquals(1f, gate.progress(), 0.001f)
    }

    @Test
    fun psiphon_ladder_has_five_rungs_with_escalating_order() {
        assertEquals(5, PsiphonLadder.LADDER.size)
        assertEquals((0..4).toList(), PsiphonLadder.LADDER.map { it.id })
        // time budgets are positive and the region budget matches upstream
        assertTrue(PsiphonLadder.LADDER.all { it.timeoutMs > 0 })
        assertEquals(25_000L, PsiphonLadder.REGION_PHASE_BUDGET_MS)
    }

    @Test
    fun psiphon_ladder_remembers_winner() {
        assertEquals(0, PsiphonLadder.startingRung(null).id)
        assertEquals(0, PsiphonLadder.startingRung(99).id) // out-of-range falls back
        assertEquals(2, PsiphonLadder.startingRung(2).id)
    }

    @Test
    fun psiphon_ladder_escalates_then_exhausts() {
        var current = PsiphonLadder.startingRung(0)
        var steps = 0
        while (true) {
            val next = PsiphonLadder.nextRung(current.id)
            if (next == null) break
            current = next
            steps++
        }
        assertEquals(4, steps)
        assertEquals(4, current.id)
        assertNull(PsiphonLadder.nextRung(4))
        assertNotNull(PsiphonLadder.nextRung(0))
    }
}
