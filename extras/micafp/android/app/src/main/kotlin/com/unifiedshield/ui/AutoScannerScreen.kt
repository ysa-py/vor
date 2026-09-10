package com.unifiedshield.ui

import androidx.compose.animation.*
import androidx.compose.animation.core.*
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
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
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.aiorchestrator.IspDpiCorrelationPredictor
import com.unifiedshield.aiorchestrator.PreemptiveRiskLevel
import com.unifiedshield.profile.ProfileManager
import com.unifiedshield.scanner.AutoScannerEngine
import com.unifiedshield.scanner.DynamicDiscoveryScale
import com.unifiedshield.scanner.IranInternetThreatLevel
import com.unifiedshield.scanner.ScanExecutionMode
import com.unifiedshield.scanner.ScanTargetResult
import com.unifiedshield.scanner.ScannerCategory

enum class ResultSortOrder(val titleFa: String) {
    BEST_SCORE("بیشترین امتیاز"),
    LOWEST_PING("کمترین پینگ"),
    LOWEST_JITTER("پایدارترین (کمترین جیتر)"),
    HIGHEST_SPEED("بالاترین سرعت")
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AutoScannerScreen() {
    val context = LocalContext.current
    val clipboardManager = LocalClipboardManager.current
    val scannerEngine = remember { AutoScannerEngine.getInstance(context) }
    val scannerState by scannerEngine.scannerState.collectAsState()
    val mlPredictor = remember { IspDpiCorrelationPredictor.getInstance() }
    val mlState by mlPredictor.correlationState.collectAsState()
    val predictionHistory by mlPredictor.predictionHistory.collectAsState()
    val profileManager = remember { ProfileManager.getInstance(context) }

    var selectedFilterCategory by remember { mutableStateOf(ScannerCategory.ALL) }
    var selectedExecutionMode by remember { mutableStateOf(ScanExecutionMode.TURBO_PARALLEL) }
    var selectedDiscoveryScale by remember { mutableStateOf(DynamicDiscoveryScale.DEEP_SWEEP) }
    var selectedOperatorFilter by remember { mutableStateOf("همه") }
    var selectedSortOrder by remember { mutableStateOf(ResultSortOrder.BEST_SCORE) }
    var searchQuery by remember { mutableStateOf("") }
    var copiedSnackVisible by remember { mutableStateOf(false) }
    var showExportDialog by remember { mutableStateOf(false) }
    var exportDialogTitle by remember { mutableStateOf("اکسپورت کانفیگ") }
    var exportDialogContent by remember { mutableStateOf("") }
    var expandedNodeId by remember { mutableStateOf<String?>(null) }

    val infiniteTransition = rememberInfiniteTransition(label = "scanner_pulse")
    val pulseAlpha by infiniteTransition.animateFloat(
        initialValue = 0.3f,
        targetValue = 1.0f,
        animationSpec = infiniteRepeatable(
            animation = tween(700, easing = LinearEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "pulseAlpha"
    )

    val operatorList = listOf("همه", "همراه اول", "ایرانسل", "رایتل", "مخابرات", "شاتل", "شبکه ملی")

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .testTag("auto_scanner_screen"),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        // Hero Scan Banner & Control Panel
        item {
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.35f)
                ),
                shape = RoundedCornerShape(20.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Row(
                            modifier = Modifier.weight(1f),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Box(
                                modifier = Modifier
                                    .size(48.dp)
                                    .clip(CircleShape)
                                    .background(Color(0x2210B981)),
                                contentAlignment = Alignment.Center
                            ) {
                                Icon(
                                    imageVector = Icons.Default.Radar,
                                    contentDescription = null,
                                    tint = Color(0xFF10B981),
                                    modifier = Modifier.size(30.dp)
                                )
                            }
                            Spacer(modifier = Modifier.width(12.dp))
                            Column {
                                Text(
                                    text = "پویشگر فوق‌پیشرفته و هوشمند شبکه ایران",
                                    fontSize = 15.sp,
                                    fontWeight = FontWeight.Bold
                                )
                                Text(
                                    text = "اسکن ماتریس ۱۰۰+ نود • ضد تله پویزنینگ و TCP RST",
                                    fontSize = 11.sp,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                        }

                        Row(
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            if (scannerState.isScanning) {
                                Badge(
                                    containerColor = Color(0xFF10B981).copy(alpha = pulseAlpha),
                                    contentColor = Color.White
                                ) {
                                    Text("در حال پویش (${scannerState.scannedCount}/${scannerState.totalTargetCount})")
                                }
                            } else {
                                Badge(containerColor = Color(0xFF10B981), contentColor = Color.White) {
                                    Text("${scannerState.cleanNodesCount} نود پاکیزه")
                                }
                            }
                            Badge(
                                containerColor = Color(0xFF8B5CF6),
                                contentColor = Color.White
                            ) {
                                Text("آنتروپی: ${scannerState.quantumEntropyScore}")
                            }
                        }
                    }

                    Spacer(modifier = Modifier.height(10.dp))

                    // Operator Detection & Gemini AI Pill Row
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(6.dp)
                    ) {
                        Surface(
                            color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.7f),
                            shape = RoundedCornerShape(10.dp),
                            modifier = Modifier.weight(1f)
                        ) {
                            Row(
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 6.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Icon(
                                    imageVector = Icons.Default.CellTower,
                                    contentDescription = null,
                                    tint = Color(0xFF3B82F6),
                                    modifier = Modifier.size(16.dp)
                                )
                                Spacer(modifier = Modifier.width(6.dp))
                                Text(
                                    text = "اپراتور: ${scannerState.detectedOperator}",
                                    fontSize = 10.sp,
                                    fontWeight = FontWeight.SemiBold,
                                    color = MaterialTheme.colorScheme.onSurface
                                )
                            }
                        }

                        Surface(
                            color = Color(0xFF8B5CF6).copy(alpha = 0.15f),
                            shape = RoundedCornerShape(10.dp),
                            modifier = Modifier.weight(1.2f)
                        ) {
                            Row(
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 6.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Icon(
                                    imageVector = Icons.Default.AutoAwesome,
                                    contentDescription = null,
                                    tint = Color(0xFF8B5CF6),
                                    modifier = Modifier.size(16.dp)
                                )
                                Spacer(modifier = Modifier.width(6.dp))
                                Text(
                                    text = "دستیار AI جمینای: پویش داینامیک فعال",
                                    fontSize = 10.sp,
                                    fontWeight = FontWeight.Bold,
                                    color = Color(0xFF7C3AED)
                                )
                            }
                        }
                    }

                    Spacer(modifier = Modifier.height(10.dp))

                    // Enterprise Quantum Zero-Touch AI Auto-Pilot Card
                    Surface(
                        color = Color(0xFF10B981).copy(alpha = 0.12f),
                        shape = RoundedCornerShape(12.dp),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Column(modifier = Modifier.padding(10.dp)) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.SpaceBetween
                            ) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Row(verticalAlignment = Alignment.CenterVertically) {
                                        Icon(
                                            imageVector = Icons.Default.Psychology,
                                            contentDescription = null,
                                            tint = Color(0xFF10B981),
                                            modifier = Modifier.size(18.dp)
                                        )
                                        Spacer(modifier = Modifier.width(6.dp))
                                        Text(
                                            text = "خلبان خودکار هوش‌مصنوعی (AI Auto-Pilot Zero-Touch)",
                                            fontSize = 11.sp,
                                            fontWeight = FontWeight.Bold,
                                            color = Color(0xFF047857)
                                        )
                                    }
                                    Text(
                                        text = scannerState.lastAiAutoPilotAction,
                                        fontSize = 9.sp,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        lineHeight = 13.sp
                                    )
                                }
                                Switch(
                                    checked = scannerState.isAutonomousZeroTouchEnabled,
                                    onCheckedChange = { scannerEngine.toggleAutonomousZeroTouch(it) },
                                    modifier = Modifier.testTag("ai_autopilot_switch")
                                )
                            }

                            HorizontalDivider(
                                modifier = Modifier.padding(vertical = 8.dp),
                                color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.5f)
                            )

                            // Battery-Saver Mode & CPU Conservation Row
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.SpaceBetween
                            ) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Row(verticalAlignment = Alignment.CenterVertically) {
                                        Icon(
                                            imageVector = Icons.Default.BatteryChargingFull,
                                            contentDescription = null,
                                            tint = if (scannerState.isBatterySaverModeActive) Color(0xFFF59E0B) else Color(0xFF3B82F6),
                                            modifier = Modifier.size(18.dp)
                                        )
                                        Spacer(modifier = Modifier.width(6.dp))
                                        Text(
                                            text = "حالت ذخیره انرژی و کاهش مصرف پردازنده (Battery Saver)",
                                            fontSize = 11.sp,
                                            fontWeight = FontWeight.Bold,
                                            color = if (scannerState.isBatterySaverModeActive) Color(0xFFB45309) else MaterialTheme.colorScheme.onSurface
                                        )
                                    }
                                    Text(
                                        text = if (scannerState.isBatterySaverModeActive)
                                            "فرکانس بررسی پس‌زمینه کاهش یافته • مصرف CPU و باتری در کمترین حد ممکن"
                                        else
                                            "حالت عملکرد پرسرعت با بررسی بلادرنگ هر ۶ ثانیه",
                                        fontSize = 9.sp,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        lineHeight = 13.sp
                                    )
                                }
                                Switch(
                                    checked = scannerState.isBatterySaverModeActive,
                                    onCheckedChange = { scannerEngine.toggleBatterySaverMode(it) },
                                    modifier = Modifier.testTag("battery_saver_switch")
                                )
                            }
                        }
                    }

                    Spacer(modifier = Modifier.height(10.dp))

                    // Real-Time Path Validation & DPI Resilience Indicator
                    Surface(
                        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                        shape = RoundedCornerShape(10.dp),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Row(
                            modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Icon(
                                imageVector = Icons.Default.Sync,
                                contentDescription = null,
                                tint = Color(0xFF10B981),
                                modifier = Modifier.size(16.dp)
                            )
                            Spacer(modifier = Modifier.width(6.dp))
                            Text(
                                text = "اعتبارسنجی بلادرنگ مسیر (${scannerState.realTimeValidationCount} بار): ${scannerState.lastPathValidationStatus}",
                                fontSize = 9.5.sp,
                                color = MaterialTheme.colorScheme.onSurface,
                                fontWeight = FontWeight.Medium,
                                maxLines = 1
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(10.dp))

                    // Execution Mode Selector
                    Text(
                        text = "حالت اجرای پویش هوشمند و تخصصی:",
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.primary
                    )

                    Spacer(modifier = Modifier.height(6.dp))

                    LazyRow(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(6.dp)
                    ) {
                        items(ScanExecutionMode.values()) { mode ->
                            val isSelected = selectedExecutionMode == mode
                            Surface(
                                modifier = Modifier
                                    .clickable { selectedExecutionMode = mode },
                                shape = RoundedCornerShape(8.dp),
                                color = if (isSelected) Color(0xFF10B981) else MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)
                            ) {
                                Box(
                                    modifier = Modifier.padding(vertical = 6.dp, horizontal = 8.dp),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Text(
                                        text = mode.titleFa.split(" (").first(),
                                        fontSize = 10.sp,
                                        fontWeight = if (isSelected) FontWeight.Bold else FontWeight.Normal,
                                        color = if (isSelected) Color.White else MaterialTheme.colorScheme.onSurface
                                    )
                                }
                            }
                        }
                    }

                    Spacer(modifier = Modifier.height(10.dp))

                    // Dynamic Scale Selector
                    Text(
                        text = "مقیاس کاوش پویا و تولید داینامیک آی‌پی:",
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.primary
                    )

                    Spacer(modifier = Modifier.height(6.dp))

                    LazyRow(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(6.dp)
                    ) {
                        items(DynamicDiscoveryScale.values()) { scale ->
                            val isSelected = selectedDiscoveryScale == scale
                            Surface(
                                modifier = Modifier
                                    .clickable { selectedDiscoveryScale = scale },
                                shape = RoundedCornerShape(8.dp),
                                color = if (isSelected) Color(0xFF8B5CF6) else MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)
                            ) {
                                Box(
                                    modifier = Modifier.padding(vertical = 6.dp, horizontal = 8.dp),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Text(
                                        text = "${scale.titleFa.split(" (").first()} (${scale.count})",
                                        fontSize = 10.sp,
                                        fontWeight = if (isSelected) FontWeight.Bold else FontWeight.Normal,
                                        color = if (isSelected) Color.White else MaterialTheme.colorScheme.onSurface
                                    )
                                }
                            }
                        }
                    }

                    Spacer(modifier = Modifier.height(10.dp))

                    Text(
                        text = scannerState.statusMessage,
                        fontSize = 11.sp,
                        color = MaterialTheme.colorScheme.primary,
                        fontWeight = FontWeight.Medium,
                        lineHeight = 16.sp
                    )

                    if (scannerState.isScanning) {
                        Spacer(modifier = Modifier.height(8.dp))
                        LinearProgressIndicator(
                            progress = { scannerState.scanProgress },
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(6.dp)
                                .clip(RoundedCornerShape(3.dp)),
                            color = Color(0xFF10B981)
                        )
                    }

                    Spacer(modifier = Modifier.height(12.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Button(
                            onClick = {
                                scannerEngine.startFullAutoScan(
                                    category = selectedFilterCategory,
                                    mode = selectedExecutionMode,
                                    scale = selectedDiscoveryScale
                                ) { bestNode ->
                                    profileManager.autoApplyFromScanner(bestNode)
                                }
                            },
                            enabled = !scannerState.isScanning,
                            modifier = Modifier
                                .weight(1.3f)
                                .testTag("start_auto_scan_btn"),
                            colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF10B981)),
                            shape = RoundedCornerShape(12.dp)
                        ) {
                            if (scannerState.isScanning) {
                                CircularProgressIndicator(modifier = Modifier.size(18.dp), color = Color.White, strokeWidth = 2.dp)
                                Spacer(modifier = Modifier.width(6.dp))
                                Text("در حال اسکن پویا...", fontSize = 11.sp)
                            } else {
                                Icon(Icons.Default.Bolt, contentDescription = null, modifier = Modifier.size(18.dp))
                                Spacer(modifier = Modifier.width(6.dp))
                                Text("اسکن پویا و اتصال خودکار", fontSize = 11.sp, fontWeight = FontWeight.Bold)
                            }
                        }

                        Button(
                            onClick = {
                                scannerEngine.startAutonomousAiBlackoutConnect()
                            },
                            enabled = !scannerState.isScanning,
                            modifier = Modifier
                                .weight(1f)
                                .testTag("ai_blackout_solver_btn"),
                            colors = ButtonDefaults.buttonColors(containerColor = Color(0xFFDC2626)),
                            shape = RoundedCornerShape(12.dp)
                        ) {
                            Icon(Icons.Default.Shield, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(modifier = Modifier.width(6.dp))
                            Text("میانبر خاموشی AI", fontSize = 11.sp, fontWeight = FontWeight.Bold)
                        }
                    }
                }
            }
        }

        // Live Telemetry Grid (4 Cards)
        item {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(6.dp)
            ) {
                Card(
                    modifier = Modifier.weight(1f),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f)),
                    shape = RoundedCornerShape(12.dp)
                ) {
                    Column(modifier = Modifier.padding(8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                        Text("کمترین پینگ", fontSize = 9.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Text("${scannerState.minLatencyMs}ms", fontSize = 14.sp, fontWeight = FontWeight.Bold, color = Color(0xFF10B981))
                    }
                }
                Card(
                    modifier = Modifier.weight(1f),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f)),
                    shape = RoundedCornerShape(12.dp)
                ) {
                    Column(modifier = Modifier.padding(8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                        Text("نودهای سالم", fontSize = 9.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Text("${scannerState.cleanNodesCount}", fontSize = 14.sp, fontWeight = FontWeight.Bold, color = Color(0xFF3B82F6))
                    }
                }
                Card(
                    modifier = Modifier.weight(1f),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f)),
                    shape = RoundedCornerShape(12.dp)
                ) {
                    Column(modifier = Modifier.padding(8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                        Text("عبور هوش‌مصنوعی", fontSize = 9.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Text("${scannerState.aiConfidenceRate}%", fontSize = 14.sp, fontWeight = FontWeight.Bold, color = Color(0xFF8B5CF6))
                    }
                }
                Card(
                    modifier = Modifier.weight(1f),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f)),
                    shape = RoundedCornerShape(12.dp)
                ) {
                    Column(modifier = Modifier.padding(8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                        Text("حداکثر سرعت", fontSize = 9.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Text(scannerState.maxBandwidthScore, fontSize = 14.sp, fontWeight = FontWeight.Bold, color = Color(0xFF0D9488))
                    }
                }
            }
        }

        // Client-Side Machine Learning Model Card: ISP Latency Trend & DPI Spike Correlation
        item {
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.6f)
                ),
                shape = RoundedCornerShape(16.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(
                                imageVector = Icons.Default.Timeline,
                                contentDescription = null,
                                tint = Color(0xFF8B5CF6),
                                modifier = Modifier.size(20.dp)
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Column {
                                Text(
                                    text = "مدل یادگیری ماشین کلاینت: تحلیل روند پینگ و اسپایک‌های DPI",
                                    fontSize = 11.5.sp,
                                    fontWeight = FontWeight.Bold,
                                    color = MaterialTheme.colorScheme.onSurface
                                )
                                Text(
                                    text = "سوییچ پیش‌دستانه هسته قبل از وقوع قطعی ارتباط (Preemptive Zero-Drop)",
                                    fontSize = 9.5.sp,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                        }

                        Badge(
                            containerColor = Color(mlState.riskLevel.badgeColorHex),
                            contentColor = Color.White
                        ) {
                            Text(
                                text = mlState.riskLevel.labelFa,
                                fontSize = 9.sp,
                                fontWeight = FontWeight.Bold,
                                modifier = Modifier.padding(horizontal = 4.dp, vertical = 2.dp)
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(10.dp))

                    // ML Metrics Matrix
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(6.dp)
                    ) {
                        Surface(
                            shape = RoundedCornerShape(8.dp),
                            color = MaterialTheme.colorScheme.surface.copy(alpha = 0.8f),
                            modifier = Modifier.weight(1f)
                        ) {
                            Column(modifier = Modifier.padding(6.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                                Text("ریسک قطعی DPI", fontSize = 8.5.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text("${(mlState.dropRiskProbability * 100).toInt()}%", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = if (mlState.dropRiskProbability > 0.5f) Color(0xFFEF4444) else Color(0xFF10B981))
                            }
                        }
                        Surface(
                            shape = RoundedCornerShape(8.dp),
                            color = MaterialTheme.colorScheme.surface.copy(alpha = 0.8f),
                            modifier = Modifier.weight(1f)
                        ) {
                            Column(modifier = Modifier.padding(6.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                                Text("شتاب پینگ dRTT/dt", fontSize = 8.5.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text("${Math.round(mlState.rttVelocity * 10.0) / 10.0} ms/s", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFF3B82F6))
                            }
                        }
                        Surface(
                            shape = RoundedCornerShape(8.dp),
                            color = MaterialTheme.colorScheme.surface.copy(alpha = 0.8f),
                            modifier = Modifier.weight(1f)
                        ) {
                            Column(modifier = Modifier.padding(6.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                                Text("اسپایک DPI (30s)", fontSize = 8.5.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text("${mlState.dpiBurstCount30s}", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFFF59E0B))
                            }
                        }
                        Surface(
                            shape = RoundedCornerShape(8.dp),
                            color = MaterialTheme.colorScheme.surface.copy(alpha = 0.8f),
                            modifier = Modifier.weight(1.3f)
                        ) {
                            Column(modifier = Modifier.padding(6.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                                Text("هسته هدف پیش‌دستانه", fontSize = 8.5.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text(mlState.recommendedTargetCore.split(" ").first(), fontSize = 11.sp, fontWeight = FontWeight.Bold, color = Color(0xFF8B5CF6), maxLines = 1)
                            }
                        }
                    }

                    if (predictionHistory.isNotEmpty()) {
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = "آخرین سوییچ پیشگیرانه ML: [${predictionHistory.first().timestampStr}] سوییچ به '${predictionHistory.first().switchedToCore}' به علت ${predictionHistory.first().reason}",
                            fontSize = 9.sp,
                            color = Color(0xFF10B981),
                            lineHeight = 13.sp
                        )
                    }
                }
            }
        }

        // AI Blackout / Intranet Resilience Status Card
        if (scannerState.isInternationalBlackoutDetected) {
            item {
                Card(
                    colors = CardDefaults.cardColors(containerColor = Color(0xFFEF4444).copy(alpha = 0.15f)),
                    shape = RoundedCornerShape(14.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(
                        modifier = Modifier.padding(12.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Icon(
                            imageVector = Icons.Default.Shield,
                            contentDescription = null,
                            tint = Color(0xFFEF4444),
                            modifier = Modifier.size(28.dp)
                        )
                        Spacer(modifier = Modifier.width(10.dp))
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = "حالت استقامت خاموشی بین‌الملل فعال است (AI Blackout Mode)",
                                fontSize = 12.sp,
                                fontWeight = FontWeight.Bold,
                                color = Color(0xFFEF4444)
                            )
                            Text(
                                text = "مسیریابی اتوماتیک از طریق رله‌های داخلی NIN و خنثی‌سازی تراتلینگ اینترنت ملی",
                                fontSize = 10.sp,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }
            }
        }

        // Auto-Healing & Clean Nodes Export
        item {
            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(14.dp)
            ) {
                Column(modifier = Modifier.padding(12.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = "سیستم خودکار ترمیم و تعویض مداوم (Auto-Healing)",
                                fontSize = 12.sp,
                                fontWeight = FontWeight.SemiBold
                            )
                            Text(
                                text = "جابجایی خودکار به نود پشتیبان در صورت افزایش تاخیر یا اختلال شبکه",
                                fontSize = 10.sp,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                        Switch(
                            checked = scannerState.isContinuousAutoHealingEnabled,
                            onCheckedChange = { scannerEngine.toggleContinuousAutoHealing(it) },
                            modifier = Modifier.testTag("auto_healing_switch")
                        )
                    }

                    Spacer(modifier = Modifier.height(8.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        OutlinedButton(
                            onClick = {
                                exportDialogTitle = "📦 خروجی Sing-Box (JSON)"
                                exportDialogContent = scannerEngine.exportSingBoxFullConfig(scannerState.results)
                                showExportDialog = true
                            },
                            contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                            shape = RoundedCornerShape(8.dp),
                            modifier = Modifier.weight(1f).testTag("export_singbox_btn")
                        ) {
                            Text("Sing-Box", fontSize = 10.sp, fontWeight = FontWeight.Bold)
                        }

                        OutlinedButton(
                            onClick = {
                                exportDialogTitle = "🛡️ خروجی Xray-Core / V2Ray (JSON)"
                                exportDialogContent = scannerEngine.exportXrayFullConfig(scannerState.results)
                                showExportDialog = true
                            },
                            contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                            shape = RoundedCornerShape(8.dp),
                            modifier = Modifier.weight(1f).testTag("export_xray_btn")
                        ) {
                            Text("Xray/V2Ray", fontSize = 10.sp, fontWeight = FontWeight.Bold)
                        }

                        OutlinedButton(
                            onClick = {
                                exportDialogTitle = "⚡ خروجی Clash Meta / Mihomo (YAML)"
                                exportDialogContent = scannerEngine.exportClashMetaFullYaml(scannerState.results)
                                showExportDialog = true
                            },
                            contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                            shape = RoundedCornerShape(8.dp),
                            modifier = Modifier.weight(1f).testTag("export_clash_btn")
                        ) {
                            Text("Clash Meta", fontSize = 10.sp, fontWeight = FontWeight.Bold)
                        }

                        IconButton(
                            onClick = {
                                val cleanUris = scannerEngine.exportAllRawUris(scannerState.results)
                                clipboardManager.setText(AnnotatedString(cleanUris))
                                copiedSnackVisible = true
                            },
                            modifier = Modifier.size(36.dp).testTag("copy_all_uris_btn")
                        ) {
                            Icon(Icons.Default.ContentCopy, contentDescription = "کپی لینک‌های خام", modifier = Modifier.size(16.dp))
                        }
                    }
                }
            }
        }

        // Search Bar
        item {
            OutlinedTextField(
                value = searchQuery,
                onValueChange = { searchQuery = it },
                modifier = Modifier.fillMaxWidth().testTag("scanner_search_input"),
                placeholder = { Text("جستجوی نود، دامنه، اپراتور، تکنولوژی یا پروتکل...", fontSize = 11.sp) },
                leadingIcon = { Icon(Icons.Default.Search, contentDescription = null, modifier = Modifier.size(18.dp)) },
                trailingIcon = {
                    if (searchQuery.isNotEmpty()) {
                        IconButton(onClick = { searchQuery = "" }) {
                            Icon(Icons.Default.Clear, contentDescription = null, modifier = Modifier.size(18.dp))
                        }
                    }
                },
                singleLine = true,
                shape = RoundedCornerShape(12.dp)
            )
        }

        // Operator Filter Chips
        item {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(
                    text = "فیلتر بر اساس اپراتور:",
                    fontSize = 11.sp,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.primary
                )

                LazyRow(
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    items(operatorList) { op ->
                        FilterChip(
                            selected = selectedOperatorFilter == op,
                            onClick = { selectedOperatorFilter = op },
                            label = { Text(op, fontSize = 10.sp) }
                        )
                    }
                }
            }
        }

        // Category Filter Chips
        item {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(
                    text = "دسته‌بندی پروتکل و فناوری (${ScannerCategory.values().size} دسته):",
                    fontSize = 11.sp,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.primary
                )

                LazyRow(
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    items(ScannerCategory.values()) { cat ->
                        FilterChip(
                            selected = selectedFilterCategory == cat,
                            onClick = { selectedFilterCategory = cat },
                            label = { Text(cat.label, fontSize = 10.sp) }
                        )
                    }
                }
            }
        }

        // Sort Options Row
        item {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text(
                    text = "مرتب‌سازی نتایج:",
                    fontSize = 11.sp,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.primary
                )

                LazyRow(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    items(ResultSortOrder.values()) { sort ->
                        Surface(
                            modifier = Modifier
                                .clip(RoundedCornerShape(6.dp))
                                .clickable { selectedSortOrder = sort },
                            color = if (selectedSortOrder == sort) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)
                        ) {
                            Text(
                                text = sort.titleFa,
                                fontSize = 9.sp,
                                color = if (selectedSortOrder == sort) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurface,
                                modifier = Modifier.padding(horizontal = 6.dp, vertical = 3.dp)
                            )
                        }
                    }
                }
            }
        }

        // Filter and Sort results
        val filteredResults = scannerState.results.filter {
            val matchesCategory = (selectedFilterCategory == ScannerCategory.ALL || it.category == selectedFilterCategory)
            val matchesOperator = (selectedOperatorFilter == "همه" || it.operatorAffinity.contains(selectedOperatorFilter) || it.target.contains(selectedOperatorFilter))
            val matchesSearch = searchQuery.isEmpty() ||
                    it.target.contains(searchQuery, ignoreCase = true) ||
                    it.extraInfo.contains(searchQuery, ignoreCase = true) ||
                    it.dpiBypassTechnique.contains(searchQuery, ignoreCase = true) ||
                    it.operatorAffinity.contains(searchQuery, ignoreCase = true)
            matchesCategory && matchesOperator && matchesSearch
        }.let { list ->
            when (selectedSortOrder) {
                ResultSortOrder.BEST_SCORE -> list.sortedByDescending { it.score }
                ResultSortOrder.LOWEST_PING -> list.sortedBy { it.latencyMs }
                ResultSortOrder.LOWEST_JITTER -> list.sortedBy { it.jitterMs }
                ResultSortOrder.HIGHEST_SPEED -> list.sortedByDescending { it.score }
            }
        }

        items(filteredResults, key = { it.id }) { item ->
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(
                        width = if (item.isAutoApplied) 2.dp else 1.dp,
                        color = if (item.isAutoApplied) Color(0xFF10B981) else MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.35f),
                        shape = RoundedCornerShape(14.dp)
                    )
                    .testTag("scan_result_${item.id}"),
                colors = CardDefaults.cardColors(
                    containerColor = if (item.isAutoApplied) Color(0x0F10B981) else MaterialTheme.colorScheme.surface
                ),
                shape = RoundedCornerShape(14.dp)
            ) {
                Column(modifier = Modifier.padding(12.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.weight(1f)) {
                            Icon(
                                imageVector = if (item.isClean) Icons.Default.CheckCircle else Icons.Default.Warning,
                                contentDescription = null,
                                tint = if (item.isClean) Color(0xFF10B981) else Color(0xFFEF4444),
                                modifier = Modifier.size(18.dp)
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(item.target, fontSize = 12.sp, fontWeight = FontWeight.Bold, maxLines = 1)
                        }

                        if (item.isAutoApplied) {
                            Badge(containerColor = Color(0xFF10B981), contentColor = Color.White) {
                                Text("فعال و در حال استفاده")
                            }
                        } else {
                            Badge(containerColor = MaterialTheme.colorScheme.surfaceVariant) {
                                Text("امتیاز: ${item.score}/100")
                            }
                        }
                    }

                    Spacer(modifier = Modifier.height(4.dp))
                    Text(item.extraInfo, fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)

                    Spacer(modifier = Modifier.height(6.dp))

                    // Tech Badges Row
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(4.dp)
                    ) {
                        Surface(
                            color = Color(0xFF3B82F6).copy(alpha = 0.12f),
                            shape = RoundedCornerShape(6.dp)
                        ) {
                            Text(
                                text = item.operatorAffinity,
                                fontSize = 9.sp,
                                color = Color(0xFF2563EB),
                                fontWeight = FontWeight.SemiBold,
                                modifier = Modifier.padding(horizontal = 5.dp, vertical = 2.dp)
                            )
                        }
                        Surface(
                            color = Color(0xFF8B5CF6).copy(alpha = 0.12f),
                            shape = RoundedCornerShape(6.dp)
                        ) {
                            Text(
                                text = item.dpiBypassTechnique,
                                fontSize = 9.sp,
                                color = Color(0xFF7C3AED),
                                modifier = Modifier.padding(horizontal = 5.dp, vertical = 2.dp)
                            )
                        }
                        Surface(
                            color = Color(0xFF10B981).copy(alpha = 0.12f),
                            shape = RoundedCornerShape(6.dp)
                        ) {
                            Text(
                                text = "MSS ${item.mtuClampingValue}B",
                                fontSize = 9.sp,
                                color = Color(0xFF059669),
                                modifier = Modifier.padding(horizontal = 5.dp, vertical = 2.dp)
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(8.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Row(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
                            Text("پینگ: ${item.latencyMs}ms", fontSize = 10.sp, fontWeight = FontWeight.Bold, color = Color(0xFF10B981))
                            Text("جیتر: ±${item.jitterMs}ms", fontSize = 9.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text("سرعت: ${item.bandwidthEstimate}", fontSize = 9.sp, color = Color(0xFF0D9488), fontWeight = FontWeight.Medium)
                        }

                        Row(horizontalArrangement = Arrangement.spacedBy(4.dp), verticalAlignment = Alignment.CenterVertically) {
                            IconButton(
                                onClick = {
                                    expandedNodeId = if (expandedNodeId == item.id) null else item.id
                                },
                                modifier = Modifier.size(28.dp)
                            ) {
                                Icon(
                                    imageVector = if (expandedNodeId == item.id) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                                    contentDescription = "اطلاعات تشخیصی",
                                    modifier = Modifier.size(16.dp),
                                    tint = MaterialTheme.colorScheme.primary
                                )
                            }

                            if (!item.isAutoApplied) {
                                OutlinedButton(
                                    onClick = {
                                        scannerEngine.applyScannedNode(item.id)
                                        profileManager.autoApplyFromScanner(item)
                                    },
                                    contentPadding = PaddingValues(horizontal = 8.dp, vertical = 2.dp),
                                    shape = RoundedCornerShape(8.dp),
                                    modifier = Modifier.height(30.dp).testTag("apply_node_${item.id}")
                                ) {
                                    Text("جایگذاری و اتصال", fontSize = 10.sp)
                                }
                            }
                        }
                    }

                    // Expandable Diagnostic Details
                    AnimatedVisibility(visible = expandedNodeId == item.id) {
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(top = 8.dp)
                                .background(MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.35f), RoundedCornerShape(8.dp))
                                .padding(8.dp)
                        ) {
                            Text(
                                text = "🔬 پارامترهای پیشرفته هسته و تشخیص عمیق شبکه:",
                                fontSize = 10.sp,
                                fontWeight = FontWeight.Bold,
                                color = MaterialTheme.colorScheme.primary
                            )
                            Spacer(modifier = Modifier.height(4.dp))
                            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                                Text("TCP Handshake: ${item.tcpHandshakeMs}ms", fontSize = 9.sp)
                                Text("TLS Negotiate: ${item.tlsNegotiateMs}ms", fontSize = 9.sp)
                            }
                            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                                Text("ALPN: ${item.tlsAlpn.joinToString(", ")}", fontSize = 9.sp)
                                Text("uTLS: ${item.uTlsFingerprint}", fontSize = 9.sp)
                            }
                            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                                Text("QUIC / HTTP3: ${if (item.quicSupport) "فعال" else "غیرفعال"}", fontSize = 9.sp)
                                Text("Multiplex MUX: ${item.muxConcurrency} Streams", fontSize = 9.sp)
                            }
                            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                                Text("SNI Whitelist: ${item.sniHostname}", fontSize = 9.sp)
                                Text("پایداری اتصال: ${item.stabilityScore}%", fontSize = 9.sp, fontWeight = FontWeight.SemiBold, color = Color(0xFF10B981))
                            }

                            Spacer(modifier = Modifier.height(6.dp))

                            // Individual Export Action Buttons for this node
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(4.dp)
                            ) {
                                OutlinedButton(
                                    onClick = {
                                        clipboardManager.setText(AnnotatedString(scannerEngine.exportSingBoxOutbound(item)))
                                        copiedSnackVisible = true
                                    },
                                    contentPadding = PaddingValues(horizontal = 4.dp, vertical = 2.dp),
                                    shape = RoundedCornerShape(6.dp),
                                    modifier = Modifier.weight(1f).height(26.dp)
                                ) {
                                    Text("Sing-Box", fontSize = 8.sp)
                                }
                                OutlinedButton(
                                    onClick = {
                                        clipboardManager.setText(AnnotatedString(scannerEngine.exportXrayOutbound(item)))
                                        copiedSnackVisible = true
                                    },
                                    contentPadding = PaddingValues(horizontal = 4.dp, vertical = 2.dp),
                                    shape = RoundedCornerShape(6.dp),
                                    modifier = Modifier.weight(1f).height(26.dp)
                                ) {
                                    Text("Xray", fontSize = 8.sp)
                                }
                                OutlinedButton(
                                    onClick = {
                                        clipboardManager.setText(AnnotatedString(scannerEngine.exportClashMetaProxy(item)))
                                        copiedSnackVisible = true
                                    },
                                    contentPadding = PaddingValues(horizontal = 4.dp, vertical = 2.dp),
                                    shape = RoundedCornerShape(6.dp),
                                    modifier = Modifier.weight(1f).height(26.dp)
                                ) {
                                    Text("Clash", fontSize = 8.sp)
                                }
                                OutlinedButton(
                                    onClick = {
                                        clipboardManager.setText(AnnotatedString(scannerEngine.exportRawUri(item)))
                                        copiedSnackVisible = true
                                    },
                                    contentPadding = PaddingValues(horizontal = 4.dp, vertical = 2.dp),
                                    shape = RoundedCornerShape(6.dp),
                                    modifier = Modifier.weight(1f).height(26.dp)
                                ) {
                                    Text("URI", fontSize = 8.sp)
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Export Dialog Modal
    if (showExportDialog) {
        AlertDialog(
            onDismissRequest = { showExportDialog = false },
            title = {
                Text(exportDialogTitle, fontSize = 14.sp, fontWeight = FontWeight.Bold)
            },
            text = {
                Column(modifier = Modifier.fillMaxWidth()) {
                    Text(
                        text = "کانفیگ استاندارد آماده استفاده در کلاینت‌های هسته‌ای Sing-Box / Xray / Clash Meta:",
                        fontSize = 11.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Surface(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(260.dp),
                        shape = RoundedCornerShape(8.dp),
                        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.6f)
                    ) {
                        LazyColumn(modifier = Modifier.padding(8.dp)) {
                            item {
                                Text(
                                    text = exportDialogContent,
                                    fontSize = 10.sp,
                                    fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace
                                )
                            }
                        }
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        clipboardManager.setText(AnnotatedString(exportDialogContent))
                        showExportDialog = false
                        copiedSnackVisible = true
                    }
                ) {
                    Icon(Icons.Default.ContentCopy, contentDescription = null, modifier = Modifier.size(14.dp))
                    Spacer(modifier = Modifier.width(4.dp))
                    Text("کپی کانفیگ", fontSize = 11.sp)
                }
            },
            dismissButton = {
                TextButton(onClick = { showExportDialog = false }) {
                    Text("بستن", fontSize = 11.sp)
                }
            }
        )
    }
}
