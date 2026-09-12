package com.vor.license

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.security.SecureRandom

/**
 * Publisher key pairing codec (v1.5.0) — the format that lets a reseller's
 * on-device issuer authorize a stock Vor build with zero network and zero
 * rebuilds. Every defect class below would break a real seller's business,
 * so each is pinned: wrong prefix, wrong checksum, wrong key length,
 * non-canonical base64, whitespace tolerance, round-trips, and the
 * fingerprint stability the buyer compares with the seller out-of-band.
 */
class PublisherKeyCodecTest {

    private fun randomKey(): ByteArray = ByteArray(32).also { SecureRandom().nextBytes(it) }

    @Test
    fun `encode then parse round-trips`() {
        val key = randomKey()
        val code = PublisherKeyCodec.encode(key)
        val parsed = PublisherKeyCodec.parse(code)
        assertNotNull(parsed)
        assertTrue(key.contentEquals(parsed!!.publicKeyBytes))
        assertEquals(Base64Url.encode(key), parsed.publicKeyB64Url)
        assertEquals(code, parsed.canonicalCode)
        assertEquals(PublisherKeyCodec.fingerprint(key), parsed.fingerprintHex)
    }

    @Test
    fun `encodeFromBase64Url matches encode`() {
        val key = randomKey()
        val b64 = Base64Url.encode(key)
        val fromB64 = PublisherKeyCodec.encodeFromBase64Url(b64)
        assertEquals(PublisherKeyCodec.encode(key), fromB64)
    }

    @Test
    fun `parse tolerates surrounding whitespace`() {
        val code = PublisherKeyCodec.encode(randomKey())
        assertNotNull(PublisherKeyCodec.parse("  $code\n"))
    }

    @Test
    fun `parse rejects wrong prefix`() {
        val code = PublisherKeyCodec.encode(randomKey())
        assertNull(PublisherKeyCodec.parse(code.replaceFirst("VORP1", "VORP2")))
    }

    @Test
    fun `parse rejects corrupted checksum`() {
        val key = randomKey()
        val b64 = Base64Url.encode(key)
        val good = PublisherKeyCodec.checksumOf(key)
        val bad = if (good[0] == '0') good.replaceFirst("0", "1") else good.replaceFirst(good[0], '0')
        assertNull(PublisherKeyCodec.parse("VORP1.$b64.$bad"))
    }

    @Test
    fun `parse rejects uppercase and short checksums`() {
        val key = randomKey()
        val b64 = Base64Url.encode(key)
        assertNull(PublisherKeyCodec.parse("VORP1.$b64.${PublisherKeyCodec.checksumOf(key).uppercase()}"))
        assertNull(PublisherKeyCodec.parse("VORP1.$b64.${PublisherKeyCodec.checksumOf(key).take(4)}"))
        assertNull(PublisherKeyCodec.parse("VORP1.$b64"))
    }

    @Test
    fun `parse rejects non-32-byte keys`() {
        val short = Base64Url.encode(ByteArray(31))
        val long = Base64Url.encode(ByteArray(33))
        assertNull(PublisherKeyCodec.parse("VORP1.$short.${PublisherKeyCodec.checksumOf(ByteArray(31))}"))
        assertNull(PublisherKeyCodec.parse("VORP1.$long.${PublisherKeyCodec.checksumOf(ByteArray(33))}"))
    }

    @Test
    fun `parse rejects garbage without throwing`() {
        listOf(
            "",
            "VORP1",
            "VORP1.",
            ".....",
            "VORP1.!.!",
            "VORP1.abc.12345678",
            "random text",
        ).forEach { code ->
            assertNull("must reject without throwing: '$code'", PublisherKeyCodec.parse(code))
        }
    }

    @Test
    fun `parse rejects standard-base64 payloads`() {
        // '+' and '/' are NOT base64url — a standard-base64 paste must not
        // silently decode into a different key. FIXED bytes (not random):
        // their standard base64 provably contains both '+' and '/', so the
        // rejection is deterministic rather than key-dependent.
        val key = ByteArray(32) { if (it < 3) byteArrayOf(0xFB.toByte(), 0xFF.toByte(), 0xBF.toByte())[it] else 0xED.toByte() }
        val std = java.util.Base64.getEncoder().encodeToString(key)
        assertTrue("fixture must contain '+' and '/'", '+' in std && '/' in std)
        val code = "VORP1.$std.${PublisherKeyCodec.checksumOf(key)}"
        assertNull(PublisherKeyCodec.parse(code))
    }

    @Test
    fun `fingerprints differ between keys and match across derivations`() {
        val a = randomKey()
        val b = randomKey()
        assertNotEquals(PublisherKeyCodec.fingerprint(a), PublisherKeyCodec.fingerprint(b))
        assertEquals(PublisherKeyCodec.fingerprint(a), PublisherKeyCodec.fingerprintOfBase64Url(Base64Url.encode(a)))
        assertEquals(8, PublisherKeyCodec.fingerprint(a).length)
        assertTrue(PublisherKeyCodec.fingerprint(a).matches(Regex("^[0-9a-f]{8}$")))
    }

    @Test
    fun `parse rejects payload with extra dots`() {
        val code = PublisherKeyCodec.encode(randomKey())
        assertNull(PublisherKeyCodec.parse("$code.extra"))
    }
}
