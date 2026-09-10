package com.unifiedshield.aiproviders

import android.content.Context
import android.util.Log
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken

// =============================================================================
// MICAFP Directive v6 — C3: encrypted on-device storage for the AI API layer.
// * API keys: EncryptedSharedPreferences (AES-256-GCM, Android Keystore).
// * User-defined failover priority (B.9: user order first).
// * Master switch (user-controlled).
// * Per-provider editable Base URL + mirror URL (B.10).
// No key is ever hardcoded, logged, or shipped (C3 rule).
// =============================================================================

data class AiProviderUserConfig(
    val enabled: Boolean = true,          // per-provider enable
    val apiKey: String = "",              // stored encrypted
    val baseUrlOverride: String = "",     // editable Base URL (C3)
    val mirrorUrl: String = "",           // user-supplied mirror (B.10a)
    val model: String = ""                // selected model (registry default if empty)
)

class AiProviderStore private constructor(private val context: Context) {

    private val TAG = "MicafpAiStore"
    private val gson = Gson()

    private val prefs by lazy {
        val masterKey = MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()
        EncryptedSharedPreferences.create(
            context, "micafp_ai_secure", masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
        )
    }

    private val plainPrefs = context.getSharedPreferences("micafp_ai_ui", Context.MODE_PRIVATE)

    // ---- master switch ----
    @Volatile private var masterSwitchCache: Boolean? = null
    val masterSwitch: Boolean
        get() = masterSwitchCache ?: plainPrefs.getBoolean("master_switch", false).also { masterSwitchCache = it }

    fun setMasterSwitch(enabled: Boolean) {
        plainPrefs.edit().putBoolean("master_switch", enabled).apply()
        masterSwitchCache = enabled
        Log.i(TAG, "External AI layer master switch set to $enabled")
    }

    // ---- per-provider config ----
    fun getConfig(providerId: String): AiProviderUserConfig {
        val parsed: AiProviderUserConfig? = runCatching {
            val raw: String? = prefs.getString("cfg_$providerId", "{}")
            gson.fromJson(raw, AiProviderUserConfig::class.java)
        }.getOrNull()
        return parsed ?: AiProviderUserConfig()
    }

    fun saveConfig(providerId: String, cfg: AiProviderUserConfig) {
        prefs.edit().putString("cfg_$providerId", gson.toJson(cfg)).apply()
        Log.i(TAG, "Provider config updated: $providerId (key=${cfg.apiKey.isNotBlank()})")
    }

    // ---- failover priority (user-defined order) ----
    fun getPriorityOrder(): List<String> {
        val parsed: List<String>? = runCatching {
            val raw: String? = plainPrefs.getString("priority_order", null)
            gson.fromJson<List<String>>(
                raw,
                object : TypeToken<List<String>>() {}.type
            )
        }.getOrNull()
        return parsed ?: emptyList()
    }

    fun setPriorityOrder(order: List<String>) {
        plainPrefs.edit().putString("priority_order", gson.toJson(order)).apply()
    }

    companion object {
        @Volatile private var instance: AiProviderStore? = null
        fun getInstance(context: Context): AiProviderStore =
            instance ?: synchronized(this) {
                instance ?: AiProviderStore(context.applicationContext).also { instance = it }
            }
    }
}
