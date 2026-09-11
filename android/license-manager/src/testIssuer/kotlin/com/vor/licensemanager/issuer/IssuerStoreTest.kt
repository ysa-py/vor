package com.vor.licensemanager.issuer

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pure-JVM tests for the issuer's local store codecs and time formatting.
 * These live in src/testIssuer (flavor-scoped unit tests): they only exist
 * for the issuer variant, where the issuer classes are on the classpath.
 */
class IssuerStoreTest {

    @Test
    fun tiers_codec_roundTrip_preservingOrder() {
        val tiers = listOf("standard", "طلا", "trial-2027", "b")
        assertEquals(tiers, IssuerStore.decodeTiers(IssuerStore.encodeTiers(tiers)))
    }

    @Test
    fun tiers_corruptInput_fallsBackToDefault() {
        assertEquals(IssuerStore.DEFAULT_TIERS, IssuerStore.decodeTiers("not json"))
        assertEquals(IssuerStore.DEFAULT_TIERS, IssuerStore.decodeTiers(null))
        assertEquals(listOf("standard"), IssuerStore.DEFAULT_TIERS)
    }

    @Test
    fun history_codec_roundTrip_allFields() {
        val entries = listOf(
            IssuerStore.IssuedEntry(
                id = "کاربر-تهران-۰۰۱", tier = "standard",
                expiresAt = "2027-01-01T00:00:00Z", issuedAt = "2026-09-12T00:00:00Z",
                token = "VOR1.abc.def", notes = "paid cash", createdAtMs = 1L,
            ),
            IssuerStore.IssuedEntry(
                id = "u2", tier = "pro",
                expiresAt = "2028-01-01T00:00:00Z", issuedAt = "2026-09-12T01:00:00Z",
                token = "VOR1.xxx.yyy", createdAtMs = 2L,
            ),
        )
        assertEquals(entries, IssuerStore.decodeHistory(IssuerStore.encodeHistory(entries)))
    }

    @Test
    fun history_corruptInput_isEmpty_neverThrows() {
        assertTrue(IssuerStore.decodeHistory("garbage").isEmpty())
        assertTrue(IssuerStore.decodeHistory(null).isEmpty())
        assertTrue(IssuerStore.decodeHistory("[{}]").isEmpty()) // missing required fields
    }

    @Test
    fun meta_codec_roundTrip() {
        val meta = IssuerStore.KeyMeta(
            pubB64Url = "f54nNpWuth1MHZsbi6sdEODSDvWp7V6XSSDqWtmCyMA",
            createdAt = "2026-09-12T00:00:00Z",
            source = "generated",
        )
        assertEquals(meta, IssuerStore.decodeMeta(IssuerStore.encodeMeta(meta)))
        assertNull(IssuerStore.decodeMeta("garbage"))
        assertNull(IssuerStore.decodeMeta(null))
    }

    @Test
    fun timeFormatting_matchesReferenceShape() {
        // Exactly the Python reference emitter's format:
        // dt.datetime(2027, 1, 2, 3, 4, 5, utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        assertEquals(
            "2027-01-02T03:04:05Z",
            IssuerTime.epochSecondToRfc3339(java.time.ZonedDateTime
                .parse("2027-01-02T03:04:05Z").toEpochSecond()),
        )
        // Sub-second precision is truncated, never rounded.
        assertEquals(
            "2027-01-02T03:04:05Z",
            IssuerTime.epochMilliToRfc3339(java.time.ZonedDateTime
                .parse("2027-01-02T03:04:05.999Z").toInstant().toEpochMilli()),
        )
        // Epoch zero and the verifier's own parser agree.
        assertEquals("1970-01-01T00:00:00Z", IssuerTime.epochSecondToRfc3339(0))
        assertEquals(0L, com.vor.license.Rfc3339.parseEpoch(IssuerTime.epochSecondToRfc3339(0)))
    }

    @Test
    fun timeValidation_acceptsVerifierForms_rejectsGarbage() {
        assertTrue(IssuerTime.isValidRfc3339("2027-01-01T00:00:00Z"))
        assertTrue(IssuerTime.isValidRfc3339("2027-01-01T00:00:00+03:30"))
        assertTrue(IssuerTime.isValidRfc3339("2027-01-01T00:00:00.123Z"))
        assertEquals(null, IssuerTime.isValidRfc3339("tomorrow").let { if (it) "" else null })
    }

    @Test
    fun parseTimeOfDay_validAndInvalid() {
        assertEquals(java.time.LocalTime.of(23, 59), parseTimeOfDay("23:59"))
        assertEquals(java.time.LocalTime.of(0, 0), parseTimeOfDay("0:00"))
        assertEquals(java.time.LocalTime.of(9, 5), parseTimeOfDay("9:05"))
        assertNull(parseTimeOfDay("24:00"))
        assertNull(parseTimeOfDay("12:60"))
        assertNull(parseTimeOfDay("12"))
        assertNull(parseTimeOfDay("12:3:4"))
        assertNull(parseTimeOfDay("ab:cd"))
    }
}
