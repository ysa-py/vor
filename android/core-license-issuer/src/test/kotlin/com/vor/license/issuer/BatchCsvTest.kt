package com.vor.license.issuer

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Batch CSV parsing: field defaults, comments, validation errors with line
 * numbers, filename sanitization for the QR ZIP, and end-to-end parsing to
 * tokens through the canonical issuance engine.
 */
class BatchCsvTest {

    @Test
    fun parse_fullRows() {
        val result = BatchCsv.parse(
            """
            # batch for September
            alice,pro,2027-01-01T00:00:00Z
            bob,standard,2026-12-31T23:59:59Z

            carol
            dave,trial
            """.trimIndent(),
            defaultTier = "standard",
            defaultExpiryRfc3339 = "2027-06-01T00:00:00Z",
        )
        assertEquals(
            listOf(
                BatchCsv.Row("alice", "pro", "2027-01-01T00:00:00Z"),
                BatchCsv.Row("bob", "standard", "2026-12-31T23:59:59Z"),
                BatchCsv.Row("carol", "standard", "2027-06-01T00:00:00Z"),
                BatchCsv.Row("dave", "trial", "2027-06-01T00:00:00Z"),
            ),
            result.rows,
        )
        assertTrue(result.errors.isEmpty())
    }

    @Test
    fun parse_fieldTrimmingAndPersian() {
        val result = BatchCsv.parse(
            "  کاربر-۱  ,  طلا  ,  2027-01-01T00:00:00Z  ",
            defaultTier = "standard", defaultExpiryRfc3339 = "2027-06-01T00:00:00Z",
        )
        assertEquals(listOf(BatchCsv.Row("کاربر-۱", "طلا", "2027-01-01T00:00:00Z")), result.rows)
    }

    @Test
    fun parse_errors_reportLineNumbers() {
        val result = BatchCsv.parse(
            """
            good-id,standard,2027-01-01T00:00:00Z
            ,missing-id
            bad-expiry,standard,not-a-date
            too,many,fields,here
            ${"x".repeat(200)}
            """.trimIndent(),
            defaultTier = "standard", defaultExpiryRfc3339 = "2027-06-01T00:00:00Z",
        )
        assertEquals(listOf(BatchCsv.Row("good-id", "standard", "2027-01-01T00:00:00Z")), result.rows)
        assertEquals(4, result.errors.size)
        assertTrue(result.errors.any { it.startsWith("line 2:") && it.contains("empty license id") })
        assertTrue(result.errors.any { it.startsWith("line 3:") && it.contains("RFC 3339") })
        assertTrue(result.errors.any { it.startsWith("line 4:") && it.contains("too many fields") })
        assertTrue(result.errors.any { it.startsWith("line 5:") && it.contains("invalid license id") })
    }

    @Test
    fun parse_acceptsOffsets_parsesWithSameVerifier() {
        // RFC 3339 with a numeric offset: +03:30 (Iran Standard Time).
        val result = BatchCsv.parse(
            "offset-id,standard,2027-01-01T00:00:00+03:30",
            defaultTier = "standard", defaultExpiryRfc3339 = "2027-06-01T00:00:00Z",
        )
        assertTrue(result.errors.isEmpty())
        // The verifier's own parser accepts the same form (same Rfc3339).
        assertEquals(
            java.time.Instant.parse("2026-12-31T20:30:00Z").epochSecond,
            com.vor.license.Rfc3339.parseEpoch(result.rows.single().expiresAt),
        )
    }

    @Test
    fun parse_rejectsControlCharacters() {
        val result = BatchCsv.parse(
            "bad\u0001id,standard,2027-01-01T00:00:00Z",
            defaultTier = "standard", defaultExpiryRfc3339 = "2027-06-01T00:00:00Z",
        )
        assertEquals(0, result.rows.size)
        assertEquals(1, result.errors.size)
    }

    @Test
    fun parse_emptyAndCommentOnly_isUsableFalse() {
        val result = BatchCsv.parse(
            "\n# only comments\n\n",
            defaultTier = "standard", defaultExpiryRfc3339 = "2027-06-01T00:00:00Z",
        )
        assertTrue(result.rows.isEmpty())
        assertTrue(!result.isUsable)
    }

    @Test
    fun qrFileName_sanitizesAndDeduplicates() {
        assertEquals("qr-alice.png", BatchCsv.qrFileName("alice"))
        // Persian letters map to underscores; the ASCII '-' is preserved.
        assertEquals("qr-_____-___.png", BatchCsv.qrFileName("کاربر-۱۲۳"))
        assertEquals("qr-slash___dot.png", BatchCsv.qrFileName("slash/\\:dot"))
        assertEquals("qr-___.png", BatchCsv.qrFileName("***"))
        assertEquals("qr-license.png", BatchCsv.qrFileName(""))
        assertEquals(64 + 3 + 4, BatchCsv.qrFileName("x".repeat(200)).length) // cap: qr- + 64 + .png
    }

    @Test
    fun parseToTokens_endToEnd_goldenSeed() {
        val seed = SoftwareEd25519.seedFromBase64Url(GoldenVectors.DEV_SEED_B64URL)!!
        val parsed = BatchCsv.parse(
            "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0,standard,2027-09-12T00:00:00Z",
            defaultTier = "standard", defaultExpiryRfc3339 = "2027-01-01T00:00:00Z",
        )
        val payload = parsed.rows.single()
        val token = CanonicalJson.issueToken(
            seed,
            CanonicalJson.IssuerPayload(
                id = payload.id, issuedAt = GoldenVectors.FIXED_ISSUED_AT,
                expiresAt = payload.expiresAt, tier = payload.tier,
                platforms = listOf("android", "windows", "linux", "openwrt", "ios"),
            ),
        )
        // Byte-identical to the Python-issued golden token (same inputs).
        assertEquals(GoldenVectors.TOKEN_PARITY.first().expectedToken, token)
    }

    @Test
    fun renderTokensText_onePerLine() {
        assertEquals("a\nb\nc", BatchCsv.renderTokensText(listOf("a", "b", "c")))
        assertEquals("", BatchCsv.renderTokensText(emptyList()))
    }
}
