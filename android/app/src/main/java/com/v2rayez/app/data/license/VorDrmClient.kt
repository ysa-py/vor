package com.v2rayez.app.data.license

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull

/**
 * Bridge to the native vor-drm client core (`libvor_drm.so`, `jni-client`
 * flavor of core/vor-drm). OPTIONAL hardening layer: when the library is
 * present it verifies BOTH token generations (VOR1 JSON + VOR2
 * hardware-locked encrypted envelopes) with the full native enforcement
 * ladder — Ed25519 signature, HKDF+AES-GCM payload, constant-time HWID
 * binding, anti-clock-rollback and the anti-analysis triage — before any
 * Kotlin verification runs. When the library is absent (stock builds) every
 * method degrades to "unavailable" and the pure-Kotlin verifier keeps
 * working exactly as before: ZERO behavior change, zero new failure modes.
 *
 * Contract (mirrors core/vor-drm/src/jni_client.rs):
 *  * every native call is wrapped — the bridge NEVER throws;
 *  * the native side never panics across the boundary either (catch_unwind
 *    there, runCatching here — belt and braces);
 *  * results are JSON strings parsed here into typed data.
 */
object VorDrmClient {

    /** Native verdict (typed view of the JNI JSON). */
    data class NativeVerdict(
        val status: String,          // VALID | EXPIRED | INVALID
        val now: Long,               // effective now the decision used
        val newRatchet: Long,        // ratchet to persist (>= previous)
        val rolledBack: Boolean,     // wall-clock rollback detected
        val tampered: Boolean,       // debugger / instrumentation detected
        val payload: NativePayload?,  // present when the envelope was authentic
    )

    /** Neutral payload shape (both token generations). */
    data class NativePayload(
        val id: String,
        val issuedAt: String,
        val expiresAt: String,
        val expiresAtEpoch: Long,
        val bandwidthLimitMib: Long,
        val hwidLocked: Boolean,
        val generation: Int,
        val tier: String?,
        val platforms: List<String>,
    )

    @Volatile
    private var loadAttempted = false

    @Volatile
    private var libraryLoaded = false

    /** True when libvor_drm.so was loaded into this process. */
    val available: Boolean
        get() {
            if (!loadAttempted) {
                synchronized(this) {
                    if (!loadAttempted) {
                        libraryLoaded = runCatching {
                            System.loadLibrary("vor_drm")
                        }.isSuccess
                        loadAttempted = true
                    }
                }
            }
            return libraryLoaded
        }

    // ---- native surface (present only when available) -------------------

    private external fun nativeVersion(): String

    private external fun nativeHwidHex(
        drmId: String,
        androidId: String,
        fingerprint: String,
    ): String

    private external fun nativeVerify(
        token: String,
        publicKeyB64: String,
        persistedRatchet: Long,
        trusted: Long,
        deviceWall: Long,
        hwidDrmId: String,
        hwidAndroidId: String,
        hwidFingerprint: String,
    ): String

    private external fun nativeTriage(force: Boolean): Boolean

    // ---- typed wrappers ---------------------------------------------------

    /** Library version JSON, or null when unavailable. */
    fun version(): String? = callNative { nativeVersion() }

    /**
     * Hardware fingerprint digest (64 lowercase hex chars) for the given
     * identity components, or null when unavailable.
     */
    fun hwidHex(drmId: String, androidId: String, fingerprint: String): String? =
        callNative { nativeHwidHex(drmId, androidId, fingerprint) }

    /**
     * Full native verification. Null when the library is absent or the call
     * failed — the caller MUST then fall back to the Kotlin verifier.
     */
    fun verify(
        token: String,
        publicKeyB64: String,
        persistedRatchet: Long,
        trusted: Long,
        deviceWall: Long,
        hwid: Triple<String, String, String>?,
    ): NativeVerdict? {
        val raw = callNative {
            nativeVerify(
                token,
                publicKeyB64,
                persistedRatchet,
                trusted,
                deviceWall,
                hwid?.first.orEmpty(),
                hwid?.second.orEmpty(),
                hwid?.third.orEmpty(),
            )
        } ?: return null
        return runCatching { parseVerdict(raw) }.getOrNull()
    }

    /** Anti-analysis triage: true when the environment looks instrumented. */
    fun triage(force: Boolean = false): Boolean =
        available && runCatching { nativeTriage(force) }.getOrDefault(false)

    /** Guarded native invocation: never throws, never runs when absent. */
    private inline fun <T> callNative(block: () -> T): T? {
        if (!available) return null
        return runCatching(block).getOrNull()
    }

    /** Parse the native verdict JSON (pure — unit-tested without the .so).
     * NEVER throws: garbage degrades to null (the bridge's core contract). */
    internal fun parseVerdict(raw: String): NativeVerdict? {
        val root = runCatching {
            Json.parseToJsonElement(raw).jsonObject
        }.getOrNull() ?: return null
        if (root["ok"]?.jsonPrimitive?.booleanOrNull != true) return null
        val payload = root["payload"]?.jsonObject
        return NativeVerdict(
            status = root["status"]?.jsonPrimitive?.content ?: "INVALID",
            now = root["now"]?.jsonPrimitive?.longOrNull ?: 0L,
            newRatchet = root["new_ratchet"]?.jsonPrimitive?.longOrNull ?: 0L,
            rolledBack = root["rolled_back"]?.jsonPrimitive?.booleanOrNull ?: false,
            tampered = root["tampered"]?.jsonPrimitive?.booleanOrNull ?: false,
            payload = payload?.let(::parsePayload),
        )
    }

    private fun parsePayload(obj: JsonObject): NativePayload = NativePayload(
        id = obj.stringOf("id"),
        issuedAt = obj.stringOf("issued_at"),
        expiresAt = obj.stringOf("expires_at"),
        expiresAtEpoch = obj.longOf("expires_at_epoch"),
        bandwidthLimitMib = obj.longOf("bandwidth_limit_mib"),
        hwidLocked = obj.booleanOf("hwid_locked"),
        generation = obj.longOf("generation").toInt(),
        tier = obj["tier"]?.jsonPrimitive?.content,
        platforms = obj["platforms"]?.jsonArray
            ?.mapNotNull { it.jsonPrimitive.content } ?: emptyList(),
    )
}

private fun JsonObject.stringOf(key: String): String = this[key]?.jsonPrimitive?.content.orEmpty()

private fun JsonObject.longOf(key: String): Long =
    this[key]?.jsonPrimitive?.longOrNull ?: 0L

private fun JsonObject.booleanOf(key: String): Boolean =
    this[key]?.jsonPrimitive?.booleanOrNull ?: false
