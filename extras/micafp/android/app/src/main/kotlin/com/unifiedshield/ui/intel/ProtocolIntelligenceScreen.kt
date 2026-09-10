package com.unifiedshield.ui.intel

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.TunnelManager
import com.unifiedshield.autopilot.AutoPilotEngine
import com.unifiedshield.license.AuditLog
import com.unifiedshield.localization.MicafpLangManager
import com.unifiedshield.tunnel.TunnelType
import com.unifiedshield.ui.theme.MicafpTokens
import com.unifiedshield.ui.theme.MicafpType
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

// =============================================================================
// MICAFP Directive v6 — B3.2 Protocol Intelligence.
// "Why this protocol?" bound to REAL Auto-Pilot reasoning (audit log),
// 24h timeline of REAL switch events, manual override with non-blocking
// warning, and the FULL protocol list surfaced (none hidden — A1).
// =============================================================================

@Composable
fun ProtocolIntelligenceScreen() {
    val context = LocalContext.current
    val langManager = remember { MicafpLangManager.getInstance(context) }
    val lang by langManager.lang.collectAsState()
    fun t(key: String) = MicafpLangManager.get(lang, key)

    val tm = remember { TunnelManager.getInstance(context) }
    val stats by tm.stats.collectAsState()
    val autoPilot = remember { AutoPilotEngine.getInstance(context) }
    val apState by autoPilot.state.collectAsState()
    val audit = remember { AuditLog.getInstance(context) }
    var auditTick by remember { mutableIntStateOf(0) }
    LaunchedEffect(Unit) { autoPilot.refreshRotationsCount() }

    // Re-read audit log on tick
    val switchEvents = remember(auditTick) { audit.last24h("PROTOCOL_SWITCH").reversed() }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        // ---------- Why this protocol? ----------
        Surface(
            shape = RoundedCornerShape(14.dp),
            color = MicafpTokens.SurfaceCard,
            border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(Modifier.padding(16.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Default.Psychology, contentDescription = null,
                        tint = MicafpTokens.Accent, modifier = Modifier.size(20.dp))
                    Spacer(Modifier.width(8.dp))
                    Text(t("intel.why"), style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold, color = MicafpTokens.TextPrimary)
                }
                Spacer(Modifier.height(10.dp))
                val reason = apState.lastReason.ifBlank { t("intel.reasonPlaceholder") }
                Text(
                    reason,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MicafpTokens.TextSecondary
                )
                Spacer(Modifier.height(8.dp))
                Text(
                    "${t("hub.chip.protocol")}: ${stats.currentCore}  •  DPI ${"%.2f".format(stats.dpiScore)}  •  loss ${"%.1f".format(stats.packetLossRate)}%",
                    style = MicafpType.MonoCaption, color = MicafpTokens.TextMuted
                )
            }
        }

        // ---------- Auto-Pilot + Manual override ----------
        Surface(
            shape = RoundedCornerShape(14.dp),
            color = MicafpTokens.SurfaceCard,
            border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(Modifier.padding(16.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                    Column(Modifier.weight(1f)) {
                        Text(t("intel.autoPilot"), style = MaterialTheme.typography.titleSmall,
                            color = MicafpTokens.TextPrimary)
                        Text(
                            if (apState.pinnedCore != null) t("intel.pinned")
                            else if (apState.enabled) t("intel.autoPilotOn") else t("intel.autoPilotOff"),
                            style = MicafpType.MonoCaption,
                            color = if (apState.pinnedCore != null) MicafpTokens.Warn else MicafpTokens.TextMuted
                        )
                    }
                    Switch(
                        checked = apState.enabled,
                        onCheckedChange = { autoPilot.setEnabled(it) },
                        enabled = apState.pinnedCore == null
                    )
                }

                Spacer(Modifier.height(10.dp))
                HorizontalDivider(color = MicafpTokens.BorderSubtle, thickness = 1.dp)
                Spacer(Modifier.height(10.dp))

                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Default.AdminPanelSettings, contentDescription = null,
                        tint = MicafpTokens.Warn, modifier = Modifier.size(16.dp))
                    Spacer(Modifier.width(6.dp))
                    Text(t("intel.override"), style = MaterialTheme.typography.titleSmall,
                        color = MicafpTokens.TextPrimary)
                }
                Spacer(Modifier.height(6.dp))
                Text(t("intel.overrideWarning"), style = MaterialTheme.typography.bodySmall,
                    color = MicafpTokens.TextMuted)
                Spacer(Modifier.height(10.dp))
                if (apState.pinnedCore != null) {
                    Button(
                        onClick = { autoPilot.pinCore(null); auditTick++ },
                        colors = ButtonDefaults.buttonColors(containerColor = MicafpTokens.AccentDeep)
                    ) { Text("↺ " + t("intel.autoPilotOn")) }
                    Spacer(Modifier.height(8.dp))
                }
            }
        }

        // ---------- Full protocol surface (ALL entries — none hidden) ----------
        Surface(
            shape = RoundedCornerShape(14.dp),
            color = MicafpTokens.SurfaceCard,
            border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(Modifier.padding(16.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        "${t("intel.override")} — ${TunnelType.values().size} ${t("intel.surfaceAll")}",
                        style = MicafpType.SectionLabel, color = MicafpTokens.TextSecondary
                    )
                }
                Spacer(Modifier.height(10.dp))
                TunnelType.values().forEach { proto ->
                    val selected = apState.pinnedCore == proto.id
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { autoPilot.pinCore(proto.id); auditTick++ }
                            .padding(vertical = 7.dp)
                    ) {
                        Icon(
                            if (selected) Icons.Default.RadioButtonChecked else Icons.Default.RadioButtonUnchecked,
                            contentDescription = null,
                            tint = if (selected) MicafpTokens.Accent else MicafpTokens.TextMuted,
                            modifier = Modifier.size(16.dp)
                        )
                        Spacer(Modifier.width(10.dp))
                        Column(Modifier.weight(1f)) {
                            Text(
                                proto.title,
                                style = MaterialTheme.typography.bodyMedium,
                                fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Normal,
                                color = MicafpTokens.TextPrimary,
                                maxLines = 1, overflow = TextOverflow.Ellipsis
                            )
                            Text(
                                proto.titlePersian,
                                style = MaterialTheme.typography.bodySmall,
                                color = MicafpTokens.TextMuted,
                                maxLines = 1, overflow = TextOverflow.Ellipsis
                            )
                        }
                        if (proto.isDpiResistant) {
                            Surface(
                                shape = RoundedCornerShape(6.dp),
                                color = MicafpTokens.AccentContainer
                            ) {
                                Text(
                                    "DPI✓",
                                    style = MicafpType.MonoCaption,
                                    color = MicafpTokens.AccentText,
                                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                                )
                            }
                        }
                    }
                }
            }
        }

        // ---------- 24h timeline (real audit events) ----------
        Surface(
            shape = RoundedCornerShape(14.dp),
            color = MicafpTokens.SurfaceCard,
            border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(Modifier.padding(16.dp)) {
                Text(t("intel.timeline24h"), style = MicafpType.SectionLabel,
                    color = MicafpTokens.TextSecondary)
                Spacer(Modifier.height(12.dp))
                if (switchEvents.isEmpty()) {
                    Text(t("intel.noEvents"), style = MaterialTheme.typography.bodySmall,
                        color = MicafpTokens.TextMuted)
                } else {
                    val fmt = SimpleDateFormat("HH:mm:ss", Locale.US)
                    switchEvents.take(12).forEach { rec ->
                        TimelineDot()
                        Text(
                            "${fmt.format(Date(rec.tsUtcMillis))}  ${rec.message.take(120)}",
                            style = MicafpType.MonoCaption,
                            color = MicafpTokens.TextSecondary,
                            modifier = Modifier.padding(start = 16.dp)
                        )
                        Spacer(Modifier.height(8.dp))
                    }
                    Text(
                        "rotations(24h) = ${switchEvents.size}",
                        style = MicafpType.MonoCaption, color = MicafpTokens.TextMuted
                    )
                }
            }
        }
    }
}

@Composable
private fun TimelineDot() {
    Box(
        modifier = Modifier
            .padding(start = 4.dp)
            .size(8.dp)
            .background(MicafpTokens.Accent, androidx.compose.foundation.shape.CircleShape)
    )
}
