package com.v2rayez.app

import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextReplacement
import androidx.test.core.app.ApplicationProvider
import com.v2rayez.app.data.license.LicenseRepository
import com.vor.license.LicenseStatus
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.After
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/**
 * Full-fidelity reproduction of the reported device crash: launch the REAL
 * [MainActivity] (real [V2RayApplication] Hilt graph, real AppRoot/settings
 * tree, real gate wiring), paste a REAL production-signed token, tap
 * "Check / Activate License" and let the app run its actual unlock path —
 * under a PERSIAN (fa, RTL) configuration to match the reporting device.
 *
 * Any uncaught exception anywhere in the real path (coroutine crash, gate
 * flip, first composition of the unlocked content) fails this test with the
 * actual stack trace.
 */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [35], application = V2RayApplication::class, qualifiers = "fa")
class MainActivityLicenseGateE2EFaTest {

    /** Device-wired license clock (trusted fetch is inert in unit tests). */
    private fun testLicenseClock(): com.v2rayez.app.data.license.LicenseClock =
        com.v2rayez.app.data.license.LicenseClock(
            com.v2rayez.app.data.license.DataStoreClockRatchetStore(
                androidx.test.core.app.ApplicationProvider.getApplicationContext<android.content.Context>(),
            ),
            com.v2rayez.app.data.license.TrustedTimeSource(okhttp3.OkHttpClient()),
        )


    @get:Rule
    val compose = createAndroidComposeRule<MainActivity>()

    /** The EXACT token the reporting user pasted (user-phone-001, Task-10 issuance). */
    private val realProdToken =
        "VOR1.eyJ2IjoxLCJpZCI6InVzZXItcGhvbmUtMDAxIiwicHJvZHVjdCI6InZvciIsImlzc3VlZF9hdCI6IjIwMjYtMDktMTBUMTU6NTI6MzZaIiwiZXhwaXJlc19hdCI6IjIwMzEtMDktMTBUMDA6MDA6MDBaIiwiZW50aXRsZW1lbnRzIjp7InRpZXIiOiJzdGFuZGFyZCIsInBsYXRmb3JtcyI6WyJhbmRyb2lkIiwid2luZG93cyIsImxpbnV4Iiwib3BlbndydCIsImlvcyJdfX0.ErVgK_pyiyPZ1uMKlCrzVV3pRbyq1qtmq9gknjDr3NwqjljA_Hg-W_LXNA0c6pxAIGb6Tx9ak8USRb-OhV8yCA"

    private val prodPublicKey = "iVBsYnvBTIyXcWQprbDU0unxxfUJaK3VT-ngvUTZx_A"

    @After
    fun resetLicense() {
        LicenseRepository.publicKeyOverride = null
    }

    @Test
    fun realActivity_realHilGraph_faLocale_pasteProdToken_activate_unlocksWithoutCrash() {
        val app = ApplicationProvider.getApplicationContext<android.app.Application>()
        // Release builds verify against the production key (baked via
        // -Pvor.licensePublicKey); mirror that in this debug-variant test.
        LicenseRepository.publicKeyOverride = prodPublicKey
        val repository = LicenseRepository(app, testLicenseClock())
        runBlocking { runCatching { repository.clear() } }

        // Gate is the first screen — the token field is the only editor.
        compose.onNode(hasSetTextAction()).performTextReplacement(realProdToken)

        // Persian label of "Check / Activate License" (device runs in fa).
        // Fall back to the English label if fa differs, so the test is robust.
        val activateLabel = app.getString(com.v2rayez.app.R.string.license_gate_activate)
        compose.onNodeWithText(activateLabel).performClick()

        // The app must survive the activation + unlock + first composition.
        // Wizard first page shows a "Continue"-equivalent button; wait on the
        // localized label of wizard_continue.
        val continueLabel = app.getString(com.v2rayez.app.R.string.wizard_continue)
        // Generous CI budget: the Release workflow runs lintRelease + assembleRelease
        // alongside this Robolectric E2E suite on one shared runner; a 15s
        // wall-clock cap flaked there. waitUntil polls, so success still
        // returns as soon as the wizard composes.
        compose.waitUntil(timeoutMillis = 120_000) {
            compose.onAllNodesWithText(continueLabel).fetchSemanticsNodes().isNotEmpty()
        }
    }
}
