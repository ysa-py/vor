package com.unifiedshield.ui

import androidx.compose.animation.*
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.UnifiedShieldStore
import com.unifiedshield.DpiHeatmapNode

@Composable
fun DPIHeatmapPanel(
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current
    val store = remember { UnifiedShieldStore.getInstance(context) }
    val storeState by store.state.collectAsState()

    Card(
        modifier = modifier
            .fillMaxWidth()
            .testTag("dpi_heatmap_panel"),
        shape = RoundedCornerShape(18.dp),
        colors = CardDefaults.cardColors(
            containerColor = if (storeState.isPanicModeActive) Color(0xFF3B1212) else MaterialTheme.colorScheme.surface
        )
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
                            .background(if (storeState.isPanicModeActive) Color(0xFFEF4444) else Color(0x22F59E0B)),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            imageVector = if (storeState.isPanicModeActive) Icons.Default.Warning else Icons.Default.GridOn,
                            contentDescription = "DPI Heatmap",
                            tint = if (storeState.isPanicModeActive) Color.White else Color(0xFFF59E0B),
                            modifier = Modifier.size(24.dp)
                        )
                    }
                    Spacer(modifier = Modifier.width(10.dp))
                    Column {
                        Text(
                            text = "Iran ISP DPI Heatmap & Panic Core",
                            fontSize = 15.sp,
                            fontWeight = FontWeight.Bold,
                            color = if (storeState.isPanicModeActive) Color(0xFFFCA5A5) else MaterialTheme.colorScheme.onSurface
                        )
                        Text(
                            text = "ماتریس حرارتی شدت فیلترینگ در هاب‌های اپراتوری و سوئیچ خودکار به هسته سایه",
                            fontSize = 11.sp,
                            color = if (storeState.isPanicModeActive) Color(0xFFFECACA) else MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }

                if (storeState.isPanicModeActive) {
                    Badge(containerColor = Color(0xFFEF4444), contentColor = Color.White) {
                        Text("PANIC ACTIVE")
                    }
                } else {
                    Badge(containerColor = Color(0x2210B981), contentColor = Color(0xFF10B981)) {
                        Text("SECURE")
                    }
                }
            }

            // Panic Mode Active Banner
            if (storeState.isPanicModeActive) {
                Spacer(modifier = Modifier.height(12.dp))
                Surface(
                    color = Color(0xFF7F1D1D),
                    shape = RoundedCornerShape(12.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(modifier = Modifier.padding(12.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(Icons.Default.Lock, contentDescription = null, tint = Color(0xFFFCA5A5), modifier = Modifier.size(18.dp))
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(
                                text = "حالت اضطراری پَنیک فعال شد!",
                                fontWeight = FontWeight.Bold,
                                color = Color.White,
                                fontSize = 13.sp
                            )
                        }
                        Spacer(modifier = Modifier.height(4.dp))
                        Text(
                            text = "دلیل: ${storeState.panicTriggerReason}",
                            fontSize = 11.sp,
                            color = Color(0xFFFECACA)
                        )
                        Text(
                            text = "• هسته فعال تغییر یافت به: ${storeState.activeCore}\n• قفل شبکه (Network Lock) فعال شد\n• حافظه RAM با موفقیت پاکسازی شد",
                            fontSize = 11.sp,
                            color = Color(0xFFFDE8E8)
                        )
                        Spacer(modifier = Modifier.height(8.dp))
                        Button(
                            onClick = { store.deactivatePanicMode() },
                            colors = ButtonDefaults.buttonColors(containerColor = Color(0xFFDC2626)),
                            shape = RoundedCornerShape(8.dp),
                            modifier = Modifier
                                .fillMaxWidth()
                                .testTag("deactivate_panic_button")
                        ) {
                            Text("خروج از حالت اضطراری و آزادسازی قفل شبکه", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(14.dp))

            // Critical Threshold Slider
            Text(
                text = "آستانه خطر بحرانی DPI (Critical Threshold): ${storeState.dpiCriticalThreshold}%",
                fontSize = 12.sp,
                fontWeight = FontWeight.SemiBold
            )
            Text(
                text = "در صورت فراتر رفتن شدت اختلال در هر یک از نودها، حالت Panic و جابجایی به هسته سایه خودکار اجرا می‌شود.",
                fontSize = 10.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            Slider(
                value = storeState.dpiCriticalThreshold.toFloat(),
                onValueChange = { store.setDpiCriticalThreshold(it.toInt()) },
                valueRange = 50f..95f,
                steps = 9,
                modifier = Modifier.testTag("dpi_threshold_slider")
            )

            Spacer(modifier = Modifier.height(10.dp))

            // Heatmap Grid
            Text(
                text = "Regional ISP Core Heatmap Nodes (شدت بازرسی در استان‌ها)",
                fontSize = 13.sp,
                fontWeight = FontWeight.Bold
            )
            Spacer(modifier = Modifier.height(6.dp))

            storeState.heatmapNodes.forEach { node ->
                val nodeColor = when {
                    node.dpiThreatScore >= storeState.dpiCriticalThreshold -> Color(0xFFEF4444)
                    node.dpiThreatScore >= 70 -> Color(0xFFF59E0B)
                    else -> Color(0xFF10B981)
                }

                Surface(
                    color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.35f),
                    shape = RoundedCornerShape(10.dp),
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 3.dp)
                        .border(
                            width = if (node.isBreached) 1.5.dp else 0.dp,
                            color = if (node.isBreached) Color(0xFFEF4444) else Color.Transparent,
                            shape = RoundedCornerShape(10.dp)
                        )
                ) {
                    Row(
                        modifier = Modifier.padding(10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Box(
                                modifier = Modifier
                                    .size(8.dp)
                                    .clip(CircleShape)
                                    .background(nodeColor)
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Column {
                                Text(node.regionPersian, fontSize = 12.sp, fontWeight = FontWeight.Bold)
                                Text("${node.regionName} • ${node.ispName}", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                        }

                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Column(horizontalAlignment = Alignment.End) {
                                Text(
                                    text = "${node.dpiThreatScore}% Threat",
                                    fontSize = 12.sp,
                                    fontWeight = FontWeight.Bold,
                                    color = nodeColor
                                )
                                Text(
                                    text = "RST: ${(node.rstRate * 100).toInt()}% | Drop: ${(node.packetDropRate * 100).toInt()}%",
                                    fontSize = 9.sp,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                            Spacer(modifier = Modifier.width(6.dp))
                            // Simulate Spike button to test auto panic
                            IconButton(
                                onClick = {
                                    val newScore = if (node.dpiThreatScore < storeState.dpiCriticalThreshold) 89 else 65
                                    store.updateHeatmapScore(node.id, newScore)
                                },
                                modifier = Modifier.size(28.dp).testTag("spike_node_${node.id}")
                            ) {
                                Icon(
                                    imageVector = Icons.Default.Bolt,
                                    contentDescription = "Simulate Threat",
                                    tint = nodeColor,
                                    modifier = Modifier.size(16.dp)
                                )
                            }
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(10.dp))

            // Manual Panic Button
            Button(
                onClick = {
                    if (storeState.isPanicModeActive) store.deactivatePanicMode()
                    else store.triggerPanicMode("دکمه اضطراری پَنیک توسط کاربر فشرده شد")
                },
                colors = ButtonDefaults.buttonColors(
                    containerColor = if (storeState.isPanicModeActive) Color(0xFF10B981) else Color(0xFFDC2626)
                ),
                shape = RoundedCornerShape(10.dp),
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("panic_trigger_button")
            ) {
                Icon(
                    imageVector = if (storeState.isPanicModeActive) Icons.Default.LockOpen else Icons.Default.Lock,
                    contentDescription = null,
                    modifier = Modifier.size(18.dp)
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = if (storeState.isPanicModeActive) "آزادسازی قفل شبکه و خروج از Panic" else "فعال‌سازی فوری حالت اضطراری پَنیک (Panic Mode)",
                    fontWeight = FontWeight.Bold,
                    fontSize = 12.sp
                )
            }
        }
    }
}
