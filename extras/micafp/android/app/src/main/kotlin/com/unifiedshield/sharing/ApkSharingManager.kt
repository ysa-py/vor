package com.unifiedshield.sharing

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.util.Log
import androidx.core.content.FileProvider
import com.unifiedshield.logging.DebugLogger
import java.io.File

class ApkSharingManager(private val context: Context) {

    private val TAG = "ApkSharingManager"
    private val logger = DebugLogger.getInstance()

    /**
     * Prepares and shares the APK file directly to nearby devices via Bluetooth,
     * Nearby Share, or messaging apps in case of an Internet blackout.
     */
    fun shareAppApk(): Intent? {
        return try {
            val appInfo = context.applicationInfo
            val originalApk = File(appInfo.sourceDir)

            if (!originalApk.exists()) {
                logger.warn(TAG, "Source APK file not found at: ${originalApk.absolutePath}")
                return null
            }

            val shareDir = File(context.cacheDir, "shared_apks")
            if (!shareDir.exists()) shareDir.mkdirs()

            val targetApk = File(shareDir, "UnifiedShield_v3.2_Release.apk")
            originalApk.copyTo(targetApk, overwrite = true)

            val apkUri: Uri = try {
                FileProvider.getUriForFile(
                    context,
                    "${context.packageName}.fileprovider",
                    targetApk
                )
            } catch (e: Exception) {
                Uri.fromFile(targetApk)
            }

            val shareIntent = Intent(Intent.ACTION_SEND).apply {
                type = "application/vnd.android.package-archive"
                putExtra(Intent.EXTRA_STREAM, apkUri)
                putExtra(Intent.EXTRA_SUBJECT, "UnifiedShield Offline Anti-Censorship APK")
                putExtra(
                    Intent.EXTRA_TEXT,
                    "فایل نصبی یونیفاید شیلد (UnifiedShield v3.2) - ابزار جامع ضد فیلترینگ و دورزدن قطعی اینترنت همراه با پشتیبانی از DNSTT، NoizDNS، VayDNS، SSH، NaiveProxy و Tor."
                )
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }

            logger.info(TAG, "Prepared APK share intent for offline distribution.")
            shareIntent
        } catch (e: Exception) {
            logger.warn(TAG, "Failed to prepare APK share: ${e.message}")
            null
        }
    }
}
