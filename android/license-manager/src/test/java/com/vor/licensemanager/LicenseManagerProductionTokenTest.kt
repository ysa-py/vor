package com.vor.licensemanager

import com.vor.license.LicenseStatus
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * PRODUCTION-token coverage for the License Manager ("both apps" criterion):
 * a license issued by the repo's real `Issue License` workflow (production
 * Ed25519 key — the same key CI embeds into BOTH release apps via
 * VOR_LICENSE_PUBLIC_KEY) must verify in the License Manager exactly as it
 * does in the main app. The two samples below are REAL workflow issuances,
 * not synthetic CI tokens:
 *
 *  - user-phone-001    (expires 2031-09-10) — the license shipped to the
 *    reporting user of the activation crash.
 *  - crash-repro-001   (expires 2027-09-11) — issued via workflow_dispatch to
 *    reproduce that crash.
 *
 * Fixed verification epochs (not wall clock) keep these assertions stable
 * forever — status at any epoch is a pure function of the signed expiry
 * claim vs. that epoch, which is precisely the offline verification contract.
 */
class LicenseManagerProductionTokenTest {

    private val prodPublicKey = "iVBsYnvBTIyXcWQprbDU0unxxfUJaK3VT-ngvUTZx_A"

    private val userPhone001 =
        "VOR1.eyJ2IjoxLCJpZCI6InVzZXItcGhvbmUtMDAxIiwicHJvZHVjdCI6InZvciIsImlzc3VlZF9hdCI6IjIwMjYtMDktMTBUMTU6NTI6MzZaIiwiZXhwaXJlc19hdCI6IjIwMzEtMDktMTBUMDA6MDA6MDBaIiwiZW50aXRsZW1lbnRzIjp7InRpZXIiOiJzdGFuZGFyZCIsInBsYXRmb3JtcyI6WyJhbmRyb2lkIiwid2luZG93cyIsImxpbnV4Iiwib3BlbndydCIsImlvcyJdfX0.ErVgK_pyiyPZ1uMKlCrzVV3pRbyq1qtmq9gknjDr3NwqjljA_Hg-W_LXNA0c6pxAIGb6Tx9ak8USRb-OhV8yCA"
    private val crashRepro001 =
        "VOR1.eyJ2IjoxLCJpZCI6ImNyYXNoLXJlcHJvLTAwMSIsInByb2R1Y3QiOiJ2b3IiLCJpc3N1ZWRfYXQiOiIyMDI2LTA5LTEwVDE5OjEyOjA3WiIsImV4cGlyZXNfYXQiOiIyMDI3LTA5LTExVDAwOjAwOjAwWiIsImVudGl0bGVtZW50cyI6eyJ0aWVyIjoic3RhbmRhcmQiLCJwbGF0Zm9ybXMiOlsiYW5kcm9pZCIsIndpbmRvd3MiLCJsaW51eCIsIm9wZW53cnQiLCJpb3MiXX19.MHykE-LDnuxVya8BnE7t4ytN1pqHO2K7rst1DiwGUhfX2h2Rp4c9xYgQNBpxvcior8jvLhgLg6nRVzIyIF7oDw"

    /** 2027-01-15-ish: after issuance, before both expiries. */
    private val nowEpoch = 1_800_000_000L

    @Test
    fun `workflow-issued production token verifies VALID in the license manager (both-apps criterion)`() {
        val view = LicenseStore.view(userPhone001, 0L, nowEpoch, prodPublicKey)
        assertEquals(LicenseStatus.VALID, view.result.status)
        assertEquals("user-phone-001", view.result.payload?.id)
        assertEquals("standard", view.result.payload?.tier)
        assertNotNull(view.expiryLocal)            // expiry rendered in local time
        assertNotNull(view.remainingMs)            // countdown computable
        assertTrue(view.remainingMs!! > 0)
    }

    @Test
    fun `second workflow issuance verifies VALID and stores alongside the first (multi-license)`() {
        val list = listOf(
            LicenseStore.StoredLicense(userPhone001, 1L),
            LicenseStore.StoredLicense(crashRepro001, 2L),
        )
        val decoded = LicenseStore.decode(LicenseStore.encode(list))
        assertEquals(2, decoded.size)

        val v1 = LicenseStore.view(decoded[0].token, decoded[0].addedAtMs, nowEpoch, prodPublicKey)
        val v2 = LicenseStore.view(decoded[1].token, decoded[1].addedAtMs, nowEpoch, prodPublicKey)
        assertEquals(LicenseStatus.VALID, v1.result.status)
        assertEquals(LicenseStatus.VALID, v2.result.status)
        assertEquals("user-phone-001", v1.result.payload?.id)
        assertEquals("crash-repro-001", v2.result.payload?.id)
    }

    @Test
    fun `epoch past the signed expiry - status flips to EXPIRED offline (LM side)`() {
        // 2032: past BOTH expiries — no network, just the signed claim vs. epoch.
        val pastBoth = 1_960_000_000L
        val view = LicenseStore.view(userPhone001, 0L, pastBoth, prodPublicKey)
        assertEquals(LicenseStatus.EXPIRED, view.result.status)
        // Expired licenses: no countdown, not "expiring soon", but still listed
        // with a rendered expiry date (status is recomputed live against the clock).
        assertEquals(null, LicenseStore.countdownText(view.remainingMs))
        assertTrue(!LicenseStore.isExpiringSoon(view.remainingMs))
    }

    @Test
    fun `wrong key (dev) cannot verify production tokens - key separation holds`() {
        val devKey = "f54nNpWuth1MHZsbi6sdEODSDvWp7V6XSSDqWtmCyMA"
        val view = LicenseStore.view(userPhone001, 0L, nowEpoch, devKey)
        assertEquals(LicenseStatus.INVALID, view.result.status)
    }
}
