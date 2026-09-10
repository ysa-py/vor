package com.unifiedshield.ui

import androidx.compose.animation.*
import androidx.compose.animation.core.*
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
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.AiStealthEngine
import com.unifiedshield.NativeCoreLoader
import com.unifiedshield.profile.ProfileManager
import com.unifiedshield.ui.components.CyberHudCard
import com.unifiedshield.ui.components.EnterpriseStatusPill
import com.unifiedshield.ui.components.MetricCard
import com.unifiedshield.ui.components.StatusPillType
import com.unifiedshield.ui.theme.EnterpriseColors

@Composable
fun StatusCard(
    isConnected: Boolean,
    currentCore: String,
    ispName: String,
    uploadSpeed: String,
    downloadSpeed: String,
    dpiScore: Double,
    onConnectClick: () -> Unit,
    onDisconnectClick: () -> Unit
) {
    val context = LocalContext.current
    val aiEngine = remember { AiStealthEngine.getInstance() }
    val stealthState by aiEngine.stealthState.collectAsState()
    val isNativeReady by NativeCoreLoader.isNativeReady.collectAsState()
    val profileManager = remember { ProfileManager.getInstance(context) }

    var isTechnicalDetailsExpanded by remember { mutableStateOf(false) }

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp, vertical = 8.dp)
            .testTag("status_card_container"),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        // =========================================================
        // 1. Primary Focal Point: Minimalist Connection Hero Card
        // =========================================================
        Card(
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
            shape = RoundedCornerShape(16.dp),
            border = androidx.compose.foundation.BorderStroke(
                1.dp,
                if (isConnected) MaterialTheme.colorScheme.primary.copy(alpha = 0.45f) else MaterialTheme.colorScheme.outline
            ),
            elevation = CardDefaults.cardElevation(defaultElevation = 0.dp),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(20.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                // Top Status Header Row
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Box(
                            modifier = Modifier
                                .size(28.dp)
                                .clip(RoundedCornerShape(7.dp))
                                .background(MaterialTheme.colorScheme.surfaceVariant),
                            contentAlignment = Alignment.Center
                        ) {
                            androidx.compose.foundation.Image(
                                painter = painterResource(id = com.unifiedshield.R.drawable.ic_micafp_shield_logo),
                                contentDescription = "UnifiedShield Logo",
                                modifier = Modifier.size(20.dp),
                                contentScale = ContentScale.Fit
                            )
                        }
                        Text(
                            text = "UnifiedShield",
                            style = MaterialTheme.typography.titleSmall,
                            fontWeight = FontWeight.SemiBold,
                            color = MaterialTheme.colorScheme.onSurface
                        )
                    }

                    EnterpriseStatusPill(
                        text = if (isConnected) "محافظت‌شده" else "آماده اتصال",
                        type = if (isConnected) StatusPillType.SUCCESS else StatusPillType.INFO
                    )
                }

                Spacer(modifier = Modifier.height(24.dp))

                // Central Connection Button (Calm, elegant, minimum 72dp)
                Box(
                    modifier = Modifier
                        .size(112.dp)
                        .clip(CircleShape)
                        .background(
                            if (isConnected) {
                                MaterialTheme.colorScheme.primary
                            } else {
                                MaterialTheme.colorScheme.surfaceVariant
                            }
                        )
                        .border(
                            width = 1.5.dp,
                            color = if (isConnected) MaterialTheme.colorScheme.primary.copy(alpha = 0.6f) else MaterialTheme.colorScheme.outline,
                            shape = CircleShape
                        )
                        .clickable {
                            if (isConnected) onDisconnectClick() else onConnectClick()
                        }
                        .testTag("connect_toggle_button"),
                    contentAlignment = Alignment.Center
                ) {
                    Icon(
                        imageVector = if (isConnected) Icons.Default.Shield else Icons.Default.PowerSettingsNew,
                        contentDescription = if (isConnected) "قطع اتصال" else "برقراری اتصال",
                        tint = if (isConnected) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(44.dp)
                    )
                }

                Spacer(modifier = Modifier.height(18.dp))

                // Clear Human-Readable Status Text
                Text(
                    text = if (isConnected) "حفاظت فعال است" else "آماده اتصال",
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = if (isConnected) {
                        "تمام ترافیک رمزنگاری شده و ضد تحریم و DPI فعال است"
                    } else {
                        "برای برقراری اتصال امن و خصوصی کلید بالا را لمس کنید"
                    },
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = androidx.compose.ui.text.style.TextAlign.Center
                )
            }
        }

        // =========================================================
        // 2. Metrics Row (Tabular figures, clean whitespace)
        // =========================================================
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(10.dp)
        ) {
            MetricCard(
                title = "دریافت",
                value = if (isConnected) downloadSpeed else "۰ KB/s",
                subtitle = "سرعت فعلی",
                icon = Icons.Default.ArrowDownward,
                modifier = Modifier.weight(1f)
            )
            MetricCard(
                title = "ارسال",
                value = if (isConnected) uploadSpeed else "۰ KB/s",
                subtitle = "سرعت فعلی",
                icon = Icons.Default.ArrowUpward,
                modifier = Modifier.weight(1f)
            )
            MetricCard(
                title = "تأخیر (Ping)",
                value = if (isConnected) "${stealthState.estimatedLatencyMs} ms" else "-- ms",
                subtitle = "زمان پاسخ‌دهی",
                icon = Icons.Default.Timer,
                modifier = Modifier.weight(1f)
            )
        }

        // =========================================================
        // 3. Network & Profile Overview Card
        // =========================================================
        CyberHudCard {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    Box(
                        modifier = Modifier
                            .size(36.dp)
                            .clip(RoundedCornerShape(8.dp))
                            .background(MaterialTheme.colorScheme.surfaceVariant),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            Icons.Default.Wifi,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(18.dp)
                        )
                    }
                    Column {
                        Text(
                            text = "اپراتور شبکه: $ispName",
                            style = MaterialTheme.typography.titleSmall,
                            fontWeight = FontWeight.SemiBold,
                            color = MaterialTheme.colorScheme.onSurface
                        )
                        val activeProf = profileManager.getActiveProfile()
                        Text(
                            text = "پروتکل: ${activeProf.tunnelType.title} • هسته: $currentCore",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }

                EnterpriseStatusPill(
                    text = if (isNativeReady) "هسته آماده" else "حالت هیبرید",
                    type = if (isNativeReady) StatusPillType.SUCCESS else StatusPillType.INFO
                )
            }
        }

        // =========================================================
        // 4. Secondary Technical Details Accordion (For power users)
        // =========================================================
        Card(
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
            shape = RoundedCornerShape(12.dp),
            border = androidx.compose.foundation.BorderStroke(1.dp, MaterialTheme.colorScheme.outline),
            elevation = CardDefaults.cardElevation(defaultElevation = 0.dp),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(14.dp)
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable { isTechnicalDetailsExpanded = !isTechnicalDetailsExpanded },
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Icon(
                            Icons.Default.Tune,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.size(16.dp)
                        )
                        Text(
                            text = "جزئیات فنی و پایش هوشمند",
                            style = MaterialTheme.typography.labelLarge,
                            color = MaterialTheme.colorScheme.onSurface,
                            fontWeight = FontWeight.SemiBold
                        )
                    }

                    Icon(
                        imageVector = if (isTechnicalDetailsExpanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                        contentDescription = "باز/بستن جزئیات فنی",
                        tint = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(18.dp)
                    )
                }

                AnimatedVisibility(visible = isTechnicalDetailsExpanded) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(top = 12.dp),
                        verticalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        HorizontalDivider(color = MaterialTheme.colorScheme.outline)

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Text(
                                text = "شاخص استتار هوش مصنوعی:",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                            Text(
                                text = "${(stealthState.stealthScore * 100).toInt()}%",
                                style = MaterialTheme.typography.bodySmall,
                                fontWeight = FontWeight.Bold,
                                color = MaterialTheme.colorScheme.primary
                            )
                        }

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Text(
                                text = "شاخص خنثی‌سازی DPI:",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                            Text(
                                text = "${(dpiScore * 100).toInt()}%",
                                style = MaterialTheme.typography.bodySmall,
                                fontWeight = FontWeight.Bold,
                                color = MaterialTheme.colorScheme.onSurface
                            )
                        }

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Text(
                                text = "وضعیت ماژول بومی (Native Core):",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                            Text(
                                text = if (isNativeReady) "فعال (Zero-Copy)" else "استاندارد",
                                style = MaterialTheme.typography.bodySmall,
                                fontWeight = FontWeight.Medium,
                                color = MaterialTheme.colorScheme.onSurface
                            )
                        }
                    }
                }
            }
        }
    }
}
