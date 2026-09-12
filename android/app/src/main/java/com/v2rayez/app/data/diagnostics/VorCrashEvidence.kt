package com.v2rayez.app.data.diagnostics

import android.content.Context
import android.content.Intent
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * Persistent crash evidence (v1.5.0): when the process dies from an uncaught
 * throwable, write the stack to a small rotating file set BEFORE delegating
 * to the platform handler.
 *
 * Why: the in-app log stream already records fatals, but it is only visible
 * AFTER the user manages to reopen the app, and the previous sessions could
 * never obtain the actual stack trace from the reporting device — every
 * fix was aimed at the plausible cause, not the proven one. This file closes
 * that loop: the NEXT time anything crashes on the seller's or a buyer's
 * device, the license gate itself offers a "share crash report" button with
 * the exact stack, transferable through any channel (Telegram, Bluetooth,
 * screenshot) with zero developer tools.
 *
 * Design rules:
 *  - runs inside a crashing process: no coroutines, no DataStore, no
 *    Android components — just one bounded file write on the crashing
 *    thread, wrapped in a catch-Throwable so it can never make things worse;
 *  - strictly bounded: header (device/app/version/time/thread) + up to
 *    [MAX_STACK_BYTES] of stack, one file per crash;
 *  - rotating: keep the newest [MAX_FILES] crashes;
 *  - file I/O only — fully unit-testable on Robolectric.
 */
object VorCrashEvidence {

    const val DIR_NAME = "diagnostics"
    const val FILE_PREFIX = "crash"

    /** Keep the 3 most recent crash files. */
    const val MAX_FILES = 3

    /** Cap the stored stack so a crashing flood can never fill storage. */
    const val MAX_STACK_BYTES = 16 * 1024

    /** Appends a header with app/device context, for triage without logcat. */
    fun header(context: Context, thread: Thread, throwable: Throwable): String {
        val stamp = SimpleDateFormat("yyyy-MM-dd HH:mm:ss.SSS", Locale.US).format(Date())
        val appVersion = runCatching {
            context.packageManager.getPackageInfo(context.packageName, 0).let { info ->
                val code = if (android.os.Build.VERSION.SDK_INT >= 28) {
                    info.longVersionCode
                } else {
                    @Suppress("DEPRECATION") // pre-P fallback, guarded by SDK check
                    info.versionCode.toLong()
                }
                "${info.versionName} (#$code)"
            }
        }.getOrDefault("unknown")
        val device = "Android ${android.os.Build.VERSION.RELEASE} " +
            "(SDK ${android.os.Build.VERSION.SDK_INT}) ${android.os.Build.MANUFACTURER} " +
            "${android.os.Build.MODEL}"
        return StringBuilder(256)
            .append("time: ").append(stamp).append('\n')
            .append("app: ").append(context.packageName).append(' ').append(appVersion).append('\n')
            .append("device: ").append(device).append('\n')
            .append("thread: ").append(thread.name).append('\n')
            .append("exception: ").append(throwable.javaClass.name)
            .append(throwable.message?.let { ": $it" } ?: "").append('\n')
            .toString()
    }

    /**
     * Persist one crash. Best-effort, never throws, safe to call from a
     * crashing thread. Rotates the oldest file away first.
     */
    fun write(context: Context, thread: Thread, throwable: Throwable): File? {
        return try {
            val dir = File(context.filesDir, DIR_NAME)
            if (!dir.exists() && !dir.mkdirs()) return null
            rotate(dir)
            val body = (header(context, thread, throwable) +
                throwable.stackTraceToString())
                .take(MAX_STACK_BYTES + 1024) // header + bounded stack
            val target = File(dir, "${FILE_PREFIX}_${System.currentTimeMillis()}.txt")
            target.writeText(body)
            target
        } catch (_: Throwable) {
            null
        }
    }

    /** Newest crash file, or null when none was ever recorded. */
    fun latestFile(context: Context): File? = crashFiles(context).firstOrNull()

    /** Newest crash report text, or null. */
    fun latestText(context: Context): String? =
        latestFile(context)?.let { file -> runCatching { file.readText() }.getOrNull() }

    /** Remove all stored crash evidence. */
    fun clear(context: Context) {
        runCatching { crashFiles(context).forEach { it.delete() } }
    }

    /** Share intent for any channel (Telegram, save-to-file, …). */
    fun shareIntent(context: Context): Intent = Intent(Intent.ACTION_SEND).apply {
        type = "text/plain"
        putExtra(
            Intent.EXTRA_SUBJECT,
            "Vor crash report (${context.packageName})",
        )
        putExtra(
            Intent.EXTRA_TEXT,
            latestText(context) ?: "(no crash recorded)",
        )
    }

    /** Crash files, newest first. */
    private fun crashFiles(context: Context): List<File> {
        val dir = File(context.filesDir, DIR_NAME)
        if (!dir.isDirectory) return emptyList()
        return runCatching {
            dir.listFiles { f -> f.name.startsWith(FILE_PREFIX) && f.name.endsWith(".txt") }
                ?.sortedByDescending { it.name }
                ?.take(MAX_FILES)
                ?: emptyList()
        }.getOrDefault(emptyList())
    }

    /** Drop the oldest beyond [MAX_FILES] (called before writing a new one). */
    private fun rotate(dir: File) {
        runCatching {
            val files = dir.listFiles { f -> f.name.startsWith(FILE_PREFIX) && f.name.endsWith(".txt") }
                ?.sortedByDescending { it.name }
                ?: return
            files.drop(MAX_FILES - 1).forEach { it.delete() }
        }
    }
}
