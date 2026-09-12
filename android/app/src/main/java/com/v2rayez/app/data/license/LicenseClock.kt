package com.v2rayez.app.data.license

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import java.util.Locale
import javax.inject.Inject
import javax.inject.Singleton

/**
 * Offline anti-clock-rollback for the license gate (spec §3, 2026-09 v1.0.4).
 *
 * The signed license token embeds an absolute expiry. Verifying it against a
 * device clock that the user can freely wind backward would make expiry
 * enforcement trivially bypassable (`settings → date → 2020` and the license
 * never expires). [LicenseClockCore] composes three time sources:
 *
 *  1. the device clock (what the OS says right now),
 *  2. a monotonic **ratchet** — the highest time this installation has EVER
 *     observed (persisted via [ClockRatchetStore]; it can only grow), and
 *  3. an opportunistic **trusted time** — a majority vote over the `Date`
 *     response headers of several independent HTTPS endpoints, fetched at
 *     most once per [FETCH_INTERVAL_MS] and never required for verification.
 *
 * The effective "now" is `max(device, ratchet, trusted)`:
 *
 *  - Winding the clock backward can never move now below the ratchet — an
 *    expired license stays expired. The rollback survives a reboot because the
 *    ratchet is persisted.
 *  - A legitimately-offline device still verifies exactly as before (the
 *    trusted fetch simply fails and is ignored).
 *  - Trusted time is accepted only within [MAX_TRUSTED_AHEAD_SECONDS] of the
 *    device clock, so a hijacked captive portal cannot lock a user out by
 *    serving a far-future date either.
 *
 * Design rules (same as [LicenseWatchdog]):
 *  - NEVER throw — any failure degrades to plain device-clock verification,
 *    which is the behavior the app had before this class existed.
 *  - Pure JVM (injectable lambdas) so every rule above is unit-testable.
 *
 * Honest limits (documented, not hidden): a user with root and a patched
 * binary can defeat ANY client-side time source; this raises the bar from
 * "open settings, set the date back" to "root + code modification".
 */
open class LicenseClockCore(
    private val store: ClockRatchetStore,
    private val deviceClockSeconds: () -> Long = { System.currentTimeMillis() / 1000 },
    private val wallClockMs: () -> Long = { System.currentTimeMillis() },
    private val trustedFetch: suspend () -> Long? = { null },
) {

    @Volatile
    private var ratchetSeconds: Long = 0L

    /**
     * Ratchet hydration state. `0` = not attempted, `1` = in progress,
     * `2` = done (success OR failure — never retried, so a dead store can
     * never wedge verification; the clock then behaves like v1.0.3).
     */
    @Volatile
    private var hydrationState: Int = 0

    @Volatile
    private var trustedSeconds: Long = 0L

    @Volatile
    private var lastFetchAttemptMs: Long = Long.MIN_VALUE / 2

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    /**
     * The effective current time in epoch seconds — the value every license
     * verification MUST use instead of `System.currentTimeMillis()/1000`.
     * Non-throwing, and feeds the ratchet on every call so the device's honest
     * progression is itself monotonic-persisted.
     *
     * The FIRST call of a process hydrates the persisted ratchet with a
     * bounded blocking read ([HYDRATION_TIMEOUT_MS] — once, worst case ~50 ms,
     * far below ANR thresholds). Without this, a restart right after a clock
     * rollback could verify one token against the raw device clock before the
     * async hydration lands — exactly the hole the ratchet exists to close.
     * A failed/timed-out hydration degrades to v1.0.3 behavior (device clock
     * only) and is never retried, so a dead store cannot stall verification.
     */
    fun nowSeconds(): Long {
        if (hydrationState == 0) hydrateRatchetOnce()
        val device = runCatching(deviceClockSeconds).getOrDefault(0L)
        val now = maxOf(device, ratchetSeconds, trustedSeconds)
        if (now > ratchetSeconds) {
            ratchetSeconds = now
            persistAsync(now)
        }
        return now
    }

    private fun hydrateRatchetOnce() {
        hydrationState = 1
        runCatching {
            kotlinx.coroutines.runBlocking {
                kotlinx.coroutines.withTimeoutOrNull(HYDRATION_TIMEOUT_MS) {
                    store.read()
                }
            }
        }.getOrNull()?.let { persisted ->
            if (persisted > ratchetSeconds) ratchetSeconds = persisted
        }
        hydrationState = 2
    }

    /** Latest trusted-time sample observed (for diagnostics/UI; 0 = none). */
    fun trustedSecondsSnapshot(): Long = trustedSeconds

    /** Latest monotonic ratchet value observed (for the native DRM core; 0 = none). */
    fun ratchetSecondsSnapshot(): Long = ratchetSeconds

    /**
     * Opportunistic trusted-time refresh — fire-and-forget, throttled to one
     * network attempt per [FETCH_INTERVAL_MS]. Safe to call from any hot path
     * (activation, app start, watchdog ticks): when the throttle trips it is a
     * no-op, and the fetch itself never blocks the caller.
     */
    fun refreshTrustedTimeAsync() {
        val nowMs = wallClockMs()
        if (nowMs - lastFetchAttemptMs < FETCH_INTERVAL_MS) return
        lastFetchAttemptMs = nowMs
        scope.launch {
            runCatching { trustedFetch() }.getOrNull()?.let { trusted ->
                acceptTrustedSeconds(trusted)
            }
        }
    }

    /** Accept a trusted-time sample (also called directly by tests). */
    fun acceptTrustedSeconds(trusted: Long) {
        val device = runCatching(deviceClockSeconds).getOrDefault(0L)
        // Anti-future-date guard: a hijacked endpoint must not be able to lock
        // the user out by claiming a date far beyond the device clock.
        if (trusted > device + MAX_TRUSTED_AHEAD_SECONDS) return
        if (trusted > trustedSeconds) trustedSeconds = trusted
        if (trusted > ratchetSeconds) {
            ratchetSeconds = trusted
            persistAsync(trusted)
        }
    }

    private fun persistAsync(seconds: Long) {
        scope.launch {
            runCatching { store.write(seconds) }
        }
    }

    companion object {
        /** One trusted-time network attempt per 6 hours. */
        const val FETCH_INTERVAL_MS: Long = 6 * 3600_000L

        /** Bounded blocking budget for the one-shot ratchet hydration. */
        const val HYDRATION_TIMEOUT_MS: Long = 50L

        /** Trusted time is only believable within 90 days of the device clock. */
        const val MAX_TRUSTED_AHEAD_SECONDS: Long = 90L * 24 * 3600
    }
}

/** Persistence for the monotonic ratchet (DataStore-backed on device). */
interface ClockRatchetStore {
    /** Highest epoch-second ever persisted (0 when nothing stored). */
    suspend fun read(): Long

    /** Persist the new ratchet value (monotonic by construction). */
    suspend fun write(seconds: Long)
}

/**
 * Majority vote over the `Date` headers of several independent HTTPS endpoints.
 *
 * Pure function so the voting rule is unit-testable. Dates are clustered with
 * a ±[CLUSTER_WINDOW_SECONDS] tolerance (HTTP Date headers are second-granular
 * and small clock skews are normal):
 *  - the largest cluster with ≥2 agreeing endpoints wins — a single lying
 *    endpoint cannot outvote two honest ones;
 *  - if every endpoint disagrees (all singleton clusters) the answer is null
 *    (no majority — refuse to guess);
 *  - a single response cannot form a majority and returns null.
 */
object TrustedTimeVote {

    const val CLUSTER_WINDOW_SECONDS: Long = 120L

    fun majorityDateSeconds(dates: List<Long>): Long? {
        val valid = dates.filter { it > 0 }.sorted()
        if (valid.size < 2) return null
        // Sliding window over the sorted dates: find the densest ±window cluster.
        var bestStart = 0
        var bestCount = 1
        var start = 0
        for (end in valid.indices) {
            while (valid[end] - valid[start] > CLUSTER_WINDOW_SECONDS) start++
            if (end - start + 1 > bestCount) {
                bestCount = end - start + 1
                bestStart = start
            }
        }
        if (bestCount < 2) return null
        val cluster = valid.subList(bestStart, bestStart + bestCount)
        return cluster[(cluster.size - 1) / 2] // lower median of the majority cluster
    }
}

/**
 * Device-side wiring: OkHttp-backed trusted-time fetcher + DataStore-backed
 * ratchet. Both are failure-tolerant by construction — see [LicenseClockCore].
 */
@Singleton
class LicenseClock @Inject constructor(
    store: DataStoreClockRatchetStore,
    source: TrustedTimeSource,
) : LicenseClockCore(
    store = store,
    deviceClockSeconds = { System.currentTimeMillis() / 1000 },
    wallClockMs = { System.currentTimeMillis() },
    trustedFetch = { source.fetchMajorityDateSeconds() },
)

/**
 * HTTPS `Date`-header fetcher. Endpoints are independent, globally anycast CDNs;
 * each request is a plain GET whose body is never read (headers only). No
 * retries, short timeouts — a filtered/offline network simply yields no vote.
 */
@Singleton
class TrustedTimeSource @Inject constructor(
    private val client: okhttp3.OkHttpClient,
) {
    private val quickClient = client.newBuilder()
        .connectTimeout(java.time.Duration.ofSeconds(5))
        .readTimeout(java.time.Duration.ofSeconds(5))
        .callTimeout(java.time.Duration.ofSeconds(8))
        .build()

    suspend fun fetchMajorityDateSeconds(): Long? {
        val dates = ENDPOINTS.mapNotNull { url ->
            runCatching {
                quickClient.newCall(
                    okhttp3.Request.Builder().url(url).head().build()
                ).execute().use { response ->
                    response.header("Date")?.let { parseRfc1123(it) }
                }
            }.getOrNull()
        }
        return TrustedTimeVote.majorityDateSeconds(dates)
    }

    companion object {
        /** Independent endpoints whose Date headers carry server time. */
        val ENDPOINTS: List<String> = listOf(
            "https://www.google.com/generate_204",
            "https://cloudflare.com/cdn-cgi/trace",
            "https://www.microsoft.com/",
            "https://github.com/",
            "https://captive.apple.com/hotspot-detect.html",
        )

        /** RFC 1123 / RFC 7231 IMF-fixdate, always GMT. */
        fun parseRfc1123(value: String): Long? = runCatching {
            val format = java.text.SimpleDateFormat("EEE, dd MMM yyyy HH:mm:ss 'GMT'", Locale.US)
            format.timeZone = java.util.TimeZone.getTimeZone("GMT")
            format.parse(value)?.time?.div(1000)
        }.getOrNull()
    }
}
