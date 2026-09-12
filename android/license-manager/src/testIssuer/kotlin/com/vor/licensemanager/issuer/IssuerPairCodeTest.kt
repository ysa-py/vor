package com.vor.licensemanager.issuer

import com.vor.license.Base64Url
import com.vor.license.PublisherKeyCodec
import com.vor.license.issuer.SoftwareEd25519
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.security.SecureRandom

/**
 * The Keys-tab pairing export (v1.5.0): the code shown to the seller must be
 * EXACTLY what the buyer app's gate accepts, for ANY key state the vault can
 * be in (generated on-device, imported, restored from backup). These tests
 * pin the format contract between the two apps without needing the app
 * module on the classpath: encode from the vault's base64url public key ->
 * parse back -> same key bytes, same fingerprint shown on both sides.
 */
class IssuerPairCodeTest {

    @Test
    fun keysTabCode_parsesBackToTheSameKey() {
        val seed = ByteArray(32).also { SecureRandom().nextBytes(it) }
        val pubB64Url = SoftwareEd25519.publicToBase64Url(SoftwareEd25519.publicKeyOf(seed))

        val code = PublisherKeyCodec.encodeFromBase64Url(pubB64Url)
        assertNotNull(code)

        val parsed = PublisherKeyCodec.parse(code!!)
        assertNotNull(parsed)
        assertTrue(pubB64Url.contentEquals(parsed!!.publicKeyB64Url))
        assertEquals(code, parsed.canonicalCode)
    }

    @Test
    fun generatedKeys_alwaysProduceParseableCodes() {
        repeat(16) {
            val seed = ByteArray(32).also { SecureRandom().nextBytes(it) }
            val pub = SoftwareEd25519.publicKeyOf(seed)
            val code = PublisherKeyCodec.encode(pub)
            val parsed = PublisherKeyCodec.parse(code)
            assertNotNull("iteration $it must produce a parseable code", parsed)
            assertTrue(pub.contentEquals(parsed!!.publicKeyBytes))
        }
    }

    @Test
    fun fingerprint_matchesTheVaultsDerivation() {
        // IssuerVault.fingerprint(): first 8 hex of SHA-256(publicKey bytes).
        val seed = ByteArray(32).also { SecureRandom().nextBytes(it) }
        val pub = SoftwareEd25519.publicKeyOf(seed)
        val digest = java.security.MessageDigest.getInstance("SHA-256").digest(pub)
        val expected = digest.take(4).joinToString("") { "%02x".format(it) }
        assertEquals(expected, PublisherKeyCodec.fingerprint(pub))
    }

    @Test
    fun issuedToken_afterBuyerPairing_verifyShapeStaysVOR1() {
        // The token the issuer signs is a plain VOR1 envelope; the buyer app
        // (post-pairing) verifies it with the standard verifier. Pin the wire
        // shape here so the two apps can never drift apart.
        val seed = ByteArray(32).also { SecureRandom().nextBytes(it) }
        val pub = SoftwareEd25519.publicKeyOf(seed)
        val code = PublisherKeyCodec.encode(pub)
        val parsed = PublisherKeyCodec.parse(code)!!

        val payload = ("{\"v\":1,\"id\":\"shape-001\",\"product\":\"vor\"," +
            "\"issued_at\":\"2026-09-12T00:00:00Z\",\"expires_at\":\"2031-01-01T00:00:00Z\"," +
            "\"entitlements\":{\"tier\":\"standard\",\"platforms\":[\"android\"]}}")
            .toByteArray(Charsets.UTF_8)
        val signature = SoftwareEd25519.sign(seed, payload)
        val token = "VOR1.${Base64Url.encode(payload)}.${Base64Url.encode(signature)}"

        val parts = token.split(".")
        assertEquals(3, parts.size)
        assertEquals("VOR1", parts[0])
        // And the buyer-side verifier (shared core-license) accepts it with
        // the paired key — proving both apps agree on the whole contract:
        val verdict = com.vor.license.LicenseVerifier.verify(
            parsed.publicKeyB64Url, token, 1780000000L,
        )
        assertEquals(com.vor.license.LicenseStatus.VALID, verdict.status)
    }
}
