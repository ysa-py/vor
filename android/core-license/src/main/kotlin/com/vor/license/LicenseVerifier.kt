package com.vor.license

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

/**
 * Ed25519 offline license verification (Kotlin port of the Rust reference in
 * `core/vor-core/src/license.rs` and the Python reference in
 * `license/python/vor_license.py`).
 *
 * The algorithm is pinned by the shared conformance vectors in
 * `license/vectors.json` — see `LicenseConformanceTest`.
 *
 * No network access, ever. Only the public key is embedded in clients.
 */
object LicenseVerifier {

    private const val TOKEN_PREFIX = "VOR1"
    private const val PRODUCT = "vor"
    private const val PAYLOAD_VERSION = 1

    private val lenientJson = Json { ignoreUnknownKeys = true }

    /**
     * Verify a token against a base64url Ed25519 public key at the given
     * unix epoch (seconds).
     */
    fun verify(publicKeyBase64: String, token: String, nowEpochSeconds: Long): LicenseResult {
        val invalid = LicenseResult(LicenseStatus.INVALID, null)

        val keyBytes = Base64Url.decode(publicKeyBase64.trim()) ?: return invalid
        if (keyBytes.size != 32) return invalid

        val parts = token.trim().split(".")
        if (parts.size != 3 || parts[0] != TOKEN_PREFIX) return invalid
        val payloadBytes = Base64Url.decode(parts[1]) ?: return invalid
        val signatureBytes = Base64Url.decode(parts[2]) ?: return invalid
        if (signatureBytes.size != 64) return invalid

        // Ed25519 verify (BouncyCastle lightweight API — no provider setup
        // required, works on Android and plain JVM identically).
        val publicKey = try {
            Ed25519PublicKeyParametersWrapper(keyBytes)
        } catch (_: IllegalArgumentException) {
            return invalid
        }
        if (!publicKey.verify(payloadBytes, signatureBytes)) return invalid

        // Payload semantics
        val root = try {
            lenientJson.parseToJsonElement(String(payloadBytes, Charsets.UTF_8)).jsonObject
        } catch (_: Exception) {
            return invalid
        }
        if (root.intOrNull("v") != PAYLOAD_VERSION) return invalid
        if (root.stringOrNull("product") != PRODUCT) return invalid
        val id = root.stringOrNull("id")
        val issuedAt = root.stringOrNull("issued_at")
        val expiresAt = root.stringOrNull("expires_at")
        if (id.isNullOrEmpty() || issuedAt.isNullOrEmpty() || expiresAt.isNullOrEmpty()) {
            return invalid
        }

        val expiryEpoch = Rfc3339.parseEpoch(expiresAt) ?: return invalid
        val entitlements = (root["entitlements"] as? JsonObject)
        val platforms = (entitlements?.get("platforms") as? JsonArray)
            ?.mapNotNull { (it as? kotlinx.serialization.json.JsonPrimitive)?.content?.takeIf(String::isNotEmpty) }
            ?: emptyList()
        val parsed = LicensePayload(
            version = PAYLOAD_VERSION,
            id = id,
            product = PRODUCT,
            issuedAt = issuedAt,
            expiresAt = expiresAt,
            tier = (entitlements?.get("tier") as? kotlinx.serialization.json.JsonPrimitive)?.content,
            platforms = platforms,
        )
        val status = if (nowEpochSeconds >= expiryEpoch) LicenseStatus.EXPIRED else LicenseStatus.VALID
        return LicenseResult(status, parsed)
    }

    private fun JsonObject.stringOrNull(key: String): String? =
        (this[key] as? kotlinx.serialization.json.JsonPrimitive)?.content

    private fun JsonObject.intOrNull(key: String): Int? =
        (this[key] as? kotlinx.serialization.json.JsonPrimitive)?.content?.toIntOrNull()
}

/** Thin wrapper so the BouncyCastle types stay out of the public API. */
private class Ed25519PublicKeyParametersWrapper(keyBytes: ByteArray) {
    private val parameters = org.bouncycastle.crypto.params.Ed25519PublicKeyParameters(keyBytes, 0)

    fun verify(payload: ByteArray, signature: ByteArray): Boolean {
        val signer = org.bouncycastle.crypto.signers.Ed25519Signer()
        signer.init(false, parameters)
        signer.update(payload, 0, payload.size)
        return signer.verifySignature(signature)
    }
}
