package com.unifiedshield.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.aiorchestrator.AdaptiveNetworkProfiler
import com.unifiedshield.ui.components.EnterpriseStatusPill
import com.unifiedshield.ui.components.SectionHeader
import com.unifiedshield.ui.components.StatusPillType
import com.unifiedshield.ui.theme.EnterpriseColors

@Composable
fun SettingsScreen() {
    var killSwitchEnabled by remember { mutableStateOf(true) }
    var splitTunnelEnabled by remember { mutableStateOf(true) }
    var autoCoreSwitchEnabled by remember { mutableStateOf(true) }
    var startOnBootEnabled by remember { mutableStateOf(false) }
    var dnsProvider by remember { mutableStateOf("alibaba") }

    val profiler = remember { AdaptiveNetworkProfiler.getInstance() }
    val isBatterySaverEnabled by profiler.isBatterySaverEnabled.collectAsState()
    val context = LocalContext.current

    // Register battery level reader
    val batteryIntent = remember(context) {
        context.registerReceiver(null, android.content.IntentFilter(android.content.Intent.ACTION_BATTERY_CHANGED))
    }
    val batteryLevel = remember(batteryIntent) {
        val level = batteryIntent?.getIntExtra(android.os.BatteryManager.EXTRA_LEVEL, -1) ?: -1
        val scale = batteryIntent?.getIntExtra(android.os.BatteryManager.EXTRA_SCALE, -1) ?: -1
        if (level >= 0 && scale > 0) (level * 100 / scale) else 100
    }

    LaunchedEffect(batteryLevel) {
        profiler.updateBatteryLevel(batteryLevel)
    }

    val isPowerSaverActive = isBatterySaverEnabled || batteryLevel < 20

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        SectionHeader(
            title = "تنظیمات عمومی و امنیت شبکه (Settings)",
            subtitle = "پیکربندی سطح دسترسی، کیل‌سوئیچ، تونل دوگانه و الگوریتم‌های مدیریت توان",
            icon = Icons.Default.Settings
        )

        // Security & Tunneling
        Card(
            shape = RoundedCornerShape(18.dp),
            colors = CardDefaults.cardColors(
                containerColor = EnterpriseColors.CyberSurfaceCard.copy(alpha = 0.92f)
            ),
            border = androidx.compose.foundation.BorderStroke(1.dp, EnterpriseColors.CyberBorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text(
                    text = "امنیت اتصال و تونلینگ (Security & Tunneling)",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    color = Color.White
                )
                Spacer(modifier = Modifier.height(12.dp))

                SettingsSwitchItem(
                    title = "کیل‌سوئیچ سیستمی (Kill Switch)",
                    description = "قطع کامل دسترسی اینترنت غیرایمن در صورت قطع ناگهانی تونل VPN",
                    checked = killSwitchEnabled,
                    onCheckedChange = { killSwitchEnabled = it },
                    tag = "kill_switch_toggle"
                )

                HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp), color = EnterpriseColors.CyberBorderSubtle.copy(alpha = 0.4f))

                SettingsSwitchItem(
                    title = "تونل دوگانه و تفکیک سایت‌های ایرانی (Split Tunnel)",
                    description = "هدایت مستقیم ترافیک بانک‌ها و سایت‌های داخلی و عبور ترافیک بین‌الملل از تونل امن",
                    checked = splitTunnelEnabled,
                    onCheckedChange = { splitTunnelEnabled = it },
                    tag = "split_tunnel_toggle"
                )

                HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp), color = EnterpriseColors.CyberBorderSubtle.copy(alpha = 0.4f))

                SettingsSwitchItem(
                    title = "سوییچ خودکار هسته (Auto Core Switch)",
                    description = "تغییر هوشمند و بدون قطعی هسته در صورت افزایش شاخص DPI به بیش از ۷۲٪",
                    checked = autoCoreSwitchEnabled,
                    onCheckedChange = { autoCoreSwitchEnabled = it },
                    tag = "auto_core_switch_toggle"
                )
            }
        }

        // DNS Configuration
        Card(
            shape = RoundedCornerShape(18.dp),
            colors = CardDefaults.cardColors(
                containerColor = EnterpriseColors.CyberSurfaceCard.copy(alpha = 0.92f)
            ),
            border = androidx.compose.foundation.BorderStroke(1.dp, EnterpriseColors.CyberBorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = "سرورهای امن DoH DNS",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = Color.White
                    )
                    EnterpriseStatusPill(
                        text = "DoH Encrypted",
                        type = StatusPillType.INFO
                    )
                }

                Text(
                    text = "سرورهای DNS مقاوم در برابر مسموم‌سازی کش و فیلترهای SNI در ایران.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 4.dp)
                )

                Spacer(modifier = Modifier.height(12.dp))

                val dnsOptions = listOf(
                    "alibaba" to "Alibaba DNS DoH (223.5.5.5 - کمترین مسدودی)",
                    "tencent" to "Tencent DNS DoH (119.29.29.29 - مسیر دوم)",
                    "tencent-backup" to "Tencent Backup DoH (1.12.12.12 - رله رزرو)"
                )

                dnsOptions.forEach { (value, label) ->
                    val isSelected = dnsProvider == value
                    Surface(
                        color = if (isSelected) EnterpriseColors.IndigoDeep.copy(alpha = 0.5f) else EnterpriseColors.CyberSurfaceDark,
                        shape = RoundedCornerShape(12.dp),
                        border = androidx.compose.foundation.BorderStroke(
                            1.dp,
                            if (isSelected) EnterpriseColors.IndigoNeon.copy(alpha = 0.6f) else EnterpriseColors.CyberBorderSubtle
                        ),
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 4.dp)
                    ) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 10.dp, vertical = 6.dp)
                        ) {
                            RadioButton(
                                selected = isSelected,
                                onClick = { dnsProvider = value },
                                colors = RadioButtonDefaults.colors(
                                    selectedColor = EnterpriseColors.IndigoNeon
                                )
                            )
                            Spacer(modifier = Modifier.width(6.dp))
                            Text(
                                text = label,
                                style = MaterialTheme.typography.bodyMedium,
                                color = if (isSelected) Color.White else MaterialTheme.colorScheme.onSurfaceVariant,
                                fontWeight = if (isSelected) FontWeight.Bold else FontWeight.Normal
                            )
                        }
                    }
                }
            }
        }

        // Battery Saver & Power Optimization
        Card(
            shape = RoundedCornerShape(18.dp),
            colors = CardDefaults.cardColors(
                containerColor = EnterpriseColors.CyberSurfaceCard.copy(alpha = 0.92f)
            ),
            border = androidx.compose.foundation.BorderStroke(1.dp, EnterpriseColors.CyberBorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = "حالت ذخیره انرژی هوشمند (Battery Saver)",
                            style = MaterialTheme.typography.titleSmall,
                            fontWeight = FontWeight.Bold,
                            color = Color.White
                        )
                        Text(
                            text = "کاهش فرکانس پایش پس‌زمینه (۳۰ ثانیه به ۱۸۰ ثانیه) جهت حداقل مصرف باتری و رم",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            lineHeight = 15.sp
                        )
                    }
                    Switch(
                        checked = isBatterySaverEnabled,
                        onCheckedChange = { profiler.setBatterySaverEnabled(it) },
                        colors = SwitchDefaults.colors(
                            checkedThumbColor = Color.White,
                            checkedTrackColor = EnterpriseColors.EmeraldPrimaryDark
                        ),
                        modifier = Modifier.testTag("battery_saver_toggle")
                    )
                }

                Spacer(modifier = Modifier.height(12.dp))

                Surface(
                    color = if (isPowerSaverActive) EnterpriseColors.WarningContainer.copy(alpha = 0.4f) else EnterpriseColors.CyberSurfaceDark,
                    shape = RoundedCornerShape(12.dp),
                    border = androidx.compose.foundation.BorderStroke(
                        1.dp,
                        if (isPowerSaverActive) EnterpriseColors.Warning.copy(alpha = 0.5f) else EnterpriseColors.CyberBorderSubtle
                    ),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Row(
                        modifier = Modifier.padding(12.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = if (batteryLevel < 20) "🪫" else "🔋",
                            fontSize = 20.sp
                        )
                        Spacer(modifier = Modifier.width(10.dp))
                        Column {
                            Text(
                                text = "میزان شارژ: $batteryLevel% • وضعیت: ${if (isPowerSaverActive) "ذخیره باتری فعال (بازه ۱۸۰ ثانیه)" else "پایش بلادرنگ با فرکانس بالا (۳۰ ثانیه)"}",
                                style = MaterialTheme.typography.labelMedium,
                                fontWeight = FontWeight.Bold,
                                color = if (isPowerSaverActive) EnterpriseColors.WarningText else Color.White
                            )
                            if (batteryLevel < 20) {
                                Text(
                                    text = "باتری کمتر از ۲۰٪ شناسایی شد؛ مصرف CPU خودکار کاهش یافت.",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                        }
                    }
                }
            }
        }

        // General
        Card(
            shape = RoundedCornerShape(18.dp),
            colors = CardDefaults.cardColors(
                containerColor = EnterpriseColors.CyberSurfaceCard.copy(alpha = 0.92f)
            ),
            border = androidx.compose.foundation.BorderStroke(1.dp, EnterpriseColors.CyberBorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                SettingsSwitchItem(
                    title = "شروع خودکار در روشن شدن دستگاه (Start on Boot)",
                    description = "برقراری خودکار شیلد امنیتی بلافاصله پس از ری‌استارت گوشی",
                    checked = startOnBootEnabled,
                    onCheckedChange = { startOnBootEnabled = it },
                    tag = "start_on_boot_toggle"
                )

                HorizontalDivider(modifier = Modifier.padding(vertical = 10.dp), color = EnterpriseColors.CyberBorderSubtle.copy(alpha = 0.4f))

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Column {
                        Text(
                            "MICAFP UnifiedShield Cyber Enterprise",
                            style = MaterialTheme.typography.titleSmall,
                            fontWeight = FontWeight.Bold,
                            color = Color.White
                        )
                        Text(
                            "نسخه کوانتومی پایدار • معماری چند هسته‌ای ضد DPI",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                    EnterpriseStatusPill(
                        text = "v8.5-ULTRA",
                        type = StatusPillType.SUCCESS,
                        isPulsating = true
                    )
                }
            }
        }
    }
}

@Composable
private fun SettingsSwitchItem(
    title: String,
    description: String,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    tag: String
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
                color = Color.White
            )
            Text(
                text = description,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                lineHeight = 15.sp
            )
        }
        Spacer(modifier = Modifier.width(8.dp))
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange,
            colors = SwitchDefaults.colors(
                checkedThumbColor = Color.White,
                checkedTrackColor = EnterpriseColors.EmeraldPrimaryDark
            ),
            modifier = Modifier.testTag(tag)
        )
    }
}
