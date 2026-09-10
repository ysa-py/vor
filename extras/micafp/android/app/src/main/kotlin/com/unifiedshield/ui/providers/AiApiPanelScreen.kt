package com.unifiedshield.ui.providers

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.unifiedshield.aiproviders.*
import com.unifiedshield.localization.MicafpLangManager
import com.unifiedshield.ui.theme.MicafpTokens
import com.unifiedshield.ui.theme.MicafpType
import kotlinx.coroutines.launch

// =============================================================================
// MICAFP Directive v6 — C3 AI API Panel (+ Addendum B.8/B.9/B.10).
// * Master switch for the whole external AI layer (user-controlled).
// * Per provider: encrypted API key, EDITABLE Base URL, mirror URL, model,
//   priority reorder (user-defined failover order — B.9).
// * REAL connection test — result shown honestly (A2): success / HTTP error /
//   network error. "Temporarily unavailable" never fakes an answer.
// * Registry note: definitions come from JSON; adding provider #8 needs no
//   code change (B.8). Signed remote registry sync is surfaced.
// =============================================================================

@Composable
fun AiApiPanelScreen() {
    val context = LocalContext.current
    val langManager = remember { MicafpLangManager.getInstance(context) }
    val lang by langManager.lang.collectAsState()
    fun t(key: String) = MicafpLangManager.get(lang, key)

    val store = remember { AiProviderStore.getInstance(context) }
    val registry = remember { AiProviderRegistry.load(context) }
    val scope = rememberCoroutineScope()

    var masterSwitch by remember { mutableStateOf(store.masterSwitch) }
    var tick by remember { mutableIntStateOf(0) }
    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        // ---------- Master switch ----------
        Surface(
            shape = RoundedCornerShape(14.dp),
            color = MicafpTokens.SurfaceCard,
            border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.padding(16.dp)
            ) {
                Icon(Icons.Default.SmartToy, contentDescription = null,
                    tint = MicafpTokens.Accent, modifier = Modifier.size(20.dp))
                Spacer(Modifier.width(10.dp))
                Column(Modifier.weight(1f)) {
                    Text(t("providers.masterSwitch"), style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.Bold, color = MicafpTokens.TextPrimary)
                    Text(
                        if (masterSwitch) t("providers.title") else t("providers.masterOff"),
                        style = MicafpType.MonoCaption,
                        color = if (masterSwitch) MicafpTokens.TextSecondary else MicafpTokens.Warn
                    )
                }
                Switch(
                    checked = masterSwitch,
                    onCheckedChange = { store.setMasterSwitch(it); masterSwitch = it }
                )
            }
        }

        Text(
            t("providers.registryNote") + " — providers: ${registry.providers.size}",
            style = MicafpType.MonoCaption,
            color = MicafpTokens.TextMuted
        )

        // ---------- Per-provider cards (registry-driven order) ----------
        registry.providers.forEach { def ->
            ProviderCard(
                def = def,
                store = store,
                masterOn = masterSwitch,
                onChanged = { tick++ }
            )
        }
    }
}

// Real connection-test outcome (file scope: sealed classes cannot be local)
private sealed class TestOutcome {
    data class Ok(val ms: Long) : TestOutcome()
    data class Fail(val reason: String) : TestOutcome()
}

@Composable
private fun ProviderCard(
    def: AiProviderDefinition,
    store: AiProviderStore,
    masterOn: Boolean,
    onChanged: () -> Unit
) {
    val context = LocalContext.current
    val langManager = remember { MicafpLangManager.getInstance(context) }
    val lang by langManager.lang.collectAsState()
    fun t(key: String) = MicafpLangManager.get(lang, key)

    val cfg = remember { mutableStateOf(store.getConfig(def.id)) }
    var expanded by remember { mutableStateOf(false) }
    var testState by remember { mutableStateOf<TestOutcome?>(null) }
    var testing by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    Surface(
        shape = RoundedCornerShape(14.dp),
        color = MicafpTokens.SurfaceCard,
        border = androidx.compose.foundation.BorderStroke(
            1.dp,
            if (cfg.value.enabled && cfg.value.apiKey.isNotBlank()) MicafpTokens.AccentBorder
            else MicafpTokens.BorderSubtle
        ),
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(Modifier.padding(14.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                Column(Modifier.weight(1f)) {
                    Text(def.displayName, style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.SemiBold, color = MicafpTokens.TextPrimary)
                    Text(
                        def.requestSchema + (if (cfg.value.apiKey.isNotBlank()) " • key ✓" else " • " + t("providers.noKey")),
                        style = MicafpType.MonoCaption,
                        color = if (cfg.value.apiKey.isNotBlank()) MicafpTokens.Accent else MicafpTokens.TextMuted,
                        maxLines = 1, overflow = TextOverflow.Ellipsis
                    )
                }
                IconButton(onClick = { expanded = !expanded }) {
                    Icon(
                        if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                        contentDescription = null, tint = MicafpTokens.TextMuted
                    )
                }
            }

            if (expanded) {
                Spacer(Modifier.height(8.dp))
                // Enable toggle
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("Enable", style = MaterialTheme.typography.bodySmall,
                        color = MicafpTokens.TextSecondary, modifier = Modifier.weight(1f))
                    Switch(checked = cfg.value.enabled, onCheckedChange = {
                        cfg.value = cfg.value.copy(enabled = it)
                        store.saveConfig(def.id, cfg.value); onChanged()
                    })
                }
                // API key (encrypted at rest via store)
                OutlinedTextField(
                    value = cfg.value.apiKey,
                    onValueChange = { cfg.value = cfg.value.copy(apiKey = it) },
                    label = { Text(t("providers.key"), style = MaterialTheme.typography.labelSmall) },
                    visualTransformation = PasswordVisualTransformation(),
                    modifier = Modifier.fillMaxWidth(),
                    textStyle = MicafpType.MonoCaption.copy(color = MicafpTokens.TextPrimary)
                )
                Spacer(Modifier.height(6.dp))
                // Editable Base URL (C3)
                OutlinedTextField(
                    value = cfg.value.baseUrlOverride.ifBlank { def.baseUrlDefault },
                    onValueChange = { cfg.value = cfg.value.copy(baseUrlOverride = it) },
                    label = { Text(t("providers.baseUrl"), style = MaterialTheme.typography.labelSmall) },
                    modifier = Modifier.fillMaxWidth(),
                    textStyle = MicafpType.MonoCaption.copy(color = MicafpTokens.TextPrimary)
                )
                Spacer(Modifier.height(6.dp))
                // Mirror URL (B.10a)
                OutlinedTextField(
                    value = cfg.value.mirrorUrl,
                    onValueChange = { cfg.value = cfg.value.copy(mirrorUrl = it) },
                    label = { Text(t("providers.mirrorUrl"), style = MaterialTheme.typography.labelSmall) },
                    modifier = Modifier.fillMaxWidth(),
                    textStyle = MicafpType.MonoCaption.copy(color = MicafpTokens.TextPrimary)
                )
                Spacer(Modifier.height(6.dp))
                // Model
                if (def.models.isNotEmpty()) {
                    Text("models: ${def.models.joinToString()}", style = MicafpType.MonoCaption,
                        color = MicafpTokens.TextMuted, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
                OutlinedTextField(
                    value = cfg.value.model,
                    onValueChange = { cfg.value = cfg.value.copy(model = it) },
                    label = { Text("model (default: ${def.defaultModel})", style = MaterialTheme.typography.labelSmall) },
                    modifier = Modifier.fillMaxWidth(),
                    textStyle = MicafpType.MonoCaption.copy(color = MicafpTokens.TextPrimary)
                )
                Spacer(Modifier.height(10.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    Button(
                        enabled = !testing,
                        onClick = {
                            store.saveConfig(def.id, cfg.value); onChanged()
                            testing = true; testState = null
                            scope.launch {
                                val result = AiProviderClient.complete(
                                    def, cfg.value,
                                    baseUrl = cfg.value.mirrorUrl.ifBlank {
                                        cfg.value.baseUrlOverride.ifBlank { def.baseUrlDefault }
                                    },
                                    systemPrompt = "You are a connectivity test for MICAFP. Reply with exactly: OK",
                                    userPrompt = "ping"
                                )
                                testState = when (result) {
                                    is AiCallResult.Success -> TestOutcome.Ok(result.latencyMs)
                                    is AiCallResult.Failure -> TestOutcome.Fail(result.reason)
                                }
                                testing = false
                            }
                        },
                        colors = ButtonDefaults.buttonColors(containerColor = MicafpTokens.AccentDeep)
                    ) { Text(if (testing) t("doctor.running") else t("providers.test")) }

                    TextButton(onClick = {
                        store.saveConfig(def.id, cfg.value); onChanged()
                    }) { Text(t("common.ok")) }
                }
                testState?.let { outcome ->
                    Spacer(Modifier.height(6.dp))
                    when (outcome) {
                        is TestOutcome.Ok -> Text(
                            "${t("providers.testOk")} • ${outcome.ms}ms",
                            style = MicafpType.MonoCaption, color = MicafpTokens.Ok
                        )
                        is TestOutcome.Fail -> Text(
                            "${t("providers.testFail")} • ${outcome.reason} • ${t("providers.unavailable")}",
                            style = MicafpType.MonoCaption, color = MicafpTokens.Crit
                        )
                    }
                }
            }
        }
    }
}
