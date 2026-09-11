package com.v2rayez.app

import android.app.Application
import android.content.Context
import androidx.test.core.app.ApplicationProvider
import com.v2rayez.app.data.license.LicenseRepository
import com.vor.license.LicenseStatus
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.After
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/**
 * Expiry auto-revocation enforcement (engineering-task requirement): with a
 * stored license whose SIGNED expiry claim is in the past, the repository must
 * report "not valid" from every check the app performs — launch-time gate
 * hydration (gateState), connect-time gating (recheck/isValidNow) — purely
 * offline against the device clock, with no network call and no manual step.
 *
 * Tokens are REAL Ed25519-signed licenses (dev keypair, same wire format as
 * production issuance): one already expired, one still valid.
 */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [35], application = Application::class)
class LicenseExpiryEnforcementTest {

    private val expiredToken =
        "VOR1.eyJ2IjoxLCJpZCI6ImV4cGlyZWQtdGVzdC0wMDEiLCJwcm9kdWN0Ijoidm9yIiwiaXNzdWVkX2F0IjoiMjAyNi0wOS0xMFQxOTo0ODo1OFoiLCJleHBpcmVzX2F0IjoiMjAyNi0wMS0wMVQwMDowMDowMFoiLCJlbnRpdGxlbWVudHMiOnsidGllciI6InN0YW5kYXJkIiwicGxhdGZvcm1zIjpbImFuZHJvaWQiLCJ3aW5kb3dzIiwibGludXgiLCJvcGVud3J0IiwiaW9zIl19fQ.uATC5ahR89pGhn25Sq5fzoHkSNclS3K5R1lBGdzsCNTIyx082nI1n6CqXloHo_J7StAT04XOqtFDONLpcxkXAw"
    private val validToken =
        "VOR1.eyJ2IjoxLCJpZCI6InZhbGlkLXRlc3QtMDAxIiwicHJvZHVjdCI6InZvciIsImlzc3VlZF9hdCI6IjIwMjYtMDktMTBUMTk6NDg6NThaIiwiZXhwaXJlc19hdCI6IjIwMzAtMDEtMDFUMDA6MDA6MDBaIiwiZW50aXRsZW1lbnRzIjp7InRpZXIiOiJzdGFuZGFyZCIsInBsYXRmb3JtcyI6WyJhbmRyb2lkIiwid2luZG93cyIsImxpbnV4Iiwib3BlbndydCIsImlvcyJdfX0.LujI-T06B61sjhAjysCt823aTyLhkvx4j7lCRvTY-dOg-9tp6KVFUESfsoSSXuBM99WJzTLoLofjdU6408GXCQ"

    private lateinit var repository: LicenseRepository

    /** Device-wired license clock (trusted fetch is inert in unit tests). */
    private fun testLicenseClock(): com.v2rayez.app.data.license.LicenseClock =
        com.v2rayez.app.data.license.LicenseClock(
            com.v2rayez.app.data.license.DataStoreClockRatchetStore(ApplicationProvider.getApplicationContext<Context>()),
            com.v2rayez.app.data.license.TrustedTimeSource(okhttp3.OkHttpClient()),
        )

    @Before
    fun setUp() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        repository = LicenseRepository(context, testLicenseClock())
        runBlocking { runCatching { repository.clear() } }
    }

    @After
    fun tearDown() {
        runBlocking { runCatching { repository.clear() } }
    }

    private fun storedState(): com.v2rayez.app.data.license.LicenseRepository.GateState =
        runBlocking {
            withTimeout(5_000) { repository.gateState.first { it.hydrated } }
        }

    @Test
    fun expiredLicense_launchGateState_showsExpired_notValid() {
        runBlocking { repository.activate(expiredToken) }
        val state = storedState()
        check(state.status == LicenseStatus.EXPIRED) { "expected EXPIRED, got ${state.status}" }
        check(state.payload?.id == "expired-test-001")
    }

    @Test
    fun expiredLicense_connectTimeRecheck_refusesConnection() {
        runBlocking { repository.activate(expiredToken) }
        val allowed = runBlocking { repository.recheck() }
        check(!allowed) { "VPN connect must be refused for an expired license (recheck()==true)" }
    }

    @Test
    fun expiredLicense_isValidNow_isFalse() {
        runBlocking { repository.activate(expiredToken) }
        check(!repository.isValidNow()) { "isValidNow() must be false for an expired license" }
    }

    @Test
    fun validLicense_allChecksPass() {
        runBlocking { repository.activate(validToken) }
        check(storedState().status == LicenseStatus.VALID)
        check(runBlocking { repository.recheck() })
        check(repository.isValidNow())
    }

    @Test
    fun garbageToken_neverCrashes_activateReturnsInvalid() {
        val result = runBlocking { repository.activate("VOR1.not-a-real.token") }
        check(result.status == LicenseStatus.INVALID)
        // Gate stays locked-but-hydrated.
        val state = storedState()
        check(state.hydrated && state.status == LicenseStatus.INVALID)
    }

    @Test
    fun refresh_afterExpiry_reLocksAutomatically() {
        // Activate the VALID token, then simulate time passing its expiry by
        // re-activating the expired one and refreshing — the gate must re-lock
        // purely from the clock, no network involved.
        runBlocking {
            repository.activate(validToken)
            check(storedState().status == LicenseStatus.VALID)
            repository.clear()
            repository.activate(expiredToken)
            repository.refresh()
        }
        val state = storedState()
        check(state.status == LicenseStatus.EXPIRED) {
            "gate must re-lock to EXPIRED after refresh, got ${state.status}"
        }
    }
}
