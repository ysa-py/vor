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
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.ui.components.EnterpriseStatusPill
import com.unifiedshield.ui.components.SectionHeader
import com.unifiedshield.ui.components.StatusPillType
import com.unifiedshield.ui.theme.EnterpriseColors

@Composable
fun ObfuscationScreen() {
    var tlsFragmentation by remember { mutableStateOf(true) }
    var http3Masquerade by remember { mutableStateOf(true) }
    var timingJitter by remember { mutableStateOf(true) }
    var steganographicHeaders by remember { mutableStateOf(true) }
    var cdnRelayProvider by remember { mutableStateOf("alibaba") }

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        SectionHeader(
            title = "استتار و خنثی‌سازی فیلترینگ عمیق (Anti-DPI Obfuscation)",
            subtitle = "پیکربندی هوشمند لایه بسته برای دورزدن فیلترینگ عمیق پکت‌های اینترنت ایران",
            icon = Icons.Default.VisibilityOff
        )

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
                        "تکنیک‌های شکل‌دهی ترافیک (Traffic Shaping)",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = Color.White
                    )
                    EnterpriseStatusPill(
                        text = "4 ماژول فعال",
                        type = StatusPillType.SUCCESS,
                        isPulsating = true
                    )
                }

                Spacer(modifier = Modifier.height(12.dp))

                SwitchRow(
                    title = "تکه‌تکه‌سازی ClientHello در TLS (TLS Fragmentation)",
                    desc = "تفکیک نام دامنه SNI میان سگمنت‌های TCP جهت عبور نامرئی از فیلتر SNI",
                    checked = tlsFragmentation,
                    onChecked = { tlsFragmentation = it },
                    tag = "switch_tls_frag"
                )

                HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp), color = EnterpriseColors.CyberBorderSubtle.copy(alpha = 0.4f))

                SwitchRow(
                    title = "پوشش چندرسانه‌ای HTTP/3 و QUIC (Media Masquerade)",
                    desc = "استتار بسته‌های پراکسی در قالب استریم‌های صوتی و تصویری مجاز",
                    checked = http3Masquerade,
                    onChecked = { http3Masquerade = it },
                    tag = "switch_http3_masq"
                )

                HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp), color = EnterpriseColors.CyberBorderSubtle.copy(alpha = 0.4f))

                SwitchRow(
                    title = "جیتر زمانی و تزریق تأخیر رندوم (Timing Jitter)",
                    desc = "تزریق میکروتأخیرهای تصادفی جهت محو کردن الگوی حجم و طول بسته‌ها",
                    checked = timingJitter,
                    onChecked = { timingJitter = it },
                    tag = "switch_timing_jitter"
                )

                HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp), color = EnterpriseColors.CyberBorderSubtle.copy(alpha = 0.4f))

                SwitchRow(
                    title = "هدرهای استگانوگرافی (Steganographic Headers)",
                    desc = "جاسازی فریم‌های رمزنگاری شده در ساختار هدرهای استاندارد تصویر وب",
                    checked = steganographicHeaders,
                    onChecked = { steganographicHeaders = it },
                    tag = "switch_stegano_hdr"
                )
            }
        }

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
                        "شبکه توزیع محتوا و رله‌های Domain Fronting",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = Color.White
                    )
                    EnterpriseStatusPill(
                        text = "Zero-Block Mesh",
                        type = StatusPillType.INFO
                    )
                }

                Text(
                    text = "با توجه به مسدودی گسترده کلادفلر در ایران، رله‌های پیش‌فرض بر بستر شبکه‌های ابری آسیایی هدایت می‌شوند.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 4.dp)
                )

                Spacer(modifier = Modifier.height(12.dp))

                val cdns = listOf(
                    "alibaba" to "Alibaba Cloud OSS (Primary High-Throughput CDN)",
                    "tencent" to "Tencent Cloud COS (Secondary Backup Relay)",
                    "baidu" to "Baidu Cloud Edge Relay",
                    "cloudflare" to "Cloudflare Worker (Fallback Egress)"
                )

                cdns.forEach { (key, label) ->
                    val isSelected = cdnRelayProvider == key
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
                                onClick = { cdnRelayProvider = key },
                                colors = RadioButtonDefaults.colors(
                                    selectedColor = EnterpriseColors.IndigoNeon
                                ),
                                modifier = Modifier.testTag("cdn_option_$key")
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
    }
}

@Composable
private fun SwitchRow(
    title: String,
    desc: String,
    checked: Boolean,
    onChecked: (Boolean) -> Unit,
    tag: String = "switch_row"
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp)
            .testTag(tag),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                title,
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
                color = Color.White
            )
            Text(
                desc,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                lineHeight = 15.sp
            )
        }
        Spacer(modifier = Modifier.width(8.dp))
        Switch(
            checked = checked,
            onCheckedChange = onChecked,
            colors = SwitchDefaults.colors(
                checkedThumbColor = Color.White,
                checkedTrackColor = EnterpriseColors.EmeraldPrimaryDark
            )
        )
    }
}
