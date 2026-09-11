package com.vor.license.issuer

import com.vor.license.LicenseStatus
import com.vor.license.LicenseVerifier
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.Base64

/**
 * Byte-exact parity of the Kotlin canonical serializer + token issuance with
 * the repository's Python reference implementation (license/python/
 * vor_license.py — the tool behind the GitHub Actions issuance workflow).
 *
 * Every case verifies, in order of strength:
 *  1. canonical payload bytes match the Python-emitted bytes;
 *  2. the full token string matches the Python-issued token;
 *  3. the token verifies VALID through the REAL client-side verifier
 *     (:core-license — the same code every shipped Vor app runs).
 */
class CanonicalJsonTest {

    private val devSeed = SoftwareEd25519.seedFromBase64Url(GoldenVectors.DEV_SEED_B64URL)!!

    @Test
    fun parity_withPythonReference_everyCase() {
        GoldenVectors.TOKEN_PARITY.forEach { case ->
            val payload = CanonicalJson.IssuerPayload(
                id = case.id,
                issuedAt = GoldenVectors.FIXED_ISSUED_AT,
                expiresAt = case.expiresAt,
                tier = case.tier,
                platforms = case.platforms,
                extraEntitlements = case.extraEntitlements,
            )
            val canonical = CanonicalJson.canonicalBytes(payload)
            val expected =
                Base64.getDecoder().decode(case.expectedCanonicalBase64)
            assertArrayEquals("canonical bytes differ (${case.name})", expected, canonical)

            val token = CanonicalJson.issueToken(devSeed, payload)
            assertEquals("token differs (${case.name})", case.expectedToken, token)
        }
    }

    @Test
    fun parity_tokens_verify_withTheRealVerifier() {
        val nowEpoch = 1_785_000_000L // 2026-07-25: before some expiries, after others
        GoldenVectors.TOKEN_PARITY.forEach { case ->
            val result = LicenseVerifier.verify(
                GoldenVectors.DEV_PUBLIC_KEY_B64URL, case.expectedToken, nowEpoch,
            )
            // Expected status is DERIVED with the verifier's own RFC 3339
            // parser — status is a pure function of signed expiry vs epoch.
            val expectedStatus = if (
                com.vor.license.Rfc3339.parseEpoch(case.expiresAt)!! <= nowEpoch
            ) LicenseStatus.EXPIRED else LicenseStatus.VALID
            assertEquals("status (${case.name})", expectedStatus, result.status)
            assertEquals("id (${case.name})", case.id, result.payload?.id)
            assertEquals("tier (${case.name})", case.tier, result.payload?.tier)
        }
    }

    @Test
    fun deterministic_repeatedIssuance_identicalToken() {
        val payload = GoldenVectors.TOKEN_PARITY.first()
        val built = CanonicalJson.IssuerPayload(
            id = payload.id, issuedAt = GoldenVectors.FIXED_ISSUED_AT,
            expiresAt = payload.expiresAt, tier = payload.tier,
            platforms = payload.platforms, extraEntitlements = payload.extraEntitlements,
        )
        val first = CanonicalJson.issueToken(devSeed, built)
        val second = CanonicalJson.issueToken(devSeed, built)
        assertEquals(first, second)
    }

    @Test
    fun canonical_form_hasNoWhitespace_andExpectedFieldOrder() {
        val payload = CanonicalJson.IssuerPayload(
            id = "order-check", issuedAt = "2026-09-12T00:00:00Z",
            expiresAt = "2027-09-12T00:00:00Z", tier = "standard",
            platforms = listOf("android"),
        )
        val text = String(CanonicalJson.canonicalBytes(payload), Charsets.UTF_8)
        assertEquals(
            "{\"v\":1,\"id\":\"order-check\",\"product\":\"vor\"," +
                "\"issued_at\":\"2026-09-12T00:00:00Z\"," +
                "\"expires_at\":\"2027-09-12T00:00:00Z\"," +
                "\"entitlements\":{\"tier\":\"standard\",\"platforms\":[\"android\"]}}",
            text,
        )
    }

    @Test
    fun escaping_matchesPythonJsonDumps() {
        // Python: json.dumps('a"b\\c<newline>d<0x01>x', ensure_ascii=False)
        //   emits "a\"b\\c\nd\u0001x" — control chars escaped, lowercase hex.
        // The INPUT below uses Kotlin escapes (\n, \u0001) which become real
        // control characters at compile time; the EXPECTED JSON TEXT contains
        // the six-character sequence \u0001, written in Kotlin as "\\u0001".
        val tierValue = "a\"b\\c\nd\u0001x"      // a " b \ c <NL> d <0x01> x
        val payload = CanonicalJson.IssuerPayload(
            id = "esc", issuedAt = "2026-09-12T00:00:00Z",
            expiresAt = "2027-01-01T00:00:00Z", tier = tierValue,
            platforms = emptyList(),
        )
        val text = String(CanonicalJson.canonicalBytes(payload), Charsets.UTF_8)
        val expectedJsonText = "\"tier\":\"a\\\"b\\\\c\\nd\\u0001x\""
        assertTrue("escaping mismatch in: " + text, text.contains(expectedJsonText))
    }

    @Test
    fun extras_renderAfterTierAndPlatforms_inSortedKeyOrder() {
        val payload = CanonicalJson.IssuerPayload(
            id = "x", issuedAt = "2026-09-12T00:00:00Z",
            expiresAt = "2027-01-01T00:00:00Z", tier = "standard",
            platforms = listOf("android"),
            extraEntitlements = mapOf(
                "zeta" to CanonicalJson.EntitlementValue.Text("last"),
                "alpha" to CanonicalJson.EntitlementValue.Number(42),
                "mid" to CanonicalJson.EntitlementValue.Flag(true),
            ),
        )
        val text = String(CanonicalJson.canonicalBytes(payload), Charsets.UTF_8)
        val expectedEntitlements =
            "\"entitlements\":{\"tier\":\"standard\",\"platforms\":[\"android\"]," +
                "\"alpha\":42,\"mid\":true,\"zeta\":\"last\"}"
        assertTrue(text.endsWith(expectedEntitlements + "}"))
    }
}
