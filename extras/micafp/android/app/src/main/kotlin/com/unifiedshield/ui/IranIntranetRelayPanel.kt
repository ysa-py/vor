package com.unifiedshield.ui

import androidx.compose.animation.*
import androidx.compose.animation.core.*
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.WarTimeResilienceEngine
import com.unifiedshield.MeshPeerNode
import kotlin.math.cos
import kotlin.math.sin

@Composable
fun IranIntranetRelayPanel(
    modifier: Modifier = Modifier
) {
    val warEngine = remember { WarTimeResilienceEngine.getInstance() }
    val warState by warEngine.warState.collectAsState()
    var selectedNode by remember { mutableStateOf<MeshPeerNode?>(null) }

    val infiniteTransition = rememberInfiniteTransition(label = "radarPulse")
    val radarSweepAngle by infiniteTransition.animateFloat(
        initialValue = 0f,
        targetValue = 360f,
        animationSpec = infiniteRepeatable(
            animation = tween(4000, easing = LinearEasing),
            repeatMode = RepeatMode.Restart
        ),
        label = "radarSweep"
    )
    val radarPulseRadius by infiniteTransition.animateFloat(
        initialValue = 0.2f,
        targetValue = 1.0f,
        animationSpec = infiniteRepeatable(
            animation = tween(2000, easing = LinearEasing),
            repeatMode = RepeatMode.Restart
        ),
        label = "radarPulseRadius"
    )

    Card(
        modifier = modifier
            .fillMaxWidth()
            .testTag("iran_intranet_relay_panel"),
        shape = RoundedCornerShape(18.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            // Header
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(
                        modifier = Modifier
                            .size(40.dp)
                            .clip(CircleShape)
                            .background(Color(0x2210B981)),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            imageVector = Icons.Default.CellTower,
                            contentDescription = "Off-grid Relay",
                            tint = Color(0xFF10B981),
                            modifier = Modifier.size(24.dp)
                        )
                    }
                    Spacer(modifier = Modifier.width(10.dp))
                    Column {
                        Text(
                            text = "Iran Intranet & Mesh Relay Diagnostics",
                            fontSize = 15.sp,
                            fontWeight = FontWeight.Bold
                        )
                        Text(
                            text = "پایشگر زنده گره‌های مش BLE و Wi-Fi Aware برای بقای آفلاین",
                            fontSize = 11.sp,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }

                Badge(
                    containerColor = if (warState.isEmergencyModeActive) Color(0xFFEF4444) else Color(0x2210B981),
                    contentColor = if (warState.isEmergencyModeActive) Color.White else Color(0xFF10B981)
                ) {
                    Text("${warState.activePeerNodes.size} گره کشف‌شده")
                }
            }

            Spacer(modifier = Modifier.height(14.dp))

            // Radar Chart / Circle Packing Mesh Visualizer
            Surface(
                color = Color(0xFF0F172A),
                shape = RoundedCornerShape(16.dp),
                modifier = Modifier
                    .fillMaxWidth()
                    .height(210.dp)
                    .border(1.dp, Color(0xFF1E293B), RoundedCornerShape(16.dp))
            ) {
                Box(modifier = Modifier.fillMaxSize()) {
                    Canvas(modifier = Modifier.fillMaxSize().padding(12.dp)) {
                        val center = Offset(size.width / 2f, size.height / 2f)
                        val maxRadius = minOf(size.width, size.height) / 2.2f

                        // Draw concentric radar range rings
                        val ringDistances = listOf(0.33f, 0.66f, 1.0f)
                        ringDistances.forEach { fraction ->
                            drawCircle(
                                color = Color(0xFF10B981).copy(alpha = 0.18f),
                                radius = maxRadius * fraction,
                                center = center,
                                style = Stroke(width = 1.2f)
                            )
                        }

                        // Expanding radar pulse wave
                        drawCircle(
                            color = Color(0xFF10B981).copy(alpha = (1.0f - radarPulseRadius) * 0.4f),
                            radius = maxRadius * radarPulseRadius,
                            center = center,
                            style = Stroke(width = 2.dp.toPx())
                        )

                        // Crosshairs
                        drawLine(
                            color = Color(0xFF334155),
                            start = Offset(center.x - maxRadius, center.y),
                            end = Offset(center.x + maxRadius, center.y),
                            strokeWidth = 1f
                        )
                        drawLine(
                            color = Color(0xFF334155),
                            start = Offset(center.x, center.y - maxRadius),
                            end = Offset(center.x, center.y + maxRadius),
                            strokeWidth = 1f
                        )

                        // Radar sweep line
                        val sweepRad = Math.toRadians(radarSweepAngle.toDouble())
                        val sweepEnd = Offset(
                            (center.x + maxRadius * cos(sweepRad)).toFloat(),
                            (center.y + maxRadius * sin(sweepRad)).toFloat()
                        )
                        drawLine(
                            color = Color(0xFF10B981).copy(alpha = 0.6f),
                            start = center,
                            end = sweepEnd,
                            strokeWidth = 1.8f
                        )

                        // Center Local Device Pin
                        drawCircle(
                            color = Color(0xFF3B82F6),
                            radius = 6.dp.toPx(),
                            center = center
                        )
                        drawCircle(
                            color = Color.White,
                            radius = 2.5.dp.toPx(),
                            center = center
                        )

                        // Map nearby discovered mesh nodes (Circle-Packing / Polar Coordinates)
                        val nodes = warState.activePeerNodes
                        val stepAngle = 360f / maxOf(1, nodes.size)

                        nodes.forEachIndexed { idx, node ->
                            val angleRad = Math.toRadians((idx * stepAngle + 35.0))
                            // Distance normalized to radar ring
                            val normDist = (node.distanceMeters.coerceIn(5, 500) / 500f).coerceIn(0.25f, 0.92f)
                            val nodeX = (center.x + maxRadius * normDist * cos(angleRad)).toFloat()
                            val nodeY = (center.y + maxRadius * normDist * sin(angleRad)).toFloat()
                            val nodePos = Offset(nodeX, nodeY)

                            val signalColor = when {
                                node.signalStrengthDbm >= -55 -> Color(0xFF10B981)
                                node.signalStrengthDbm >= -70 -> Color(0xFF3B82F6)
                                node.signalStrengthDbm >= -85 -> Color(0xFFF59E0B)
                                else -> Color(0xFFEF4444)
                            }

                            // Dynamic speed aura radius
                            val speedRadius = when {
                                node.bandwidthRating.contains("42") || node.bandwidthRating.contains("35") -> 9.dp.toPx()
                                node.bandwidthRating.contains("18") -> 7.dp.toPx()
                                else -> 5.5.dp.toPx()
                            }

                            // Halo ring
                            drawCircle(
                                color = signalColor.copy(alpha = 0.35f),
                                radius = speedRadius * 1.6f,
                                center = nodePos
                            )
                            // Solid core
                            drawCircle(
                                color = signalColor,
                                radius = speedRadius,
                                center = nodePos
                            )
                            // Egress indicator dot
                            if (node.isEgressCapable) {
                                drawCircle(
                                    color = Color.White,
                                    radius = 2.dp.toPx(),
                                    center = nodePos
                                )
                            }
                        }
                    }

                    // Legend overlay inside radar
                    Row(
                        modifier = Modifier
                            .align(Alignment.BottomCenter)
                            .padding(bottom = 6.dp),
                        horizontalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        LegendItem("عالی (> -55 dBm)", Color(0xFF10B981))
                        LegendItem("خوب (-70 dBm)", Color(0xFF3B82F6))
                        LegendItem("متوسط (-85 dBm)", Color(0xFFF59E0B))
                    }
                }
            }

            Spacer(modifier = Modifier.height(12.dp))

            // Transport Spectrum Chips
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "Direct Radio Transports",
                    fontSize = 12.sp,
                    fontWeight = FontWeight.SemiBold
                )
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    SignalBandChip("BLE 5.2", warState.isBleMeshActive)
                    SignalBandChip("Wi-Fi Aware", warState.isWifiAwareActive)
                    SignalBandChip("Ultrasonic", warState.isNtpCovertActive)
                }
            }

            Spacer(modifier = Modifier.height(10.dp))

            Text(
                text = "Nearby Discovered Nodes (گره‌های در دسترس در شعاع ۱۰۰ متری)",
                fontSize = 13.sp,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.padding(vertical = 4.dp)
            )

            // Nodes List
            warState.activePeerNodes.forEach { node ->
                val isSelected = selectedNode?.id == node.id
                val signalQuality = getSignalQuality(node.signalStrengthDbm)

                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .border(
                            width = if (isSelected) 1.5.dp else 0.dp,
                            color = if (isSelected) Color(0xFF10B981) else Color.Transparent,
                            shape = RoundedCornerShape(12.dp)
                        )
                        .clickable { selectedNode = if (isSelected) null else node }
                        .testTag("mesh_node_${node.id}"),
                    colors = CardDefaults.cardColors(
                        containerColor = if (isSelected) Color(0x1510B981) else MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.35f)
                    ),
                    shape = RoundedCornerShape(12.dp)
                ) {
                    Column(modifier = Modifier.padding(12.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Box(
                                    modifier = Modifier
                                        .size(10.dp)
                                        .clip(CircleShape)
                                        .background(signalQuality.color)
                                )
                                Spacer(modifier = Modifier.width(8.dp))
                                Text(node.alias, fontSize = 13.sp, fontWeight = FontWeight.Bold)
                            }

                            Row(verticalAlignment = Alignment.CenterVertically) {
                                SignalBars(signalDbm = node.signalStrengthDbm)
                                Spacer(modifier = Modifier.width(6.dp))
                                Text(
                                    text = "${node.signalStrengthDbm} dBm",
                                    fontSize = 11.sp,
                                    fontWeight = FontWeight.SemiBold,
                                    color = signalQuality.color
                                )
                            }
                        }

                        Spacer(modifier = Modifier.height(6.dp))

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(
                                text = "فاصله: ~${node.distanceMeters} متر • پروتکل: ${node.transportType}",
                                fontSize = 11.sp,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )

                            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                                Badge(containerColor = Color(0x223B82F6), contentColor = Color(0xFF3B82F6)) {
                                    Text(node.bandwidthRating)
                                }
                                if (node.isEgressCapable) {
                                    Badge(containerColor = Color(0x2210B981), contentColor = Color(0xFF10B981)) {
                                        Text("خروجی اینترنت")
                                    }
                                } else {
                                    Badge(containerColor = MaterialTheme.colorScheme.surfaceVariant) {
                                        Text("رله محلی")
                                    }
                                }
                            }
                        }

                        // Detailed expanded telemetry
                        if (isSelected) {
                            Spacer(modifier = Modifier.height(10.dp))
                            Divider(color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f))
                            Spacer(modifier = Modifier.height(8.dp))

                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween
                            ) {
                                Column {
                                    Text("کیفیت سیگنال", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                    Text(signalQuality.label, fontSize = 12.sp, fontWeight = FontWeight.Bold, color = signalQuality.color)
                                }
                                Column {
                                    Text("سرعت اتصال و پهنای باند", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                    Text(node.bandwidthRating, fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFF10B981))
                                }
                                Column {
                                    Text("امنیت اتصال", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                    Text("Noise IK / AES-GCM", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFF6366F1))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun LegendItem(label: String, color: Color) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Box(
            modifier = Modifier
                .size(6.dp)
                .clip(CircleShape)
                .background(color)
        )
        Spacer(modifier = Modifier.width(4.dp))
        Text(label, color = Color.White.copy(alpha = 0.8f), fontSize = 9.sp)
    }
}

@Composable
private fun SignalBandChip(name: String, active: Boolean) {
    Surface(
        color = if (active) Color(0x2210B981) else MaterialTheme.colorScheme.surfaceVariant,
        shape = RoundedCornerShape(6.dp)
    ) {
        Text(
            text = "$name: ${if (active) "فعال" else "آماده"}",
            fontSize = 10.sp,
            fontWeight = FontWeight.SemiBold,
            color = if (active) Color(0xFF10B981) else MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
        )
    }
}

@Composable
private fun SignalBars(signalDbm: Int) {
    val barCount = when {
        signalDbm >= -55 -> 4
        signalDbm >= -70 -> 3
        signalDbm >= -85 -> 2
        else -> 1
    }

    Row(
        horizontalArrangement = Arrangement.spacedBy(2.dp),
        verticalAlignment = Alignment.Bottom,
        modifier = Modifier.height(14.dp)
    ) {
        for (i in 1..4) {
            Box(
                modifier = Modifier
                    .width(3.dp)
                    .height((i * 3.5).dp)
                    .clip(RoundedCornerShape(1.dp))
                    .background(if (i <= barCount) Color(0xFF10B981) else Color.Gray.copy(alpha = 0.3f))
            )
        }
    }
}

private data class SignalQuality(val label: String, val color: Color)

private fun getSignalQuality(dbm: Int): SignalQuality {
    return when {
        dbm >= -55 -> SignalQuality("عالی (Excellent)", Color(0xFF10B981))
        dbm >= -70 -> SignalQuality("خوب (Good)", Color(0xFF3B82F6))
        dbm >= -85 -> SignalQuality("متوسط (Fair)", Color(0xFFF59E0B))
        else -> SignalQuality("ضعیف (Weak)", Color(0xFFEF4444))
    }
}
