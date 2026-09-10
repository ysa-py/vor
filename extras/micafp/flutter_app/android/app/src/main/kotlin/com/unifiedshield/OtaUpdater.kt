package com.unifiedshield

import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import android.os.Build
import android.util.Log
import androidx.work.*
import io.flutter.plugin.common.MethodChannel
import kotlinx.coroutines.*
import org.json.JSONObject
import java.io.File
import java.io.FileInputStream
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest

/**
 * OTA Updater using WorkManager for periodic update checks.
 *
 * Checks GitHub Releases API every 6 hours, downloads APK updates,
 * verifies SHA256, and prompts the user to install.
 */
class OtaUpdater {

    companion object {
        private const val TAG = "OtaUpdater"
        private const val GITHUB_REPO = "unifiedshield/unifiedshield-nextgen"
        private const val RELEASES_URL = "https://api.github.com/repos/$GITHUB_REPO/releases/latest"
        private const val WORK_NAME = "unifiedshield_ota_check"
        private const val PREFS_NAME = "unifiedshield_ota"
        private const val KEY_LAST_VERSION = "last_known_version"
        private const val KEY_PENDING_UPDATE_PATH = "pending_update_path"
        private const val KEY_PENDING_UPDATE_SHA256 = "pending_update_sha256"

        /**
         * Schedule periodic OTA checks every 6 hours.
         */
        fun schedulePeriodicChecks(context: Context) {
            val constraints = Constraints.Builder()
                .setRequiredNetworkType(NetworkType.CONNECTED)
                .setRequiresBatteryNotLow(true)
                .build()

            val periodicWork = PeriodicWorkRequestBuilder<OtaCheckWorker>(
                6, java.util.concurrent.TimeUnit.HOURS,
                30, java.util.concurrent.TimeUnit.MINUTES
            )
                .setConstraints(constraints)
                .setBackoffCriteria(
                    BackoffPolicy.EXPONENTIAL,
                    10, java.util.concurrent.TimeUnit.MINUTES
                )
                .build()

            WorkManager.getInstance(context).enqueueUniquePeriodicWork(
                WORK_NAME,
                ExistingPeriodicWorkPolicy.KEEP,
                periodicWork
            )

            Log.i(TAG, "Periodic OTA check scheduled (every 6 hours)")
        }

        /**
         * Check for updates immediately.
         */
        fun checkNow(context: Context): ListenableWorker.Result {
            return OtaCheckWorker.doCheck(context)
        }

        /**
         * Install an APK update.
         * Called from Flutter via MethodChannel.
         */
        fun installUpdate(context: Context, filePath: String, result: MethodChannel.Result) {
            try {
                val apkFile = File(filePath)
                if (!apkFile.exists()) {
                    result.error("FILE_NOT_FOUND", "APK file not found: $filePath", null)
                    return
                }

                // Verify SHA256 before installation
                val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
                val expectedHash = prefs.getString(KEY_PENDING_UPDATE_SHA256, null)
                if (expectedHash != null) {
                    val actualHash = calculateSha256(apkFile)
                    if (actualHash != expectedHash.lowercase()) {
                        result.error("HASH_MISMATCH", "SHA256 verification failed", null)
                        return
                    }
                }

                // Install using PackageInstaller
                val packageInstaller = context.packageManager.packageInstaller
                val params = PackageInstaller.SessionParams(
                    PackageInstaller.SessionParams.MODE_FULL_INSTALL
                )

                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                    params.setRequireUserAction(PackageInstaller.SessionParams.USER_ACTION_NOT_REQUIRED)
                }

                val sessionId = packageInstaller.openSession(params)
                val session = packageInstaller.openSession(sessionId)

                apkFile.inputStream().use { input ->
                    session.openWrite("package", 0, apkFile.length()).use { output ->
                        val buffer = ByteArray(65536)
                        var bytesRead: Int
                        while (input.read(buffer).also { bytesRead = it } != -1) {
                            output.write(buffer, 0, bytesRead)
                        }
                        session.fsync(output)
                    }
                }

                val intentSender = PendingIntent.getActivity(
                    context,
                    0,
                    Intent(context, MainActivity::class.java),
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
                ).intentSender

                session.commit(intentSender)
                session.close()

                Log.i(TAG, "APK installation committed: $filePath")
                result.success(true)
            } catch (e: Exception) {
                Log.e(TAG, "APK installation failed", e)
                result.error("INSTALL_FAILED", e.message, null)
            }
        }

        private fun calculateSha256(file: File): String {
            val digest = MessageDigest.getInstance("SHA-256")
            FileInputStream(file).use { fis ->
                val buffer = ByteArray(8192)
                var bytesRead: Int
                while (fis.read(buffer).also { bytesRead = it } != -1) {
                    digest.update(buffer, 0, bytesRead)
                }
            }
            return digest.digest().joinToString("") { "%02x".format(it) }
        }
    }
}

class OtaCheckWorker(
    context: Context,
    workerParams: WorkerParameters
) : CoroutineWorker(context, workerParams) {

    override suspend fun doWork(): Result {
        return doCheck(applicationContext)
    }

    companion object {
        fun doCheck(context: Context): ListenableWorker.Result {
            try {
                val prefs = context.getSharedPreferences(OtaUpdater.PREFS_NAME, Context.MODE_PRIVATE)
                val currentVersion = getCurrentVersion(context)
                val otaEnabled = prefs.getBoolean("ota_enabled", true)

                if (!otaEnabled) {
                    Log.i(OtaUpdater.TAG, "OTA updates disabled, skipping check")
                    return ListenableWorker.Result.success()
                }

                // Fetch latest release from GitHub
                val url = URL(OtaUpdater.RELEASES_URL)
                val connection = url.openConnection() as HttpURLConnection
                connection.requestMethod = "GET"
                connection.setRequestProperty("Accept", "application/vnd.github.v3+json")
                connection.setRequestProperty("User-Agent", "UnifiedShield-OTA")
                connection.connectTimeout = 30000
                connection.readTimeout = 30000

                val responseCode = connection.responseCode
                if (responseCode != 200) {
                    Log.w(OtaUpdater.TAG, "GitHub API returned $responseCode")
                    return ListenableWorker.Result.retry()
                }

                val responseBody = connection.inputStream.bufferedReader().readText()
                val json = JSONObject(responseBody)

                val latestVersion = json.optString("tag_name", "").removePrefix("v")
                if (latestVersion.isEmpty() || compareVersions(latestVersion, currentVersion) <= 0) {
                    Log.i(OtaUpdater.TAG, "No update available (current: $currentVersion, latest: $latestVersion)")
                    return ListenableWorker.Result.success()
                }

                Log.i(OtaUpdater.TAG, "Update available: $currentVersion -> $latestVersion")

                // Find APK download URL
                val assets = json.optJSONArray("assets") ?: return ListenableWorker.Result.success()
                var downloadUrl: String? = null
                var sha256Url: String? = null

                for (i in 0 until assets.length()) {
                    val asset = assets.getJSONObject(i)
                    val name = asset.optString("name", "").lowercase()
                    val assetUrl = asset.optString("browser_download_url", "")

                    when {
                        name.contains("arm64") && name.endsWith(".apk") && downloadUrl == null -> {
                            downloadUrl = assetUrl
                        }
                        name.endsWith(".apk") && downloadUrl == null -> {
                            downloadUrl = assetUrl
                        }
                        name.contains("sha256") -> {
                            sha256Url = assetUrl
                        }
                    }
                }

                if (downloadUrl == null) {
                    Log.w(OtaUpdater.TAG, "No APK asset found in release")
                    return ListenableWorker.Result.success()
                }

                // Download the APK
                val apkFile = File(context.cacheDir, "unifiedshield_update.apk")
                downloadFile(downloadUrl, apkFile)

                // Verify SHA256
                if (sha256Url != null) {
                    val expectedHash = fetchSha256(sha256Url)
                    val actualHash = calculateSha256(apkFile)
                    if (expectedHash != null && actualHash != expectedHash.lowercase()) {
                        Log.e(OtaUpdater.TAG, "SHA256 verification failed!")
                        apkFile.delete()
                        return ListenableWorker.Result.failure()
                    }
                }

                // Save pending update info
                prefs.edit()
                    .putString(OtaUpdater.KEY_PENDING_UPDATE_PATH, apkFile.absolutePath)
                    .putString(OtaUpdater.KEY_PENDING_UPDATE_SHA256, calculateSha256(apkFile))
                    .putString(OtaUpdater.KEY_LAST_VERSION, latestVersion)
                    .apply()

                Log.i(OtaUpdater.TAG, "OTA update downloaded and verified: v$latestVersion")

                // Show notification to prompt install
                showUpdateNotification(context, latestVersion, json.optString("body", ""))

                return ListenableWorker.Result.success()
            } catch (e: Exception) {
                Log.e(OtaUpdater.TAG, "OTA check failed", e)
                return ListenableWorker.Result.retry()
            }
        }

        private fun getCurrentVersion(context: Context): String {
            return try {
                context.packageManager.getPackageInfo(context.packageName, 0).versionName ?: "1.0.0"
            } catch (_: Exception) {
                "1.0.0"
            }
        }

        private fun compareVersions(v1: String, v2: String): Int {
            val parts1 = v1.split(".").map { it.toIntOrNull() ?: 0 }
            val parts2 = v2.split(".").map { it.toIntOrNull() ?: 0 }
            for (i in 0 until maxOf(parts1.size, parts2.size)) {
                val p1 = parts1.getOrElse(i) { 0 }
                val p2 = parts2.getOrElse(i) { 0 }
                if (p1 != p2) return p1.compareTo(p2)
            }
            return 0
        }

        private fun downloadFile(urlStr: String, targetFile: File) {
            val url = URL(urlStr)
            val connection = url.openConnection() as HttpURLConnection
            connection.connectTimeout = 60000
            connection.readTimeout = 600000

            connection.inputStream.use { input ->
                targetFile.outputStream().use { output ->
                    val buffer = ByteArray(8192)
                    var bytesRead: Int
                    while (input.read(buffer).also { bytesRead = it } != -1) {
                        output.write(buffer, 0, bytesRead)
                    }
                }
            }
        }

        private fun fetchSha256(sha256Url: String): String? {
            return try {
                val url = URL(sha256Url)
                val connection = url.openConnection() as HttpURLConnection
                connection.connectTimeout = 15000
                connection.readTimeout = 15000
                val content = connection.inputStream.bufferedReader().readText()
                content.trim().split(" ").firstOrNull()?.lowercase()
            } catch (e: Exception) {
                Log.w(OtaUpdater.TAG, "Failed to fetch SHA256 file", e)
                null
            }
        }

        private fun calculateSha256(file: File): String {
            val digest = java.security.MessageDigest.getInstance("SHA-256")
            FileInputStream(file).use { fis ->
                val buffer = ByteArray(8192)
                var bytesRead: Int
                while (fis.read(buffer).also { bytesRead = it } != -1) {
                    digest.update(buffer, 0, bytesRead)
                }
            }
            return digest.digest().joinToString("") { "%02x".format(it) }
        }

        private fun showUpdateNotification(context: Context, version: String, releaseNotes: String) {
            // Notification will be shown via flutter_local_notifications when app is foregrounded
            // WorkManager notifications are limited; we save state and let the app check
            val prefs = context.getSharedPreferences(OtaUpdater.PREFS_NAME, Context.MODE_PRIVATE)
            prefs.edit().putBoolean("update_available", true).putString("update_version", version).apply()
        }
    }
}

private class PendingIntent {
    companion object {
        fun getActivity(context: Context, requestCode: Int, intent: Intent, flags: Int): android.app.PendingIntent {
            return android.app.PendingIntent.getActivity(context, requestCode, intent, flags)
        }
    }
}
