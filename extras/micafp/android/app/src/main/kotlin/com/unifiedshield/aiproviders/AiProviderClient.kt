package com.unifiedshield.aiproviders

import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.TimeUnit

// =============================================================================
// MICAFP Directive v6 — C3 thin provider-abstraction client.
// Config-driven adapters (no per-vendor code): openai-compatible |
// anthropic-native. Non-streaming completions. Real errors surface as real
// states — NEVER a fabricated AI response (A2 / C3 graceful degradation).
// =============================================================================

sealed class AiCallResult {
    data class Success(val text: String, val viaBaseUrl: String, val latencyMs: Long) : AiCallResult()
    data class Failure(val reason: String, val viaBaseUrl: String) : AiCallResult()
}

object AiProviderClient {

    private const val TAG = "MicafpAiClient"

    private val http = OkHttpClient.Builder()
        .connectTimeout(12, TimeUnit.SECONDS)
        .readTimeout(45, TimeUnit.SECONDS)
        .writeTimeout(20, TimeUnit.SECONDS)
        .build()

    private val jsonMedia = "application/json; charset=utf-8".toMediaType()

    /**
     * Single-shot completion against one base URL using the provider's
     * configured request schema. `routeThroughTunnel` is a hint reserved for
     * the human-merge-gated wiring that proxies the call through the active
     * obfuscated tunnel (B.10b) — direct OkHttp is the current honest path.
     */
    suspend fun complete(
        definition: AiProviderDefinition,
        cfg: AiProviderUserConfig,
        baseUrl: String,
        systemPrompt: String,
        userPrompt: String,
        routeThroughTunnel: Boolean = false
    ): AiCallResult = withContext(Dispatchers.IO) {
        val started = System.currentTimeMillis()
        if (cfg.apiKey.isBlank()) {
            return@withContext AiCallResult.Failure("no-api-key", baseUrl)
        }
        val model = cfg.model.ifBlank { definition.defaultModel }
        if (model.isBlank()) {
            return@withContext AiCallResult.Failure("no-model-configured", baseUrl)
        }

        val bodyJson = when (definition.requestSchema) {
            "anthropic-native" -> anthropicBody(model, systemPrompt, userPrompt)
            else -> openAiCompatibleBody(model, systemPrompt, userPrompt)
        }

        val url = baseUrl.trimEnd('/') + when (definition.requestSchema) {
            "anthropic-native" -> "/messages"
            else -> "/chat/completions"
        }

        try {
            val builder = Request.Builder()
                .url(url)
                .post(bodyJson.toString().toRequestBody(jsonMedia))
                .header(definition.authHeaderName, definition.authHeaderValuePrefix + cfg.apiKey)
            if (definition.requestSchema == "anthropic-native") {
                builder.header("anthropic-version", "2023-06-01")
            }
            val response = http.newCall(builder.build()).execute()
            response.use { resp ->
                val respBody = resp.body?.string().orEmpty()
                if (!resp.isSuccessful) {
                    Log.w(TAG, "Provider ${definition.id} HTTP ${resp.code} via $baseUrl")
                    return@withContext AiCallResult.Failure("http-${resp.code}", baseUrl)
                }
                val text = when (definition.requestSchema) {
                    "anthropic-native" -> parseAnthropic(respBody)
                    else -> parseOpenAiCompatible(respBody)
                }
                if (text.isNullOrBlank()) AiCallResult.Failure("empty-response", baseUrl)
                else AiCallResult.Success(text, baseUrl, System.currentTimeMillis() - started)
            }
        } catch (e: Exception) {
            Log.w(TAG, "Provider ${definition.id} call failed via $baseUrl: ${e.message}")
            AiCallResult.Failure("network-error: ${e.message?.take(80)}", baseUrl)
        }
    }

    // ---- request builders (config-driven, vendor-agnostic) ----
    private fun openAiCompatibleBody(model: String, system: String, user: String): JSONObject =
        JSONObject().apply {
            put("model", model)
            put("max_tokens", 700)
            put("messages", JSONArray()
                .put(JSONObject().put("role", "system").put("content", system))
                .put(JSONObject().put("role", "user").put("content", user)))
        }

    private fun anthropicBody(model: String, system: String, user: String): JSONObject =
        JSONObject().apply {
            put("model", model)
            put("max_tokens", 700)
            put("system", system)
            put("messages", JSONArray()
                .put(JSONObject().put("role", "user").put("content", user)))
        }

    private fun parseOpenAiCompatible(body: String): String? = runCatching {
        JSONObject(body)
            .getJSONArray("choices").getJSONObject(0)
            .getJSONObject("message").getString("content")
    }.getOrNull()

    private fun parseAnthropic(body: String): String? = runCatching {
        val arr = JSONObject(body).getJSONArray("content")
        buildString {
            for (i in 0 until arr.length()) append(arr.getJSONObject(i).optString("text", ""))
        }.trim().ifBlank { null }
    }.getOrNull()
}
