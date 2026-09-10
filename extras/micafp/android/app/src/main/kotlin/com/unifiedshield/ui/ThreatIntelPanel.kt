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
import com.unifiedshield.ThreatIntelSignature
import com.unifiedshield.NovelProtocolsEngine
import com.unifiedshield.AiStealthEngine

@Composable
fun ThreatIntelPanel(
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current
    val store = remember { UnifiedShieldStore.getInstance(context) }
    val storeState by store.state.collectAsState()
    val protocolsEngine = remember { NovelProtocolsEngine.getInstance() }
    val protocols by protocolsEngine.protocols.collectAsState()
    val aiEngine = remember { AiStealthEngine.getInstance() }
    val aiState by aiEngine.stealthState.collectAsState()

    val activeProtocol = protocols.firstOrNull { it.status == "ACTIVE" } ?: protocols.first()

    var activeTooltipSig by remember { mutableStateOf<ThreatIntelSignature?>(null) }
    var optimizationSuccessMsg by remember { mutableStateOf<String?>(null) }

    Card(
        modifier = modifier
            .fillMaxWidth()
            .testTag("threat_intel_panel"),
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
                            .background(Color(0x226366F1)),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            imageVector = Icons.Default.Security,
                            contentDescription = "Threat Intelligence",
                            tint = Color(0xFF6366F1),
                            modifier = Modifier.size(24.dp)
                        )
                    }
                    Spacer(modifier = Modifier.width(10.dp))
                    Column {
                        Text(
                            text = "DPI Threat Intelligence & Signature Matrix",
                            fontSize = 15.sp,
                            fontWeight = FontWeight.Bold
                        )
                        Text(
                            text = "تحلیل هوشمند بردارهای فیلترینگ و انطباق آنی با پروتکل‌های بهینه",
                            fontSize = 11.sp,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }

                Badge(
                    containerColor = Color(0x2210B981),
                    contentColor = Color(0xFF10B981)
                ) {
                    Text("${storeState.activeOptimizationCount} بهینه‌سازی فعال")
                }
            }

            if (optimizationSuccessMsg != null) {
                Spacer(modifier = Modifier.height(10.dp))
                Surface(
                    color = Color(0x1A10B981),
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(
                        modifier = Modifier.padding(10.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Icon(Icons.Default.CheckCircle, contentDescription = null, tint = Color(0xFF10B981), modifier = Modifier.size(18.dp))
                        Spacer(modifier = Modifier.width(8.dp))
                        Text(optimizationSuccessMsg ?: "", fontSize = 11.sp, color = Color(0xFF10B981))
                    }
                }
            }

            Spacer(modifier = Modifier.height(14.dp))

            // Signature correlation cards
            storeState.threatSignatures.forEach { sig ->
                val isTooltipOpen = activeTooltipSig?.id == sig.id
                val isCorrelatedWithActiveProtocol = activeProtocol.id == sig.recommendedProtocolId
                val threatColor = when (sig.threatLevel) {
                    "CRITICAL" -> Color(0xFFEF4444)
                    "HIGH" -> Color(0xFFF59E0B)
                    "MEDIUM" -> Color(0xFF3B82F6)
                    else -> Color(0xFF10B981)
                }

                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 4.dp)
                        .border(
                            width = if (isTooltipOpen) 1.5.dp else 1.dp,
                            color = if (isTooltipOpen) Color(0xFF6366F1) else MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.4f),
                            shape = RoundedCornerShape(12.dp)
                        )
                        .clickable {
                            activeTooltipSig = if (isTooltipOpen) null else sig
                        }
                        .testTag("threat_sig_${sig.id}"),
                    colors = CardDefaults.cardColors(
                        containerColor = if (isTooltipOpen) Color(0x106366F1) else MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.35f)
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
                                        .background(threatColor)
                                )
                                Spacer(modifier = Modifier.width(8.dp))
                                Column {
                                    Text(sig.namePersian, fontSize = 13.sp, fontWeight = FontWeight.Bold)
                                    Text(sig.name, fontSize = 11.sp, color = MaterialTheme.colorScheme.primary)
                                }
                            }

                            Badge(
                                containerColor = threatColor.copy(alpha = 0.15f),
                                contentColor = threatColor
                            ) {
                                Text(sig.threatLevel)
                            }
                        }

                        Spacer(modifier = Modifier.height(6.dp))

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(
                                text = "تعداد تلاش مسدودسازی: ${sig.detectedCount}",
                                fontSize = 11.sp,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )

                            Text(
                                text = "پروتکل پیشنهادی: ${sig.recommendedProtocolName}",
                                fontSize = 11.sp,
                                fontWeight = FontWeight.SemiBold,
                                color = if (isCorrelatedWithActiveProtocol) Color(0xFF10B981) else Color(0xFF6366F1)
                            )
                        }

                        // Real-Time Tooltip Summary & One-Click Apply Optimization
                        AnimatedVisibility(
                            visible = isTooltipOpen,
                            enter = expandVertically() + fadeIn(),
                            exit = shrinkVertically() + fadeOut()
                        ) {
                            Column(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(top = 10.dp)
                                    .background(
                                        color = MaterialTheme.colorScheme.surface,
                                        shape = RoundedCornerShape(10.dp)
                                    )
                                    .border(1.dp, Color(0xFF6366F1).copy(alpha = 0.5f), RoundedCornerShape(10.dp))
                                    .padding(12.dp)
                            ) {
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Row(verticalAlignment = Alignment.CenterVertically) {
                                        Icon(Icons.Default.Info, contentDescription = null, tint = Color(0xFF6366F1), modifier = Modifier.size(16.dp))
                                        Spacer(modifier = Modifier.width(6.dp))
                                        Text("تحلیل همبستگی بردار و پروتکل فعال:", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFF6366F1))
                                    }

                                    Badge(
                                        containerColor = if (isCorrelatedWithActiveProtocol) Color(0x2210B981) else Color(0x22F59E0B),
                                        contentColor = if (isCorrelatedWithActiveProtocol) Color(0xFF10B981) else Color(0xFFD97706)
                                    ) {
                                        Text(if (isCorrelatedWithActiveProtocol) "پروتکل بهینه فعال است" else "نیاز به تطبیق")
                                    }
                                }

                                Spacer(modifier = Modifier.height(6.dp))
                                Text(
                                    text = sig.recommendedActionDescriptionPersian,
                                    fontSize = 11.sp,
                                    color = MaterialTheme.colorScheme.onSurface
                                )
                                Spacer(modifier = Modifier.height(2.dp))
                                Text(
                                    text = "الگوی امضا: ${sig.signaturePattern}",
                                    fontSize = 10.sp,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )

                                Spacer(modifier = Modifier.height(8.dp))

                                // Dynamic Correlation Specs
                                Surface(
                                    color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.45f),
                                    shape = RoundedCornerShape(8.dp),
                                    modifier = Modifier.fillMaxWidth()
                                ) {
                                    Row(
                                        modifier = Modifier.padding(8.dp),
                                        horizontalArrangement = Arrangement.SpaceBetween
                                    ) {
                                        Column {
                                            Text("پروتکل جاری", fontSize = 9.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                            Text(activeProtocol.name, fontSize = 11.sp, fontWeight = FontWeight.Bold)
                                        }
                                        Column {
                                            Text("برش رکورد TLS", fontSize = 9.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                            Text("${aiState.tlsRecordSplitLength} بایت / TCP Record", fontSize = 11.sp, fontWeight = FontWeight.Bold, color = Color(0xFF6366F1))
                                        }
                                        Column {
                                            Text("تزریق نویز تصادفی", fontSize = 9.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                            Text("${aiState.randomPaddingBytes} بایت پدینگ", fontSize = 11.sp, fontWeight = FontWeight.Bold, color = Color(0xFF10B981))
                                        }
                                    }
                                }

                                Spacer(modifier = Modifier.height(10.dp))

                                Button(
                                    onClick = {
                                        store.applyThreatOptimization(sig.id)
                                        optimizationSuccessMsg = "پیکربندی با موفقیت روی پروتکل ${sig.recommendedProtocolName} با برش هوشمند TLS و قطعه‌بندی پکت اعمال شد."
                                    },
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .testTag("apply_opt_${sig.id}"),
                                    colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF10B981)),
                                    shape = RoundedCornerShape(8.dp)
                                ) {
                                    Icon(Icons.Default.AutoFixHigh, contentDescription = null, modifier = Modifier.size(16.dp))
                                    Spacer(modifier = Modifier.width(6.dp))
                                    Text("اعمال پیکربندی بهینه‌شده (Apply Optimized Configuration)", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
