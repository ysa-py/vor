package com.v2rayez.app.data.guard

/**
 * Vor Guard — the MSN-GUARD port layer (optional add-on, per the merge
 * spec: "Carry over MSN-GUARD's guard/security module as an optional
 * add-on layer").
 *
 * [VerifiedConnectGate] — MSN-GUARD's "verified-connect" fix: never show
 * "Connected" until real payload bytes have crossed the tunnel. Kills the
 * "fake-connected" state where the UI reports success while the data path
 * is dead. Upstream constant: VERIFIED_RX_BYTES = 4096 (4 KiB).
 */
class VerifiedConnectGate(
    private val requiredRxBytes: Long = REQUIRED_RX_BYTES,
) {
    companion object {
        /** MSN-GUARD upstream constant (mirrored in the service). */
        const val REQUIRED_RX_BYTES: Long = 4096
    }

    private val threshold: Long = requiredRxBytes

    private var observedRx: Long = 0

    /** Record RX bytes observed on the tunnel data path. */
    fun onRx(bytes: Long) {
        if (bytes > 0) {
            observedRx += bytes
        }
    }

    /** Reset for a new connection attempt. */
    fun reset() {
        observedRx = 0
    }

    /** True only when real payload bytes crossed the tunnel. */
    fun isVerified(): Boolean = observedRx >= threshold

    /** Progress 0..1 for UI (0 until verified). */
    fun progress(): Float =
        if (threshold <= 0) 1f else (observedRx.toFloat() / threshold.toFloat()).coerceIn(0f, 1f)
}
