package com.vor.license.issuer

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import org.bouncycastle.crypto.generators.Argon2BytesGenerator
import org.bouncycastle.crypto.params.Argon2Parameters
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec

/**
 * Passphrase-protected backup of the issuer's Ed25519 seed.
 *
 * Envelope: Argon2id-derived AES-256 key + AES-GCM authenticated encryption
 * of the 32-byte seed, serialized as a versioned JSON file the maintainer
 * exports MANUALLY via the system file picker (never auto-uploaded, never
 * synced — the app has zero network capability by construction).
 *
 * KDF is BouncyCastle's Argon2id (org.bouncycastle.crypto.generators.
 * Argon2BytesGenerator) — the same maintained library already used for
 * Ed25519. Correctness is pinned against argon2-cffi golden vectors.
 *
 * Default work factors (t=3, m=64 MiB, p=1) are a deliberate mobile/backup
 * compromise: ~1s on a modern phone, ~0.5s on a desktop. The parameters are
 * recorded in every envelope so future readers can raise them without
 * breaking old backups (the decrypt path enforces v==1 and alg=="argon2id").
 */
object KeyBackup {

    /** Argon2id work factors used for NEW backups (recorded per envelope). */
    val DEFAULT_TIME_COST = 3
    val DEFAULT_MEMORY_KIB = 65_536 // 64 MiB
    val DEFAULT_PARALLELISM = 1

    private const val KEY_LENGTH_BYTES = 32
    private const val GCM_IV_LENGTH = 12
    private const val GCM_TAG_BITS = 128
    private const val FORMAT = "vor-issuer-key-backup"
    private const val APP_ID = "vor-license-manager-issuer"

    @Serializable
    data class Envelope(
        val v: Int,
        val format: String,
        val app: String,
        val kdf: Kdf,
        val wrap: String,
        @SerialName("iv_b64") val ivBase64: String,
        @SerialName("seed_enc_b64") val seedEncBase64: String,
        @SerialName("pub_b64url") val pubBase64Url: String,
        @SerialName("created_at") val createdAt: String,
    )

    @Serializable
    data class Kdf(
        val alg: String,
        val t: Int,
        @SerialName("m_kib") val memoryKib: Int,
        val p: Int,
        @SerialName("salt_b64") val saltBase64: String,
    )

    class BackupException(message: String, cause: Throwable? = null) : Exception(message, cause)

    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = true
        prettyPrint = false
    }

    /**
     * Wrap a 32-byte seed into a passphrase-protected JSON envelope.
     * The passphrase itself never leaves the caller's memory.
     */
    fun encrypt(
        seed: ByteArray,
        passphrase: CharArray,
        createdAt: String,
        random: SecureRandom = SecureRandom(),
        timeCost: Int = DEFAULT_TIME_COST,
        memoryKib: Int = DEFAULT_MEMORY_KIB,
        parallelism: Int = DEFAULT_PARALLELISM,
    ): String {
        require(seed.size == SoftwareEd25519.SEED_LENGTH) { "seed must be 32 bytes" }
        val salt = ByteArray(16).also(random::nextBytes)
        val iv = ByteArray(GCM_IV_LENGTH).also(random::nextBytes)

        val key = deriveKey(passphrase, salt, timeCost, memoryKib, parallelism)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, SecretKeySpec(key, "AES"), GCMParameterSpec(GCM_TAG_BITS, iv))
        val encrypted = cipher.doFinal(seed)

        val envelope = Envelope(
            v = 1,
            format = FORMAT,
            app = APP_ID,
            kdf = Kdf(
                alg = "argon2id",
                t = timeCost,
                memoryKib = memoryKib,
                p = parallelism,
                saltBase64 = java.util.Base64.getEncoder().encodeToString(salt),
            ),
            wrap = "aes-256-gcm",
            ivBase64 = java.util.Base64.getEncoder().encodeToString(iv),
            seedEncBase64 = java.util.Base64.getEncoder().encodeToString(encrypted),
            pubBase64Url = SoftwareEd25519.publicToBase64Url(SoftwareEd25519.publicKeyOf(seed)),
            createdAt = createdAt,
        )
        return json.encodeToString(Envelope.serializer(), envelope)
    }

    /**
     * Recover a 32-byte seed from a backup envelope.
     * Throws [BackupException] for any structural/auth failure (wrong
     * passphrase included — indistinguishable from corruption by design).
     */
    fun decrypt(text: String, passphrase: CharArray): ByteArray {
        val envelope = try {
            json.decodeFromString(Envelope.serializer(), text)
        } catch (e: Exception) {
            throw BackupException("Not a valid Vor issuer key backup", e)
        }
        if (envelope.v != 1 || envelope.format != FORMAT || envelope.kdf.alg != "argon2id" ||
            envelope.wrap != "aes-256-gcm"
        ) {
            throw BackupException("Unsupported backup format")
        }
        val salt = try {
            java.util.Base64.getDecoder().decode(envelope.kdf.saltBase64)
        } catch (e: IllegalArgumentException) {
            throw BackupException("Corrupt backup: bad salt", e)
        }
        val iv = try {
            java.util.Base64.getDecoder().decode(envelope.ivBase64)
        } catch (e: IllegalArgumentException) {
            throw BackupException("Corrupt backup: bad IV", e)
        }
        val encrypted = try {
            java.util.Base64.getDecoder().decode(envelope.seedEncBase64)
        } catch (e: IllegalArgumentException) {
            throw BackupException("Corrupt backup: bad ciphertext", e)
        }
        val key = deriveKey(passphrase, salt, envelope.kdf.t, envelope.kdf.memoryKib, envelope.kdf.p)
        val seed = try {
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.DECRYPT_MODE, SecretKeySpec(key, "AES"), GCMParameterSpec(GCM_TAG_BITS, iv))
            cipher.doFinal(encrypted)
        } catch (e: Exception) {
            throw BackupException("Wrong passphrase or corrupted backup (authentication failed)", e)
        }
        if (seed.size != SoftwareEd25519.SEED_LENGTH) {
            throw BackupException("Backup contained a malformed seed")
        }
        // Cross-check the recorded public key when present (cheap integrity win).
        if (envelope.pubBase64Url.isNotEmpty() &&
            SoftwareEd25519.publicToBase64Url(SoftwareEd25519.publicKeyOf(seed)) != envelope.pubBase64Url
        ) {
            throw BackupException("Backup self-check failed: seed does not match recorded public key")
        }
        return seed
    }

    /** Extract the recorded public key without a passphrase (for display). */
    fun recordedPublicKey(text: String): String? = try {
        json.decodeFromString(Envelope.serializer(), text).pubBase64Url.ifEmpty { null }
    } catch (_: Exception) {
        null
    }

    /** Argon2id(passphrase, salt, t, m, p) -> 32 bytes (BouncyCastle). */
    internal fun deriveKey(
        passphrase: CharArray,
        salt: ByteArray,
        timeCost: Int,
        memoryKib: Int,
        parallelism: Int,
    ): ByteArray {
        val parameters = Argon2Parameters.Builder(Argon2Parameters.ARGON2_id)
            .withVersion(Argon2Parameters.ARGON2_VERSION_13)
            .withIterations(timeCost)
            .withMemoryAsKB(memoryKib)
            .withParallelism(parallelism)
            .withSalt(salt)
            .build()
        val generator = Argon2BytesGenerator()
        generator.init(parameters)
        val output = ByteArray(KEY_LENGTH_BYTES)
        // UTF-8 password bytes — byte-for-byte what argon2-cffi hashes, so the
        // golden vectors pin BC's Argon2id against the reference implementation.
        generator.generateBytes(String(passphrase).toByteArray(Charsets.UTF_8), output)
        return output
    }
}
