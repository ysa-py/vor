package com.unifiedshield.ui

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.widget.Toast
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.logging.DebugLogger
import com.unifiedshield.logging.LogLevel
import com.unifiedshield.profile.ProfileManager
import com.unifiedshield.resilience.NetworkClientManager
import com.unifiedshield.sharing.ApkSharingManager
import com.unifiedshield.tunnel.LocalProxyConfig

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AdvancedToolsScreen() {
    val context = LocalContext.current
    val logger = remember { DebugLogger.getInstance() }
    val logs by logger.logs.collectAsState()
    val profileManager = remember { ProfileManager.getInstance(context) }
    val profileState by profileManager.state.collectAsState()
    val apkSharingManager = remember { ApkSharingManager(context) }
    val resilienceManager = remember { NetworkClientManager.getInstance() }
    val telemetry by resilienceManager.telemetry.collectAsState()
    val activeSockets by resilienceManager.activeSockets.collectAsState()
    val retryConfig by resilienceManager.retryConfig.collectAsState()

    var selectedLogLevel by remember { mutableStateOf<LogLevel?>(null) }
    var showProxyDialog by remember { mutableStateOf(false) }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .testTag("advanced_tools_screen"),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        // Banner
        item {
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.35f)
                ),
                shape = RoundedCornerShape(18.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    modifier = Modifier.padding(16.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Box(
                        modifier = Modifier
                            .size(44.dp)
                            .clip(CircleShape)
                            .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.2f)),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            imageVector = Icons.Default.Construction,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(26.dp)
                        )
                    }
                    Spacer(modifier = Modifier.width(12.dp))
                    Column {
                        Text(
                            text = "Advanced Tools & Debugging (ابزارها و لاگ)",
                            fontSize = 16.sp,
                            fontWeight = FontWeight.Bold
                        )
                        Text(
                            text = "اشتراک بلوتوثی APK، پراکسی محلی، کاشی اعلان‌ها و لاگ زنده ترافیک",
                            fontSize = 11.sp,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
            }
        }

        // Network Client Telemetry & Resilience Card
        item {
            Card(
                colors = CardDefaults.cardColors(containerColor = Color(0x1510B981)),
                border = androidx.compose.foundation.BorderStroke(1.dp, Color(0xFF10B981)),
                shape = RoundedCornerShape(16.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(Icons.Default.Speed, contentDescription = null, tint = Color(0xFF10B981), modifier = Modifier.size(22.dp))
                            Spacer(modifier = Modifier.width(8.dp))
                            Text("پایش و تاب‌آوری کلاینت شبکه (Network Resilience)", fontSize = 14.sp, fontWeight = FontWeight.Bold)
                        }
                        Badge(containerColor = Color(0xFF10B981), contentColor = Color.White) {
                            Text("RFC 8446 / 9000")
                        }
                    }

                    Spacer(modifier = Modifier.height(10.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Surface(
                            shape = RoundedCornerShape(10.dp),
                            color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                            modifier = Modifier.weight(1f)
                        ) {
                            Column(modifier = Modifier.padding(8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                                Text("SRTT (RFC 6298)", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text("${telemetry.smoothedRttMs} ms", fontSize = 13.sp, fontWeight = FontWeight.Bold, color = Color(0xFF10B981))
                            }
                        }
                        Surface(
                            shape = RoundedCornerShape(10.dp),
                            color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                            modifier = Modifier.weight(1f)
                        ) {
                            Column(modifier = Modifier.padding(8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                                Text("تایم‌اوت تطبیقی RTO", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text("${telemetry.adaptiveTimeoutMs} ms", fontSize = 13.sp, fontWeight = FontWeight.Bold, color = Color(0xFF3B82F6))
                            }
                        }
                        Surface(
                            shape = RoundedCornerShape(10.dp),
                            color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                            modifier = Modifier.weight(1f)
                        ) {
                            Column(modifier = Modifier.padding(8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                                Text("شاخص پایداری", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text("${telemetry.linkStabilityIndex}%", fontSize = 13.sp, fontWeight = FontWeight.Bold, color = Color(0xFF8B5CF6))
                            }
                        }
                    }

                    Spacer(modifier = Modifier.height(10.dp))

                    Text(
                        text = "الگوریتم Exponential Backoff با Full Jitter فعال است. سوکت‌های همزمان (${telemetry.activeSocketCount}) با رمزنگاری TLS 1.3 / QUIC آماده جابجایی ترافیک هستند.",
                        fontSize = 11.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )

                    Spacer(modifier = Modifier.height(10.dp))

                    activeSockets.forEach { socket ->
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(vertical = 4.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Column {
                                Text("${socket.targetHost}:${socket.port} (${socket.security.rfcStandard})", fontSize = 11.sp, fontWeight = FontWeight.Medium)
                                Text("وضعیت: ${socket.connectionState} • پینگ: ${socket.latencyMs}ms", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                            OutlinedButton(
                                onClick = { resilienceManager.triggerManualProbe(socket.socketId) },
                                contentPadding = PaddingValues(horizontal = 8.dp, vertical = 2.dp),
                                modifier = Modifier.height(28.dp)
                            ) {
                                Text("تست سوکت", fontSize = 10.sp)
                            }
                        }
                    }
                }
            }
        }

        // Offline APK Sharing Card
        item {
            Card(
                colors = CardDefaults.cardColors(containerColor = Color(0x153B82F6)),
                border = androidx.compose.foundation.BorderStroke(1.dp, Color(0xFF3B82F6)),
                shape = RoundedCornerShape(16.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(Icons.Default.Bluetooth, contentDescription = null, tint = Color(0xFF3B82F6), modifier = Modifier.size(22.dp))
                            Spacer(modifier = Modifier.width(8.dp))
                            Text("اشتراک‌گذاری آفلاین APK (Bluetooth / Nearby)", fontSize = 14.sp, fontWeight = FontWeight.Bold)
                        }
                        Badge(containerColor = Color(0xFF3B82F6), contentColor = Color.White) {
                            Text("آفلاین")
                        }
                    }

                    Spacer(modifier = Modifier.height(6.dp))
                    Text(
                        text = "در زمان قطعی کامل اینترنت بین‌الملل، برنامه را مستقیماً از طریق بلوتوث یا Nearby Share برای اطرافیان ارسال کنید.",
                        fontSize = 11.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )

                    Spacer(modifier = Modifier.height(10.dp))

                    Button(
                        onClick = {
                            val shareIntent = apkSharingManager.shareAppApk()
                            if (shareIntent != null) {
                                context.startActivity(Intent.createChooser(shareIntent, "اشتراک فایل نصبی UnifiedShield"))
                            } else {
                                Toast.makeText(context, "فایل نصبی آماده‌سازی نشد", Toast.LENGTH_SHORT).show()
                            }
                        },
                        modifier = Modifier.fillMaxWidth().testTag("share_apk_btn"),
                        colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF3B82F6)),
                        shape = RoundedCornerShape(10.dp)
                    ) {
                        Icon(Icons.Default.Share, contentDescription = null, modifier = Modifier.size(18.dp))
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("ارسال مستقیم فایل نصبی به دستگاه‌های دیگر", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                    }
                }
            }
        }

        // Configurable Inbound Local Proxy Card
        item {
            val proxyConfig = profileState.proxyConfig
            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(14.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(Icons.Default.SettingsEthernet, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(20.dp))
                            Spacer(modifier = Modifier.width(8.dp))
                            Text("پراکسی محلی ورودی (Inbound Proxy)", fontSize = 13.sp, fontWeight = FontWeight.Bold)
                        }

                        IconButton(onClick = { showProxyDialog = true }) {
                            Icon(Icons.Default.Edit, contentDescription = "ویرایش", modifier = Modifier.size(18.dp))
                        }
                    }

                    Spacer(modifier = Modifier.height(4.dp))
                    Text(
                        text = "Listen: ${proxyConfig.listenAddress} • SOCKS5: ${proxyConfig.socks5Port} • HTTP: ${proxyConfig.httpPort}",
                        fontSize = 12.sp,
                        fontWeight = FontWeight.SemiBold,
                        color = Color(0xFF10B981)
                    )
                    Text(
                        text = "امکان اتصال تلگرام، مرورگرها و سایر برنامه‌ها از طریق پورت محلی به تونل یونیفاید شیلد.",
                        fontSize = 11.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }

        // Quick Settings Tile & Boot Settings
        item {
            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(14.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text("اتصال خودکار پس از روشن شدن دستگاه (Auto-connect on Boot)", fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                            Text("با بالا آمدن اندروید، اتصال محافظت‌شده برقرار می‌گردد.", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        Switch(
                            checked = profileState.startOnBoot,
                            onCheckedChange = { profileManager.toggleStartOnBoot(it) },
                            modifier = Modifier.testTag("boot_switch")
                        )
                    }

                    Divider(modifier = Modifier.padding(vertical = 8.dp), color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Icon(Icons.Default.DashboardCustomize, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(18.dp))
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("کاشی تنظیمات سریع (Quick Settings Tile): از منوی اعلانات اندروید اضافه نمایید.", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
            }
        }

        // Real-Time Debug Logging Section
        item {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = "لاگ زنده ترافیک و عیب‌یابی (Debug Logs)",
                        fontSize = 14.sp,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.primary
                    )

                    Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        IconButton(
                            onClick = {
                                val text = logger.exportLogsText()
                                val shareIntent = Intent().apply {
                                    action = Intent.ACTION_SEND
                                    putExtra(Intent.EXTRA_TEXT, text)
                                    putExtra(Intent.EXTRA_SUBJECT, "UnifiedShield Connection Diagnostic Logs")
                                    type = "text/plain"
                                }
                                context.startActivity(Intent.createChooser(shareIntent, "اشتراک‌گذاری لاگ‌های اتصال"))
                            },
                            modifier = Modifier.size(32.dp).testTag("share_logs_btn")
                        ) {
                            Icon(Icons.Default.Share, contentDescription = "اشتراک‌گذاری لاگ", modifier = Modifier.size(18.dp))
                        }

                        IconButton(
                            onClick = {
                                val file = logger.exportLogsToFile(context)
                                if (file != null) {
                                    Toast.makeText(context, "لاگ‌های اتصال در فایل ${file.name} ذخیره شدند", Toast.LENGTH_LONG).show()
                                    val viewIntent = Intent().apply {
                                        action = Intent.ACTION_SEND
                                        putExtra(Intent.EXTRA_TEXT, file.readText())
                                        type = "text/plain"
                                    }
                                    context.startActivity(Intent.createChooser(viewIntent, "ذخیره یا ارسال فایل لاگ"))
                                } else {
                                    Toast.makeText(context, "خطا در خروجی فایل لاگ", Toast.LENGTH_SHORT).show()
                                }
                            },
                            modifier = Modifier.size(32.dp).testTag("export_logs_file_btn")
                        ) {
                            Icon(Icons.Default.Download, contentDescription = "خروجی فایل لاگ", modifier = Modifier.size(18.dp))
                        }

                        IconButton(
                            onClick = {
                                val text = logger.exportLogsText()
                                val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                                val clip = ClipData.newPlainText("UnifiedShield Logs", text)
                                clipboard.setPrimaryClip(clip)
                                Toast.makeText(context, "لاگ‌ها در کلیپ‌بورد کپی شدند", Toast.LENGTH_SHORT).show()
                            },
                            modifier = Modifier.size(32.dp).testTag("copy_logs_btn")
                        ) {
                            Icon(Icons.Default.ContentCopy, contentDescription = "کپی لاگ", modifier = Modifier.size(18.dp))
                        }

                        IconButton(
                            onClick = { logger.clearLogs() },
                            modifier = Modifier.size(32.dp).testTag("clear_logs_btn")
                        ) {
                            Icon(Icons.Default.DeleteSweep, contentDescription = "پاکسازی", modifier = Modifier.size(18.dp))
                        }
                    }
                }

                // Log Level Filter Chips
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(4.dp)
                ) {
                    FilterChip(
                        selected = selectedLogLevel == null,
                        onClick = { selectedLogLevel = null },
                        label = { Text("ALL (${logs.size})", fontSize = 10.sp) }
                    )
                    LogLevel.values().forEach { lvl ->
                        FilterChip(
                            selected = selectedLogLevel == lvl,
                            onClick = { selectedLogLevel = if (selectedLogLevel == lvl) null else lvl },
                            label = { Text(lvl.label, fontSize = 10.sp) }
                        )
                    }
                }
            }
        }

        // Logs Terminal View
        val filteredLogs = logs.filter { selectedLogLevel == null || it.level == selectedLogLevel }

        items(filteredLogs, key = { it.id }) { logItem ->
            val levelColor = when (logItem.level) {
                LogLevel.INFO -> Color(0xFF10B981)
                LogLevel.WARN -> Color(0xFFF59E0B)
                LogLevel.DPI -> Color(0xFFEF4444)
                LogLevel.PACKET -> Color(0xFF3B82F6)
                LogLevel.TUNNEL -> Color(0xFF8B5CF6)
                LogLevel.SCANNER -> Color(0xFF06B6D4)
            }

            Surface(
                color = Color(0xFF1E1E2E),
                shape = RoundedCornerShape(8.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 6.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = logItem.formattedTime,
                        fontSize = 10.sp,
                        fontFamily = FontFamily.Monospace,
                        color = Color(0xFF94A3B8)
                    )
                    Spacer(modifier = Modifier.width(6.dp))

                    Surface(
                        color = levelColor.copy(alpha = 0.2f),
                        shape = RoundedCornerShape(4.dp)
                    ) {
                        Text(
                            text = logItem.level.label,
                            fontSize = 9.sp,
                            fontFamily = FontFamily.Monospace,
                            fontWeight = FontWeight.Bold,
                            color = levelColor,
                            modifier = Modifier.padding(horizontal = 4.dp, vertical = 1.dp)
                        )
                    }

                    Spacer(modifier = Modifier.width(6.dp))

                    Text(
                        text = "[${logItem.tag}] ${logItem.message}",
                        fontSize = 10.sp,
                        fontFamily = FontFamily.Monospace,
                        color = Color(0xFFE2E8F0),
                        modifier = Modifier.weight(1f)
                    )
                }
            }
        }
    }

    // Proxy Dialog
    if (showProxyDialog) {
        var socksPort by remember { mutableStateOf(profileState.proxyConfig.socks5Port.toString()) }
        var httpPort by remember { mutableStateOf(profileState.proxyConfig.httpPort.toString()) }
        var listenAddr by remember { mutableStateOf(profileState.proxyConfig.listenAddress) }

        AlertDialog(
            onDismissRequest = { showProxyDialog = false },
            title = { Text("پیکربندی پراکسی محلی (Proxy Ports)", fontSize = 16.sp, fontWeight = FontWeight.Bold) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = listenAddr,
                        onValueChange = { listenAddr = it },
                        label = { Text("آدرس گوش دادن (Listen Address)") },
                        modifier = Modifier.fillMaxWidth()
                    )
                    OutlinedTextField(
                        value = socksPort,
                        onValueChange = { socksPort = it },
                        label = { Text("پورت SOCKS5") },
                        modifier = Modifier.fillMaxWidth()
                    )
                    OutlinedTextField(
                        value = httpPort,
                        onValueChange = { httpPort = it },
                        label = { Text("پورت HTTP") },
                        modifier = Modifier.fillMaxWidth()
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        profileManager.updateProxyConfig(
                            LocalProxyConfig(
                                socks5Port = socksPort.toIntOrNull() ?: 10808,
                                httpPort = httpPort.toIntOrNull() ?: 10809,
                                listenAddress = listenAddr
                            )
                        )
                        showProxyDialog = false
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF10B981))
                ) {
                    Text("ذخیره")
                }
            },
            dismissButton = {
                OutlinedButton(onClick = { showProxyDialog = false }) {
                    Text("انصراف")
                }
            }
        )
    }
}
