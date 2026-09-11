package com.vor.license.issuer

import com.vor.license.Base64Url
import org.bouncycastle.crypto.params.Ed25519PrivateKeyParameters
import org.bouncycastle.crypto.params.Ed25519PublicKeyParameters
import org.bouncycastle.crypto.signers.Ed25519Signer
import java.security.SecureRandom

/**
 * Software Ed25519 (RFC 8032) signing on the SAME BouncyCastle lightweight
 * API the verifier already uses for verification (`:core-license`). This is
 * deliberately not a new cryptographic dependency: bcprov is already shipped
 * by every Vor client for offline Ed25519 verification, and its signer is
 * the natural counterpart of its verifier.
 *
 * The private key is a 32-byte seed (base64url on the wire — the exact
 * format of the repository secret `LICENSE_SIGNING_KEY`, so a maintainer
 * can migrate an existing production key onto the device by importing it).
 *
 * Conformance is pinned against:
 *  - RFC 8032 test vectors 1-2 (independently confirmed against OpenSSL);
 *  - the repository's Python reference implementation (golden tokens).
 *
 * The seed NEVER lives in this class's callers' static state; callers are
 * expected to keep it inside the Android-Keystore-wrapped vault and only
 * hold plaintext in memory for the duration of one signing operation.
 */
object SoftwareEd25519 {

    const val SEED_LENGTH = 32
    const val PUBLIC_KEY_LENGTH = 32
    const val SIGNATURE_LENGTH = 64

    /** Fresh random seed (the "generate a new key on this device" path). */
    fun generateSeed(random: SecureRandom = SecureRandom()): ByteArray =
        Ed25519PrivateKeyParameters(random).getEncoded()

    /** Derive the 32-byte public key from a 32-byte seed. */
    fun publicKeyOf(seed: ByteArray): ByteArray {
        requireSeed(seed)
        return Ed25519PrivateKeyParameters(seed, 0).generatePublicKey().getEncoded()
    }

    /** Sign a message with a 32-byte seed; returns the 64-byte signature. */
    fun sign(seed: ByteArray, message: ByteArray): ByteArray {
        requireSeed(seed)
        val signer = Ed25519Signer()
        signer.init(true, Ed25519PrivateKeyParameters(seed, 0))
        signer.update(message, 0, message.size)
        return signer.generateSignature()
    }

    /** Verify a signature (used by the issuer's built-in self-test only). */
    fun verify(publicKey: ByteArray, message: ByteArray, signature: ByteArray): Boolean {
        require(signature.size == SIGNATURE_LENGTH) { "signature must be 64 bytes" }
        val verifier = Ed25519Signer()
        verifier.init(false, Ed25519PublicKeyParameters(publicKey, 0))
        verifier.update(message, 0, message.size)
        return verifier.verifySignature(signature)
    }

    // ---- wire helpers -----------------------------------------------------

    /** Parse a base64url 32-byte seed (LICENSE_SIGNING_KEY secret format). */
    fun seedFromBase64Url(text: String): ByteArray? {
        val trimmed = text.trim()
        if (trimmed.isEmpty()) return null
        val seed = Base64Url.decode(trimmed) ?: return null
        return if (seed.size == SEED_LENGTH) seed else null
    }

    /** base64url of a public key (the format embedded in client builds). */
    fun publicToBase64Url(publicKey: ByteArray): String {
        require(publicKey.size == PUBLIC_KEY_LENGTH) { "public key must be 32 bytes" }
        return Base64Url.encode(publicKey)
    }

    private fun requireSeed(seed: ByteArray) {
        require(seed.size == SEED_LENGTH) { "seed must be 32 bytes, got ${seed.size}" }
    }
}
