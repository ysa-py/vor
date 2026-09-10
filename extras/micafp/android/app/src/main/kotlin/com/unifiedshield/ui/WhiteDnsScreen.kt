package com.unifiedshield.ui

import androidx.compose.animation.*
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.whitedns.*

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WhiteDnsScreen() {
    val context = LocalContext.current
    val engine = remember { WhiteDnsScannerEngine.getInstance(context) }
    val state by engine.state.collectAsState()

    var showExportDialog by remember { mutableStateOf(false) }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .testTag("whitedns_screen"),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        // Hero Header Card
        item {
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surface
                ),
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.primary.copy(alpha = 0.3f)),
                shape = RoundedCornerShape(20.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .background(
                            Brush.linearGradient(
                                colors = listOf(
                                    Color(0xFF0284C7).copy(alpha = 0.18f),
                                    Color(0xFF0F172A).copy(alpha = 0.90f)
                                )
                            )
                        )
                        .padding(18.dp)
                ) {
                    Column {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Box(
                                    modifier = Modifier
                                        .size(46.dp)
                                        .clip(CircleShape)
                                        .background(Color(0xFF0284C7).copy(alpha = 0.25f)),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Icon(
                                        imageVector = Icons.Default.Radar,
                                        contentDescription = null,
                                        tint = Color(0xFF38BDF8),
                                        modifier = Modifier.size(28.dp)
                                    )
                                }
                                Spacer(modifier = Modifier.width(12.dp))
                                Column {
                                    Text(
                                        text = "WhiteDNS Engine 1.3.7",
                                        fontSize = 17.sp,
                                        fontWeight = FontWeight.ExtraBold,
                                        color = Color.White
                                    )
                                    Text(
                                        text = "Clean-IP Discovery & uTLS Resolver Scanner",
                                        fontSize = 11.sp,
                                        color = Color(0xFFBAE6FD)
                                    )
                                }
                            }

                            Badge(containerColor = Color(0xFF0284C7), contentColor = Color.White) {
                                Text("v1.3.7 Native Go", fontSize = 11.sp, fontWeight = FontWeight.Bold)
                            }
                        }

                        Spacer(modifier = Modifier.height(12.dp))

                        Text(
                            text = "موتور اسکنر پیشرفته کشف آی‌پی‌های تمیز (Clean-IP) و رِزولورهای بدون فیلتر با هندشیک شبیه‌ساز uTLS کروم، اعتبارسنجی عبور تونل TXT، بررسی دستکاری NXDOMAIN و استخراج خودکار بر اساس ASN.",
                            fontSize = 12.sp,
                            color = Color(0xFFE2E8F0),
                            lineHeight = 18.sp
                        )

                        Spacer(modifier = Modifier.height(10.dp))

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            AssistChip(
                                onClick = { engine.updateScanType(WhiteDnsScanType.DNS_RESOLVER) },
                                label = { Text("uTLS DoT/DoH/UDP", fontSize = 10.sp) },
                                leadingIcon = { Icon(Icons.Default.Dns, contentDescription = null, modifier = Modifier.size(14.dp)) }
                            )
                            AssistChip(
                                onClick = { engine.updateScanType(WhiteDnsScanType.IP_CIDR) },
                                label = { Text("CIDR Expansion", fontSize = 10.sp) },
                                leadingIcon = { Icon(Icons.Default.Lan, contentDescription = null, modifier = Modifier.size(14.dp)) }
                            )
                            AssistChip(
                                onClick = { showExportDialog = true },
                                label = { Text("خروجی اسناد", fontSize = 10.sp) },
                                leadingIcon = { Icon(Icons.Default.FileDownload, contentDescription = null, modifier = Modifier.size(14.dp)) }
                            )
                        }
                    }
                }
            }
        }

        // Scanner Configurations & Controls Card
        item {
            Card(
                shape = RoundedCornerShape(18.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.35f)),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text("نوع اسکن (Scan Type):", fontSize = 13.sp, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(6.dp)
                    ) {
                        FilterChip(
                            selected = state.selectedScanType == WhiteDnsScanType.DNS_RESOLVER,
                            onClick = { engine.updateScanType(WhiteDnsScanType.DNS_RESOLVER) },
                            label = { Text("رِزولور DNS (uTLS)", fontSize = 11.sp) }
                        )
                        FilterChip(
                            selected = state.selectedScanType == WhiteDnsScanType.IP_CIDR,
                            onClick = { engine.updateScanType(WhiteDnsScanType.IP_CIDR) },
                            label = { Text("رنج IP/CIDR", fontSize = 11.sp) }
                        )
                        FilterChip(
                            selected = state.selectedScanType == WhiteDnsScanType.SNI_SCANNER,
                            onClick = { engine.updateScanType(WhiteDnsScanType.SNI_SCANNER) },
                            label = { Text("SNI/TLS 1.3", fontSize = 11.sp) }
                        )
                    }

                    // Depth & ASN Preset Row
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text("عمق اسکن:", fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
                        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                            FilterChip(
                                selected = state.scanDepth == WhiteDnsScanDepth.FAST,
                                onClick = { engine.updateScanDepth(WhiteDnsScanDepth.FAST) },
                                label = { Text("Fast (سریع)", fontSize = 10.sp) }
                            )
                            FilterChip(
                                selected = state.scanDepth == WhiteDnsScanDepth.FULL,
                                onClick = { engine.updateScanDepth(WhiteDnsScanDepth.FULL) },
                                label = { Text("Full (کامل)", fontSize = 10.sp) }
                            )
                        }
                    }

                    // Input CIDR / Target Domain
                    OutlinedTextField(
                        value = state.inputCidrOrDomain,
                        onValueChange = { engine.updateInputCidr(it) },
                        label = { Text("رنج CIDR یا دامنه هدف") },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                        leadingIcon = { Icon(Icons.Default.Language, contentDescription = null) }
                    )

                    // ASN Quick Selector Chips
                    Text("انتخاب پیش‌تنظیم ASN پاکیزه (Embedded ASNs):", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(4.dp)
                    ) {
                        engine.embeddedAsnDatasets.take(3).forEach { asn ->
                            AssistChip(
                                onClick = { engine.updateInputCidr(asn.defaultCidrs.first()) },
                                label = { Text("${asn.asn} (${asn.organization.split(" ").first()})", fontSize = 10.sp) }
                            )
                        }
                    }

                    // Workers Slider
                    Column {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Text("تعداد کارهای همزمان (Concurrency Workers):", fontSize = 11.sp)
                            Text("${state.concurrencyWorkers} Threads", fontSize = 11.sp, fontWeight = FontWeight.Bold, color = Color(0xFF0284C7))
                        }
                        Slider(
                            value = state.concurrencyWorkers.toFloat(),
                            onValueChange = { engine.updateConcurrency(it.toInt()) },
                            valueRange = 4f..64f
                        )
                    }

                    // Action Buttons Row (Start / Pause / Stop)
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Button(
                            onClick = { engine.startScan() },
                            colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF0284C7)),
                            modifier = Modifier.weight(1.5f).testTag("whitedns_start_scan")
                        ) {
                            Icon(
                                imageVector = if (state.isPaused) Icons.Default.PlayArrow else Icons.Default.PlayArrow,
                                contentDescription = null,
                                modifier = Modifier.size(18.dp)
                            )
                            Spacer(modifier = Modifier.width(6.dp))
                            Text(if (state.isPaused) "ادامه اسکن" else "شروع اسکن WhiteDNS", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                        }

                        if (state.isScanning && !state.isPaused) {
                            OutlinedButton(
                                onClick = { engine.pauseScan() },
                                modifier = Modifier.weight(1f)
                            ) {
                                Icon(Icons.Default.Pause, contentDescription = null, modifier = Modifier.size(18.dp))
                                Spacer(modifier = Modifier.width(4.dp))
                                Text("توقف موقت", fontSize = 11.sp)
                            }
                        }

                        if (state.isScanning || state.isPaused) {
                            OutlinedButton(
                                onClick = { engine.stopScan() },
                                colors = ButtonDefaults.outlinedButtonColors(contentColor = Color(0xFFEF4444)),
                                modifier = Modifier.weight(0.9f)
                            ) {
                                Icon(Icons.Default.Stop, contentDescription = null, modifier = Modifier.size(18.dp), tint = Color(0xFFEF4444))
                                Spacer(modifier = Modifier.width(4.dp))
                                Text("لغو", fontSize = 11.sp)
                            }
                        }
                    }
                }
            }
        }

        // Live Scanning Metrics & Progress
        if (state.isScanning || state.progressPercentage > 0f) {
            item {
                Card(
                    colors = CardDefaults.cardColors(containerColor = Color(0xFF0F172A)),
                    shape = RoundedCornerShape(16.dp),
                    border = BorderStroke(1.dp, Color(0xFF0284C7).copy(alpha = 0.5f)),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text("وضعیت زنده اسکنر WhiteDNS", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFF38BDF8))
                            Text("${(state.progressPercentage * 100).toInt()}%", fontSize = 13.sp, fontWeight = FontWeight.ExtraBold, color = Color.White)
                        }

                        LinearProgressIndicator(
                            progress = { state.progressPercentage },
                            modifier = Modifier.fillMaxWidth().height(8.dp).clip(RoundedCornerShape(4.dp)),
                            color = Color(0xFF38BDF8),
                            trackColor = Color(0xFF1E293B)
                        )

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Text(
                                text = "در حال بررسی: ${state.currentScanningTarget}",
                                fontSize = 11.sp,
                                fontFamily = FontFamily.Monospace,
                                color = Color(0xFFE2E8F0),
                                modifier = Modifier.weight(1f)
                            )
                            Text(
                                text = "${state.currentSpeedPps} pps",
                                fontSize = 11.sp,
                                fontWeight = FontWeight.Bold,
                                color = Color(0xFF34D399)
                            )
                        }

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Text("اسکن شده: ${state.scannedTargets} / ${state.totalTargets}", fontSize = 11.sp, color = Color(0xFF94A3B8))
                            Text("نودهای سالم کشف شده: ${state.cleanTargetsFound}", fontSize = 11.sp, fontWeight = FontWeight.Bold, color = Color(0xFF10B981))
                        }
                    }
                }
            }
        }

        // Export Message Alert
        if (state.exportStatusMessage != null) {
            item {
                Surface(
                    color = Color(0xFF10B981).copy(alpha = 0.15f),
                    shape = RoundedCornerShape(12.dp),
                    border = BorderStroke(1.dp, Color(0xFF10B981))
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(12.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Icon(Icons.Default.CheckCircle, contentDescription = null, tint = Color(0xFF10B981), modifier = Modifier.size(20.dp))
                        Spacer(modifier = Modifier.width(8.dp))
                        Text(state.exportStatusMessage ?: "", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurface)
                    }
                }
            }
        }

        // Discovered Clean Nodes List Header
        item {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "نتایج کشف شده (${state.results.size} هدف):",
                    fontSize = 14.sp,
                    fontWeight = FontWeight.Bold
                )

                Button(
                    onClick = { showExportDialog = true },
                    contentPadding = PaddingValues(horizontal = 12.dp, vertical = 6.dp)
                ) {
                    Icon(Icons.Default.FileDownload, contentDescription = null, modifier = Modifier.size(16.dp))
                    Spacer(modifier = Modifier.width(6.dp))
                    Text("خروجی فایل (CSV/JSON/XLSX)", fontSize = 11.sp)
                }
            }
        }

        // Scan Results List
        items(state.results, key = { it.id }) { node ->
            Card(
                shape = RoundedCornerShape(14.dp),
                border = BorderStroke(1.dp, if (node.isClean) Color(0xFF10B981).copy(alpha = 0.5f) else Color(0xFFEF4444).copy(alpha = 0.3f)),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                modifier = Modifier.fillMaxWidth().testTag("whitedns_node_${node.id}")
            ) {
                Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Column {
                            Text(
                                text = node.target,
                                fontSize = 14.sp,
                                fontWeight = FontWeight.Bold,
                                fontFamily = FontFamily.Monospace
                            )
                            Text(
                                text = "${node.asn} • ${node.org}",
                                fontSize = 11.sp,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }

                        Badge(containerColor = Color(0xFF10B981), contentColor = Color.White) {
                            Text("امتیاز: ${node.ratingScore}", fontSize = 11.sp, fontWeight = FontWeight.Bold)
                        }
                    }

                    Divider(color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(Icons.Default.Speed, contentDescription = null, tint = Color(0xFF38BDF8), modifier = Modifier.size(14.dp))
                            Spacer(modifier = Modifier.width(4.dp))
                            Text("پینگ: ${node.pingLatencyMs}ms", fontSize = 11.sp)
                        }

                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(Icons.Default.Download, contentDescription = null, tint = Color(0xFF10B981), modifier = Modifier.size(14.dp))
                            Spacer(modifier = Modifier.width(4.dp))
                            Text("سرعت: ${node.downloadSpeedMbps} Mbps", fontSize = 11.sp)
                        }

                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(Icons.Default.Verified, contentDescription = null, tint = Color(0xFF10B981), modifier = Modifier.size(14.dp))
                            Spacer(modifier = Modifier.width(4.dp))
                            Text("بافر EDNS: ${node.ednsBufferSize}B", fontSize = 11.sp)
                        }
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(6.dp)
                    ) {
                        SuggestionChip(
                            onClick = {},
                            label = { Text(if (node.txtTunnelPassthrough) "پاساژ TXT سالم" else "مسدود", fontSize = 9.sp) }
                        )
                        SuggestionChip(
                            onClick = {},
                            label = { Text("هندشیک uTLS OK", fontSize = 9.sp) }
                        )
                        SuggestionChip(
                            onClick = {},
                            label = { Text("ضد جعل NXDOMAIN", fontSize = 9.sp) }
                        )
                    }
                }
            }
        }
    }

    // Export Format Dialog
    if (showExportDialog) {
        AlertDialog(
            onDismissRequest = { showExportDialog = false },
            title = { Text("استخراج گزارش اسکن WhiteDNS", fontSize = 16.sp, fontWeight = FontWeight.Bold) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Text("فرمت مورد نظر برای ذخیره در پوشه Documents/WhiteDNS Scanner را انتخاب فرمایید:", fontSize = 12.sp)

                    Button(
                        onClick = {
                            engine.exportResults(WhiteDnsExportFormat.CSV)
                            showExportDialog = false
                        },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("خروجی اکسل و جدول (CSV / Spreadsheet)")
                    }

                    Button(
                        onClick = {
                            engine.exportResults(WhiteDnsExportFormat.JSON)
                            showExportDialog = false
                        },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("خروجی ساختاریافته (JSON Dataset)")
                    }

                    Button(
                        onClick = {
                            engine.exportResults(WhiteDnsExportFormat.TXT)
                            showExportDialog = false
                        },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("خروجی لیست متنی ساده (TXT IP List)")
                    }

                    Button(
                        onClick = {
                            engine.exportResults(WhiteDnsExportFormat.XLSX)
                            showExportDialog = false
                        },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("خروجی مایکروسافت اکسل (XLSX)")
                    }
                }
            },
            confirmButton = {},
            dismissButton = {
                OutlinedButton(onClick = { showExportDialog = false }) {
                    Text("بستن")
                }
            }
        )
    }
}
