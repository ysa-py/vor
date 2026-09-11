package com.vor.license.issuer

import com.vor.license.Base64Url

/**
 * Canonical payload serialization for token issuance — a byte-exact Kotlin
 * port of `canonical_payload()` in license/python/vor_license.py (the
 * repository's reference issuer, also used by the GitHub Actions workflow).
 *
 * Byte-exactness is what makes an on-device-issued token indistinguishable
 * from a workflow-issued one: BOTH produce
 * `VOR1.<b64url(canonical_json)>.<b64url(ed25519_sig)>` with identical
 * payload bytes for identical inputs. The parity is pinned by golden
 * vectors generated with the Python tool itself (CanonicalJsonTest /
 * SoftwareEd25519Test), including non-ASCII (UTF-8, `ensure_ascii=False`
 * behavior) and escaped-character cases.
 *
 * Canonical form (license/SPEC.md):
 *  - compact separators (`,` and `:`), no insignificant whitespace;
 *  - field order v, id, product, issued_at, expires_at, entitlements;
 *  - inside entitlements: tier, platforms, then any extra keys sorted;
 *  - strings escaped exactly like Python's json.dumps(ensure_ascii=False):
 *    `"` and `\` escaped, control chars as \b \t \n \f \r or \u00xx,
 *    everything else raw UTF-8.
 */
object CanonicalJson {

    /** One issued-license payload in canonical, signature-ready form. */
    data class IssuerPayload(
        val id: String,
        val issuedAt: String,
        val expiresAt: String,
        val tier: String,
        val platforms: List<String>,
        /** Optional extra entitlement keys (sorted after tier/platforms). */
        val extraEntitlements: Map<String, EntitlementValue> = emptyMap(),
    )

    /** Extra entitlement values we can serialize (keeps parity testing honest). */
    sealed interface EntitlementValue {
        data class Text(val value: String) : EntitlementValue
        data class Number(val value: Long) : EntitlementValue
        data class Flag(val value: Boolean) : EntitlementValue
    }

    /**
     * Serialize to the exact canonical bytes the reference tools emit.
     * `product` is fixed to "vor" and `v` to 1 — the only values the
     * verifier accepts, so the issuer cannot produce anything else.
     */
    fun canonicalBytes(payload: IssuerPayload): ByteArray {
        val sb = StringBuilder(256)
        sb.append('{')
        appendUnprefixedField(sb, "v", 1L)
        appendField(sb, "id", payload.id)
        appendField(sb, "product", PRODUCT)
        appendField(sb, "issued_at", payload.issuedAt)
        appendField(sb, "expires_at", payload.expiresAt)
        sb.append(",\"entitlements\":{")
        appendUnprefixedField(sb, "tier", payload.tier)
        sb.append(',')
        appendString(sb, "platforms")
        sb.append(':').append('[')
        payload.platforms.forEachIndexed { index, platform ->
            if (index > 0) sb.append(',')
            appendString(sb, platform)
        }
        sb.append(']')
        payload.extraEntitlements.keys.sorted().forEach { key ->
            sb.append(',')
            appendString(sb, key)
            sb.append(':')
            appendValue(sb, payload.extraEntitlements.getValue(key))
        }
        sb.append("}}")
        return sb.toString().toByteArray(Charsets.UTF_8)
    }

    /**
     * Sign `payload` with `seed` and emit the wire token. Identical inputs
     * always produce an identical token — the deterministic pairing used
     * by the golden-vector parity tests.
     */
    fun issueToken(seed: ByteArray, payload: IssuerPayload): String =
        issueToken(payload) { message -> SoftwareEd25519.sign(seed, message) }

    /**
     * Token assembly around an injected signer (the Android vault passes a
     * Keystore-gated signer; the pure-JVM tests pass the software signer).
     */
    fun issueToken(payload: IssuerPayload, signer: (ByteArray) -> ByteArray): String {
        val payloadBytes = canonicalBytes(payload)
        val signature = signer(payloadBytes)
        return "VOR1.${Base64Url.encode(payloadBytes)}.${Base64Url.encode(signature)}"
    }

    const val PRODUCT = "vor"

    // ---- internals --------------------------------------------------------

    /** First field inside an object: no leading comma. */
    private fun appendUnprefixedField(sb: StringBuilder, name: String, value: Long) {
        appendString(sb, name)
        sb.append(':').append(value)
    }

    /** First field inside an object: no leading comma. */
    private fun appendUnprefixedField(sb: StringBuilder, name: String, value: String) {
        appendString(sb, name)
        sb.append(':')
        appendString(sb, value)
    }

    /** Subsequent field: leading comma separator. */
    private fun appendField(sb: StringBuilder, name: String, value: String) {
        sb.append(',')
        appendUnprefixedField(sb, name, value)
    }

    private fun appendValue(sb: StringBuilder, value: EntitlementValue) {
        when (value) {
            is EntitlementValue.Text -> appendString(sb, value.value)
            is EntitlementValue.Number -> sb.append(value.value)
            is EntitlementValue.Flag -> sb.append(value.value)
        }
    }

    /** Python json.dumps(ensure_ascii=False) string escaping, byte-exact. */
    private fun appendString(sb: StringBuilder, text: String) {
        sb.append('"')
        for (char in text) {
            when {
                char == '"' -> sb.append("\\\"")
                char == '\\' -> sb.append("\\\\")
                char == '\b' -> sb.append("\\b")
                char == '\u000C' -> sb.append("\\f")
                char == '\n' -> sb.append("\\n")
                char == '\r' -> sb.append("\\r")
                char == '\t' -> sb.append("\\t")
                char < ' ' -> {
                    // Python: \u00xx with exactly four lowercase hex digits.
                    sb.append("\\u")
                        .append(HEX[(char.code shr 12) and 0xF])
                        .append(HEX[(char.code shr 8) and 0xF])
                        .append(HEX[(char.code shr 4) and 0xF])
                        .append(HEX[char.code and 0xF])
                }
                else -> sb.append(char)
            }
        }
        sb.append('"')
    }

    private val HEX = "0123456789abcdef".toCharArray()
}
