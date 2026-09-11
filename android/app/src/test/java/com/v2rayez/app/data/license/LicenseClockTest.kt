package com.v2rayez.app.data.license

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import java.util.concurrent.CopyOnWriteArrayList

/**
 * Anti-clock-rollback license clock (v1.0.4, spec §3): the effective
 * verification time is max(device clock, monotonic ratchet, trusted HTTPS
 * time), the ratchet is persisted monotonic, trusted time is bounded so a
 * far-future lie cannot lock the user out, and no failure ever throws.
 */
class LicenseClockTest {

    /** In-memory ratchet store (records writes). */
    private class MemStore(initial: Long = 0L) : ClockRatchetStore {
        @Volatile var value = initial
        val writes = CopyOnWriteArrayList<Long>()
        override suspend fun read(): Long = value
        override suspend fun write(seconds: Long) {
            value = maxOf(value, seconds)
            writes.add(seconds)
        }
    }

    private class ThrowingStore : ClockRatchetStore {
        override suspend fun read(): Long = throw IllegalStateException("store read failed")
        override suspend fun write(seconds: Long) { throw IllegalStateException("store write failed") }
    }

    private fun clock(
        device: () -> Long,
        store: ClockRatchetStore = MemStore(),
        wall: () -> Long = { 0L },
        trusted: suspend () -> Long? = { null },
    ) = LicenseClockCore(store = store, deviceClockSeconds = device, wallClockMs = wall, trustedFetch = trusted)

    @Test
    fun `plain device clock passes through when nothing else is known`() {
        assertEquals(1_000L, clock(device = { 1_000L }).nowSeconds())
        assertEquals(9_999L, clock(device = { 9_999L }).nowSeconds())
    }

    @Test
    fun `wound-back device clock cannot move now below the ratchet`() {
        val store = MemStore()
        val first = clock(device = { 5_000L }, store = store)
        first.nowSeconds() // observes 5_000 → ratchet/persist
        awaitWrites(store)
        // Same persisted store, device wound far back → ratchet still wins.
        val second = clock(device = { 100L }, store = store)
        assertEquals(5_000L, second.nowSeconds())
    }

    @Test
    fun `trusted time raises now past a wound-back device clock`() {
        val c = clock(device = { 1_000L })
        c.acceptTrustedSeconds(50_000L)
        assertEquals(50_000L, c.nowSeconds())
    }

    @Test
    fun `far-future trusted time is rejected (anti lockout guard)`() {
        val c = clock(device = { 1_000L })
        val bogus = 1_000L + LicenseClockCore.MAX_TRUSTED_AHEAD_SECONDS + 5
        c.acceptTrustedSeconds(bogus)
        assertEquals(1_000L, c.nowSeconds())
    }

    @Test
    fun `trusted time below the device clock is harmless`() {
        val c = clock(device = { 60_000L })
        c.acceptTrustedSeconds(59_999L)
        assertEquals(60_000L, c.nowSeconds())
    }

    @Test
    fun `accepted trusted time is itself ratcheted`() {
        val store = MemStore()
        val c = clock(device = { 1_000L }, store = store)
        c.acceptTrustedSeconds(80_000L)
        awaitWrites(store)
        assertEquals(listOf(80_000L), store.writes.filter { it == 80_000L })
    }

    @Test
    fun `throwing store never breaks verification`() {
        val c = clock(device = { 2_000L }, store = ThrowingStore())
        assertEquals(2_000L, c.nowSeconds())
        c.acceptTrustedSeconds(3_000L)
        assertEquals(3_000L, c.nowSeconds())
    }

    @Test
    fun `refreshTrustedTimeAsync is throttled per interval`() {
        var calls = 0
        var wall = 0L
        val c = clock(
            device = { 1L },
            wall = { wall },
            trusted = { calls++; 42L },
        )
        c.refreshTrustedTimeAsync()
        c.refreshTrustedTimeAsync()
        c.refreshTrustedTimeAsync()
        Thread.sleep(80) // let the IO scope run the fetch
        assertEquals(1, calls)
        wall += LicenseClockCore.FETCH_INTERVAL_MS
        c.refreshTrustedTimeAsync()
        Thread.sleep(80)
        assertEquals(2, calls)
    }

    @Test
    fun `trusted fetch failure is silently ignored`() {
        val c = clock(device = { 1_234L }, trusted = { throw java.io.IOException("offline") })
        c.refreshTrustedTimeAsync()
        Thread.sleep(50)
        assertEquals(1_234L, c.nowSeconds())
    }

    private fun awaitWrites(store: MemStore, timeoutMs: Long = 2_000L) {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (store.writes.isEmpty() && System.currentTimeMillis() < deadline) {
            Thread.sleep(10)
        }
    }
}

/**
 * Majority vote over HTTPS Date headers: ≥2 endpoints agreeing within the
 * cluster window (median of that cluster); single answers and full
 * disagreement are refused (null).
 */
class TrustedTimeVoteTest {

    @Test
    fun `two agreeing endpoints form a majority`() {
        assertEquals(100L, TrustedTimeVote.majorityDateSeconds(listOf(100L, 101L)))
    }

    @Test
    fun `one lying endpoint loses against two honest ones`() {
        // 900 is a liar; 200/201 agree.
        assertEquals(200L, TrustedTimeVote.majorityDateSeconds(listOf(900L, 200L, 201L)))
    }

    @Test
    fun `median of the majority cluster`() {
        assertEquals(201L, TrustedTimeVote.majorityDateSeconds(listOf(200L, 201L, 203L, 999L)))
    }

    @Test
    fun `single endpoint cannot form a majority`() {
        assertNull(TrustedTimeVote.majorityDateSeconds(listOf(500L)))
    }

    @Test
    fun `full disagreement yields no answer`() {
        assertNull(TrustedTimeVote.majorityDateSeconds(listOf(100L, 900L, 50_000L)))
    }

    @Test
    fun `cluster window tolerates second-granularity skew`() {
        val base = 1_000_000L
        val dates = (0..4).map { base + it * 30L } // all within ±120s
        assertEquals(base + 60L, TrustedTimeVote.majorityDateSeconds(dates))
    }

    @Test
    fun `empty and invalid inputs`() {
        assertNull(TrustedTimeVote.majorityDateSeconds(emptyList()))
        assertNull(TrustedTimeVote.majorityDateSeconds(listOf(0L, -5L)))
        assertEquals(7L, TrustedTimeVote.majorityDateSeconds(listOf(0L, 7L, 8L)))
    }

    @Test
    fun `rfc1123 parsing`() {
        assertEquals(
            1_728_613_442L, // 2024-10-11T02:24:02Z
            TrustedTimeSource.parseRfc1123("Fri, 11 Oct 2024 02:24:02 GMT"),
        )
        assertNull(TrustedTimeSource.parseRfc1123("not a date"))
    }
}
