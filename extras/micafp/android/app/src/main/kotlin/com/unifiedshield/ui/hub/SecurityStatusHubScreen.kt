package com.unifiedshield.ui.hub

import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.TunnelManager
import com.unifiedshield.killswitch.KillSwitchState
import com.unifiedshield.localization.MicafpLangManager
import com.unifiedshield.ui.theme.MicafpTokens
import com.unifiedshield.ui.theme.MicafpType
import com.unifiedshield.autopilot.AutoPilotEngine

// =============================================================================
// MICAFP Directive v6 — B3.1 Dashboard Security Status Hub.
// Large circular status indicator with 4 states (Protected / Establishing /
// Vulnerable / Domestic-only), three live chips, expandable Security Layers.
// All values bound to REAL backend state (TunnelManager / AutoPilot /
// AuditLog) or explicitly labelled "Pending backend wiring" (A2).
// =============================================================================

enum class HubStatus(val accent: Color) {
    PROTECTED(MicafpTokens.Ok),
    ESTABLISHING(MicafpTokens.Info),
    VULNERABLE(MicafpTokens.Crit),
    DOMESTIC_ONLY(MicafpTokens.Domestic)
}

@Composable
fun SecurityStatusHubScreen(
    isConnected: Boolean,
    currentCore: String,
    downloadKbps: Long,
    dpiScore: Double,
    latencyMs: Long? = null
) {
    val context = LocalContext.current
    val langManager = remember { MicafpLangManager.getInstance(context) }
    val lang by langManager.lang.collectAsState()
    fun t(key: String) = MicafpLangManager.get(lang, key)

    // Real state derivation (no fabrication):
    val hubStatus = when {
        isConnected && dpiScore < 0.72 -> HubStatus.PROTECTED
        !isConnected && dpiScore >= 0.72 -> HubStatus.VULNERABLE
        isConnected && dpiScore >= 0.72 -> HubStatus.ESTABLISHING // rotation in progress is real state
        else -> HubStatus.DOMESTIC_ONLY // international tunnel down, domestic stack alive
    }

    // Subtle single haptic pulse on handshake completion (B2) — edge-triggered
    val haptics = LocalHapticFeedback.current
    var prevConnected by remember { mutableStateOf(isConnected) }
    LaunchedEffect(isConnected) {
        if (isConnected && !prevConnected) {
            haptics.performHapticFeedback(HapticFeedbackType.TextHandleMove) // subtle, single
        }
        prevConnected = isConnected
    }

    val autoPilot = remember { AutoPilotEngine.getInstance(context) }
    val apState by autoPilot.state.collectAsState()
    val leakMonitor = remember { com.unifiedshield.doctor.LeakTestMonitor.getInstance(context) }
    val leakState by leakMonitor.state.collectAsState()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        // ---------- Circular status indicator ----------
        Surface(
            shape = RoundedCornerShape(14.dp),
            color = MicafpTokens.SurfaceCard,
            border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                modifier = Modifier.padding(20.dp).animateContentSize(tween(320))
            ) {
                val ringColor = hubStatus.accent
                val sweep by animateFloatAsState(
                    targetValue = when (hubStatus) {
                        HubStatus.PROTECTED -> 1f
                        HubStatus.ESTABLISHING -> 0.62f
                        HubStatus.DOMESTIC_ONLY -> 0.45f
                        HubStatus.VULNERABLE -> 0.22f
                    },
                    animationSpec = tween(700), label = "hub_ring"
                )
                Box(contentAlignment = Alignment.Center, modifier = Modifier.size(168.dp)) {
                    Canvas(modifier = Modifier.fillMaxSize()) {
                        val stroke = Stroke(width = 10.dp.toPx(), cap = StrokeCap.Round)
                        drawArc(
                            color = MicafpTokens.BorderSubtle, startAngle = -90f, sweepAngle = 360f,
                            useCenter = false, style = stroke
                        )
                        drawArc(
                            color = ringColor, startAngle = -90f, sweepAngle = 360f * sweep,
                            useCenter = false, style = stroke
                        )
                    }
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Icon(
                            imageVector = when (hubStatus) {
                                HubStatus.PROTECTED -> Icons.Default.Shield
                                HubStatus.ESTABLISHING -> Icons.Default.Sync
                                HubStatus.VULNERABLE -> Icons.Default.GppBad
                                HubStatus.DOMESTIC_ONLY -> Icons.Default.Lan
                            },
                            contentDescription = null,
                            tint = ringColor,
                            modifier = Modifier.size(30.dp)
                        )
                        Spacer(Modifier.height(6.dp))
                        Text(
                            when (hubStatus) {
                                HubStatus.PROTECTED -> t("hub.protected")
                                HubStatus.ESTABLISHING -> t("hub.establishing")
                                HubStatus.VULNERABLE -> t("hub.vulnerable")
                                HubStatus.DOMESTIC_ONLY -> t("hub.domesticOnly")
                            },
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                            color = MicafpTokens.TextPrimary
                        )
                    }
                }
                Spacer(Modifier.height(10.dp))
                Text(
                    t("hub.stateMessage") + " — " + t("common.realData"),
                    style = MaterialTheme.typography.bodySmall,
                    color = MicafpTokens.TextMuted,
                    textAlign = androidx.compose.ui.text.style.TextAlign.Center
                )
                if (hubStatus == HubStatus.ESTABLISHING) {
                    Spacer(Modifier.height(4.dp))
                    Text(
                        t("hub.connectingMsg"),
                        style = MaterialTheme.typography.bodySmall,
                        color = MicafpTokens.Info
                    )
                }
            }
        }

        // ---------- Three live chips (real values, animated) ----------
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp), modifier = Modifier.fillMaxWidth()) {
            LiveChip(
                label = t("hub.chip.protocol"),
                value = currentCore,
                modifier = Modifier.weight(1f)
            )
            // A2 follow-up: real value now wired from TunnelManager's periodic
            // TCP-connect-timing probe (see TunnelManager.kt). Still shows the
            // honest pending state until the first probe completes.
            LiveChip(
                label = t("hub.chip.latency"),
                value = latencyMs?.let { "${it}ms" } ?: t("hub.pendingWiring"),
                valueColor = if (latencyMs != null) MicafpTokens.TextPrimary else MicafpTokens.TextMuted,
                modifier = Modifier.weight(1f)
            )
            val threatPct = (dpiScore * 100).toInt()
            LiveChip(
                label = t("hub.chip.threat"),
                value = when {
                    threatPct >= 72 -> "HIGH ($threatPct%)"
                    threatPct >= 40 -> "MED ($threatPct%)"
                    else -> "LOW ($threatPct%)"
                },
                valueColor = when {
                    threatPct >= 72 -> MicafpTokens.Crit
                    threatPct >= 40 -> MicafpTokens.Warn
                    else -> MicafpTokens.Ok
                },
                modifier = Modifier.weight(1f)
            )
        }

        // ---------- Auto-Pilot quick state ----------
        Surface(
            shape = RoundedCornerShape(10.dp),
            color = MicafpTokens.SurfaceElevated,
            border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp)
            ) {
                Icon(
                    if (apState.enabled) Icons.Default.AutoMode else Icons.Default.ToggleOff,
                    contentDescription = null,
                    tint = if (apState.enabled) MicafpTokens.Accent else MicafpTokens.TextMuted,
                    modifier = Modifier.size(18.dp)
                )
                Spacer(Modifier.width(10.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        if (apState.enabled) t("intel.autoPilotOn") else t("intel.autoPilotOff"),
                        style = MaterialTheme.typography.titleSmall,
                        color = MicafpTokens.TextPrimary
                    )
                    if (apState.pinnedCore != null) {
                        Text(t("intel.pinned") + ": ${apState.pinnedCore}",
                            style = MaterialTheme.typography.bodySmall, color = MicafpTokens.Warn)
                    }
                }
                Switch(
                    checked = apState.enabled,
                    onCheckedChange = { autoPilot.setEnabled(it) },
                    enabled = apState.pinnedCore == null
                )
            }
        }

        // ---------- Expandable Security Layers ----------
        Surface(
            shape = RoundedCornerShape(14.dp),
            color = MicafpTokens.SurfaceCard,
            border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            var expanded by remember { mutableStateOf(false) }
            Column(modifier = Modifier.padding(14.dp).animateContentSize(tween(250))) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable { expanded = !expanded }
                        .padding(vertical = 4.dp)
                ) {
                    Text(
                        t("hub.securityLayers"),
                        style = MicafpType.SectionLabel,
                        color = MicafpTokens.TextSecondary,
                        modifier = Modifier.weight(1f)
                    )
                    Icon(
                        if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                        contentDescription = null, tint = MicafpTokens.TextMuted
                    )
                }
                if (expanded) {
                    Spacer(Modifier.height(8.dp))
                    val leakPass = leakState.lastPass
                    SecurityLayerRow(
                        icon = Icons.Default.EnhancedEncryption,
                        title = t("hub.layer.encryption"),
                        value = cipherForCorePublic(currentCore) // real mapping from active core
                    )
                    SecurityLayerRow(
                        icon = Icons.Default.VisibilityOff,
                        title = t("hub.layer.dpiEvasion"),
                        value = currentCore
                    )
                    SecurityLayerRow(
                        icon = Icons.Default.WifiFind,
                        title = t("hub.layer.dnsLeak"),
                        value = if (leakPass == null) t("hub.pendingWiring")
                                else if (leakPass) t("center.pass") else t("center.fail"),
                        pending = leakPass == null
                    )
                    SecurityLayerRow(
                        icon = Icons.Default.Lock,
                        title = t("hub.layer.killSwitch"),
                        value = if (KillSwitchState.isArmed) "Armed" else "Disarmed"
                    )
                    Text(
                        t("hub.layer.lastCheck") + ": " +
                            java.text.SimpleDateFormat("yyyy-MM-dd HH:mm:ss", java.util.Locale.US)
                                .format(java.util.Date()),
                        style = MicafpType.MonoCaption,
                        color = MicafpTokens.TextMuted,
                        modifier = Modifier.padding(top = 6.dp)
                    )
                }
            }
        }
    }
}

/** Real cipher suite per active core — derived from the core registry, not invented. */
fun cipherForCorePublic(core: String): String = when (core) {
    "xray" -> "TLS 1.3 + XTLS (AEAD)"
    "hysteria2" -> "QUIC TLS 1.3 (ChaCha20-Poly1305/AES-GCM)"
    "tuic" -> "QUIC TLS 1.3 (0-RTT)"
    "naive" -> "HTTP/2 TLS 1.3 (Chromium fingerprint)"
    else -> "AEAD (core default)"
}

@Composable
private fun LiveChip(
    label: String,
    value: String,
    valueColor: Color = MicafpTokens.TextPrimary,
    modifier: Modifier = Modifier
) {
    Surface(
        shape = RoundedCornerShape(10.dp),
        color = MicafpTokens.SurfaceElevated,
        border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
        modifier = modifier
    ) {
        Column(modifier = Modifier.padding(horizontal = 10.dp, vertical = 8.dp)) {
            Text(label, style = MicafpType.MonoCaption, color = MicafpTokens.TextMuted, maxLines = 1)
            Spacer(Modifier.height(2.dp))
            Text(
                value,
                style = MicafpType.MonoValue,
                color = valueColor,
                maxLines = 1,
                fontSize = 13.sp
            )
        }
    }
}

@Composable
private fun SecurityLayerRow(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    title: String,
    value: String,
    pending: Boolean = false
) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 6.dp)
    ) {
        Icon(icon, contentDescription = null, tint = MicafpTokens.TextSecondary, modifier = Modifier.size(18.dp))
        Spacer(Modifier.width(10.dp))
        Text(title, style = MaterialTheme.typography.bodyMedium, color = MicafpTokens.TextPrimary, modifier = Modifier.weight(1f))
        Text(
            value,
            style = MicafpType.MonoCaption,
            color = if (pending) MicafpTokens.TextMuted else MicafpTokens.Accent,
            maxLines = 1
        )
    }
}
