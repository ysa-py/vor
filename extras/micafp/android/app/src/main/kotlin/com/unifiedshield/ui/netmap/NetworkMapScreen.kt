package com.unifiedshield.ui.netmap

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
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
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.unifiedshield.SplitTunnel
import com.unifiedshield.TunnelManager
import com.unifiedshield.localization.MicafpLangManager
import com.unifiedshield.ui.theme.MicafpTokens
import com.unifiedshield.ui.theme.MicafpType

// =============================================================================
// MICAFP Directive v6 — B3.4 Network Map.
// Device → Obfuscation Layer → Exit Node with a SUBTLE animated flow
// (no neon, no particles — calm dashes, enterprise register).
// Split Tunneling panel is a NEW VISUAL LAYER over the EXISTING SplitTunnel
// backend — routing logic is NOT reimplemented (Directive B3.4/A1).
// =============================================================================

@Composable
fun NetworkMapScreen() {
    val context = LocalContext.current
    val langManager = remember { MicafpLangManager.getInstance(context) }
    val lang by langManager.lang.collectAsState()
    fun t(key: String) = MicafpLangManager.get(lang, key)

    val tm = remember { TunnelManager.getInstance(context) }
    val stats by tm.stats.collectAsState()

    val splitTunnel = remember { SplitTunnel(context) }
    // Real backend data: existing IRANIAN_IP_RANGES list (read-only view)
    val ranges = remember { splitTunnel.getIranianRanges().ifEmpty { SplitTunnel.IRANIAN_IP_RANGES } }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        // ---------- Node-to-node flow ----------
        Surface(
            shape = RoundedCornerShape(14.dp),
            color = MicafpTokens.SurfaceCard,
            border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(Modifier.padding(16.dp)) {
                Text(t("map.title"), style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold, color = MicafpTokens.TextPrimary)
                Spacer(Modifier.height(14.dp))

                val flowPhase by rememberInfiniteTransition(label = "flow")
                    .animateFloat(
                        initialValue = 0f, targetValue = 1f,
                        animationSpec = infiniteRepeatable(
                            tween(2400, easing = LinearEasing), RepeatMode.Restart
                        ), label = "flow_phase"
                    )

                Canvas(modifier = Modifier.fillMaxWidth().height(150.dp)) {
                    val nodeY = size.height * 0.35f
                    val xs = listOf(
                        size.width * 0.14f,
                        size.width * 0.5f,
                        size.width * 0.86f
                    )
                    val radius = 12.dp.toPx()

                    // Connective curve (calm, thin)
                    val path = Path().apply {
                        moveTo(xs[0], nodeY)
                        cubicTo(xs[1], nodeY - 46f, xs[1], nodeY + 46f, xs[2], nodeY)
                    }
                    drawPath(
                        path, color = MicafpTokens.BorderStrong,
                        style = androidx.compose.ui.graphics.drawscope.Stroke(
                            width = 2.dp.toPx(), cap = StrokeCap.Round,
                            pathEffect = PathEffect.dashPathEffect(floatArrayOf(14f, 10f), phase = flowPhase * 24f)
                        )
                    )

                    // Flow dot travelling the path (single, subtle)
                    // Cubic Bezier position by parameter t — no android.graphics dependency
                    val tParam = flowPhase.coerceIn(0f, 1f)
                    val mt = 1f - tParam
                    val bez = { a: Float, b: Float, c: Float, d: Float ->
                        mt * mt * mt * a + 3f * mt * mt * tParam * b + 3f * mt * tParam * tParam * c + tParam * tParam * tParam * d
                    }
                    val px = bez(xs[0], xs[1], xs[1], xs[2])
                    val py = bez(nodeY, nodeY - 46f, nodeY + 46f, nodeY)
                    drawCircle(color = MicafpTokens.Accent, radius = 4.dp.toPx(), center = Offset(px, py))

                    // Nodes
                    xs.forEachIndexed { i, x ->
                        drawCircle(
                            color = if (stats.connected || i == 0) MicafpTokens.Accent else MicafpTokens.BorderStrong,
                            radius = radius, center = Offset(x, nodeY),
                            style = androidx.compose.ui.graphics.drawscope.Stroke(width = 3.dp.toPx())
                        )
                    }
                }

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    MapNodeLabel(t("map.device"), stats.activeIsp, Modifier.weight(1f))
                    MapNodeLabel(t("map.obfuscation"), stats.currentCore, Modifier.weight(1f))
                    MapNodeLabel(t("map.exit"), if (stats.connected) "established" else "—", Modifier.weight(1f))
                }
            }
        }

        // ---------- Split Tunneling panel (visual layer over existing backend) ----------
        Surface(
            shape = RoundedCornerShape(14.dp),
            color = MicafpTokens.SurfaceCard,
            border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(Modifier.padding(16.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Default.CallSplit, contentDescription = null,
                        tint = MicafpTokens.Accent, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text(t("map.splitTunnel"), style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold, color = MicafpTokens.TextPrimary)
                }
                Spacer(Modifier.height(8.dp))
                Text(t("map.splitTunnelDesc"), style = MaterialTheme.typography.bodySmall,
                    color = MicafpTokens.TextSecondary)
                Spacer(Modifier.height(12.dp))

                Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    Surface(
                        shape = RoundedCornerShape(10.dp),
                        color = MicafpTokens.SurfaceElevated,
                        border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.Info),
                        modifier = Modifier.weight(1f)
                    ) {
                        Column(Modifier.padding(12.dp)) {
                            Text(t("map.domestic"), style = MicafpType.SectionLabel, color = MicafpTokens.Info)
                            Text("DIRECT", style = MicafpType.MonoValue, color = MicafpTokens.TextPrimary)
                            Text("${ranges.size} CIDR ranges", style = MicafpType.MonoCaption,
                                color = MicafpTokens.TextMuted)
                        }
                    }
                    Surface(
                        shape = RoundedCornerShape(10.dp),
                        color = MicafpTokens.SurfaceElevated,
                        border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.Accent),
                        modifier = Modifier.weight(1f)
                    ) {
                        Column(Modifier.padding(12.dp)) {
                            Text(t("map.international"), style = MicafpType.SectionLabel, color = MicafpTokens.Accent)
                            Text("TUNNEL", style = MicafpType.MonoValue, color = MicafpTokens.TextPrimary)
                            Text(
                                if (stats.connected) "active · ${stats.currentCore}" else "idle",
                                style = MicafpType.MonoCaption, color = MicafpTokens.TextMuted
                            )
                        }
                    }
                }

                Spacer(Modifier.height(12.dp))
                Text("Loaded ranges (first 6):", style = MicafpType.MonoCaption, color = MicafpTokens.TextMuted)
                ranges.take(6).forEach { r ->
                    Text(r, style = MicafpType.MonoCaption, color = MicafpTokens.TextSecondary)
                }
                if (ranges.size > 6) {
                    Text("+${ranges.size - 6} more…", style = MicafpType.MonoCaption, color = MicafpTokens.TextMuted)
                }
            }
        }
    }
}

@Composable
private fun MapNodeLabel(title: String, value: String, modifier: Modifier = Modifier) {
    Column(modifier = modifier, horizontalAlignment = Alignment.CenterHorizontally) {
        Text(title, style = MaterialTheme.typography.labelSmall, color = MicafpTokens.TextMuted,
            textAlign = TextAlign.Center)
        Text(value, style = MicafpType.MonoCaption, color = MicafpTokens.TextPrimary,
            textAlign = TextAlign.Center, maxLines = 1)
    }
}
