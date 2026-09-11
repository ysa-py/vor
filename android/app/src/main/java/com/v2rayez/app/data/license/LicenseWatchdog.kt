package com.v2rayez.app.data.license

/**
 * Tunnel-time license watchdog (2026-09, "expiry auto-revocation" completion).
 *
 * The connect-time gate in [com.v2rayez.app.data.service.V2RayVpnService.onStartCommand]
 * blocks NEW tunnels when the license is expired/invalid, and the ON_RESUME refresh
 * re-locks the gate UI. Neither of those, however, tears down a tunnel that is
 * ALREADY running when the device clock passes the signed expiry mid-session —
 * the user's requirement is "clock moved past expiry -> VPN disconnects,
 * automatically, offline".
 *
 * This class answers exactly one question on every stats tick:
 * "is it time to re-verify the license, and if so, must the tunnel stop?"
 *
 * Design constraints, in order:
 * 1. NEVER throw — a watchdog that itself crashes the stats loop would be worse
 *    than no watchdog (that is the same class of bug as the P0 activation crash).
 *    A throwing validity probe is treated as "still licensed" so a transient
 *    storage error never kills a user's active tunnel.
 * 2. Pure JVM (injectable clock, injectable probe) so the cadence and the
 *    stop decision are unit-tested without a VPN service harness.
 * 3. Offline by construction — the probe is [LicenseRepository.isValidNow],
 *    which verifies the signed expiry claim against the current clock with
 *    no network call.
 *
 * Cadence: default 60s. Each connection generation creates a fresh watchdog,
 * so the first check lands one interval after connect (the connect-time gate
 * has JUST verified the license; re-verifying in the same second is redundant).
 */
class LicenseWatchdog(
    private val isLicensed: () -> Boolean,
    private val intervalMs: Long = DEFAULT_INTERVAL_MS,
    private val clock: () -> Long = System::currentTimeMillis,
) {

    /** Result of one watchdog tick. */
    enum class Decision {
        /** Not due yet this tick — keep the tunnel and do nothing. */
        SKIP,

        /** Due and the license still verifies — keep the tunnel. */
        KEEP,

        /** Due and the license no longer verifies — stop the tunnel now. */
        STOP,
    }

    private var nextCheckAtMs: Long = clock() + intervalMs

    /**
     * One tick. Decision [Decision.STOP] means the license re-verification
     * failed and the caller must tear the tunnel down. Never throws.
     */
    fun tick(): Decision {
        val now = clock()
        if (now < nextCheckAtMs) return Decision.SKIP
        nextCheckAtMs = now + intervalMs
        val licensed = runCatching { isLicensed() }.getOrDefault(true)
        return if (licensed) Decision.KEEP else Decision.STOP
    }

    companion object {
        /** Re-verify at most once a minute while a tunnel is up. */
        const val DEFAULT_INTERVAL_MS: Long = 60_000L
    }
}
