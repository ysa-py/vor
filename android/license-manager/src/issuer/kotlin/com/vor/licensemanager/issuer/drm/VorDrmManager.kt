package com.vor.licensemanager.issuer.drm

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

/**
 * Bridge to the native vor-drm MANAGER core (`libvor_drm.so`, `jni-manager`
 * flavor of core/vor-drm). ISSUER FLAVOR ONLY — this file lives under
 * src/issuer/kotlin so the verifyVerifierPurity gate structurally guarantees
 * it can never ship inside the public verifier APK (signing capability must
 * not exist there, not even transitively).
 *
 * When the library is present the issuer gains: native Ed25519 seed
 * generation, single + batch VOR2 issuance (hardware-locked, encrypted,
 * signed — built and verified entirely in native code), the Argon2id
 * passphrase vault for air-gapped key transport, and the sign-and-verify
 * self test. When absent, every call degrades to null and the pure-Kotlin
 * issuer (core-license-issuer) keeps working exactly as before.
 *
 * Seed discipline (mirrors jni_manager.rs): seeds cross the boundary as
 * byte arrays, live in native memory only inside one call, and are zeroized
 * by the native core on every exit path. The Kotlin copy handed to
 * [issue] & co. should be overwritten by the caller as soon as the call
 * returns.
 */
object VorDrmManager {

    /** Typed issue result. */
    data class IssueResult(val ok: Boolean, val token: String?, val error: String?)

    /** Typed batch result. */
    data class BatchResult(val ok: Boolean, val tokens: List<String>, val error: String?)

    @Volatile
    private var loadAttempted = false

    @Volatile
    private var libraryLoaded = false

    /** True when libvor_drm.so (manager flavor) was loaded into this process. */
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

    // ---- native surface ---------------------------------------------------

    private external fun nativeVersion(): String

    private external fun nativeGenerateKey(): ByteArray?

    private external fun nativePublicKeyOf(seed: ByteArray): String?

    private external fun nativeIssueV2(seed: ByteArray, paramsJson: String): String?

    private external fun nativeIssueV2Batch(seed: ByteArray, batchJson: String): String?

    private external fun nativeVaultSeal(seed: ByteArray, passphrase: String): String?

    private external fun nativeVaultOpen(envelope: String, passphrase: String): ByteArray?

    private external fun nativeSelfTest(seed: ByteArray): String?

    // ---- typed wrappers ---------------------------------------------------

    /** Library version JSON, or null when unavailable. */
    fun version(): String? = callNative { nativeVersion() }

    /**
     * Fresh Ed25519 seed (32 random bytes from OS entropy) — wrap it into the
     * Keystore vault IMMEDIATELY after this returns. Null when unavailable.
     */
    fun generateKey(): ByteArray? = callNative { nativeGenerateKey() }

    /** Public key (base64url) of a seed, or null. */
    fun publicKeyOf(seed: ByteArray): String? = callNative { nativePublicKeyOf(seed) }

    /**
     * Issue one VOR2 token. `paramsJson`:
     * `{"id","tier","issued_at","expires_at","bandwidth_mib","platforms":[],"hwid_hex"}`
     * (epochs as numbers or RFC 3339 strings). Null when the library is
     * absent; [IssueResult.error] carries the native validation message.
     */
    fun issue(seed: ByteArray, paramsJson: String): IssueResult? {
        val raw = callNative { nativeIssueV2(seed, paramsJson) } ?: return null
        return runCatching { parseIssue(raw) }.getOrNull()
    }

    /** Issue a batch (`{"specs":[<spec>,…]}`); all-or-nothing validation. */
    fun issueBatch(seed: ByteArray, batchJson: String): BatchResult? {
        val raw = callNative { nativeIssueV2Batch(seed, batchJson) } ?: return null
        return runCatching { parseBatch(raw) }.getOrNull()
    }

    /** Seal a seed into the passphrase-protected portable vault envelope. */
    fun vaultSeal(seed: ByteArray, passphrase: String): String? =
        callNative { nativeVaultSeal(seed, passphrase) }

    /** Open a vault envelope (null on wrong passphrase / corruption). */
    fun vaultOpen(envelope: String, passphrase: String): ByteArray? =
        callNative { nativeVaultOpen(envelope, passphrase) }

    /** Sign-and-verify self test: (verdict, detail). */
    fun selfTest(seed: ByteArray): Pair<Boolean, String>? {
        val raw = callNative { nativeSelfTest(seed) } ?: return null
        return runCatching {
            val root = Json.parseToJsonElement(raw).jsonObject
            (root["signs_and_verifies"]?.jsonPrimitive?.content == "true") to
                (root["detail"]?.jsonPrimitive?.content ?: "")
        }.getOrNull()
    }

    private inline fun <T> callNative(block: () -> T): T? {
        if (!available) return null
        return runCatching(block).getOrNull()
    }

    // ---- pure parse helpers (unit-tested without the .so) ------------------

    internal fun parseIssue(raw: String): IssueResult? {
        val root = runCatching {
            Json.parseToJsonElement(raw).jsonObject
        }.getOrNull() ?: return null
        val ok = root["ok"]?.jsonPrimitive?.content == "true"
        return IssueResult(
            ok = ok,
            token = root["token"]?.jsonPrimitive?.content,
            error = root["error"]?.jsonPrimitive?.content,
        )
    }

    internal fun parseBatch(raw: String): BatchResult? {
        val root = runCatching {
            Json.parseToJsonElement(raw).jsonObject
        }.getOrNull() ?: return null
        val ok = root["ok"]?.jsonPrimitive?.content == "true"
        val tokens = root["tokens"]
            ?.jsonArray
            ?.mapNotNull { it.jsonPrimitive.content }
            ?: emptyList()
        return BatchResult(
            ok = ok,
            tokens = tokens,
            error = root["error"]?.jsonPrimitive?.content,
        )
    }
}
