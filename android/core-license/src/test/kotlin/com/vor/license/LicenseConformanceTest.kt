package com.vor.license

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Test
import java.io.File

/**
 * Shared cross-platform license conformance vectors.
 *
 * The exact same `license/vectors.json` is consumed by:
 *   * Rust  — core/vor-core/tests/conformance.rs
 *   * Kotlin — this test (main app + License Manager share the verifier)
 *   * Go — desktop/internal/license/license_test.go
 *   * Python — license/python/test_vectors.py
 * so every platform provably agrees on accept/reject decisions.
 */
class LicenseConformanceTest {

    private fun resource(name: String): String {
        val url = Thread.currentThread().contextClassLoader.getResource(name)
            ?: throw AssertionError("missing test resource: $name")
        return File(url.toURI()).readText()
    }

    private fun vectorsRoot() = Json.parseToJsonElement(resource("license-vectors.json")).jsonObject

    private fun devPublicKey(): String = resource("VOR_LICENSE_PUBLIC_KEY.txt").trim()

    @Test
    fun all_vectors_agree() {
        val root = vectorsRoot()
        val now = Rfc3339.parseEpoch(root["spec"]!!.jsonObject["now"]!!.jsonPrimitive.content)!!
        val publicKey = devPublicKey()
        assertEquals(root["public_key"]!!.jsonPrimitive.content, publicKey)

        val cases = root["cases"]!!.jsonArray
        val failures = mutableListOf<String>()
        cases.forEach { element ->
            val case = element.jsonObject
            val name = case["name"]!!.jsonPrimitive.content
            val token = case["token"]!!.jsonPrimitive.content
            val expected = case["expected"]!!.jsonPrimitive.content
            val result = LicenseVerifier.verify(publicKey, token, now)
            val actual = when (result.status) {
                LicenseStatus.VALID -> "VALID"
                LicenseStatus.EXPIRED -> "EXPIRED"
                LicenseStatus.INVALID -> "INVALID"
            }
            if (actual != expected) {
                failures.add("$name: got $actual, want $expected")
            }
        }
        assertEquals("vector failures:\n${failures.joinToString("\n")}", 0, failures.size)
    }

    @Test
    fun rfc3339_matches_reference_constants() {
        assertEquals(0L, Rfc3339.parseEpoch("1970-01-01T00:00:00Z"))
        assertEquals(1704067200L, Rfc3339.parseEpoch("2024-01-01T00:00:00Z"))
        assertEquals(1798761600L, Rfc3339.parseEpoch("2027-01-01T00:00:00Z"))
        assertEquals(1789041600L, Rfc3339.parseEpoch("2026-09-10T12:00:00Z"))
        assertEquals(1789034400L, Rfc3339.parseEpoch("2026-09-10T12:00:00+02:00"))
        assertEquals(1789041600L, Rfc3339.parseEpoch("2026-09-10T12:00:00.123Z"))
        assertEquals(null, Rfc3339.parseEpoch("garbage"))
        assertEquals(null, Rfc3339.parseEpoch("2026-13-01T00:00:00Z"))
    }

    @Test
    fun base64url_roundtrip() {
        for (length in 0..40) {
            val data = ByteArray(length) { it.toByte() }
            val encoded = Base64Url.encode(data)
            assertEquals(data.toList(), Base64Url.decode(encoded)?.toList())
        }
        assertEquals("Zm9vYmFy", Base64Url.encode("foobar".toByteArray()))
        assertEquals(null, Base64Url.decode("!!invalid"))
    }
}
