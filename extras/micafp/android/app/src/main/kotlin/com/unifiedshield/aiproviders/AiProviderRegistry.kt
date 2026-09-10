package com.unifiedshield.aiproviders

import android.content.Context
import android.util.Log
import com.google.gson.Gson
import com.google.gson.annotations.SerializedName
import java.io.File

// =============================================================================
// MICAFP Directive v6 — C3 + Addendum B.8: config-driven provider registry.
//
// * Provider *definitions* live in JSON (bundled asset), NOT in code.
//   Adding the "7th provider" = adding a JSON entry, no Kotlin/Dart change.
// * requestSchema: openai-compatible | anthropic-native | custom.
//   openai-compatible is the DEFAULT path for any new vendor.
// * Optional signed remote registry sync reuses the existing signed-update
//   distribution path (config/ipfs_updater route); unsigned/invalid updates
//   are rejected and bundled defaults stay in effect (same rule as C.2).
// * This layer is UI/config only: it NEVER touches transport, obfuscation,
//   security or the VPN connection path (isolated in aiproviders/).
// =============================================================================

data class AiProviderDefinition(
    @SerializedName("id") val id: String,
    @SerializedName("displayName") val displayName: String,
    @SerializedName("displayNameFa") val displayNameFa: String = "",
    @SerializedName("baseUrlDefault") val baseUrlDefault: String,
    @SerializedName("authHeaderName") val authHeaderName: String = "Authorization",
    @SerializedName("authHeaderValuePrefix") val authHeaderValuePrefix: String = "Bearer ",
    @SerializedName("requestSchema") val requestSchema: String = "openai-compatible",
    @SerializedName("streamingSupported") val streamingSupported: Boolean = true,
    @SerializedName("models") val models: List<String> = emptyList(),
    @SerializedName("defaultModel") val defaultModel: String = "",
    @SerializedName("mirrorUrls") val mirrorUrls: List<String> = emptyList()
)

data class AiProviderRegistryFile(
    @SerializedName("schemaVersion") val schemaVersion: Int = 1,
    @SerializedName("registrySignature") val registrySignature: String? = null,
    @SerializedName("providers") val providers: List<AiProviderDefinition> = emptyList()
)

object AiProviderRegistry {

    private const val TAG = "MicafpAiRegistry"
    private const val ASSET_PATH = "ai_providers/registry.json"
    private const val REMOTE_OVERRIDE_NAME = "micafp_ai_registry_remote.json"

    private val gson = Gson()

    @Volatile
    private var cached: AiProviderRegistryFile? = null

    /**
     * Load order: signed remote override (if present and signature valid)
     * → bundled asset → hard safety default (generic OpenAI-compatible).
     * Unsigned/invalid remote registries are rejected (B.8 rule).
     */
    @Synchronized
    fun load(context: Context): AiProviderRegistryFile {
        cached?.let { return it }

        val remote = File(context.filesDir, REMOTE_OVERRIDE_NAME)
        if (remote.exists()) {
            runCatching {
                val file = gson.fromJson(remote.readText(), AiProviderRegistryFile::class.java)
                require(verifyRemoteSignature(file)) { "unsigned-or-invalid-registry" }
                file
            }.onSuccess {
                Log.i(TAG, "Remote signed provider registry accepted (${it.providers.size} providers)")
                cached = it
                return it
            }.onFailure {
                Log.w(TAG, "Remote registry REJECTED (${it.message}) — falling back to bundled defaults")
                remote.delete()
            }
        }

        val bundled = runCatching {
            context.assets.open(ASSET_PATH).bufferedReader().use { it.readText() }
                .let { gson.fromJson(it, AiProviderRegistryFile::class.java) }
        }.getOrNull() ?: AiProviderRegistryFile(
            providers = listOf(
                AiProviderDefinition(
                    id = "custom-openai", displayName = "Custom OpenAI-compatible",
                    baseUrlDefault = "https://api.example.com/v1",
                    requestSchema = "openai-compatible", models = emptyList()
                )
            )
        )
        cached = bundled
        return bundled
    }

    fun get(context: Context, id: String): AiProviderDefinition? =
        load(context).providers.firstOrNull { it.id == id }

    /**
     * B.8 signature verification for remote registries.
     * Uses the same Ed25519 public key authority as the license system;
     * until the signing service publishes the key this returns false, which
     * safely rejects remote overrides and keeps bundled defaults (A2 honest).
     */
    private fun verifyRemoteSignature(file: AiProviderRegistryFile): Boolean {
        val sig = file.registrySignature ?: return false
        if (sig.isBlank() || sig == "UNSIGNED") return false
        // Full Ed25519 verification over the canonical provider list is wired
        // to the license key authority in the human-merge-gated commit.
        // Current safe behaviour: any non-blank signature token is treated as
        // unverified and rejected — bundled defaults remain in effect.
        return false
    }

    /** Accepts a remote registry payload after upstream signed distribution. */
    @Synchronized
    fun applyRemoteRegistry(context: Context, jsonText: String): Boolean {
        return runCatching {
            val file = gson.fromJson(jsonText, AiProviderRegistryFile::class.java)
            require(verifyRemoteSignature(file)) { "unsigned-or-invalid-registry" }
            File(context.filesDir, REMOTE_OVERRIDE_NAME).writeText(jsonText)
            cached = null // force reload
            true
        }.getOrElse {
            Log.w(TAG, "applyRemoteRegistry rejected: ${it.message}")
            false
        }
    }
}
