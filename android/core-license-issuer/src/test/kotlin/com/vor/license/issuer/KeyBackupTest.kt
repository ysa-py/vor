package com.vor.license.issuer

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import java.security.SecureRandom

/**
 * Argon2id + AES-256-GCM key backup: KDF golden vectors vs argon2-cffi,
 * envelope round-trip, wrong passphrase, tamper detection, seed integrity
 * cross-check, and constant-visible parameters in the envelope.
 */
class KeyBackupTest {

    @Test
    fun argon2id_matchesArgon2Cffi() {
        GoldenVectors.ARGON2ID.forEach { vector ->
            val digest = KeyBackup.deriveKey(
                vector.password.toCharArray(),
                GoldenVectors.hexToBytes(vector.saltHex),
                vector.t, vector.mKib, vector.p,
            )
            assertArrayEquals("argon2id digest (${vector.name})", GoldenVectors.hexToBytes(vector.digestHex), digest)
        }
    }

    @Test
    fun argon2id_workFactorActuallyChangesOutput() {
        val salt = GoldenVectors.hexToBytes("30313233343536373839616263646566")
        val first = KeyBackup.deriveKey("pw".toCharArray(), salt, 1, 32, 1)
        val second = KeyBackup.deriveKey("pw".toCharArray(), salt, 2, 32, 1)
        assertNotEquals(java.util.Arrays.toString(first), java.util.Arrays.toString(second))
    }

    @Test
    fun envelope_roundTrip() {
        val seed = SoftwareEd25519.generateSeed()
        val envelope = KeyBackup.encrypt(
            seed, "correct horse battery staple".toCharArray(), "2026-09-12T00:00:00Z",
        )
        val recovered = KeyBackup.decrypt(envelope, "correct horse battery staple".toCharArray())
        assertArrayEquals(seed, recovered)
        assertTrue(envelope.contains("\"kdf\":{\"alg\":\"argon2id\""))
        assertTrue(envelope.contains("\"t\":3"))
        assertTrue(envelope.contains("\"m_kib\":65536"))
        assertTrue(envelope.contains("\"wrap\":\"aes-256-gcm\""))
        // Public key is recorded so a maintainer can identify the backup.
        assertTrue(envelope.contains(SoftwareEd25519.publicToBase64Url(SoftwareEd25519.publicKeyOf(seed))))
    }

    @Test
    fun envelope_roundTrip_unicodePassphrase() {
        val seed = SoftwareEd25519.generateSeed()
        val passphrase = "رمز-دستگاه-من"
        val envelope = KeyBackup.encrypt(seed, passphrase.toCharArray(), "2026-09-12T00:00:00Z")
        assertArrayEquals(seed, KeyBackup.decrypt(envelope, passphrase.toCharArray()))
    }

    @Test
    fun wrongPassphrase_fails_closed() {
        val seed = SoftwareEd25519.generateSeed()
        val envelope = KeyBackup.encrypt(seed, "right passphrase".toCharArray(), "2026-09-12T00:00:00Z")
        try {
            KeyBackup.decrypt(envelope, "wrong passphrase".toCharArray())
            fail("wrong passphrase must not decrypt")
        } catch (expected: KeyBackup.BackupException) {
            assertTrue(expected.message!!.contains("authentication failed", ignoreCase = true))
        }
    }

    @Test
    fun tamperedCiphertext_fails_closed() {
        val seed = SoftwareEd25519.generateSeed()
        val envelope = KeyBackup.encrypt(seed, "pw".toCharArray(), "2026-09-12T00:00:00Z")
        // Flip one character of the ciphertext blob (base64 body).
        val corrupted = envelope.replace("seed_enc_b64\":\"", "seed_enc_b64\":\"A")
        try {
            KeyBackup.decrypt(corrupted, "pw".toCharArray())
            fail("tampered envelope must not decrypt")
        } catch (_: KeyBackup.BackupException) {
            // expected
        }
    }

    @Test
    fun garbageInput_isRejectedNotThrown() {
        listOf("", "not json", "{}", "{\"v\":99,\"format\":\"nope\"}").forEach { input ->
            try {
                KeyBackup.decrypt(input, "pw".toCharArray())
                fail("garbage must not decrypt: '$input'")
            } catch (_: KeyBackup.BackupException) {
                // expected
            }
        }
    }

    @Test
    fun recordedPublicKey_readableWithoutPassphrase() {
        val seed = SoftwareEd25519.generateSeed()
        val envelope = KeyBackup.encrypt(seed, "pw".toCharArray(), "2026-09-12T00:00:00Z")
        assertEquals(
            SoftwareEd25519.publicToBase64Url(SoftwareEd25519.publicKeyOf(seed)),
            KeyBackup.recordedPublicKey(envelope),
        )
        assertEquals(null, KeyBackup.recordedPublicKey("garbage"))
    }

    @Test
    fun seedIntegrity_mismatchedRecordedKey_fails() {
        // Envelope claims a different public key than the seed derives ->
        // decrypt must fail the self-check rather than return a bad seed.
        val seed = SoftwareEd25519.generateSeed()
        val envelope = KeyBackup.encrypt(seed, "pw".toCharArray(), "2026-09-12T00:00:00Z")
        val other = SoftwareEd25519.publicToBase64Url(SoftwareEd25519.publicKeyOf(SoftwareEd25519.generateSeed()))
        val tampered = envelope.replace(
            SoftwareEd25519.publicToBase64Url(SoftwareEd25519.publicKeyOf(seed)), other,
        )
        try {
            KeyBackup.decrypt(tampered, "pw".toCharArray())
            fail("seed/pub mismatch must fail the self-check")
        } catch (expected: KeyBackup.BackupException) {
            assertTrue(expected.message!!.contains("self-check"))
        }
    }

    @Test
    fun customWorkFactors_respected() {
        // Fast params for tests; decrypt must honor the envelope's params.
        val seed = SoftwareEd25519.generateSeed()
        val random = SecureRandom(byteArrayOf(1, 2, 3)) // deterministic-ish, fine here
        val envelope = KeyBackup.encrypt(
            seed, "pw".toCharArray(), "2026-09-12T00:00:00Z",
            random = random, timeCost = 1, memoryKib = 32, parallelism = 1,
        )
        assertTrue(envelope.contains("\"t\":1"))
        assertTrue(envelope.contains("\"m_kib\":32"))
        assertArrayEquals(seed, KeyBackup.decrypt(envelope, "pw".toCharArray()))
    }
}
