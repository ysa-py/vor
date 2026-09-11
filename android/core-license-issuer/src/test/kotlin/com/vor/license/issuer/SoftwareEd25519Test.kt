package com.vor.license.issuer

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Ed25519 signing conformance:
 *  - RFC 8032 test vectors 1-2 (independently confirmed against OpenSSL);
 *  - seed -> public-key derivation vs OpenSSL-backed `cryptography`;
 *  - golden tokens issued with the repository's Python reference tool;
 *  - seed parsing (LICENSE_SIGNING_KEY secret format).
 */
class SoftwareEd25519Test {

    @Test
    fun rfc8032_signAndVerify() {
        GoldenVectors.RFC_8032.forEach { vector ->
            val seed = GoldenVectors.hexToBytes(vector.seedHex)
            val message = GoldenVectors.hexToBytes(vector.msgHex)
            val expectedSignature = GoldenVectors.hexToBytes(vector.sigHex)
            val expectedPublic = GoldenVectors.hexToBytes(vector.pubHex)

            assertArrayEquals("public derivation (${vector.seedHex.take(8)}…)", expectedPublic, SoftwareEd25519.publicKeyOf(seed))
            assertArrayEquals("signature (${vector.seedHex.take(8)}…)", expectedSignature, SoftwareEd25519.sign(seed, message))
            assertTrue("verify (${vector.seedHex.take(8)}…)", SoftwareEd25519.verify(expectedPublic, message, expectedSignature))

            // One flipped signature bit must break verification.
            val corrupted = expectedSignature.copyOf().also { it[0] = (it[0].toInt() xor 1).toByte() }
            assertFalse(SoftwareEd25519.verify(expectedPublic, message, corrupted))
        }
    }

    @Test
    fun seedToPublic_matchesOpenSSL() {
        GoldenVectors.DERIVE_PUBLIC.forEach { vector ->
            val seed = GoldenVectors.hexToBytes(vector.seedHex)
            assertEquals(vector.pubB64Url, SoftwareEd25519.publicToBase64Url(SoftwareEd25519.publicKeyOf(seed)))
        }
    }

    @Test
    fun goldenTokens_signatureMatchesPythonIssuance() {
        val seed = SoftwareEd25519.seedFromBase64Url(GoldenVectors.DEV_SEED_B64URL)!!
        GoldenVectors.TOKEN_PARITY.forEach { case ->
            val token = CanonicalJson.issueToken(seed, case.toPayload())
            // Splitting the python token and re-verifying its signature with
            // the Kotlin signer: sign the SAME canonical bytes, compare sig.
            val parts = case.expectedToken.split(".")
            val canonical = com.vor.license.Base64Url.decode(parts[1])!!
            val expectedSig = com.vor.license.Base64Url.decode(parts[2])!!
            assertArrayEquals("signature parity (${case.name})", expectedSig, SoftwareEd25519.sign(seed, canonical))
            assertEquals(case.expectedToken, token)
        }
    }

    @Test
    fun seedParsing_secretFormat() {
        // The GitHub secret format: base64url, 32 bytes, optional padding.
        val seed = SoftwareEd25519.seedFromBase64Url(GoldenVectors.DEV_SEED_B64URL)
        assertNotNull(seed)
        assertEquals(32, seed!!.size)
        assertNull(SoftwareEd25519.seedFromBase64Url("not-a-key"))
        assertNull(SoftwareEd25519.seedFromBase64Url("AAAA")) // too short
        assertNull(SoftwareEd25519.seedFromBase64Url(""))
        assertNull(SoftwareEd25519.seedFromBase64Url("   "))
        // Padded base64url variant still accepted (padding is stripped).
        assertEquals(
            java.util.Arrays.toString(seed),
            java.util.Arrays.toString(SoftwareEd25519.seedFromBase64Url(GoldenVectors.DEV_SEED_B64URL + "=")),
        )
    }

    @Test
    fun generatedSeeds_areUniqueAndWellFormed() {
        val first = SoftwareEd25519.generateSeed()
        val second = SoftwareEd25519.generateSeed()
        assertEquals(32, first.size)
        assertFalse(first.contentEquals(second))
        // A generated seed must round-trip into a valid keypair.
        val pub = SoftwareEd25519.publicKeyOf(first)
        assertEquals(32, pub.size)
        val message = "self-test".toByteArray(Charsets.UTF_8)
        val signature = SoftwareEd25519.sign(first, message)
        assertEquals(64, signature.size)
        assertTrue(SoftwareEd25519.verify(pub, message, signature))
    }

    private fun GoldenVectors.ParityCase.toPayload() = CanonicalJson.IssuerPayload(
        id = id,
        issuedAt = GoldenVectors.FIXED_ISSUED_AT,
        expiresAt = expiresAt,
        tier = tier,
        platforms = platforms,
        extraEntitlements = extraEntitlements,
    )
}
