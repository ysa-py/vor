package com.vor.licensemanager

import com.vor.license.LicenseStatus
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pure-JVM tests for the multi-license store: JSON codec round-trip,
 * per-license status recomputation against the clock, local-time expiry
 * rendering, and the nearing-expiry countdown.
 *
 * Tokens are REAL Ed25519-signed licenses (dev keypair — the LM debug build's
 * embedded key): one already expired, one valid until 2030.
 */
class LicenseStoreTest {

    private val devPublicKey = "f54nNpWuth1MHZsbi6sdEODSDvWp7V6XSSDqWtmCyMA"

    private val expiredToken =
        "VOR1.eyJ2IjoxLCJpZCI6ImV4cGlyZWQtdGVzdC0wMDEiLCJwcm9kdWN0Ijoidm9yIiwiaXNzdWVkX2F0IjoiMjAyNi0wOS0xMFQxOTo0ODo1OFoiLCJleHBpcmVzX2F0IjoiMjAyNi0wMS0wMVQwMDowMDowMFoiLCJlbnRpdGxlbWVudHMiOnsidGllciI6InN0YW5kYXJkIiwicGxhdGZvcm1zIjpbImFuZHJvaWQiLCJ3aW5kb3dzIiwibGludXgiLCJvcGVud3J0IiwiaW9zIl19fQ.uATC5ahR89pGhn25Sq5fzoHkSNclS3K5R1lBGdzsCNTIyx082nI1n6CqXloHo_J7StAT04XOqtFDONLpcxkXAw"
    private val validToken =
        "VOR1.eyJ2IjoxLCJpZCI6InZhbGlkLXRlc3QtMDAxIiwicHJvZHVjdCI6InZvciIsImlzc3VlZF9hdCI6IjIwMjYtMDktMTBUMTk6NDg6NThaIiwiZXhwaXJlc19hdCI6IjIwMzAtMDEtMDFUMDA6MDA6MDBaIiwiZW50aXRsZW1lbnRzIjp7InRpZXIiOiJzdGFuZGFyZCIsInBsYXRmb3JtcyI6WyJhbmRyb2lkIiwid2luZG93cyIsImxpbnV4Iiwib3BlbndydCIsImlvcyJdfX0.LujI-T06B61sjhAjysCt823aTyLhkvx4j7lCRvTY-dOg-9tp6KVFUESfsoSSXuBM99WJzTLoLofjdU6408GXCQ"

    private val nowEpoch = 1_785_000_000L // after the expiry of expired-test-001, before valid-test-001

    @Test
    fun codec_roundTrip_preservesOrderAndTokens() {
        val list = listOf(
            LicenseStore.StoredLicense(expiredToken, 1L),
            LicenseStore.StoredLicense(validToken, 2L),
        )
        val decoded = LicenseStore.decode(LicenseStore.encode(list))
        assertEquals(list, decoded)
    }

    @Test
    fun codec_corruptInput_yieldsEmptyList_neverThrows() {
        assertTrue(LicenseStore.decode("not json").isEmpty())
        assertTrue(LicenseStore.decode("").isEmpty())
        // Unknown fields forward-compat: older entries still decode.
        assertEquals(1, LicenseStore.decode("""[{"token":"abc","addedAtMs":5,"futureField":1}]""").size)
    }

    @Test
    fun view_expiredToken_statusExpired_noCountdown() {
        val view = LicenseStore.view(expiredToken, 0L, nowEpoch, devPublicKey)
        assertEquals(LicenseStatus.EXPIRED, view.result.status)
        assertEquals("expired-test-001", view.result.payload?.id)
        assertNull(LicenseStore.countdownText(view.remainingMs))
        assertFalse(LicenseStore.isExpiringSoon(view.remainingMs))
    }

    @Test
    fun view_validToken_statusValid_expiryRenderedInLocalTime() {
        val view = LicenseStore.view(validToken, 0L, nowEpoch, devPublicKey)
        assertEquals(LicenseStatus.VALID, view.result.status)
        assertNotNull(view.expiryLocal) // local-time rendering must succeed
        assertNotNull(view.remainingMs)
        // Rough sanity: ~3.4 years remaining at 2026-08 → more than 1000 days.
        assertTrue(view.remainingMs!! > 1000L * 86_400_000L)
    }

    @Test
    fun countdown_formatsHumanReadable() {
        assertEquals("2d 3h", LicenseStore.countdownText((2L * 86_400_000L) + 3L * 3_600_000L))
        assertEquals("5h", LicenseStore.countdownText(5L * 3_600_000L))
        assertEquals("12d 0h", LicenseStore.countdownText(12L * 86_400_000L))
        assertNull(LicenseStore.countdownText(0L))
        assertNull(LicenseStore.countdownText(-5L))
        assertNull(LicenseStore.countdownText(null))
    }

    @Test
    fun expiringSoon_boundary() {
        assertTrue(LicenseStore.isExpiringSoon(29L * 86_400_000L))
        assertFalse(LicenseStore.isExpiringSoon(31L * 86_400_000L))
        assertFalse(LicenseStore.isExpiringSoon(0L))
        assertFalse(LicenseStore.isExpiringSoon(null))
    }

    @Test
    fun view_garbageToken_isInvalid_silently() {
        val view = LicenseStore.view("VOR1.garbage.token", 0L, nowEpoch, devPublicKey)
        assertEquals(LicenseStatus.INVALID, view.result.status)
        assertNull(view.expiryLocal)
    }

    @Test
    fun tamperedExpiry_invalidatesSignature() {
        // Corrupt the tail of the VALID token — signature verification must
        // fail and the status must be INVALID (tamper-evident payload).
        val tampered = validToken.dropLast(8) + "XXXXXXXX"
        val view = LicenseStore.view(tampered, 0L, nowEpoch, devPublicKey)
        assertEquals(LicenseStatus.INVALID, view.result.status)
    }
}
