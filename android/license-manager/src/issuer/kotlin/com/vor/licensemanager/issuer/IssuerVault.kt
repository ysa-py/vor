package com.vor.licensemanager.issuer

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import android.security.keystore.UserNotAuthenticatedException
import com.vor.license.Base64Url
import com.vor.license.issuer.SoftwareEd25519
import java.io.File
import java.security.KeyStore
import java.security.MessageDigest
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * The issuer's signing-key vault: a 32-byte Ed25519 seed that is ALWAYS
 * stored wrapped by an Android-Keystore AES-256-GCM key which has
 * `setUserAuthenticationRequired(true)` — it can only be used within 60
 * seconds of a successful lock-screen authentication (the BiometricPrompt /
 * device-PIN gate the issuer app shows on every open provides exactly that).
 *
 * Why the seed is software-Ed25519 and the WRAPPER is hardware-backed:
 * Android Keystore does not support Ed25519 key generation, so the seed
 * itself must live in software — but it is never written anywhere in the
 * clear. On disk there is only an AES-GCM blob that the secure-lock-screen-
 * bound Keystore key must be asked to decrypt. There is no way to make the
 * plaintext seed leave this class other than as a return value for an
 * in-flight signing or backup operation.
 *
 * Honest limits (also shown in the app's About text): a rooted, compromised
 * OS can observe the seed while it is in use; this design raises the bar
 * (screen-locked, hardware-backed at-rest protection, per-open
 * re-authentication), it is not a guarantee against a fully compromised
 * device. The seed can be migrated from the legacy LICENSE_SIGNING_KEY
 * GitHub secret by importing it (the wire format is identical).
 */
class IssuerVault(private val context: Context) {

    /** Typed failures the UI reacts to (re-auth, restore-from-backup…). */
    sealed class VaultException(message: String, cause: Throwable? = null) : Exception(message, cause) {
        /** The 60 s keyguard window elapsed — re-authenticate and retry. */
        class NeedsAuthentication : VaultException("unlock required (authentication window elapsed)")

        /** The Keystore key was invalidated (e.g. new fingerprints enrolled). */
        class KeyInvalidated : VaultException("the device lock was changed; the stored seed can no longer be unwrapped")

        /** No seed installed yet. */
        class NoKey : VaultException("no signing key installed")

        /** Anything unexpected while talking to the Keystore/filesystem. */
        class Generic(message: String, cause: Throwable?) : VaultException(message, cause)
    }

    companion object {
        /** Purity-gate marker name (must match verifyVerifierPurity). */
        const val KEY_ALIAS = "vor_issuer_wrap"

        /** Purity-gate marker name (must match verifyVerifierPurity). */
        const val SEED_FILE_NAME = "vor_issuer_seed.bin"

        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val AUTH_WINDOW_SECONDS = 60
        private const val GCM_TAG_BITS = 128
        private const val GCM_IV_LENGTH = 12
    }

    // ---- key lifecycle ------------------------------------------------------

    /** True when a wrapped seed is present (does not touch the Keystore). */
    fun hasSeed(): Boolean = seedFile().exists() && IssuerStore.hasSeedMetadata(context)

    /** Public metadata of the installed key, or null when none. */
    fun keyInfo(): IssuerStore.KeyMeta? = IssuerStore.loadMeta(context)

    /** Public key (base64url) or null when no seed is installed. */
    fun publicKeyB64Url(): String? = keyInfo()?.pubB64Url

    /** First 8 hex chars of SHA-256(publicKey) — a display fingerprint. */
    fun fingerprint(): String? {
        val pub = Base64Url.decode(publicKeyB64Url() ?: return null) ?: return null
        val digest = MessageDigest.getInstance("SHA-256").digest(pub)
        return digest.take(4).joinToString("") { "%02x".format(it) }
    }

    /**
     * Generate a fresh keypair on this device and wrap it for storage.
     * Overwrites any existing key (caller confirms).
     * @return public metadata of the new key.
     */
    fun generateKey(): IssuerStore.KeyMeta = installSeed(SoftwareEd25519.generateSeed(SecureRandom()), "generated")

    /**
     * Install a known 32-byte seed (import from the legacy GitHub-secret
     * format, or restore from a passphrase backup). Caller validates intent;
     * returns public metadata.
     */
    fun importSeed(seed: ByteArray, source: String): IssuerStore.KeyMeta =
        installSeed(seed, source)

    private fun installSeed(seed: ByteArray, source: String): IssuerStore.KeyMeta {
        require(seed.size == SoftwareEd25519.SEED_LENGTH) { "seed must be 32 bytes" }
        val publicKey = SoftwareEd25519.publicKeyOf(seed)
        val meta = IssuerStore.KeyMeta(
            pubB64Url = SoftwareEd25519.publicToBase64Url(publicKey),
            createdAt = IssuerTime.nowRfc3339Utc(),
            source = source,
        )
        wrapSeedToDisk(seed)
        IssuerStore.saveMeta(context, meta)
        return meta
    }

    /**
     * Sign canonical payload bytes with the installed seed. Requires a
     * recent lock-screen authentication; throws [VaultException.NeedsAuthentication]
     * otherwise, and the caller re-authenticates and retries.
     */
    fun sign(payload: ByteArray): ByteArray {
        val seed = unwrapSeedFromDisk()
        try {
            return SoftwareEd25519.sign(seed, payload)
        } finally {
            seed.fill(0)
        }
    }

    /** Remove the seed (and its metadata). The ledger is NOT touched. */
    fun wipeSeed() {
        seedFile().delete()
        IssuerStore.clearMeta(context)
        runCatching {
            val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
            if (keyStore.containsAlias(KEY_ALIAS)) keyStore.deleteEntry(KEY_ALIAS)
        }
    }

    /**
     * Build a passphrase-protected backup envelope of the installed seed
     * (Argon2id + AES-256-GCM, see :core-license-issuer KeyBackup).
     */
    fun backupEnvelope(passphrase: CharArray): String {
        val seed = unwrapSeedFromDisk()
        try {
            return com.vor.license.issuer.KeyBackup.encrypt(
                seed, passphrase, IssuerTime.nowRfc3339Utc(),
            )
        } finally {
            seed.fill(0)
        }
    }

    // ---- internals -----------------------------------------------------------

    private fun seedFile(): File = File(context.filesDir, SEED_FILE_NAME)

    private fun wrapKey(): SecretKey {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        (keyStore.getEntry(KEY_ALIAS, null) as? KeyStore.SecretKeyEntry)?.let { return it.secretKey }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
        generator.init(
            KeyGenParameterSpec.Builder(
                KEY_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                // The whole point: without a recent unlock, the wrapped seed
                // is unusable, no matter where the file goes.
                .setUserAuthenticationRequired(true)
                .setUserAuthenticationValidityDurationSeconds(AUTH_WINDOW_SECONDS)
                .build(),
        )
        return generator.generateKey()
    }

    private fun wrapSeedToDisk(seed: ByteArray) {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        try {
            cipher.init(Cipher.ENCRYPT_MODE, wrapKey())
        } catch (e: UserNotAuthenticatedException) {
            throw VaultException.NeedsAuthentication()
        } catch (e: Exception) {
            throw VaultException.Generic("cannot create the wrapping key", e)
        }
        val iv = cipher.iv ?: ByteArray(GCM_IV_LENGTH).also(SecureRandom()::nextBytes)
        val encrypted = try {
            cipher.doFinal(seed)
        } catch (e: UserNotAuthenticatedException) {
            throw VaultException.NeedsAuthentication()
        }
        seedFile().writeBytes(iv + encrypted)
    }

    private fun unwrapSeedFromDisk(): ByteArray {
        val file = seedFile()
        if (!file.exists()) throw VaultException.NoKey()
        val blob = file.readBytes()
        if (blob.size <= GCM_IV_LENGTH) throw VaultException.Generic("stored seed is corrupted", null)
        val iv = blob.copyOfRange(0, GCM_IV_LENGTH)
        val encrypted = blob.copyOfRange(GCM_IV_LENGTH, blob.size)
        return try {
            val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
            val key = (keyStore.getEntry(KEY_ALIAS, null) as? KeyStore.SecretKeyEntry)?.secretKey
                ?: throw VaultException.KeyInvalidated()
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(GCM_TAG_BITS, iv))
            val seed = cipher.doFinal(encrypted)
            if (seed.size != SoftwareEd25519.SEED_LENGTH) {
                throw VaultException.Generic("stored seed is malformed", null)
            }
            seed
        } catch (e: UserNotAuthenticatedException) {
            throw VaultException.NeedsAuthentication()
        } catch (e: KeyPermanentlyInvalidatedException) {
            throw VaultException.KeyInvalidated()
        } catch (e: VaultException) {
            throw e
        } catch (e: Exception) {
            // A salt/keystore mismatch or corrupted blob — with GCM the
            // honest answer is the same: this copy is unusable.
            throw VaultException.KeyInvalidated()
        }
    }
}
