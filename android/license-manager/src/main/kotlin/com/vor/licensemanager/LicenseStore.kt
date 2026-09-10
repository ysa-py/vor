package com.vor.licensemanager

import android.content.Context
import android.content.SharedPreferences
import com.vor.license.LicenseResult
import com.vor.license.LicenseVerifier
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.text.DateFormat
import java.util.Date
import java.util.Locale

/**
 * Offline multi-license store for maintainers managing several users' tokens.
 *
 * Design constraints (from the engineering task):
 * - MULTIPLE imported licenses, each with a clear valid/expired/invalid status
 *   and its expiry date shown in LOCAL time.
 * - Countdown indicator for licenses nearing expiry.
 * - Copy-token and clear-all actions.
 * - Fully offline: SharedPreferences + JSON, no network, no database, no new
 *   permissions. Statuses are recomputed from the signed expiry claim against
 *   the device clock at every render tick — they flip to expired on their own.
 *
 * Pure JSON/parse/format helpers are separated from persistence so they are
 * unit-testable on the plain JVM (see LicenseStoreTest).
 */
object LicenseStore {

    private const val PREFS_NAME = "vor_license_manager"
    private const val KEY_LICENSES = "licenses_json"

    @Serializable
    data class StoredLicense(
        val token: String,
        val addedAtMs: Long,
    )

    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = true
    }

    // ---- persistence (thin Android wrapper; logic-free) --------------------

    private fun prefs(context: Context): SharedPreferences =
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    fun load(context: Context): List<StoredLicense> =
        decode(prefs(context).getString(KEY_LICENSES, null) ?: "[]")

    fun save(context: Context, licenses: List<StoredLicense>) {
        prefs(context).edit().putString(KEY_LICENSES, encode(licenses)).apply()
    }

    /** Insert/replace by token (re-adding refreshes the added-at timestamp). */
    fun upsert(context: Context, token: String): List<StoredLicense> {
        val normalized = token.trim()
        if (normalized.isEmpty()) return load(context)
        val updated = load(context).filter { it.token != normalized } +
            StoredLicense(normalized, System.currentTimeMillis())
        save(context, updated)
        return updated
    }

    fun remove(context: Context, token: String): List<StoredLicense> {
        val updated = load(context).filter { it.token != token }
        save(context, updated)
        return updated
    }

    fun clearAll(context: Context): List<StoredLicense> {
        save(context, emptyList())
        return emptyList()
    }

    // ---- pure codec (JVM-testable) ------------------------------------------

    fun encode(licenses: List<StoredLicense>): String =
        json.encodeToString(kotlinx.serialization.builtins.ListSerializer(StoredLicense.serializer()), licenses)

    fun decode(text: String): List<StoredLicense> =
        runCatching {
            json.decodeFromString(
                kotlinx.serialization.builtins.ListSerializer(StoredLicense.serializer()),
                text,
            )
        }.getOrDefault(emptyList())

    // ---- presentation (JVM-testable) ----------------------------------------

    /** Everything the list UI shows for one stored license. */
    data class LicenseView(
        val token: String,
        val result: LicenseResult,
        val addedAtMs: Long,
        /** Expiry rendered in the user's LOCAL time zone, e.g. "Sep 11, 2027 3:30:00 AM". */
        val expiryLocal: String?,
        /** Milliseconds until expiry (negative when already past); null when unparseable. */
        val remainingMs: Long?,
    )

    fun view(
        token: String,
        addedAtMs: Long,
        nowEpochSeconds: Long,
        publicKeyBase64: String,
    ): LicenseView {
        val result = LicenseVerifier.verify(publicKeyBase64, token, nowEpochSeconds)
        val expiryEpoch = result.payload?.expiresAtEpoch()
        val expiryLocal = expiryEpoch?.let { seconds ->
            runCatching {
                DateFormat.getDateTimeInstance(DateFormat.MEDIUM, DateFormat.SHORT, Locale.getDefault())
                    .format(Date(seconds * 1000))
            }.getOrNull()
        }
        val remainingMs = expiryEpoch?.let { it * 1000 - nowEpochSeconds * 1000 }
        return LicenseView(
            token = token,
            result = result,
            addedAtMs = addedAtMs,
            expiryLocal = expiryLocal,
            remainingMs = remainingMs,
        )
    }

    /**
     * Human countdown for a nearing-expiry license:
     * "927d" → months/days when >1 day; hours when <1 day; null when past or unknown.
     */
    fun countdownText(remainingMs: Long?): String? {
        if (remainingMs == null || remainingMs <= 0) return null
        val days = remainingMs / 86_400_000L
        val hours = (remainingMs % 86_400_000L) / 3_600_000L
        return when {
            days >= 60 -> "${days / 30}mo ${days % 30}d"
            days >= 1 -> "${days}d ${hours}h"
            else -> "${hours}h"
        }
    }

    /** True when the countdown should be visually highlighted (expires within 30 days). */
    fun isExpiringSoon(remainingMs: Long?): Boolean =
        remainingMs != null && remainingMs in 1..(30L * 86_400_000L)
}
