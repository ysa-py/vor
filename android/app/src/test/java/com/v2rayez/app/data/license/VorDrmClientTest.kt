package com.v2rayez.app.data.license

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * VorDrmClient bridge tests (pure JVM — the native library is absent by
 * definition in unit tests, which is exactly the degraded mode stock
 * builds run in; the JSON parsing is tested against the exact shapes
 * core/vor-drm/src/jni_client.rs emits).
 */
class VorDrmClientTest {

    @Test
    fun `library is unavailable in the jvm test environment`() {
        // No libvor_drm.so on a plain JVM: the bridge must degrade silently.
        assertFalse(VorDrmClient.available)
    }

    @Test
    fun `verify and triage degrade to null-false when unavailable`() {
        assertNull(VorDrmClient.verify("VOR1.x.y", "", 0, 0, 0, null))
        assertFalse(VorDrmClient.triage())
        assertNull(VorDrmClient.version())
        assertNull(VorDrmClient.hwidHex("a", "b", "c"))
    }

    @Test
    fun `parses a full VALID verdict with payload`() {
        val raw = """
            {"ok":true,"status":"VALID","now":1500000,"new_ratchet":1500001,
             "rolled_back":false,"tampered":false,
             "payload":{"id":"vip-001","issued_at":"2026-01-01T00:00:00Z",
             "expires_at":"2027-01-01T00:00:00Z","expires_at_epoch":1798761600,
             "bandwidth_limit_mib":102400,"hwid_locked":true,"generation":2,
             "tier":"vip","platforms":["android","linux"]}}
        """.trimIndent()
        val verdict = VorDrmClient.parseVerdict(raw)!!
        assertEquals("VALID", verdict.status)
        assertEquals(1_500_000L, verdict.now)
        assertEquals(1_500_001L, verdict.newRatchet)
        assertFalse(verdict.rolledBack)
        assertFalse(verdict.tampered)
        val payload = verdict.payload!!
        assertEquals("vip-001", payload.id)
        assertEquals(1_798_761_600L, payload.expiresAtEpoch)
        assertEquals(102_400L, payload.bandwidthLimitMib)
        assertTrue(payload.hwidLocked)
        assertEquals(2, payload.generation)
        assertEquals("vip", payload.tier)
        assertEquals(listOf("android", "linux"), payload.platforms)
    }

    @Test
    fun `parses a bare INVALID verdict without payload`() {
        val raw = """{"ok":true,"status":"INVALID","now":42,"new_ratchet":43,"rolled_back":false,"tampered":true}"""
        val verdict = VorDrmClient.parseVerdict(raw)!!
        assertEquals("INVALID", verdict.status)
        assertEquals(42L, verdict.now)
        assertNull(verdict.payload)
        assertTrue(verdict.tampered)
    }

    @Test
    fun `error shape parses to null`() {
        assertNull(VorDrmClient.parseVerdict("""{"ok":false,"error":"panic"}"""))
    }

    @Test
    fun `garbage input parses to null instead of throwing`() {
        assertNull(VorDrmClient.parseVerdict(""))
        assertNull(VorDrmClient.parseVerdict("not json"))
        assertNull(VorDrmClient.parseVerdict("""{"ok":}"""))
    }

    @Test
    fun `missing optional fields default safely`() {
        val raw = """{"ok":true,"status":"EXPIRED"}"""
        val verdict = VorDrmClient.parseVerdict(raw)!!
        assertEquals("EXPIRED", verdict.status)
        assertEquals(0L, verdict.now)
        assertEquals(0L, verdict.newRatchet)
        assertFalse(verdict.rolledBack)
        assertFalse(verdict.tampered)
        assertNull(verdict.payload)
    }

    @Test
    fun `hwid-locked payload without tier still maps`() {
        val raw = """
            {"ok":true,"status":"VALID","now":1,"new_ratchet":1,
             "rolled_back":false,"tampered":false,
             "payload":{"id":"x","issued_at":"2026-01-01T00:00:00Z",
             "expires_at":"2027-01-01T00:00:00Z","expires_at_epoch":1798761600,
             "bandwidth_limit_mib":0,"hwid_locked":true,"generation":2}}
        """.trimIndent()
        val verdict = VorDrmClient.parseVerdict(raw)!!
        val payload = verdict.payload!!
        assertNull(payload.tier)
        assertEquals(emptyList<String>(), payload.platforms)
        assertEquals(0L, payload.bandwidthLimitMib)
    }
}
