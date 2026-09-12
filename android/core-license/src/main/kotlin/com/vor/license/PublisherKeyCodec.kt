package com.vor.license

import java.security.MessageDigest

/**
 * Publisher key pairing codec (v1.5.0 offline-issuance flow, see
 * `license/SPEC.md` "Publisher key pairing").
 *
 * Lets a reseller's on-device issuer (License Manager `issuer` variant,
 * which generates its own Ed25519 keypair inside the Android Keystore)
 * authorize a stock Vor build WITHOUT any rebuild or network access:
 * the issuer shows a pairing code, the buyer pastes it once into the
 * license gate, and from then on tokens signed by that key verify
 * exactly like tokens signed by the build-embedded key.
 *
 * Pairing code format:
 *
 * ```
 * VORP1.<base64url(32-byte Ed25519 public key)>.<first 8 hex of SHA-256(key)>
 * ```
 *
 * Security properties:
 *  - the code carries ONLY the public key — importing it can never enable
 *    token forgery (Ed25519 signatures still have to verify);
 *  - the checksum is not a security feature, it catches typos/paste
 *    truncation before the key ever reaches storage;
 *  - the displayed fingerprint (same derivation as the issuer's Keys tab)
 *    lets a buyer confirm out-of-band with the seller that the right key
 *    was transferred;
 *  - one publisher key at a time — pairing again replaces the previous.
 */
object PublisherKeyCodec {

    const val PREFIX = "VORP1"

    /** Ed25519 public keys are exactly 32 bytes. */
    const val KEY_LENGTH = 32

    /** Human-checkable fingerprint: first 8 hex chars of SHA-256(public key). */
    const val FINGERPRINT_HEX_CHARS = 8

    /** Parse + fully validate a pairing code; null on any defect. Never throws. */
    fun parse(code: String): ParsedPairing? {
        val trimmed = code.trim()
        val parts = trimmed.split(".")
        if (parts.size != 3 || parts[0] != PREFIX) return null
        val keyBytes = Base64Url.decode(parts[1]) ?: return null
        if (keyBytes.size != KEY_LENGTH) return null
        val checksum = parts[2]
        if (!checksum.matches(Regex("^[0-9a-f]{8}$"))) return null
        if (checksum != checksumOf(keyBytes)) return null
        return ParsedPairing(
            publicKeyBytes = keyBytes,
            publicKeyB64Url = parts[1],
            checksum = checksum,
            fingerprintHex = fingerprint(keyBytes),
        )
    }

    /** Build the canonical pairing code for a 32-byte public key. */
    fun encode(publicKeyBytes: ByteArray): String {
        require(publicKeyBytes.size == KEY_LENGTH) { "Ed25519 public key must be 32 bytes" }
        val b64 = Base64Url.encode(publicKeyBytes)
        return "$PREFIX.$b64.${checksumOf(publicKeyBytes)}"
    }

    /** Build the pairing code from a base64url public key (as stored by the issuer). */
    fun encodeFromBase64Url(publicKeyB64Url: String): String? {
        val keyBytes = Base64Url.decode(publicKeyB64Url.trim()) ?: return null
        if (keyBytes.size != KEY_LENGTH) return null
        return encode(keyBytes)
    }

    /** 8-hex typo checksum. */
    fun checksumOf(publicKeyBytes: ByteArray): String =
        sha256Hex(publicKeyBytes).take(8)

    /** Display fingerprint — identical derivation on the issuer's Keys tab. */
    fun fingerprint(publicKeyBytes: ByteArray): String =
        sha256Hex(publicKeyBytes).take(FINGERPRINT_HEX_CHARS)

    /** Fingerprint of a base64url key, or null when the key is malformed. */
    fun fingerprintOfBase64Url(publicKeyB64Url: String): String? {
        val keyBytes = Base64Url.decode(publicKeyB64Url.trim()) ?: return null
        if (keyBytes.size != KEY_LENGTH) return null
        return fingerprint(keyBytes)
    }

    private fun sha256Hex(data: ByteArray): String {
        val digest = MessageDigest.getInstance("SHA-256").digest(data)
        return digest.joinToString("") { "%02x".format(it) }
    }

    /** A validated pairing ready to store. */
    data class ParsedPairing(
        val publicKeyBytes: ByteArray,
        val publicKeyB64Url: String,
        val checksum: String,
        val fingerprintHex: String,
    ) {
        /** Canonical code to re-display (round-trips through [encode]). */
        val canonicalCode: String get() = "$PREFIX.$publicKeyB64Url.$checksum"
    }
}
