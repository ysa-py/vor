package com.vor.licensemanager.issuer

import android.content.Context
import android.content.SharedPreferences
import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.builtins.serializer
import kotlinx.serialization.json.Json

/**
 * On-device issuer state: the editable LOCAL tier list, the "Issued by
 * this device" ledger, and non-secret key metadata. All of it lives in one
 * app-private SharedPreferences file ("vor_issuer_store") as JSON — the
 * same storage model as the verifier's LicenseStore: no Room, no ContentProvider,
 * no network, no new permissions.
 *
 * THE LEDGER IS NOT REVOCATION. Entries are a local history of what was
 * issued from this device; deleting one (or clearing all) changes nothing
 * about the tokens' validity — signed licenses stay valid until their
 * `expires_at`. The mitigation for a mis-issued token is short validity,
 * documented in the UI next to the history itself.
 *
 * Pure codec functions are separated from persistence so they are
 * unit-testable on the plain JVM (IssuerStoreTest).
 */
object IssuerStore {

    internal const val PREFS_NAME = "vor_issuer_store"
    private const val KEY_TIERS = "tiers_json"
    private const val KEY_HISTORY = "history_json"
    private const val KEY_META = "key_meta_json"
    private const val KEY_ONBOARDED = "onboarded"

    /** One entry of the issuance ledger. */
    @Serializable
    data class IssuedEntry(
        val id: String,
        val tier: String,
        val expiresAt: String,
        val issuedAt: String,
        val token: String,
        /** Local-only note (never serialized into any token). */
        val notes: String = "",
        /** Wall-clock millisecond timestamp of when this record was made. */
        val createdAtMs: Long,
    )

    /** Non-secret metadata describing the installed signing key. */
    @Serializable
    data class KeyMeta(
        /** base64url Ed25519 public key — safe to display anywhere. */
        val pubB64Url: String,
        /** RFC 3339 timestamp of when the key was installed on this device. */
        val createdAt: String,
        /** generated | imported | restored */
        val source: String,
    )

    /** The tier list ships with ONE neutral default, "standard" (the same
     * default as the reference Python issuer); the maintainer adds tiers
     * locally — they are free-form text, never a hardcoded enum. */
    val DEFAULT_TIERS = listOf("standard")

    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = true
    }

    // ---- persistence (thin Android wrapper) --------------------------------

    private fun prefs(context: Context): SharedPreferences =
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    fun hasSeedMetadata(context: Context): Boolean =
        loadMeta(context) != null

    fun loadMeta(context: Context): KeyMeta? =
        decodeMeta(prefs(context).getString(KEY_META, null))

    fun saveMeta(context: Context, meta: KeyMeta) {
        prefs(context).edit().putString(KEY_META, encodeMeta(meta)).apply()
    }

    fun clearMeta(context: Context) {
        prefs(context).edit().remove(KEY_META).apply()
    }

    fun loadTiers(context: Context): List<String> =
        decodeTiers(prefs(context).getString(KEY_TIERS, null)) ?: DEFAULT_TIERS

    fun saveTiers(context: Context, tiers: List<String>) {
        prefs(context).edit().putString(KEY_TIERS, encodeTiers(tiers)).apply()
    }

    fun loadHistory(context: Context): List<IssuedEntry> =
        decodeHistory(prefs(context).getString(KEY_HISTORY, null))

    fun appendHistory(context: Context, entries: List<IssuedEntry>): List<IssuedEntry> {
        val updated = loadHistory(context) + entries
        prefs(context).edit().putString(KEY_HISTORY, encodeHistory(updated)).apply()
        return updated
    }

    fun removeHistoryEntry(context: Context, entry: IssuedEntry): List<IssuedEntry> {
        val updated = loadHistory(context).filterNot {
            it.token == entry.token && it.createdAtMs == entry.createdAtMs
        }
        prefs(context).edit().putString(KEY_HISTORY, encodeHistory(updated)).apply()
        return updated
    }

    fun clearHistory(context: Context) {
        prefs(context).edit().putString(KEY_HISTORY, encodeHistory(emptyList())).apply()
    }

    fun isOnboarded(context: Context): Boolean =
        prefs(context).getBoolean(KEY_ONBOARDED, false)

    fun setOnboarded(context: Context, value: Boolean) {
        prefs(context).edit().putBoolean(KEY_ONBOARDED, value).apply()
    }

    fun wipeEverything(context: Context) {
        prefs(context).edit().clear().apply()
    }

    // ---- pure codecs (JVM-testable) -----------------------------------------

    fun encodeTiers(tiers: List<String>): String =
        json.encodeToString(ListSerializer(String.serializer()), tiers)

    fun decodeTiers(text: String?): List<String> =
        text?.let {
            runCatching { json.decodeFromString(ListSerializer(String.serializer()), it) }.getOrNull()
        } ?: DEFAULT_TIERS

    fun encodeHistory(entries: List<IssuedEntry>): String =
        json.encodeToString(ListSerializer(IssuedEntry.serializer()), entries)

    fun decodeHistory(text: String?): List<IssuedEntry> =
        text?.let {
            runCatching { json.decodeFromString(ListSerializer(IssuedEntry.serializer()), it) }.getOrNull()
        } ?: emptyList()

    fun encodeMeta(meta: KeyMeta): String = json.encodeToString(KeyMeta.serializer(), meta)

    fun decodeMeta(text: String?): KeyMeta? =
        text?.let { runCatching { json.decodeFromString(KeyMeta.serializer(), it) }.getOrNull() }
}

