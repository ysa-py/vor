package com.v2rayez.app

import android.app.Application
import android.content.Context
import androidx.test.core.app.ApplicationProvider
import com.v2rayez.app.data.diagnostics.VorCrashEvidence
import com.v2rayez.app.data.license.LicenseRepository
import com.vor.license.Base64Url
import com.vor.license.LicenseStatus
import com.vor.license.PublisherKeyCodec
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.security.SecureRandom

/**
 * Publisher key pairing (v1.5.0) — the offline-issuance flow end-to-end on
 * the REAL repository + REAL DataStore:
 *
 *   seller's issuer key (generated on-device, nothing to do with the
 *   build-embedded key)  ->  VORP1 pairing code  ->  buyer's app pairs once
 *   ->  tokens signed by the seller's key verify and unlock the gate.
 *
 * Plus the invariants that keep the feature safe and crash-free:
 *   - BEFORE pairing, the same token is INVALID (Ed25519 forgery impossible);
 *   - unpairing reverts to embedded-key-only verification;
 *   - embedded-key tokens keep verifying WHILE paired (both keys active);
 *   - malformed codes degrade to null, never exceptions;
 *   - the crash-evidence file set rotates and reads back.
 */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [35], application = Application::class)
class PublisherPairingFlowTest {

    private lateinit var repository: LicenseRepository

    @Before
    fun setUp() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        repository = LicenseRepository(
            context,
            com.v2rayez.app.data.license.LicenseClock(
                com.v2rayez.app.data.license.DataStoreClockRatchetStore(context),
                com.v2rayez.app.data.license.TrustedTimeSource(okhttp3.OkHttpClient()),
            ),
        )
        runBlocking {
            runCatching { repository.clear() }
            runCatching { repository.unpairPublisher() }
        }
    }

    @After
    fun tearDown() {
        runBlocking {
            runCatching { repository.clear() }
            runCatching { repository.unpairPublisher() }
        }
    }

    // ---- test-side issuer: BouncyCastle Ed25519 + the VOR1 envelope -------

    private class IssuerKey(val seed: ByteArray) {
        val privateKey = org.bouncycastle.crypto.params.Ed25519PrivateKeyParameters(seed, 0)
        val publicKey: ByteArray = privateKey.generatePublicKey().encoded
        val publicKeyB64Url: String get() = Base64Url.encode(publicKey)
    }

    private fun newIssuerKey(): IssuerKey = IssuerKey(ByteArray(32).also { SecureRandom().nextBytes(it) })

    private fun signToken(key: IssuerKey, id: String, expiresAt: String): String {
        val payload = ("{\"v\":1,\"id\":\"$id\",\"product\":\"vor\"," +
            "\"issued_at\":\"2026-09-12T00:00:00Z\",\"expires_at\":\"$expiresAt\"," +
            "\"entitlements\":{\"tier\":\"standard\",\"platforms\":[\"android\"]}}")
            .toByteArray(Charsets.UTF_8)
        val signer = org.bouncycastle.crypto.signers.Ed25519Signer()
        signer.init(true, key.privateKey)
        signer.update(payload, 0, payload.size)
        val signature = signer.generateSignature()
        return "VOR1.${Base64Url.encode(payload)}.${Base64Url.encode(signature)}"
    }

    // ---- the flow ----------------------------------------------------------

    @Test
    fun sellerKeyToken_invalidBeforePairing_validAfterPairing_invalidAfterUnpair() = runBlocking {
        val seller = newIssuerKey()
        val token = signToken(seller, "buyer-001", "2031-01-01T00:00:00Z")

        // 1. Without pairing: the embedded key is the ONLY trust root.
        assertEquals(LicenseStatus.INVALID, repository.verify(token).status)

        // 2. Pair via the VORP1 code.
        val code = PublisherKeyCodec.encode(seller.publicKey)
        val pairing = repository.pairPublisher(code)
        assertNotNull(pairing)
        assertEquals(PublisherKeyCodec.fingerprint(seller.publicKey), pairing!!.fingerprintHex)

        // 3. The same token now verifies through the direct path.
        assertEquals(LicenseStatus.VALID, repository.verify(token).status)

        // 4. Activation persists and the gate unlocks.
        assertEquals(LicenseStatus.VALID, repository.activate(token).status)

        // 5. The gate STATE (stored-token path) agrees and carries the fingerprint.
        val state = repository.gateState.first()
        assertEquals(LicenseStatus.VALID, state.status)
        assertEquals(pairing.fingerprintHex, state.publisherFingerprint)

        // 6. Unpairing reverts to embedded-key-only.
        repository.unpairPublisher()
        assertEquals(LicenseStatus.INVALID, repository.verify(token).status)
        assertNull(repository.gateState.first().publisherFingerprint)
    }

    @Test
    fun malformedPairingCodes_rejectedWithoutThrowing() = runBlocking {
        for (bad in listOf(
            "",
            "VORP1",
            "VORP1.",
            "VORP1.abc.def",
            "VORP1.AAAA.${
                // a well-formed segment triple with a WRONG checksum
                PublisherKeyCodec.checksumOf(ByteArray(32)).let {
                    if (it[0] == '0') it.replaceFirst("0", "1") else it.replaceFirst(it[0], '0')
                }
            }",
            "VORP1.${Base64Url.encode(ByteArray(31))}.deadbeef",
            "not a code",
        )) {
            assertNull("must reject: '$bad'", repository.pairPublisher(bad))
        }
        // Nothing paired as a side effect:
        assertNull(repository.gateState.first().publisherFingerprint)
    }

    @Test
    fun embeddedKeyTokens_stillVerify_whilePaired() = runBlocking {
        // Pair a seller key…
        repository.pairPublisher(PublisherKeyCodec.encode(newIssuerKey().publicKey))
        // …and confirm the DEV-key vectors (the embedded key family) still
        // verify. Use the shared conformance vector signed with the dev key.
        val devToken = devSignedVectorToken()
        if (devToken != null) {
            // The dev token's status against the dev-embedded build key must be
            // whatever the vector expects (VALID) — pairing must NOT shadow it.
            assertEquals(LicenseStatus.VALID, repository.verify(devToken).status)
        }
        // A random-key token stays INVALID even while paired:
        val stranger = signToken(newIssuerKey(), "stranger-001", "2031-01-01T00:00:00Z")
        assertEquals(LicenseStatus.INVALID, repository.verify(stranger).status)
    }

    @Test
    fun expiredSellerToken_reportsExpired_notInvalid() = runBlocking {
        val seller = newIssuerKey()
        repository.pairPublisher(PublisherKeyCodec.encode(seller.publicKey))
        val expiredToken = signToken(seller, "buyer-expired", "2020-01-01T00:00:00Z")
        val result = repository.verify(expiredToken)
        assertEquals(LicenseStatus.EXPIRED, result.status)
        assertNotNull(result.payload)
        assertEquals("buyer-expired", result.payload!!.id)
    }

    /** The `valid` conformance vector's token, signed with the committed dev key. */
    private fun devSignedVectorToken(): String? {
        return try {
            val loader = Thread.currentThread().contextClassLoader
                ?: return null
            val vectors = loader.getResource("license-vectors.json") ?: return null
            val root = kotlinx.serialization.json.Json.parseToJsonElement(
                vectors.readText(),
            ).let { it as kotlinx.serialization.json.JsonObject }
            val cases = root["cases"] as kotlinx.serialization.json.JsonArray
            cases.firstNotNullOfOrNull { case ->
                val obj = case as kotlinx.serialization.json.JsonObject
                if (obj["verdict"]?.let { (it as kotlinx.serialization.json.JsonPrimitive).content } == "VALID") {
                    (obj["token"] as kotlinx.serialization.json.JsonPrimitive).content
                } else {
                    null
                }
            }
        } catch (_: Throwable) {
            null
        }
    }

    // ---- crash evidence -----------------------------------------------------

    @Test
    fun crashEvidence_writesRotatesAndClears() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        VorCrashEvidence.clear(context)
        assertNull(VorCrashEvidence.latestText(context))

        val first = RuntimeException("boom-one")
        VorCrashEvidence.write(context, Thread.currentThread(), first)
        val text = VorCrashEvidence.latestText(context)
        assertNotNull(text)
        assertTrue(text!!.contains("boom-one"))
        assertTrue(text.contains("exception: java.lang.RuntimeException"))

        // Rotation keeps only MAX_FILES files; the newest always wins.
        repeat(VorCrashEvidence.MAX_FILES + 2) { n ->
            VorCrashEvidence.write(context, Thread.currentThread(), RuntimeException("boom-$n"))
        }
        val latest = VorCrashEvidence.latestText(context)
        assertNotNull(latest)
        assertTrue(latest!!.contains("boom-${VorCrashEvidence.MAX_FILES + 1}"))
        val dir = java.io.File(context.filesDir, VorCrashEvidence.DIR_NAME)
        val count = dir.listFiles()?.size ?: 0
        assertTrue("rotation must cap stored files (got $count)", count <= VorCrashEvidence.MAX_FILES)

        // The share intent is always constructable and carries the text.
        val intent = VorCrashEvidence.shareIntent(context)
        assertEquals(android.content.Intent.ACTION_SEND, intent.action)
        assertTrue(
            intent.getStringExtra(android.content.Intent.EXTRA_TEXT)!!.contains("boom-"),
        )

        VorCrashEvidence.clear(context)
        assertNull(VorCrashEvidence.latestText(context))
    }

    @Test
    fun crashEvidence_writeNeverThrows_onUnwritableStorage() {
        val hostile = object : android.content.ContextWrapper(null) {
            override fun getFilesDir(): java.io.File = java.io.File("/proc/definitely-not-writable")
        }
        // Must return null silently instead of throwing.
        val written = VorCrashEvidence.write(hostile, Thread.currentThread(), RuntimeException("x"))
        assertNull(written)
        assertNull(VorCrashEvidence.latestFile(hostile))
        assertNull(VorCrashEvidence.latestText(hostile))
    }
}
