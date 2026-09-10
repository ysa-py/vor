/*
 * MICAFP-UnifiedShield-6.0
 * SmsBootstrapReceiver.kt — SMS bootstrap receiver for out-of-band endpoint delivery
 *
 * Detects SMS messages containing a hidden Unicode control prefix (U+200B Zero-Width Space)
 * followed by an AES-256-GCM encrypted payload containing endpoint list data.
 * Validates HMAC before processing to prevent injection attacks.
 * Forwards decoded endpoints to the Rust daemon via JNI.
 *
 * Rate limited: processes at most 1 SMS per 30 seconds.
 * Permission: RECEIVE_SMS
 *
 * No root required. Cloudflare is NOT used.
 */

package org.micafp.unifiedshield.bootstrap

import android.annotation.SuppressLint
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Build
import android.provider.Telephony
import android.telephony.SmsMessage
import android.util.Log
import org.micafp.unifiedshield.jni.ShieldNativeBridge
import java.security.InvalidKeyException
import java.security.MessageDigest
import java.security.SecureRandom
import java.util.concurrent.atomic.AtomicLong
import java.util.zip.CRC32
import javax.crypto.Mac
import javax.crypto.spec.SecretKeySpec

/**
 * SMS bootstrap receiver that detects specially formatted SMS messages
 * containing encrypted endpoint data for bootstrap connectivity.
 *
 * Message format:
 *   [U+200B prefix] [Base64-encoded payload]
 *
 * Payload format (before Base64):
 *   [32 bytes HMAC-SHA256] [12 bytes IV] [N bytes AES-256-GCM ciphertext + 16 byte tag]
 *
 * Decrypted plaintext format (same as acoustic channel):
 *   [1 byte version] [2 bytes endpoint_count] [endpoint_count × endpoint_entry]
 *   endpoint_entry:
 *     [1 byte type] [1 byte addr_len] [addr_len bytes address] [2 bytes port]
 */
class SmsBootstrapReceiver : BroadcastReceiver() {

    companion object {
        private const val TAG = "Shield/SmsBootstrap"

        // Unicode Zero-Width Space as the invisible marker
        private const val MARKER_PREFIX = "\u200B"

        // Cryptographic constants
        private const val HMAC_ALGORITHM = "HmacSHA256"
        private const val HMAC_LENGTH = 32
        private const val AES_GCM_IV_LENGTH = 12
        private const val AES_GCM_TAG_LENGTH = 16
        private const val PAYLOAD_VERSION = 0x01

        // Rate limiting: minimum interval between processed messages (ms)
        private const val RATE_LIMIT_INTERVAL_MS = 30_000L

        // Maximum payload size (bytes) after Base64 decoding
        private const val MAX_PAYLOAD_SIZE = 4096

        // Shared secret is derived from the device's Shield installation key
        // and is provisioned during initial setup. In production, this is
        // loaded from encrypted storage.
        private const val HMAC_KEY_ALIAS = "shield_sms_hmac"
        private const val AES_KEY_ALIAS = "shield_sms_aes"

        // Last processed timestamp for rate limiting
        private val lastProcessedTime = AtomicLong(0L)
    }

    // JNI bridge to Rust daemon
    private val jniBridge = ShieldNativeBridge()

    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Telephony.Sms.Intents.SMS_RECEIVED_ACTION) {
            return
        }

        // Rate limit check
        val now = System.currentTimeMillis()
        val lastTime = lastProcessedTime.get()
        if (now - lastTime < RATE_LIMIT_INTERVAL_MS) {
            Log.d(TAG, "Rate limited: skipping SMS, last processed ${now - lastTime}ms ago")
            return
        }

        // Extract SMS messages
        val messages = extractSmsMessages(intent)
        if (messages.isNullOrEmpty()) {
            return
        }

        // Process each message looking for the bootstrap marker
        for (message in messages) {
            val body = message.messageBody ?: continue

            if (!body.startsWith(MARKER_PREFIX)) {
                continue
            }

            Log.i(TAG, "Detected bootstrap SMS with marker prefix")

            // Extract the payload (everything after the marker)
            val payloadBase64 = body.substring(MARKER_PREFIX.length).trim()

            if (payloadBase64.isEmpty() || payloadBase64.length > MAX_PAYLOAD_SIZE * 2) {
                Log.w(TAG, "Bootstrap SMS payload is empty or too large")
                continue
            }

            // Decode and process
            val result = processPayload(context, payloadBase64)
            if (result != null) {
                // Update rate limit timestamp
                lastProcessedTime.set(now)

                // Forward endpoints to Rust daemon
                jniBridge.onBootstrapEndpointsReceived(
                    source = "sms",
                    endpoints = result.toTypedArray()
                )
                Log.i(TAG, "Successfully processed bootstrap SMS: ${result.size} endpoints")

                // Abort broadcast so the SMS doesn't appear in the inbox
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    abortBroadcast() // Requires higher priority registration
                }
            }
        }
    }

    // ============================================================
    // SMS Extraction
    // ============================================================

    @SuppressLint("NewApi")
    private fun extractSmsMessages(intent: Intent): Array<SmsMessage>? {
        return try {
            Telephony.Sms.Intents.getMessagesFromIntent(intent)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to extract SMS messages", e)
            // Fallback for older APIs
            extractSmsMessagesLegacy(intent)
        }
    }

    @Suppress("DEPRECATION")
    private fun extractSmsMessagesLegacy(intent: Intent): Array<SmsMessage>? {
        val pdus = intent.extras?.get("pdus") as? ByteArray ?: return null
        val format = intent.extras?.getString("format") ?: "3gpp"
        return try {
            arrayOf(SmsMessage.createFromPdu(pdus, format))
        } catch (e: Exception) {
            Log.e(TAG, "Legacy SMS extraction failed", e)
            null
        }
    }

    // ============================================================
    // Payload Processing
    // ============================================================

    /**
     * Process the Base64-encoded payload: decode, validate HMAC, decrypt, parse endpoints.
     */
    private fun processPayload(context: Context, payloadBase64: String): List<String>? {
        // Step 1: Base64 decode
        val payload = try {
            android.util.Base64.decode(payloadBase64, android.util.Base64.NO_WRAP or android.util.Base64.NO_PADDING)
        } catch (e: IllegalArgumentException) {
            Log.w(TAG, "Invalid Base64 in bootstrap SMS payload", e)
            return null
        }

        if (payload.size < HMAC_LENGTH + AES_GCM_IV_LENGTH + AES_GCM_TAG_LENGTH + 1) {
            Log.w(TAG, "Payload too short: ${payload.size} bytes")
            return null
        }

        // Step 2: Split payload into HMAC and ciphertext
        val receivedHmac = payload.copyOfRange(0, HMAC_LENGTH)
        val ciphertext = payload.copyOfRange(HMAC_LENGTH, payload.size)

        // Step 3: Validate HMAC
        val hmacKey = getHmacKey(context) ?: run {
            Log.e(TAG, "Failed to retrieve HMAC key")
            return null
        }

        if (!validateHmac(ciphertext, receivedHmac, hmacKey)) {
            Log.w(TAG, "HMAC validation failed — rejecting bootstrap SMS")
            return null
        }

        // Step 4: Extract IV and encrypted data
        val iv = ciphertext.copyOfRange(0, AES_GCM_IV_LENGTH)
        val encrypted = ciphertext.copyOfRange(AES_GCM_IV_LENGTH, ciphertext.size)

        // Step 5: Decrypt with AES-256-GCM
        val aesKey = getAesKey(context) ?: run {
            Log.e(TAG, "Failed to retrieve AES key")
            return null
        }

        val plaintext = decryptAesGcm(encrypted, iv, aesKey) ?: run {
            Log.w(TAG, "AES-256-GCM decryption failed")
            return null
        }

        // Step 6: Parse endpoint list
        return parseEndpointList(plaintext)
    }

    // ============================================================
    // HMAC Validation
    // ============================================================

    private fun validateHmac(data: ByteArray, expectedHmac: ByteArray, key: ByteArray): Boolean {
        return try {
            val mac = Mac.getInstance(HMAC_ALGORITHM)
            val secretKey = SecretKeySpec(key, HMAC_ALGORITHM)
            mac.init(secretKey)
            val computedHmac = mac.doFinal(data)

            // Constant-time comparison to prevent timing attacks
            constantTimeEquals(computedHmac, expectedHmac)
        } catch (e: InvalidKeyException) {
            Log.e(TAG, "Invalid HMAC key", e)
            false
        } catch (e: Exception) {
            Log.e(TAG, "HMAC computation failed", e)
            false
        }
    }

    /**
     * Constant-time byte array comparison to prevent timing side-channel attacks.
     */
    private fun constantTimeEquals(a: ByteArray, b: ByteArray): Boolean {
        if (a.size != b.size) return false
        var result = 0
        for (i in a.indices) {
            result = result or (a[i].toInt() xor b[i].toInt())
        }
        return result == 0
    }

    // ============================================================
    // AES-256-GCM Decryption
    // ============================================================

    private fun decryptAesGcm(ciphertext: ByteArray, iv: ByteArray, key: ByteArray): ByteArray? {
        return try {
            val cipher = javax.crypto.Cipher.getInstance("AES/GCM/NoPadding")
            val keySpec = SecretKeySpec(key, "AES")
            val gcmSpec = javax.crypto.spec.GCMParameterSpec(AES_GCM_TAG_LENGTH * 8, iv)
            cipher.init(javax.crypto.Cipher.DECRYPT_MODE, keySpec, gcmSpec)
            cipher.doFinal(ciphertext)
        } catch (e: Exception) {
            Log.e(TAG, "AES-256-GCM decryption error", e)
            null
        }
    }

    // ============================================================
    // Endpoint List Parsing
    // ============================================================

    /**
     * Parse decrypted plaintext into endpoint list.
     * Format:
     *   [1 byte version] [2 bytes endpoint_count (big-endian)] [endpoint_count × endpoint_entry]
     *   endpoint_entry:
     *     [1 byte type] [1 byte addr_len] [addr_len bytes address] [2 bytes port (big-endian)]
     */
    private fun parseEndpointList(data: ByteArray): List<String>? {
        if (data.size < 3) {
            Log.w(TAG, "Endpoint data too short: ${data.size}")
            return null
        }

        var offset = 0

        val version = data[offset++].toInt() and 0xFF
        if (version != PAYLOAD_VERSION) {
            Log.w(TAG, "Unknown endpoint payload version: $version")
            return null
        }

        val endpointCount = ((data[offset].toInt() and 0xFF) shl 8) or (data[offset + 1].toInt() and 0xFF)
        offset += 2

        if (endpointCount > 64) {
            Log.w(TAG, "Unreasonable endpoint count: $endpointCount")
            return null
        }

        val endpoints = mutableListOf<String>()

        for (i in 0 until endpointCount) {
            if (offset + 4 > data.size) {
                Log.w(TAG, "Truncated endpoint entry at index $i")
                break
            }

            val type = data[offset++].toInt() and 0xFF
            val addrLen = data[offset++].toInt() and 0xFF

            if (addrLen == 0 || addrLen > 128 || offset + addrLen + 2 > data.size) {
                Log.w(TAG, "Invalid address length: $addrLen at endpoint $i")
                break
            }

            val address = String(data, offset, addrLen, Charsets.UTF_8)
            offset += addrLen

            val port = ((data[offset].toInt() and 0xFF) shl 8) or (data[offset + 1].toInt() and 0xFF)
            offset += 2

            val typeStr = when (type) {
                0x01 -> "wg"       // WireGuard
                0x02 -> "ygg"      // Yggdrasil
                0x03 -> "obfs4"    // obfs4
                0x04 -> "snow"     // Snowflake
                0x05 -> "ss"       // Shadowsocks
                0x06 -> "bridge"   // Tor bridge
                else -> "unknown"
            }

            endpoints.add("$typeStr://$address:$port")
        }

        return if (endpoints.isNotEmpty()) endpoints else null
    }

    // ============================================================
    // Key Management
    // ============================================================

    /**
     * Retrieve the HMAC key from the Android Keystore.
     * In production, this key is provisioned during initial device setup
     * and stored in the EncryptedSharedPreferences or Android Keystore.
     */
    @SuppressLint("HardwareIds")
    private fun getHmacKey(context: Context): ByteArray? {
        // In production: use Android Keystore to retrieve the key
        // For now, derive from the shared secret stored in encrypted prefs
        return try {
            val sharedPrefs = context.getSharedPreferences("shield_keys", Context.MODE_PRIVATE)
            val keyHex = sharedPrefs.getString(HMAC_KEY_ALIAS, null)
            if (keyHex != null) {
                hexToBytes(keyHex)
            } else {
                Log.w(TAG, "HMAC key not found in shared preferences")
                null
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to retrieve HMAC key", e)
            null
        }
    }

    /**
     * Retrieve the AES-256 key from the Android Keystore.
     */
    private fun getAesKey(context: Context): ByteArray? {
        return try {
            val sharedPrefs = context.getSharedPreferences("shield_keys", Context.MODE_PRIVATE)
            val keyHex = sharedPrefs.getString(AES_KEY_ALIAS, null)
            if (keyHex != null) {
                hexToBytes(keyHex)
            } else {
                Log.w(TAG, "AES key not found in shared preferences")
                null
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to retrieve AES key", e)
            null
        }
    }

    // ============================================================
    // Utility
    // ============================================================

    private fun hexToBytes(hex: String): ByteArray {
        val len = hex.length
        val data = ByteArray(len / 2)
        var i = 0
        while (i < len) {
            data[i / 2] = ((Character.digit(hex[i], 16) shl 4) + Character.digit(hex[i + 1], 16)).toByte()
            i += 2
        }
        return data
    }
}
