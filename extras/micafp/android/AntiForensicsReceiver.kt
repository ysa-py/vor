/*
 * MICAFP-UnifiedShield-6.0
 * AntiForensicsReceiver.kt — Anti-forensics emergency wipe receiver
 *
 * Monitors for emergency wipe triggers:
 *   1. ACTION_PACKAGE_REMOVED (self-cleanup on uninstall)
 *   2. Rapid tap detection via custom broadcast
 *   3. SMS-based wipe trigger (specific HMAC-authenticated token)
 *
 * On wipe trigger, performs a thorough data destruction sequence:
 *   1. Overwrite app data with random bytes
 *   2. Delete all databases and shared preferences
 *   3. Clear all cached files
 *   4. Request package data cleanup from Android
 *   5. Kill the process
 *   6. Show calculator UI (decoy)
 *
 * Must complete in <3 seconds.
 *
 * No root required. Cloudflare is NOT used.
 */

package org.micafp.unifiedshield.security

import android.app.ActivityManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Process
import android.provider.Settings
import android.util.Log
import org.micafp.unifiedshield.jni.ShieldNativeBridge
import java.io.File
import java.io.FileOutputStream
import java.io.RandomAccessFile
import java.security.SecureRandom
import javax.crypto.Mac
import javax.crypto.spec.SecretKeySpec

/**
 * Anti-forensics emergency wipe receiver.
 *
 * Performs rapid, thorough data destruction when triggered by any of:
 * - Self-package-removed broadcast
 * - Custom rapid-tap panic gesture
 * - HMAC-authenticated SMS wipe command
 *
 * The wipe is designed to complete in under 3 seconds, prioritizing
 * the most sensitive data first.
 */
class AntiForensicsReceiver : BroadcastReceiver() {

    companion object {
        private const val TAG = "Shield/AntiForensics"

        // Custom actions
        const val ACTION_RAPID_TAP = "org.micafp.unifiedshield.RAPID_TAP_PANIC"
        const val ACTION_SMS_WIPE = "org.micafp.unifiedshield.SMS_WIPE_TRIGGER"

        // Number of rapid taps required to trigger wipe
        const val RAPID_TAP_THRESHOLD = 5
        const val RAPID_TAP_WINDOW_MS = 3000L

        // SMS wipe token validation
        private const val HMAC_ALGORITHM = "HmacSHA256"
        private const val WIPE_TOKEN_KEY_ALIAS = "shield_wipe_hmac"

        // Maximum time for wipe operation (ms)
        private const val WIPE_TIMEOUT_MS = 3000L

        // Calculator package for decoy UI
        private const val CALCULATOR_PACKAGE = "com.android.calculator2"
        private const val CALCULATOR_PACKAGE_ALT = "com.google.android.calculator"

        // State for rapid tap detection
        private var tapCount = 0
        private var firstTapTime = 0L
    }

    // JNI bridge
    private val jniBridge = ShieldNativeBridge()

    override fun onReceive(context: Context, intent: Intent) {
        val action = intent.action ?: return

        Log.w(TAG, "Received broadcast: $action")

        when (action) {
            Intent.ACTION_PACKAGE_REMOVED -> {
                val data = intent.data
                val removedPackage = data?.schemeSpecificPart
                if (removedPackage == context.packageName) {
                    Log.w(TAG, "Self-package removal detected — triggering wipe")
                    performWipe(context)
                }
            }

            ACTION_RAPID_TAP -> {
                handleRapidTap(context)
            }

            ACTION_SMS_WIPE -> {
                val token = intent.getStringExtra("wipe_token") ?: return
                if (validateWipeToken(context, token)) {
                    Log.w(TAG, "SMS wipe token validated — triggering wipe")
                    performWipe(context)
                } else {
                    Log.w(TAG, "Invalid SMS wipe token — ignoring")
                }
            }
        }
    }

    // ============================================================
    // Rapid Tap Detection
    // ============================================================

    private fun handleRapidTap(context: Context) {
        val now = System.currentTimeMillis()

        if (now - firstTapTime > RAPID_TAP_WINDOW_MS) {
            // Reset window
            tapCount = 1
            firstTapTime = now
        } else {
            tapCount++
        }

        Log.d(TAG, "Rapid tap count: $tapCount (window: ${now - firstTapTime}ms)")

        if (tapCount >= RAPID_TAP_THRESHOLD) {
            Log.w(TAG, "Rapid tap threshold reached — triggering wipe")
            tapCount = 0
            firstTapTime = 0L
            performWipe(context)
        }
    }

    // ============================================================
    // SMS Wipe Token Validation
    // ============================================================

    private fun validateWipeToken(context: Context, token: String): Boolean {
        val key = getWipeKey(context) ?: return false

        return try {
            // The token is hex-encoded HMAC output
            val tokenBytes = hexToBytes(token)

            // Compute expected HMAC over a known challenge string
            val mac = Mac.getInstance(HMAC_ALGORITHM)
            val secretKey = SecretKeySpec(key, HMAC_ALGORITHM)
            mac.init(secretKey)
            val expectedMac = mac.doFinal("WIPE_TRIGGER".toByteArray(Charsets.UTF_8))

            constantTimeEquals(tokenBytes, expectedMac)
        } catch (e: Exception) {
            Log.e(TAG, "Wipe token validation error", e)
            false
        }
    }

    private fun constantTimeEquals(a: ByteArray, b: ByteArray): Boolean {
        if (a.size != b.size) return false
        var result = 0
        for (i in a.indices) {
            result = result or (a[i].toInt() xor b[i].toInt())
        }
        return result == 0
    }

    private fun getWipeKey(context: Context): ByteArray? {
        return try {
            val sharedPrefs = context.getSharedPreferences("shield_keys", Context.MODE_PRIVATE)
            val keyHex = sharedPrefs.getString(WIPE_TOKEN_KEY_ALIAS, null)
            if (keyHex != null) hexToBytes(keyHex) else null
        } catch (e: Exception) {
            null
        }
    }

    // ============================================================
    // Wipe Operation — Must complete in <3 seconds
    // ============================================================

    /**
     * Perform the emergency wipe operation.
     *
     * Priority order:
     * 1. Notify Rust daemon to wipe its state (fastest, most critical)
     * 2. Overwrite files with random bytes (secure erase)
     * 3. Delete databases
     * 4. Delete shared preferences
     * 5. Clear cache
     * 6. Request Android package data cleanup
     * 7. Kill the process
     * 8. Launch calculator decoy UI
     */
    private fun performWipe(context: Context) {
        val startTime = System.currentTimeMillis()
        Log.w(TAG, "EMERGENCY WIPE INITIATED")

        try {
            // Phase 1: Notify Rust daemon to wipe its in-memory state
            try {
                jniBridge.emergencyWipe()
            } catch (e: Exception) {
                Log.e(TAG, "JNI wipe call failed", e)
            }

            // Phase 2: Overwrite sensitive files with random bytes
            overwriteFilesWithRandom(context)

            // Phase 3: Delete databases
            deleteDatabases(context)

            // Phase 4: Delete shared preferences
            deleteSharedPreferences(context)

            // Phase 5: Clear cache
            clearCache(context)

            // Phase 6: Clear external cache
            clearExternalCache(context)

            // Phase 7: Request Android to clean up package data
            requestPackageDataCleanup(context)

            val elapsed = System.currentTimeMillis() - startTime
            Log.w(TAG, "Wipe completed in ${elapsed}ms")

        } catch (e: Exception) {
            Log.e(TAG, "Error during wipe", e)
        }

        // Phase 8: Launch calculator UI as decoy
        launchCalculatorDecoy(context)

        // Phase 9: Kill the process
        killProcess(context)
    }

    /**
     * Overwrite all files in the app's data directory with random bytes.
     * This prevents forensic recovery of file contents even if the files
     * themselves are not fully deleted.
     */
    private fun overwriteFilesWithRandom(context: Context) {
        val secureRandom = SecureRandom()
        val dataDir = context.dataDir

        try {
            overwriteDirectory(dataDir, secureRandom)
        } catch (e: Exception) {
            Log.e(TAG, "Error overwriting files", e)
        }
    }

    /**
     * Recursively overwrite files in a directory with random bytes.
     * Uses a single-pass overwrite for speed (3 second constraint).
     */
    private fun overwriteDirectory(dir: File, random: SecureRandom) {
        if (!dir.exists() || !dir.isDirectory) return

        val files = dir.listFiles() ?: return

        for (file in files) {
            if (file.isDirectory) {
                overwriteDirectory(file, random)
            } else if (file.isFile && file.canWrite()) {
                try {
                    overwriteFile(file, random)
                } catch (e: Exception) {
                    // Skip files that can't be overwritten
                }
            }
        }
    }

    /**
     * Overwrite a file's contents with random bytes.
     * Reads original length, overwrites with random data of the same length.
     */
    private fun overwriteFile(file: File, random: SecureRandom) {
        val length = file.length()
        if (length <= 0 || length > 50 * 1024 * 1024) {
            // Skip empty files and files larger than 50MB (time constraint)
            return
        }

        try {
            val raf = RandomAccessFile(file, "rw")
            raf.use {
                val buffer = ByteArray(minOf(length.toInt(), 8192))
                var remaining = length

                while (remaining > 0) {
                    val chunkSize = minOf(buffer.size.toLong(), remaining).toInt()
                    random.nextBytes(buffer)
                    raf.write(buffer, 0, chunkSize)
                    remaining -= chunkSize
                }

                raf.fd.sync()
            }
        } catch (_: Exception) { }
    }

    /**
     * Delete all app databases.
     */
    private fun deleteDatabases(context: Context) {
        try {
            val dbDir = context.getDatabasePath("dummy").parentFile
            if (dbDir != null && dbDir.exists()) {
                dbDir.listFiles()?.forEach { file ->
                    try {
                        file.delete()
                    } catch (_: Exception) { }
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error deleting databases", e)
        }

        // Also delete via context API for WAL files etc.
        try {
            context.databaseList().forEach { dbName ->
                try {
                    context.deleteDatabase(dbName)
                } catch (_: Exception) { }
            }
        } catch (_: Exception) { }
    }

    /**
     * Delete all shared preferences files.
     */
    private fun deleteSharedPreferences(context: Context) {
        try {
            val prefsDir = File(context.filesDir.parentFile, "shared_prefs")
            if (prefsDir.exists() && prefsDir.isDirectory) {
                prefsDir.listFiles()?.forEach { file ->
                    try {
                        file.delete()
                    } catch (_: Exception) { }
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error deleting shared preferences", e)
        }
    }

    /**
     * Clear the app's internal cache.
     */
    private fun clearCache(context: Context) {
        try {
            context.cacheDir?.let { dir ->
                deleteRecursive(dir)
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error clearing cache", e)
        }
    }

    /**
     * Clear the app's external cache.
     */
    private fun clearExternalCache(context: Context) {
        try {
            context.externalCacheDir?.let { dir ->
                deleteRecursive(dir)
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error clearing external cache", e)
        }
    }

    /**
     * Recursively delete a directory and its contents.
     */
    private fun deleteRecursive(file: File) {
        if (file.isDirectory) {
            file.listFiles()?.forEach { child ->
                deleteRecursive(child)
            }
        }
        try {
            file.delete()
        } catch (_: Exception) { }
    }

    /**
     * Request Android to perform package data cleanup.
     * On newer Android versions, this may require user confirmation.
     */
    private fun requestPackageDataCleanup(context: Context) {
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                // On Android 10+, try the storage clear intent
                val intent = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
                    data = Uri.fromParts("package", context.packageName, null)
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                }
                context.startActivity(intent)
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error requesting package data cleanup", e)
        }

        // Also try to clear app data via ActivityManager if possible
        try {
            val am = context.getSystemService(Context.ACTIVITY_SERVICE) as? ActivityManager
            am?.clearApplicationUserData()
        } catch (_: Exception) {
            // This is a system API that may not be available
        }
    }

    /**
     * Launch the calculator app as a decoy UI.
     * This makes it appear as if the user was using a calculator,
     * hiding the fact that a wipe operation just occurred.
     */
    private fun launchCalculatorDecoy(context: Context) {
        try {
            val pm = context.packageManager

            // Try primary calculator package
            val calcIntent = pm.getLaunchIntentForPackage(CALCULATOR_PACKAGE)
                ?: pm.getLaunchIntentForPackage(CALCULATOR_PACKAGE_ALT)

            if (calcIntent != null) {
                calcIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
                context.startActivity(calcIntent)
            } else {
                // Fallback: open a generic calculator search
                val fallbackIntent = Intent(Intent.ACTION_MAIN).apply {
                    addCategory(Intent.CATEGORY_APP_CALCULATOR)
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
                }
                context.startActivity(fallbackIntent)
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to launch calculator decoy", e)
        }
    }

    /**
     * Kill the app process immediately.
     */
    private fun killProcess(context: Context) {
        try {
            // Stop all services first
            val am = context.getSystemService(Context.ACTIVITY_SERVICE) as? ActivityManager
            am?.let { manager ->
                // On API 29+, we can request to restart the package
                // But since we want to kill, just use Process.killProcess
            }
        } catch (_: Exception) { }

        // Finally, kill our own process
        Process.killProcess(Process.myPid())
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
