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
import androidx.compose.ui.unit.dp
import com.unifiedshield.aiproviders.AiAnswerLayer
import com.unifiedshield.aiproviders.AiFailoverChain
import com.unifiedshield.doctor.ConnectionDoctor
import com.unifiedshield.doctor.DoctorReport
import com.unifiedshield.localization.MicafpLangManager
import com.unifiedshield.ui.theme.MicafpTokens
import com.unifiedshield.ui.theme.MicafpType
import kotlinx.coroutines.launch
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

// =============================================================================
// MICAFP Directive v6 — Connection Doctor (advanced feature).
// Real probes → root cause → suggested fix → smart explanation through the
// B.9/B.10 AI failover chain. The answering layer is ALWAYS attributed
// (external provider name or "Local analysis") — never faked (A2/B.9).
// =============================================================================

@Composable
fun ConnectionDoctorScreen() {
    val context = LocalContext.current
    val langManager = remember { MicafpLangManager.getInstance(context) }
    val lang by langManager.lang.collectAsState()
    fun t(key: String) = MicafpLangManager.get(lang, key)

    val doctor = remember { ConnectionDoctor.getInstance(context) }
    val chain = remember { AiFailoverChain.getInstance(context) }
    val scope = rememberCoroutineScope()

    var running by remember { mutableStateOf(false) }
    var report by remember { mutableStateOf<DoctorReport?>(null) }
    var aiExplanation by remember { mutableStateOf<String?>(null) }
    var aiLayer by remember { mutableStateOf<String?>(null) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        Button(
            enabled = !running,
            onClick = {
                running = true; report = null; aiExplanation = null; aiLayer = null
                scope.launch {
                    val rep = doctor.runFullCheck()
                    report = rep
                    val answer = chain.ask(
                        systemPrompt = "You are MICAFP's connectivity advisor. Explain briefly in the user's language why the probe result happened and what to do. Be factual.",
                        userPrompt = "Probes: ${rep.checks.joinToString { "${it.id}=${if (it.passed) "pass" else "FAIL"}(${it.detail})" }}. Root cause: ${rep.rootCause}"
                    )
                    aiExplanation = answer.text
                    aiLayer = when (answer.layer) {
                        AiAnswerLayer.EXTERNAL_PROVIDER -> "${t("providers.layerExternal")}: ${answer.providerName}"
                        AiAnswerLayer.INTERNAL_LOCAL_ANALYSIS -> t("providers.layerInternal")
                    }
                    running = false
                }
            },
            colors = ButtonDefaults.buttonColors(containerColor = MicafpTokens.AccentDeep),
            modifier = Modifier.fillMaxWidth()
        ) {
            Text(if (running) t("doctor.running") else t("doctor.run"))
        }

        report?.let { rep ->
            // Checks
            Surface(
                shape = RoundedCornerShape(14.dp),
                color = MicafpTokens.SurfaceCard,
                border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(Modifier.padding(16.dp)) {
                    rep.checks.forEach { check ->
                        Row(verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier.padding(vertical = 5.dp)) {
                            Icon(
                                if (check.passed) Icons.Default.CheckCircle else Icons.Default.Cancel,
                                contentDescription = null,
                                tint = if (check.passed) MicafpTokens.Ok else MicafpTokens.Crit,
                                modifier = Modifier.size(16.dp)
                            )
                            Spacer(Modifier.width(8.dp))
                            Column(Modifier.weight(1f)) {
                                Text(
                                    when (check.id) {
                                        "dns" -> t("doctor.check.dns")
                                        "tcp443" -> t("doctor.check.tcp443")
                                        "udp53" -> t("doctor.check.udp53")
                                        else -> t("doctor.check.tunnel")
                                    },
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MicafpTokens.TextPrimary
                                )
                                Text(check.detail, style = MicafpType.MonoCaption,
                                    color = MicafpTokens.TextMuted, maxLines = 2)
                            }
                            if (check.latencyMs > 0) {
                                Text("${check.latencyMs}ms", style = MicafpType.MonoCaption,
                                    color = MicafpTokens.TextSecondary)
                            }
                        }
                    }
                }
            }

            // Root cause + fix
            Surface(
                shape = RoundedCornerShape(14.dp),
                color = MicafpTokens.SurfaceCard,
                border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(Modifier.padding(16.dp)) {
                    Text(t("doctor.rootCause"), style = MicafpType.SectionLabel, color = MicafpTokens.Warn)
                    Text(rep.rootCause, style = MaterialTheme.typography.bodyMedium,
                        color = MicafpTokens.TextPrimary)
                    Spacer(Modifier.height(10.dp))
                    Text(t("doctor.suggestedFix"), style = MicafpType.SectionLabel, color = MicafpTokens.Accent)
                    Text(rep.suggestedFix, style = MaterialTheme.typography.bodyMedium,
                        color = MicafpTokens.TextSecondary)
                    val fmt = remember { SimpleDateFormat("yyyy-MM-dd HH:mm:ss", Locale.US) }
                    Spacer(Modifier.height(6.dp))
                    Text(fmt.format(Date(rep.completedAtMillis)), style = MicafpType.MonoCaption,
                        color = MicafpTokens.TextMuted)
                }
            }

            // AI explanation with attribution
            val explanation = aiExplanation
            if (explanation != null) {
                Surface(
                    shape = RoundedCornerShape(14.dp),
                    color = MicafpTokens.SurfaceCard,
                    border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.AccentBorder),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(Modifier.padding(16.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text(t("doctor.aiExplain"), style = MicafpType.SectionLabel,
                                color = MicafpTokens.Accent)
                            Spacer(Modifier.weight(1f))
                            Surface(shape = RoundedCornerShape(6.dp), color = MicafpTokens.AccentContainer) {
                                Text(
                                    "${t("providers.answeredBy")}: $aiLayer",
                                    style = MicafpType.MonoCaption,
                                    color = MicafpTokens.AccentText,
                                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp)
                                )
                            }
                        }
                        Spacer(Modifier.height(8.dp))
                        Text(explanation, style = MaterialTheme.typography.bodyMedium,
                            color = MicafpTokens.TextPrimary)
                    }
                }
            }
        }
    }
}
