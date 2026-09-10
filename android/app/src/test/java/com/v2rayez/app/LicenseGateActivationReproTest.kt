package com.v2rayez.app

import android.app.Application
import android.content.Context
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextReplacement
import androidx.test.core.app.ApplicationProvider
import com.v2rayez.app.data.license.LicenseRepository
import com.v2rayez.app.ui.screens.license.LicenseGateScreen
import com.v2rayez.app.ui.screens.license.LicenseGateViewModel
import com.v2rayez.app.ui.screens.onboarding.WelcomeWizardScreen
import com.v2rayez.app.ui.theme.V2RayEzTheme
import com.v2rayez.app.ui.viewmodel.OnboardingViewModel
import com.vor.license.LicenseStatus
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.After
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/**
 * REPRO + REGRESSION for the Priority-0 crash reported on a real device
 * (2026-09-10, screen recording): pasting a well-formed, production-signed
 * license token into the "Activate Vor" gate and tapping "Check / Activate
 * License" crashed the app immediately ("Vor keeps stopping").
 *
 * This test runs the EXACT phone path on Robolectric — real
 * [LicenseRepository] + real DataStore persistence + real
 * [LicenseGateViewModel] + the real [LicenseGateScreen] composable + the
 * first-time post-unlock content ([WelcomeWizardScreen], since a first-time
 * activating user has onboardingComplete=false) — with a REAL token issued
 * by the production `Issue License` workflow (id crash-repro-001, signed
 * with the production Ed25519 key, expiry 2027-09-11).
 *
 * An uncaught exception anywhere in the activation path (coroutine crash,
 * DataStore write, gate transition, or first composition of the unlocked
 * content) fails this test with the actual stack trace — that is the
 * reproduction mechanism, mirroring how the same exception killed the
 * process on the phone.
 */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [35], application = Application::class)
class LicenseGateActivationReproTest {

    @get:Rule
    val compose = createComposeRule()

    /** Production-issued token (workflow_dispatch run, prod signing key). */
    private val realProdToken =
        "VOR1.eyJ2IjoxLCJpZCI6ImNyYXNoLXJlcHJvLTAwMSIsInByb2R1Y3QiOiJ2b3IiLCJpc3N1ZWRfYXQiOiIyMDI2LTA5LTEwVDE5OjEyOjA3WiIsImV4cGlyZXNfYXQiOiIyMDI3LTA5LTExVDAwOjAwOjAwWiIsImVudGl0bGVtZW50cyI6eyJ0aWVyIjoic3RhbmRhcmQiLCJwbGF0Zm9ybXMiOlsiYW5kcm9pZCIsIndpbmRvd3MiLCJsaW51eCIsIm9wZW53cnQiLCJpb3MiXX19.MHykE-LDnuxVya8BnE7t4ytN1pqHO2K7rst1DiwGUhfX2h2Rp4c9xYgQNBpxvcior8jvLhgLg6nRVzIyIF7oDw"

    /** Production public key (committed at license/keys/prod/). */
    private val prodPublicKey = "iVBsYnvBTIyXcWQprbDU0unxxfUJaK3VT-ngvUTZx_A"

    private lateinit var repository: LicenseRepository

    @After
    fun cleanStoredLicense() {
        if (::repository.isInitialized) {
            // Reset the shared DataStore so tests (and repeat runs) start locked.
            runBlocking { runCatching { repository.clear() } }
        }
        LicenseRepository.publicKeyOverride = null
    }

    @Test
    fun pasteRealProductionToken_tapActivate_unlocksWithoutCrashing() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        // Mirror a release build: verify against the production public key.
        LicenseRepository.publicKeyOverride = prodPublicKey
        repository = LicenseRepository(context)

        // Precondition: the exact class of token the workflow issues verifies
        // as VALID through the repository (proves the token + key wiring).
        val pre = repository.verify(realProdToken)
        check(pre.status == LicenseStatus.VALID) {
            "precondition failed: prod token should verify VALID, got ${pre.status}"
        }

        val viewModel = LicenseGateViewModel(repository)

        // Mirror MainActivity's LicenseGateCoordinator + first-time content:
        // gate screen until VALID, then WelcomeWizardScreen (onboardingComplete=false).
        compose.setContent {
            V2RayEzTheme(darkTheme = false, accent = "Purple") {
                val state by viewModel.gateState.collectAsState()
                when {
                    !state.hydrated -> Unit
                    state.status != LicenseStatus.VALID ->
                        LicenseGateScreen(onUnlocked = {}, viewModel = viewModel)
                    else -> WelcomeWizardScreen(viewModel = OnboardingViewModel())
                }
            }
        }

        // 1. Paste the real token into the gate's text field.
        compose.onNode(hasSetTextAction()).performTextReplacement(realProdToken)

        // 2. Tap "Check / Activate License".
        compose.onNodeWithText("Check / Activate License").performClick()

        // 3. The gate must flip to the first-time content (the wizard's
        //    "Continue" button appears). Any crash on the activation path —
        //    coroutine exception, DataStore failure, or first-composition
        //    error — propagates here and fails the test with the real stack
        //    trace, exactly as it killed the process on the phone.
        compose.waitUntil(timeoutMillis = 10_000) {
            compose.onAllNodesWithText("Continue").fetchSemanticsNodes().isNotEmpty()
        }

        // 4. Post-conditions: the token is stored and verifies VALID.
        val stored = runBlocking {
            withTimeout(5_000) { viewModel.gateState.first { it.hydrated } }
        }
        check(stored.status == LicenseStatus.VALID) {
            "gate did not unlock: status=${stored.status}"
        }
        check(stored.payload?.id == "crash-repro-001") {
            "stored payload id mismatch: ${stored.payload?.id}"
        }
    }
}
