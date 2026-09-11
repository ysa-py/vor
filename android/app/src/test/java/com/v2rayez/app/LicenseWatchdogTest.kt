package com.v2rayez.app

import com.v2rayez.app.data.license.LicenseWatchdog
import com.v2rayez.app.data.license.LicenseWatchdog.Decision
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Tunnel-time license watchdog (2026-09 "expiry auto-revocation" completion).
 *
 * The completion criterion under test: "clock moved past the signed expiry ->
 * the active VPN disconnects automatically". The connect-time gate and the
 * ON_RESUME gate cover NEW activity; the watchdog covers an ALREADY-RUNNING
 * tunnel. It must:
 *  - re-verify at a bounded cadence (not every 1s stats tick),
 *  - STOP the tunnel when the license stops verifying,
 *  - NEVER kill the tunnel when the probe itself fails (transient storage
 *    error -> treat as still licensed — a crashing watchdog would recreate
 *    the P0 class of bug),
 *  - never throw.
 *
 * Pure JVM: injectable clock + probe, no Android/Robolectric dependency.
 */
class LicenseWatchdogTest {

    private class Clock(var nowMs: Long = 0L) {
        fun advance(ms: Long) { nowMs += ms }
    }

    @Test
    fun `first check is one interval after connect - connect gate just verified`() {
        val clock = Clock()
        val wd = LicenseWatchdog(isLicensed = { true }, intervalMs = 60_000, clock = { clock.nowMs })
        // Stats ticks in the first minute must be SKIP (no redundant re-verify).
        repeat(59) {
            clock.advance(1_000)
            assertEquals(Decision.SKIP, wd.tick())
        }
    }

    @Test
    fun `due and licensed - KEEP`() {
        val clock = Clock()
        val wd = LicenseWatchdog(isLicensed = { true }, intervalMs = 60_000, clock = { clock.nowMs })
        clock.advance(60_001)
        assertEquals(Decision.KEEP, wd.tick())
    }

    @Test
    fun `clock moved past expiry mid-session - STOP (the device scenario)`() {
        val clock = Clock()
        var licensed = true // flips exactly like verify(expiry) flips at the boundary
        val wd = LicenseWatchdog(isLicensed = { licensed }, intervalMs = 60_000, clock = { clock.nowMs })

        // Session starts licensed (connect gate passed).
        clock.advance(60_001)
        assertEquals(Decision.KEEP, wd.tick())

        // Device clock passes the signed expiry while the tunnel is UP.
        licensed = false
        // Watchdog re-verifies at its next due slot...
        clock.advance(60_001)
        assertEquals(Decision.STOP, wd.tick())
    }

    @Test
    fun `interval is rescheduled after each check`() {
        val clock = Clock()
        var calls = 0
        val wd = LicenseWatchdog(isLicensed = { calls++; true }, intervalMs = 60_000, clock = { clock.nowMs })

        clock.advance(60_000)
        assertEquals(Decision.KEEP, wd.tick())
        // Not due again immediately after a check.
        clock.advance(59_999)
        assertEquals(Decision.SKIP, wd.tick())
        clock.advance(2)
        assertEquals(Decision.KEEP, wd.tick())
        // Exactly two probe calls, not one per stats tick.
        assertEquals(2, calls)
    }

    @Test
    fun `probe never invoked while not due`() {
        val clock = Clock()
        var calls = 0
        val wd = LicenseWatchdog(isLicensed = { calls++; true }, intervalMs = 60_000, clock = { clock.nowMs })
        // 119 ticks of 500ms = 59_500ms < the 60s interval: no probe call at all.
        repeat(119) { clock.advance(500); wd.tick() }
        assertEquals(0, calls)
        // One more 500ms step lands exactly ON the due boundary (>= interval): due now.
        clock.advance(500)
        assertEquals(Decision.KEEP, wd.tick())
        assertEquals(1, calls)
    }

    @Test
    fun `throwing probe never kills the tunnel and never propagates`() {
        val clock = Clock()
        val wd = LicenseWatchdog(
            isLicensed = { throw IllegalStateException("transient DataStore failure") },
            intervalMs = 60_000,
            clock = { clock.nowMs },
        )
        clock.advance(60_001)
        assertEquals(Decision.KEEP, wd.tick())
        // And it keeps ticking afterwards without throwing.
        clock.advance(60_001)
        assertEquals(Decision.KEEP, wd.tick())
    }

    @Test
    fun `STOP repeats if the caller keeps ticking (idempotent verdict)`() {
        val clock = Clock()
        val wd = LicenseWatchdog(isLicensed = { false }, intervalMs = 60_000, clock = { clock.nowMs })
        clock.advance(60_001)
        assertEquals(Decision.STOP, wd.tick())
        clock.advance(60_001)
        assertEquals(Decision.STOP, wd.tick())
    }
}
