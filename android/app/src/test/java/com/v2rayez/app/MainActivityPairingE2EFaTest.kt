package com.v2rayez.app

import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextReplacement
import androidx.test.core.app.ApplicationProvider
import com.v2rayez.app.data.license.LicenseRepository
import com.vor.license.Base64Url
import com.vor.license.PublisherKeyCodec
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.security.SecureRandom

/**
 * E2E for the v1.5.0 offline-issuance flow through the REAL activity: the
 * buyer's app starts with the seller's key PAIRED (repository-level pairing
 * is unit-covered by [PublisherPairingFlowTest]; the dialog itself is plain
 * Compose with no logic), then the buyer pastes a token signed by that key —
 * a key the build-embedded verifier does NOT know — and activates.
 * The app must unlock without crashing.
 *
 * This is the exact path a real customer takes once the seller's issuer app
 * has provided a VORP1 pairing code + a license token.
 */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [35], application = V2RayApplication::class, qualifiers = "fa")
class MainActivityPairingE2EFaTest {

    @get:Rule
    val compose = createAndroidComposeRule<MainActivity>()

    private fun testLicenseClock(): com.v2rayez.app.data.license.LicenseClock =
        com.v2rayez.app.data.license.LicenseClock(
            com.v2rayez.app.data.license.DataStoreClockRatchetStore(
                androidx.test.core.app.ApplicationProvider.getApplicationContext<android.content.Context>(),
            ),
            com.v2rayez.app.data.license.TrustedTimeSource(okhttp3.OkHttpClient()),
        )

    private val sellerSeed = ByteArray(32).also { SecureRandom().nextBytes(it) }

    /** A token signed by the seller's key (NOT by the embedded build key). */
    private fun sellerToken(): String {
        val privateKey = org.bouncycastle.crypto.params.Ed25519PrivateKeyParameters(sellerSeed, 0)
        val payload = ("{\"v\":1,\"id\":\"pairing-e2e-001\",\"product\":\"vor\"," +
            "\"issued_at\":\"2026-09-12T00:00:00Z\",\"expires_at\":\"2031-01-01T00:00:00Z\"," +
            "\"entitlements\":{\"tier\":\"standard\",\"platforms\":[\"android\"]}}")
            .toByteArray(Charsets.UTF_8)
        val signer = org.bouncycastle.crypto.signers.Ed25519Signer()
        signer.init(true, privateKey)
        signer.update(payload, 0, payload.size)
        return "VOR1.${Base64Url.encode(payload)}.${Base64Url.encode(signer.generateSignature())}"
    }

    private fun pairingCode(): String {
        val privateKey = org.bouncycastle.crypto.params.Ed25519PrivateKeyParameters(sellerSeed, 0)
        return PublisherKeyCodec.encode(privateKey.generatePublicKey().encoded)
    }

    @After
    fun resetLicense() {
        val app = ApplicationProvider.getApplicationContext<android.content.Context>()
        val repository = LicenseRepository(app, testLicenseClock())
        runBlocking {
            runCatching { repository.clear() }
            runCatching { repository.unpairPublisher() }
        }
    }

    @Test
    fun pairedSellerKey_sellerToken_pasteActivate_unlocksWithoutCrash() {
        val app = ApplicationProvider.getApplicationContext<android.app.Application>()
        val repository = LicenseRepository(app, testLicenseClock())
        runBlocking {
            runCatching { repository.clear() }
            runCatching { repository.unpairPublisher() }
            // Pair the seller's key exactly like the gate dialog would.
            checkNotNull(repository.pairPublisher(pairingCode())) { "pairing failed in test setup" }
        }

        // Gate is the first screen; wait for its token field (both DataStores
        // must hydrate first — see MainActivityLicenseGateE2EFaTest).
        compose.waitUntil(timeoutMillis = 120_000) {
            compose.onAllNodes(hasSetTextAction()).fetchSemanticsNodes().isNotEmpty()
        }
        compose.onNode(hasSetTextAction()).performTextReplacement(sellerToken())

        val activateLabel = app.getString(com.v2rayez.app.R.string.license_gate_activate)
        compose.onNodeWithText(activateLabel).performClick()

        // The app must survive activation + unlock + first composition of the
        // wizard (the seller's token is NOT signed by the embedded key — only
        // the paired key can accept it).
        val continueLabel = app.getString(com.v2rayez.app.R.string.wizard_continue)
        compose.waitUntil(timeoutMillis = 120_000) {
            compose.onAllNodesWithText(continueLabel).fetchSemanticsNodes().isNotEmpty()
        }
    }
}
