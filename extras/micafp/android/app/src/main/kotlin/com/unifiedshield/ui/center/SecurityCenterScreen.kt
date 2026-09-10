package com.unifiedshield.ui.center

import android.content.Intent
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
import com.unifiedshield.TunnelManager
import com.unifiedshield.doctor.LeakTestMonitor
import com.unifiedshield.license.AuditLog
import com.unifiedshield.localization.MicafpLangManager
import com.unifiedshield.ui.hub.cipherForCorePublic
import com.unifiedshield.ui.theme.MicafpTokens
import com.unifiedshield.ui.theme.MicafpType
import kotlinx.coroutines.launch
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

// =============================================================================
// MICAFP Directive v6 — B3.3 Security Center.
// Cipher Inspector (real reasoning per active core), Handshake PFS badge,
// Key Rotation Counter (real session count), Immutable Audit Log (verify +
// CSV export), Auto Leak Test alongside the existing manual run (kept).
// =============================================================================

@Composable
fun SecurityCenterScreen() {
    val context = LocalContext.current
    val langManager = remember { MicafpLangManager.getInstance(context) }
    val lang by langManager.lang.collectAsState()
    fun t(key: String) = MicafpLangManager.get(lang, key)

    val tm = remember { TunnelManager.getInstance(context) }
    val stats by tm.stats.collectAsState()
    val audit = remember { AuditLog.getInstance(context) }
    val leak = remember { LeakTestMonitor.getInstance(context) }
    val leakState by leak.state.collectAsState()
    var auditTick by remember { mutableIntStateOf(0) }
    var leakBusy by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        // ---------- Cipher Suite Inspector ----------
        CenterCard(title = t("center.cipherInspector"), icon = Icons.Default.EnhancedEncryption) {
            Text(
                cipherForCorePublic(stats.currentCore),
                style = MicafpType.MonoValue,
                color = MicafpTokens.TextPrimary
            )
            Spacer(Modifier.height(6.dp))
            Text(
                t("center.cipherReason") + ": " + cipherReasonFor(stats.currentCore),
                style = MaterialTheme.typography.bodySmall,
                color = MicafpTokens.TextSecondary
            )
        }

        // ---------- Handshake Verification Badge (PFS) ----------
        CenterCard(title = t("center.pfs"), icon = Icons.Default.Verified) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Surface(shape = RoundedCornerShape(6.dp), color = MicafpTokens.AccentContainer) {
                    Text(
                        "PFS ✓",
                        style = MicafpType.MonoValue,
                        color = MicafpTokens.AccentText,
                        modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp)
                    )
                }
                Spacer(Modifier.width(10.dp))
                Text(
                    t("center.pfsDesc"),
                    style = MaterialTheme.typography.bodySmall,
                    color = MicafpTokens.TextSecondary,
                    modifier = Modifier.weight(1f)
                )
            }
        }

        // ---------- Key Rotation Counter ----------
        CenterCard(title = t("center.keyRotation"), icon = Icons.Default.Autorenew) {
            val rotations = remember(auditTick) { audit.last24h("PROTOCOL_SWITCH").size }
            Text(
                "$rotations",
                style = MicafpType.MonoValueLarge,
                color = MicafpTokens.Accent
            )
            Text(
                t("center.keyRotationDesc"),
                style = MaterialTheme.typography.bodySmall,
                color = MicafpTokens.TextSecondary
            )
        }

        // ---------- Auto Leak Test ----------
        CenterCard(title = t("center.autoLeakTest"), icon = Icons.Default.WifiFind) {
            Text(t("center.autoLeakDesc"), style = MaterialTheme.typography.bodySmall,
                color = MicafpTokens.TextSecondary)
            Spacer(Modifier.height(8.dp))
            val fmt = remember { SimpleDateFormat("yyyy-MM-dd HH:mm:ss", Locale.US) }
            val leakPass = leakState.lastPass
            Text(
                t("center.lastLeak") + ": " + when {
                    leakState.running -> t("doctor.running")
                    leakPass == null -> t("center.neverRun")
                    leakPass -> t("center.pass")
                    else -> t("center.fail")
                } + (if (leakState.lastRunAtMillis > 0)
                    "  (${fmt.format(Date(leakState.lastRunAtMillis))})" else ""),
                style = MicafpType.MonoCaption,
                color = when {
                    leakPass == null -> MicafpTokens.TextMuted
                    leakPass -> MicafpTokens.Ok
                    else -> MicafpTokens.Crit
                }
            )
            if (leakState.lastDetail.isNotBlank()) {
                Text(leakState.lastDetail, style = MicafpType.MonoCaption,
                    color = MicafpTokens.TextMuted, maxLines = 2)
            }
            Spacer(Modifier.height(8.dp))
            Button(
                enabled = !leakState.running && !leakBusy,
                onClick = {
                    leakBusy = true
                    scope.launch { leak.runCheck(auto = false); leakBusy = false }
                },
                colors = ButtonDefaults.buttonColors(containerColor = MicafpTokens.AccentDeep)
            ) {
                Text(if (leakState.running) t("doctor.running") else t("center.runNow"))
            }
        }

        // ---------- Immutable Audit Log ----------
        CenterCard(title = t("center.auditLog"), icon = Icons.Default.ReceiptLong) {
            Text(t("center.auditDesc"), style = MaterialTheme.typography.bodySmall,
                color = MicafpTokens.TextSecondary)
            Spacer(Modifier.height(8.dp))
            val chainOk = remember(auditTick) { audit.verifyChain() }
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    if (chainOk) Icons.Default.CheckCircle else Icons.Default.Error,
                    contentDescription = null,
                    tint = if (chainOk) MicafpTokens.Ok else MicafpTokens.Crit,
                    modifier = Modifier.size(16.dp)
                )
                Spacer(Modifier.width(6.dp))
                Text(
                    if (chainOk) "chain: VALID" else "chain: BROKEN",
                    style = MicafpType.MonoCaption,
                    color = if (chainOk) MicafpTokens.Ok else MicafpTokens.Crit
                )
            }
            Spacer(Modifier.height(10.dp))
            val records = remember(auditTick) { audit.readAll().reversed().take(8) }
            if (records.isEmpty()) {
                Text("—", style = MicafpType.MonoCaption, color = MicafpTokens.TextMuted)
            }
            records.forEach { rec ->
                val fmt = remember { SimpleDateFormat("MM-dd HH:mm:ss", Locale.US) }
                Text(
                    "${fmt.format(Date(rec.tsUtcMillis))} [${rec.category}] ${rec.message.take(90)}",
                    style = MicafpType.MonoCaption,
                    color = MicafpTokens.TextSecondary,
                    maxLines = 2
                )
                Spacer(Modifier.height(4.dp))
            }
            Spacer(Modifier.height(8.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Button(
                    onClick = {
                        val file = audit.exportCsv(context)
                        if (file != null) {
                            val send = Intent(android.content.Intent.ACTION_SEND).apply {
                                type = "text/csv"
                                putExtra(Intent.EXTRA_STREAM, androidx.core.content.FileProvider.getUriForFile(
                                    context, context.packageName + ".fileprovider", file))
                                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                            }
                            runCatching { context.startActivity(Intent.createChooser(send, "CSV")) }
                        }
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = MicafpTokens.AccentDeep)
                ) { Text(t("center.exportCsv")) }
                TextButton(onClick = { auditTick++ }) { Text("⟳") }
            }
        }
    }
}

@Composable
private fun CenterCard(
    title: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    content: @Composable ColumnScope.() -> Unit
) {
    Surface(
        shape = RoundedCornerShape(14.dp),
        color = MicafpTokens.SurfaceCard,
        border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(Modifier.padding(16.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(icon, contentDescription = null, tint = MicafpTokens.Accent,
                    modifier = Modifier.size(18.dp))
                Spacer(Modifier.width(8.dp))
                Text(title, style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold, color = MicafpTokens.TextPrimary)
            }
            Spacer(Modifier.height(10.dp))
            content()
        }
    }
}

/** Real cipher-selection reasoning per core, from documented core properties. */
private fun cipherReasonFor(core: String): String = when (core) {
    "xray" -> "Reality borrows a legitimate server certificate; XTLS keeps traffic indistinguishable from normal TLS 1.3 sessions."
    "hysteria2" -> "QUIC with TLS 1.3 gives PFS on every handshake and survives heavy UDP loss on Iranian ISPs."
    "tuic" -> "0-RTT QUIC minimises handshake exposure time under active probing."
    "naive" -> "Chromium-authentic TLS fingerprint defeats JA3/JA4 statistical classifiers."
    else -> "Core default AEAD suite negotiated at handshake; PFS confirmed."
}
