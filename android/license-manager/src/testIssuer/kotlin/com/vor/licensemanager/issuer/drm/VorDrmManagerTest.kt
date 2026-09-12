package com.vor.licensemanager.issuer.drm

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * VorDrmManager bridge tests (pure JVM, issuer flavor — matches the
 * testIssuer source set; the native library is absent by definition here,
 * which is exactly the degraded mode the pure-Kotlin issuer falls back to).
 */
class VorDrmManagerTest {

    @Test
    fun `library is unavailable in the jvm test environment`() {
        assertFalse(VorDrmManager.available)
        assertNull(VorDrmManager.version())
        assertNull(VorDrmManager.generateKey())
        assertNull(VorDrmManager.issue(ByteArray(32), "{}"))
    }

    @Test
    fun `parses a successful single issue result`() {
        val raw = """{"ok":true,"token":"VOR2.abc.def"}"""
        val parsed = VorDrmManager.parseIssue(raw)!!
        assertTrue(parsed.ok)
        assertEquals("VOR2.abc.def", parsed.token)
        assertNull(parsed.error)
    }

    @Test
    fun `parses a failed single issue result`() {
        val raw = """{"ok":false,"error":"expiry must be after issuance"}"""
        val parsed = VorDrmManager.parseIssue(raw)!!
        assertFalse(parsed.ok)
        assertNull(parsed.token)
        assertEquals("expiry must be after issuance", parsed.error)
    }

    @Test
    fun `parses a successful batch result`() {
        val raw = """{"ok":true,"tokens":["VOR2.a.b","VOR2.c.d","VOR2.e.f"]}"""
        val parsed = VorDrmManager.parseBatch(raw)!!
        assertTrue(parsed.ok)
        assertEquals(listOf("VOR2.a.b", "VOR2.c.d", "VOR2.e.f"), parsed.tokens)
        assertNull(parsed.error)
    }

    @Test
    fun `parses a failed batch result naming the row`() {
        val raw = """{"ok":false,"error":"row 4: tier must be 1-24 chars without control characters"}"""
        val parsed = VorDrmManager.parseBatch(raw)!!
        assertFalse(parsed.ok)
        assertEquals(emptyList<String>(), parsed.tokens)
        assertTrue(parsed.error!!.startsWith("row 4:"))
    }

    @Test
    fun `garbage input parses to null instead of throwing`() {
        assertNull(VorDrmManager.parseIssue(""))
        assertNull(VorDrmManager.parseIssue("not json"))
        assertNull(VorDrmManager.parseBatch("[1,2,3]"))
    }
}
