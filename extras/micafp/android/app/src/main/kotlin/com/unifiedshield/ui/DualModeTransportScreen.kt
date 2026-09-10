package com.unifiedshield.ui

import androidx.compose.animation.*
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
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
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.tunnel.*

@Composable
fun DualModeTransportScreen() {
    val context = LocalContext.current
    val engine = remember { DualModeTransportEngine.getInstance(context) }
    val state by engine.state.collectAsState()

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .testTag("dual_mode_screen"),
        verticalArrangement = Arrangement.spacedBy(14.dp),
        contentPadding = PaddingValues(bottom = 24.dp)
    ) {
        // Header & Directive v70 Badge
        item {
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)
                ),
                shape = RoundedCornerShape(16.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Column {
                            Text(
                                "هسته انتقال دوحالته (Directive v70)",
                                fontSize = 17.sp,
                                fontWeight = FontWeight.Bold,
                                color = MaterialTheme.colorScheme.onSurface
                            )
                            Text(
                                "Dual-Mode Rust Core • QUIC MultiPath & 5-Hop Sphinx",
                                fontSize = 11.sp,
                                color = MaterialTheme.colorScheme.primary
                            )
                        }
                        Surface(
                            shape = RoundedCornerShape(8.dp),
                            color = MaterialTheme.colorScheme.primaryContainer
                        ) {
                            Text(
                                "v70.0 RUST CORE",
                                fontSize = 10.sp,
                                fontWeight = FontWeight.ExtraBold,
                                color = MaterialTheme.colorScheme.onPrimaryContainer,
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(12.dp))

                    // Mode A / Mode B Segmented Selector
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(12.dp))
                            .background(MaterialTheme.colorScheme.surface)
                            .padding(4.dp),
                        horizontalArrangement = Arrangement.spacedBy(4.dp)
                    ) {
                        val isModeA = state.activeMode == TransportMode.MODE_A_MULTIPATH
                        val isModeB = state.activeMode == TransportMode.MODE_B_LAYERED

                        Surface(
                            shape = RoundedCornerShape(10.dp),
                            color = if (isModeA) MaterialTheme.colorScheme.primary else Color.Transparent,
                            modifier = Modifier
                                .weight(1f)
                                .clickable { engine.setTransportMode(TransportMode.MODE_A_MULTIPATH) }
                                .testTag("btn_mode_a_selector")
                        ) {
                            Column(
                                modifier = Modifier.padding(vertical = 10.dp, horizontal = 8.dp),
                                horizontalAlignment = Alignment.CenterHorizontally
                            ) {
                                Row(verticalAlignment = Alignment.CenterVertically) {
                                    Icon(
                                        Icons.Default.Bolt,
                                        contentDescription = null,
                                        tint = if (isModeA) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurfaceVariant,
                                        modifier = Modifier.size(16.dp)
                                    )
                                    Spacer(modifier = Modifier.width(4.dp))
                                    Text(
                                        "حالت الف (سریع)",
                                        fontSize = 12.sp,
                                        fontWeight = FontWeight.Bold,
                                        color = if (isModeA) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurface
                                    )
                                }
                                Text(
                                    "QUIC چندمسیره موازی",
                                    fontSize = 10.sp,
                                    color = if (isModeA) MaterialTheme.colorScheme.onPrimary.copy(alpha = 0.8f) else MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                        }

                        Surface(
                            shape = RoundedCornerShape(10.dp),
                            color = if (isModeB) Color(0xFF6366F1) else Color.Transparent,
                            modifier = Modifier
                                .weight(1f)
                                .clickable { engine.setTransportMode(TransportMode.MODE_B_LAYERED) }
                                .testTag("btn_mode_b_selector")
                        ) {
                            Column(
                                modifier = Modifier.padding(vertical = 10.dp, horizontal = 8.dp),
                                horizontalAlignment = Alignment.CenterHorizontally
                            ) {
                                Row(verticalAlignment = Alignment.CenterVertically) {
                                    Icon(
                                        Icons.Default.Shield,
                                        contentDescription = null,
                                        tint = if (isModeB) Color.White else MaterialTheme.colorScheme.onSurfaceVariant,
                                        modifier = Modifier.size(16.dp)
                                    )
                                    Spacer(modifier = Modifier.width(4.dp))
                                    Text(
                                        "حالت ب (پیازی ۵ لایه)",
                                        fontSize = 12.sp,
                                        fontWeight = FontWeight.Bold,
                                        color = if (isModeB) Color.White else MaterialTheme.colorScheme.onSurface
                                    )
                                }
                                Text(
                                    "Sphinx/Noise ۵ هاپ تو در تو",
                                    fontSize = 10.sp,
                                    color = if (isModeB) Color.White.copy(alpha = 0.85f) else MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                        }
                    }

                    Spacer(modifier = Modifier.height(10.dp))
                    Text(
                        state.activeMode.tagline,
                        fontSize = 11.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }

        // AI Autonomous Auto-Pilot Card
        item {
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = if (state.isAutoPilotAiEnabled) MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.35f) else MaterialTheme.colorScheme.surface
                ),
                shape = RoundedCornerShape(16.dp),
                modifier = Modifier.border(
                    1.dp,
                    if (state.isAutoPilotAiEnabled) MaterialTheme.colorScheme.primary.copy(alpha = 0.5f) else MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f),
                    RoundedCornerShape(16.dp)
                )
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(
                                Icons.Default.Psychology,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.size(22.dp)
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Column {
                                Text("اتوپایلوت هوشمند AI (هدایت خودکار ترافیک)", fontSize = 14.sp, fontWeight = FontWeight.Bold)
                                Text("Autonomous Mode Switching & Anti-DPI", fontSize = 10.sp, color = MaterialTheme.colorScheme.primary)
                            }
                        }
                        Switch(
                            checked = state.isAutoPilotAiEnabled,
                            onCheckedChange = { engine.setAutoPilot(it) },
                            modifier = Modifier.testTag("switch_auto_pilot")
                        )
                    }

                    Spacer(modifier = Modifier.height(10.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Column {
                            Text("وضعیت DPI شبکه:", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text(
                                state.dpiDetectionLevel,
                                fontSize = 12.sp,
                                fontWeight = FontWeight.Bold,
                                color = if (state.dpiDetectionLevel.contains("بحرانی")) Color(0xFFEF4444) else Color(0xFF10B981)
                            )
                        }
                        Column(horizontalAlignment = Alignment.End) {
                            Text("ضریب انتروپی ترافیک:", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text("${state.dpiEntropyScore} / 1.00", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.onSurface)
                        }
                    }

                    Spacer(modifier = Modifier.height(8.dp))

                    Surface(
                        shape = RoundedCornerShape(8.dp),
                        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.6f),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Row(
                            modifier = Modifier.padding(8.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Icon(Icons.Default.Info, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(16.dp))
                            Spacer(modifier = Modifier.width(6.dp))
                            Text(state.autoPilotReason, fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurface)
                        }
                    }

                    Spacer(modifier = Modifier.height(10.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        OutlinedButton(
                            onClick = { engine.simulateDpiSpike() },
                            modifier = Modifier.weight(1f).height(36.dp).testTag("btn_test_dpi_spike"),
                            contentPadding = PaddingValues(horizontal = 8.dp, vertical = 2.dp)
                        ) {
                            Icon(Icons.Default.BugReport, contentDescription = null, modifier = Modifier.size(14.dp))
                            Spacer(modifier = Modifier.width(4.dp))
                            Text("تست تزریق اختلال DPI", fontSize = 11.sp)
                        }
                    }
                }
            }
        }

        // TCP MSS Clamping & Cloudflare Challenge Pass-Through Safeguard Card
        item {
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.45f)
                ),
                shape = RoundedCornerShape(16.dp),
                modifier = Modifier.border(
                    1.dp,
                    Color(0xFF10B981).copy(alpha = 0.35f),
                    RoundedCornerShape(16.dp)
                )
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(
                                Icons.Default.VerifiedUser,
                                contentDescription = null,
                                tint = Color(0xFF10B981),
                                modifier = Modifier.size(20.dp)
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Column {
                                Text(
                                    "محافظت عبور از چالش‌های Cloudflare و TLS",
                                    fontSize = 13.sp,
                                    fontWeight = FontWeight.Bold
                                )
                                Text(
                                    "TCP MSS Clamping (1360B) + De-Jitter Reordering",
                                    fontSize = 10.sp,
                                    color = Color(0xFF10B981)
                                )
                            }
                        }
                        Surface(
                            shape = RoundedCornerShape(6.dp),
                            color = Color(0xFF10B981).copy(alpha = 0.15f)
                        ) {
                            Text(
                                "DF-SAFE ACTIVE",
                                fontSize = 10.sp,
                                fontWeight = FontWeight.Bold,
                                color = Color(0xFF10B981),
                                modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(10.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Column {
                            Text("مقدار کلمپ TCP MSS:", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text("${state.clampedMssValue} Bytes (RFC 1624)", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFF0284C7))
                        }
                        Column {
                            Text("تعداد چالش حل‌شده:", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text("${state.cloudflareStallsMitigated}", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFF10B981))
                        }
                        Column {
                            Text("تراز فریم‌های نامرتب:", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text("${state.outOfOrderRealignedCount} پکت", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFF8B5CF6))
                        }
                    }

                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        "تضمین عبور بی‌درنگ از تست‌های راستی‌آزمایی بات Cloudflare (Verifying you are human) بدون هنگ کردن و ممانعت از تکه‌تکه شدن بسته‌ها (IP Fragmentation).",
                        fontSize = 10.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        lineHeight = 14.sp
                    )
                }
            }
        }

        // Active Mode Specific Controls
        if (state.activeMode == TransportMode.MODE_A_MULTIPATH) {
            // Mode A Configuration & QUIC Paths
            item {
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    shape = RoundedCornerShape(16.dp),
                    modifier = Modifier.border(1.dp, MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.4f), RoundedCornerShape(16.dp))
                ) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Icon(Icons.Default.Hub, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(20.dp))
                                Spacer(modifier = Modifier.width(8.dp))
                                Text("تنظیمات انتقال چندمسیره (Mode A)", fontSize = 14.sp, fontWeight = FontWeight.Bold)
                            }
                            Text("${state.concurrentPaths} مسیر همزمان", fontSize = 12.sp, color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.Bold)
                        }

                        Spacer(modifier = Modifier.height(12.dp))

                        // Path count selector
                        Text("تعداد مسیرهای QUIC موازی:", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            (1..5).forEach { count ->
                                val isSelected = state.concurrentPaths == count
                                FilterChip(
                                    selected = isSelected,
                                    onClick = { engine.setConcurrentPaths(count) },
                                    label = { Text("$count مسیر", fontSize = 11.sp) },
                                    modifier = Modifier.weight(1f)
                                )
                            }
                        }

                        Spacer(modifier = Modifier.height(10.dp))

                        // Congestion algorithm selector
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text("الگوریتم کنترل ازدحام (CC):", fontSize = 12.sp)
                            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                FilterChip(
                                    selected = state.congestionAlgo == CongestionAlgo.BBR,
                                    onClick = { engine.setCongestionAlgo(CongestionAlgo.BBR) },
                                    label = { Text("BBR", fontSize = 11.sp) }
                                )
                                FilterChip(
                                    selected = state.congestionAlgo == CongestionAlgo.CUBIC,
                                    onClick = { engine.setCongestionAlgo(CongestionAlgo.CUBIC) },
                                    label = { Text("CUBIC", fontSize = 11.sp) }
                                )
                            }
                        }
                    }
                }
            }

            // Path Matrix
            items(state.paths) { path ->
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.35f)),
                    shape = RoundedCornerShape(12.dp),
                    modifier = Modifier.border(1.dp, MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f), RoundedCornerShape(12.dp))
                ) {
                    Column(modifier = Modifier.padding(14.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Box(
                                    modifier = Modifier
                                        .size(10.dp)
                                        .background(Color(0xFF10B981), CircleShape)
                                )
                                Spacer(modifier = Modifier.width(8.dp))
                                Text("مسیر #${path.pathId}: ${path.regionTag}", fontSize = 13.sp, fontWeight = FontWeight.Bold)
                            }
                            Surface(
                                shape = RoundedCornerShape(6.dp),
                                color = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.6f)
                            ) {
                                Text(
                                    "امتیاز زمان‌بند: ${path.liveScore}",
                                    fontSize = 10.sp,
                                    fontWeight = FontWeight.Bold,
                                    color = MaterialTheme.colorScheme.onPrimaryContainer,
                                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                                )
                            }
                        }

                        Spacer(modifier = Modifier.height(8.dp))

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Column {
                                Text("تاخیر (RTT): ${path.rttMs} ms", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text("نوسان (Jitter): ${path.jitterMs} ms", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                            Column {
                                Text("افت بسته: ${path.packetLossPct}%", fontSize = 11.sp, color = if (path.packetLossPct > 2.0) Color(0xFFEF4444) else MaterialTheme.colorScheme.onSurfaceVariant)
                                Text("PMTUD: ${path.currentMtu} B", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                            Column {
                                Text("پکت‌های ارسالی: ${path.packetsSent}", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text("وضعیت: ${path.congestionState}", fontSize = 11.sp, color = Color(0xFF10B981))
                            }
                        }
                    }
                }
            }
        } else {
            // Mode B Configuration & 5-Hop Onion Circuit
            item {
                Card(
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                    shape = RoundedCornerShape(16.dp),
                    modifier = Modifier.border(1.dp, MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.4f), RoundedCornerShape(16.dp))
                ) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Icon(Icons.Default.Security, contentDescription = null, tint = Color(0xFF6366F1), modifier = Modifier.size(20.dp))
                                Spacer(modifier = Modifier.width(8.dp))
                                Text("مدار پیازی ۵ لایه (Mode B)", fontSize = 14.sp, fontWeight = FontWeight.Bold)
                            }
                            Surface(
                                shape = RoundedCornerShape(6.dp),
                                color = if (state.isFailClosedEnabled) Color(0xFF10B981).copy(alpha = 0.15f) else Color(0xFFEF4444).copy(alpha = 0.15f)
                            ) {
                                Text(
                                    if (state.isFailClosedEnabled) "قفل نشت (Fail-Closed) فعال" else "قفل غیرفعال",
                                    fontSize = 10.sp,
                                    fontWeight = FontWeight.Bold,
                                    color = if (state.isFailClosedEnabled) Color(0xFF10B981) else Color(0xFFEF4444),
                                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                                )
                            }
                        }

                        Spacer(modifier = Modifier.height(10.dp))
                        Text(
                            "در این حالت بسته‌ها با ۵ لایه رمزنگاری متقارن درون‌هم کپسوله‌سازی می‌شوند. هر هاپ صرفاً لایه خود را رمزگشایی کرده و آدرس هاپ بعدی را بدون دسترسی به محتوا پیدا می‌کند.",
                            fontSize = 11.sp,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )

                        Spacer(modifier = Modifier.height(12.dp))

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text("سیاست قطع کامل در قطعی هاپ (Fail-Closed):", fontSize = 12.sp)
                            Switch(
                                checked = state.isFailClosedEnabled,
                                onCheckedChange = { engine.setFailClosed(it) }
                            )
                        }
                    }
                }
            }

            // 5-Hop Nodes Chain
            items(state.hops) { hop ->
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = if (hop.isHealthy) MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.35f) else Color(0xFFEF4444).copy(alpha = 0.1f)
                    ),
                    shape = RoundedCornerShape(12.dp),
                    modifier = Modifier.border(
                        1.dp,
                        if (hop.isHealthy) MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f) else Color(0xFFEF4444).copy(alpha = 0.5f),
                        RoundedCornerShape(12.dp)
                    )
                ) {
                    Column(modifier = Modifier.padding(14.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Box(
                                    modifier = Modifier
                                        .size(10.dp)
                                        .background(if (hop.isHealthy) Color(0xFF10B981) else Color(0xFFEF4444), CircleShape)
                                )
                                Spacer(modifier = Modifier.width(8.dp))
                                Text("هاپ #${hop.hopIndex}: ${hop.regionTag}", fontSize = 13.sp, fontWeight = FontWeight.Bold)
                            }
                            Button(
                                onClick = { engine.toggleHopHealth(hop.hopIndex) },
                                colors = ButtonDefaults.buttonColors(
                                    containerColor = if (hop.isHealthy) MaterialTheme.colorScheme.surfaceVariant else Color(0xFFEF4444)
                                ),
                                contentPadding = PaddingValues(horizontal = 8.dp, vertical = 2.dp),
                                modifier = Modifier.height(28.dp)
                            ) {
                                Text(
                                    if (hop.isHealthy) "تست قطعی" else "بازیابی نود",
                                    fontSize = 10.sp,
                                    color = if (hop.isHealthy) MaterialTheme.colorScheme.onSurface else Color.White
                                )
                            }
                        }

                        Spacer(modifier = Modifier.height(6.dp))
                        Text("لایه رمزنگاری: ${hop.encryptionLayer}", fontSize = 11.sp, color = MaterialTheme.colorScheme.primary)
                        Text("آدرس نود: ${hop.endpoint} • تاخیر تخمینی: ${hop.rttMs} ms", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
        }

        // Live Benchmark Harness Panel
        item {
            Card(
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                shape = RoundedCornerShape(16.dp),
                modifier = Modifier.border(1.dp, MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.4f), RoundedCornerShape(16.dp))
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(Icons.Default.Speed, contentDescription = null, tint = Color(0xFFF59E0B), modifier = Modifier.size(20.dp))
                            Spacer(modifier = Modifier.width(8.dp))
                            Text("سنجش عملکرد واقعی (Directive v70 Benchmark)", fontSize = 14.sp, fontWeight = FontWeight.Bold)
                        }

                        Button(
                            onClick = { engine.runBenchmark() },
                            enabled = !state.isBenchmarking,
                            contentPadding = PaddingValues(horizontal = 12.dp, vertical = 6.dp),
                            modifier = Modifier.height(34.dp).testTag("btn_run_benchmark")
                        ) {
                            if (state.isBenchmarking) {
                                CircularProgressIndicator(modifier = Modifier.size(14.dp), strokeWidth = 2.dp, color = Color.White)
                                Spacer(modifier = Modifier.width(6.dp))
                                Text("در حال تست...", fontSize = 11.sp)
                            } else {
                                Text("اجرای بنچمارک", fontSize = 11.sp)
                            }
                        }
                    }

                    if (state.benchmarkResults.isNotEmpty()) {
                        Spacer(modifier = Modifier.height(12.dp))
                        state.benchmarkResults.forEach { result ->
                            Surface(
                                shape = RoundedCornerShape(8.dp),
                                color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f),
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(vertical = 4.dp)
                            ) {
                                Row(
                                    modifier = Modifier.padding(10.dp),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Column {
                                        Text(result.title, fontSize = 12.sp, fontWeight = FontWeight.Bold)
                                        Text("${result.mode} • ${result.pathCount} مسیر • ${result.lossPct}% اتلاف", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                    }
                                    Column(horizontalAlignment = Alignment.End) {
                                        Text("${result.throughputMbps} Mbps", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFF10B981))
                                        Text("${result.processingTimeNanos} ns/pkt • ${result.memoryFootprintMb} MB", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Memory Ceiling & FFI Telemetry
        item {
            Card(
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.25f)),
                shape = RoundedCornerShape(16.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text("پایش سقف حافظه و FFI سیستم‌عامل (iOS & Android)", fontSize = 13.sp, fontWeight = FontWeight.Bold)
                    Spacer(modifier = Modifier.height(8.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Text("حافظه اشغال شده هسته Rust:", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Text("${state.memoryFootprintMb} MB / 35.0 MB (سقف امن)", fontSize = 11.sp, fontWeight = FontWeight.Bold, color = Color(0xFF10B981))
                    }

                    Spacer(modifier = Modifier.height(6.dp))
                    LinearProgressIndicator(
                        progress = { (state.memoryFootprintMb / 35.0).toFloat().coerceIn(0f, 1f) },
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(6.dp)
                            .clip(RoundedCornerShape(3.dp)),
                        color = Color(0xFF10B981),
                        trackColor = MaterialTheme.colorScheme.surfaceVariant
                    )

                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        "بر اساس استاندارد NetworkExtension اپل و نگاشت AsyncFd در لینوکس، مصرف حافظه در هر دو حالت زیر سقف بحرانی نگه داشته می‌شود.",
                        fontSize = 10.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }
    }
}
