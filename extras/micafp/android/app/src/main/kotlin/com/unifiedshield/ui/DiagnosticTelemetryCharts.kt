package com.unifiedshield.ui

import androidx.compose.animation.*
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
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
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.aiorchestrator.AdaptiveNetworkProfiler
import com.unifiedshield.aiorchestrator.AiCoreOrchestrator
import com.unifiedshield.aiorchestrator.DpiTfLiteAnomalyDetector
import com.unifiedshield.aiorchestrator.TelemetryDataPoint

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DiagnosticTelemetrySection() {
    val orchestrator = remember { AiCoreOrchestrator.getInstance() }
    val profiler = remember { AdaptiveNetworkProfiler.getInstance() }
    val anomalyDetector = remember { DpiTfLiteAnomalyDetector.getInstance() }

    val coresPool by orchestrator.coresPool.collectAsState()
    val activeCoreId by orchestrator.activeCoreId.collectAsState()
    val orchestratorLogs by orchestrator.orchestratorLogs.collectAsState()
    val telemetryHistory by anomalyDetector.telemetryHistory.collectAsState()
    val censorshipPressure by anomalyDetector.censorshipPressureIndex.collectAsState()
    val dpiBlocksCount by anomalyDetector.dpiBlocksDetectedCount.collectAsState()
    val anomalyEvents by anomalyDetector.anomalyEvents.collectAsState()
    val profilerStats by profiler.profilerStats.collectAsState()
    val isProfiling by profiler.isProfilingActive.collectAsState()

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .testTag("diagnostic_telemetry_section"),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        // Censorship Pressure & Threat Index Gauge Card
        Card(
            colors = CardDefaults.cardColors(
                containerColor = when {
                    censorshipPressure > 65f -> Color(0x22EF4444)
                    censorshipPressure > 35f -> Color(0x22F59E0B)
                    else -> Color(0x2210B981)
                }
            ),
            border = androidx.compose.foundation.BorderStroke(
                1.dp,
                when {
                    censorshipPressure > 65f -> Color(0xFFEF4444)
                    censorshipPressure > 35f -> Color(0xFFF59E0B)
                    else -> Color(0xFF10B981)
                }
            ),
            shape = RoundedCornerShape(18.dp),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            imageVector = Icons.Default.Security,
                            contentDescription = null,
                            tint = when {
                                censorshipPressure > 65f -> Color(0xFFEF4444)
                                censorshipPressure > 35f -> Color(0xFFF59E0B)
                                else -> Color(0xFF10B981)
                            },
                            modifier = Modifier.size(24.dp)
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Column {
                            Text("شاخص فشار سانسور (Censorship Pressure Index)", fontSize = 14.sp, fontWeight = FontWeight.Bold)
                            Text("همبستگی بلادرنگ با حملات DPI و قطعی‌های بسته‌ای", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                    Badge(
                        containerColor = when {
                            censorshipPressure > 65f -> Color(0xFFEF4444)
                            censorshipPressure > 35f -> Color(0xFFF59E0B)
                            else -> Color(0xFF10B981)
                        },
                        contentColor = Color.White
                    ) {
                        Text(
                            when {
                                censorshipPressure > 65f -> "بسیار بالا (حمله DPI)"
                                censorshipPressure > 35f -> "متوسط (پایش)"
                                else -> "عادی و امن"
                            }
                        )
                    }
                }

                Spacer(modifier = Modifier.height(14.dp))

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    // Gauge Metric 1
                    Surface(
                        shape = RoundedCornerShape(12.dp),
                        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.6f),
                        modifier = Modifier.weight(1f)
                    ) {
                        Column(modifier = Modifier.padding(10.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                            Text("شاخص فشار", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text(
                                "$censorshipPressure %",
                                fontSize = 18.sp,
                                fontWeight = FontWeight.Bold,
                                color = when {
                                    censorshipPressure > 65f -> Color(0xFFEF4444)
                                    censorshipPressure > 35f -> Color(0xFFF59E0B)
                                    else -> Color(0xFF10B981)
                                }
                            )
                        }
                    }

                    // Gauge Metric 2
                    Surface(
                        shape = RoundedCornerShape(12.dp),
                        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.6f),
                        modifier = Modifier.weight(1f)
                    ) {
                        Column(modifier = Modifier.padding(10.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                            Text("بسته‌های DPI مسدود", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text("$dpiBlocksCount", fontSize = 18.sp, fontWeight = FontWeight.Bold, color = Color(0xFF3B82F6))
                        }
                    }

                    // Gauge Metric 3
                    Surface(
                        shape = RoundedCornerShape(12.dp),
                        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.6f),
                        modifier = Modifier.weight(1f)
                    ) {
                        Column(modifier = Modifier.padding(10.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                            Text("استراتژی فعال", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text("Auto-Evade", fontSize = 14.sp, fontWeight = FontWeight.Bold, color = Color(0xFF8B5CF6))
                        }
                    }
                }
            }
        }

        // Live RTT Line Chart
        Card(
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.35f)),
            shape = RoundedCornerShape(18.dp),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(Icons.Default.Timeline, contentDescription = null, tint = Color(0xFF10B981), modifier = Modifier.size(20.dp))
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("نمودار خطی تاخیر رفت و برگشت (RTT Line Chart)", fontSize = 13.sp, fontWeight = FontWeight.Bold)
                    }
                    val latestRtt = telemetryHistory.lastOrNull()?.rttMs ?: 14f
                    Text("${Math.round(latestRtt * 10) / 10.0} ms", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFF10B981))
                }

                Spacer(modifier = Modifier.height(10.dp))

                // Canvas Line Chart
                RttLineChart(data = telemetryHistory, modifier = Modifier.fillMaxWidth().height(120.dp))
            }
        }

        // Packet Loss Scatter Plot Distribution
        Card(
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.35f)),
            shape = RoundedCornerShape(18.dp),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(Icons.Default.ScatterPlot, contentDescription = null, tint = Color(0xFF3B82F6), modifier = Modifier.size(20.dp))
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("توزیع پراکندگی افت بسته (Packet Loss Scatter Plot)", fontSize = 13.sp, fontWeight = FontWeight.Bold)
                    }
                    val avgLoss = telemetryHistory.map { it.packetLossPct }.average().takeIf { !it.isNaN() } ?: 0.2
                    Text("${Math.round(avgLoss * 10) / 10.0}% avg loss", fontSize = 12.sp, color = Color(0xFF3B82F6))
                }

                Spacer(modifier = Modifier.height(10.dp))

                // Canvas Scatter Plot
                PacketLossScatterPlot(data = telemetryHistory, modifier = Modifier.fillMaxWidth().height(110.dp))
            }
        }

        // Adaptive Network Profiler (30s Background Worker)
        Card(
            colors = CardDefaults.cardColors(containerColor = Color(0x158B5CF6)),
            border = androidx.compose.foundation.BorderStroke(1.dp, Color(0xFF8B5CF6)),
            shape = RoundedCornerShape(18.dp),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(Icons.Default.Sync, contentDescription = null, tint = Color(0xFF8B5CF6), modifier = Modifier.size(22.dp))
                        Spacer(modifier = Modifier.width(8.dp))
                        Column {
                            Text("پروفایلر تطبیقی شبکه (Adaptive Network Profiler)", fontSize = 14.sp, fontWeight = FontWeight.Bold)
                            Text("پینگ سیستماتیک هر ۳۰ ثانیه برای شبکه‌های همراه پرجیتر", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                    IconButton(onClick = {
                        if (isProfiling) profiler.stopProfiling() else profiler.startProfilingLoop()
                    }) {
                        Icon(
                            imageVector = if (isProfiling) Icons.Default.PauseCircle else Icons.Default.PlayCircle,
                            contentDescription = "Toggle Profiler",
                            tint = Color(0xFF8B5CF6)
                        )
                    }
                }

                Spacer(modifier = Modifier.height(8.dp))
                Text(profilerStats, fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }

        // Multi-Core Scoring Matrix & 5-Min Blacklist Status
        Card(
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f)),
            shape = RoundedCornerShape(18.dp),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text("ماتریس امتیازدهی و وضعیت بلک‌لیست (Scoring Matrix & Auto-Failover)", fontSize = 13.sp, fontWeight = FontWeight.Bold)
                    Badge(containerColor = Color(0xFF10B981), contentColor = Color.White) {
                        Text("${coresPool.size} هسته")
                    }
                }

                Spacer(modifier = Modifier.height(10.dp))

                coresPool.forEach { core ->
                    val now = System.currentTimeMillis()
                    val isBlacklistedNow = core.isBlacklisted && now < core.blacklistedUntilMs
                    val remainingSeconds = if (isBlacklistedNow) ((core.blacklistedUntilMs - now) / 1000).coerceAtLeast(0) else 0

                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 5.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.weight(1f)) {
                            Box(
                                modifier = Modifier
                                    .size(8.dp)
                                    .clip(CircleShape)
                                    .background(
                                        when {
                                            isBlacklistedNow -> Color(0xFFEF4444)
                                            core.coreId == activeCoreId -> Color(0xFF10B981)
                                            else -> Color(0xFF3B82F6)
                                        }
                                    )
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Column {
                                Row(verticalAlignment = Alignment.CenterVertically) {
                                    Text(core.name, fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
                                    if (core.coreId == activeCoreId) {
                                        Spacer(modifier = Modifier.width(6.dp))
                                        Badge(containerColor = Color(0xFF10B981), contentColor = Color.White) {
                                            Text("فعال", fontSize = 9.sp)
                                        }
                                    }
                                    if (isBlacklistedNow) {
                                        Spacer(modifier = Modifier.width(6.dp))
                                        Badge(containerColor = Color(0xFFEF4444), contentColor = Color.White) {
                                            Text("بلک‌لیست (${remainingSeconds}s)", fontSize = 9.sp)
                                        }
                                    }
                                }
                                Text(
                                    "${core.protocolType} • تاخیر: ${core.latencyMs}ms • افت بسته: ${core.packetLossPct}% • هندشیک: ${(core.handshakeSuccessRate * 100).toInt()}%",
                                    fontSize = 10.sp,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                        }

                        Column(horizontalAlignment = Alignment.End) {
                            Text("${core.score} pts", fontSize = 13.sp, fontWeight = FontWeight.Bold, color = if (isBlacklistedNow) Color(0xFFEF4444) else Color(0xFF10B981))
                            if (core.consecutiveFailures > 0) {
                                Text("خطا: ${core.consecutiveFailures}/3", fontSize = 9.sp, color = Color(0xFFEF4444))
                            }
                        }
                    }
                    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f), thickness = 0.5.dp)
                }
            }
        }

        // TFLite Anomaly Events & Signatures
        Card(
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f)),
            shape = RoundedCornerShape(18.dp),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Default.Bolt, contentDescription = null, tint = Color(0xFFF59E0B), modifier = Modifier.size(20.dp))
                    Spacer(modifier = Modifier.width(8.dp))
                    Text("رویدادهای تشخیص امضای سانسور TFLite (DPI Anomaly Signatures)", fontSize = 13.sp, fontWeight = FontWeight.Bold)
                }

                Spacer(modifier = Modifier.height(10.dp))

                anomalyEvents.take(4).forEach { event ->
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 4.dp)
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(event.signatureName, fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFFF59E0B))
                            Text(event.timestamp, fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        Text(event.interceptedHeader, fontSize = 10.sp, fontFamily = FontFamily.Monospace, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        if (event.triggeredCoreSwitch) {
                            Text("⚡ تغییر خودکار هسته به: ${event.switchedToCore ?: "Active Core"}", fontSize = 10.sp, fontWeight = FontWeight.SemiBold, color = Color(0xFF10B981))
                        }
                        HorizontalDivider(modifier = Modifier.padding(top = 4.dp), color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f), thickness = 0.5.dp)
                    }
                }
            }
        }
    }
}

@Composable
fun RttLineChart(data: List<TelemetryDataPoint>, modifier: Modifier = Modifier) {
    Canvas(modifier = modifier) {
        if (data.isEmpty()) return@Canvas

        val maxRtt = (data.maxOfOrNull { it.rttMs } ?: 30f).coerceAtLeast(25f)
        val minRtt = (data.minOfOrNull { it.rttMs } ?: 10f).coerceAtLeast(0f)
        val range = (maxRtt - minRtt).coerceAtLeast(1f)

        val w = size.width
        val h = size.height
        val stepX = w / (data.size - 1).coerceAtLeast(1)

        val path = Path()
        val fillPath = Path()

        data.forEachIndexed { i, point ->
            val x = i * stepX
            val normY = ((point.rttMs - minRtt) / range).coerceIn(0f, 1f)
            val y = h - (normY * (h - 20f)) - 10f

            if (i == 0) {
                path.moveTo(x, y)
                fillPath.moveTo(x, h)
                fillPath.lineTo(x, y)
            } else {
                path.lineTo(x, y)
                fillPath.lineTo(x, y)
            }

            // Draw point circle
            drawCircle(
                color = Color(0xFF10B981),
                radius = 3.5f,
                center = Offset(x, y)
            )
        }

        fillPath.lineTo((data.size - 1) * stepX, h)
        fillPath.close()

        // Gradient fill under curve
        drawPath(
            path = fillPath,
            brush = Brush.verticalGradient(
                colors = listOf(Color(0x5510B981), Color(0x0510B981))
            )
        )

        // Line stroke
        drawPath(
            path = path,
            color = Color(0xFF10B981),
            style = Stroke(width = 3.5f)
        )
    }
}

@Composable
fun PacketLossScatterPlot(data: List<TelemetryDataPoint>, modifier: Modifier = Modifier) {
    Canvas(modifier = modifier) {
        if (data.isEmpty()) return@Canvas

        val maxLoss = (data.maxOfOrNull { it.packetLossPct } ?: 3f).coerceAtLeast(2.5f)
        val w = size.width
        val h = size.height
        val stepX = w / (data.size - 1).coerceAtLeast(1)

        // Draw horizontal threshold lines
        drawLine(
            color = Color(0x33EF4444),
            start = Offset(0f, h * 0.2f),
            end = Offset(w, h * 0.2f),
            strokeWidth = 1f
        )
        drawLine(
            color = Color(0x22FFFFFF),
            start = Offset(0f, h * 0.6f),
            end = Offset(w, h * 0.6f),
            strokeWidth = 1f
        )

        data.forEachIndexed { i, point ->
            val x = i * stepX
            val normY = (point.packetLossPct / maxLoss).coerceIn(0f, 1f)
            val y = h - (normY * (h - 20f)) - 10f

            val pointColor = when {
                point.packetLossPct > 1.5f -> Color(0xFFEF4444)
                point.packetLossPct > 0.5f -> Color(0xFFF59E0B)
                else -> Color(0xFF3B82F6)
            }

            // Scatter point with outer glow ring
            drawCircle(
                color = pointColor.copy(alpha = 0.3f),
                radius = 7f,
                center = Offset(x, y)
            )
            drawCircle(
                color = pointColor,
                radius = 4f,
                center = Offset(x, y)
            )
        }
    }
}
