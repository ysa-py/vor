package com.unifiedshield.ui

import androidx.compose.animation.*
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.DpiDiagnosticEngine

@Composable
fun DpiDiagnosticScreen() {
    val diagnosticEngine = remember { DpiDiagnosticEngine.getInstance() }
    val summary by diagnosticEngine.summary.collectAsState()
    val items by diagnosticEngine.diagnosticItems.collectAsState()

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .testTag("dpi_diagnostic_screen"),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        item {
            // Overall Status Summary Card
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.35f)
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
                            Box(
                                modifier = Modifier
                                    .size(46.dp)
                                    .clip(CircleShape)
                                    .background(Color(0x2210B981)),
                                contentAlignment = Alignment.Center
                            ) {
                                Icon(
                                    imageVector = Icons.Default.Troubleshoot,
                                    contentDescription = null,
                                    tint = Color(0xFF10B981),
                                    modifier = Modifier.size(26.dp)
                                )
                            }
                            Spacer(modifier = Modifier.width(12.dp))
                            Column {
                                Text("Live DPI Inspector & Spectrum", fontSize = 16.sp, fontWeight = FontWeight.Bold)
                                Text("اسکنر زنده فیلترینگ و بردارهای سانسور هوشمند", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                        }

                        Badge(containerColor = Color(0xFF10B981), contentColor = Color.White) {
                            Text("${summary.bypassHealthScore}% سلامت عبور")
                        }
                    }

                    Spacer(modifier = Modifier.height(14.dp))

                    Text("اپراتور و گیت‌وی شناسایی شده: ${summary.ispName}", fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
                    Text("وضعیت مقابله فعال: ${summary.activeCureApplied}", fontSize = 11.sp, color = MaterialTheme.colorScheme.primary)

                    Spacer(modifier = Modifier.height(12.dp))

                    Button(
                        onClick = { diagnosticEngine.runLiveDiagnostic() },
                        enabled = !summary.isRunningTest,
                        modifier = Modifier
                            .fillMaxWidth()
                            .testTag("run_diagnostic_button"),
                        shape = RoundedCornerShape(12.dp),
                        colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF10B981))
                    ) {
                        if (summary.isRunningTest) {
                            CircularProgressIndicator(modifier = Modifier.size(18.dp), color = Color.White, strokeWidth = 2.dp)
                            Spacer(modifier = Modifier.width(8.dp))
                            Text("در حال تست بلادرنگ امواج و پروتکل‌ها...", fontSize = 13.sp)
                        } else {
                            Icon(Icons.Default.PlayArrow, contentDescription = null, modifier = Modifier.size(20.dp))
                            Spacer(modifier = Modifier.width(8.dp))
                            Text("شروع اسکن و رفع فوری فیلترینگ (Live Scan)", fontSize = 13.sp, fontWeight = FontWeight.Bold)
                        }
                    }
                }
            }
        }

        item {
            // Real-Time DPI Heatmap & Auto-Panic failover panel
            DPIHeatmapPanel()
        }

        item {
            // Real-Time Threat Intelligence & Signature Correlation Panel
            ThreatIntelPanel()
        }

        item {
            Text(
                text = "Censorship Attack Vectors & Evasion State (بررسی بردارهای مسدودسازی)",
                fontSize = 14.sp,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.padding(horizontal = 4.dp)
            )
        }

        items(items, key = { it.id }) { item ->
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f)),
                shape = RoundedCornerShape(14.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(
                                imageVector = if (item.isBlocked) Icons.Default.Cancel else Icons.Default.CheckCircle,
                                contentDescription = null,
                                tint = if (item.isBlocked) Color(0xFFEF4444) else Color(0xFF10B981),
                                modifier = Modifier.size(20.dp)
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(item.titlePersian, fontSize = 13.sp, fontWeight = FontWeight.Bold)
                        }

                        if (item.isTesting) {
                            CircularProgressIndicator(modifier = Modifier.size(14.dp), strokeWidth = 2.dp)
                        } else {
                            Text("${item.latencyMs} ms", fontSize = 12.sp, fontWeight = FontWeight.SemiBold, color = Color(0xFF10B981))
                        }
                    }

                    Spacer(modifier = Modifier.height(4.dp))
                    Text("هدف فیلترینگ: ${item.targetSignature}", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Spacer(modifier = Modifier.height(6.dp))

                    Surface(
                        color = Color(0x1510B981),
                        shape = RoundedCornerShape(8.dp)
                    ) {
                        Row(
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Icon(Icons.Default.Shield, contentDescription = null, tint = Color(0xFF10B981), modifier = Modifier.size(14.dp))
                            Spacer(modifier = Modifier.width(6.dp))
                            Text(item.statusTextPersian, fontSize = 11.sp, fontWeight = FontWeight.Medium, color = Color(0xFF10B981))
                        }
                    }
                }
            }
        }
    }
}
