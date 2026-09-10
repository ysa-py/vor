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
import com.unifiedshield.WarTimeResilienceEngine

@Composable
fun IntranetScreen() {
    val warEngine = remember { WarTimeResilienceEngine.getInstance() }
    val warState by warEngine.warState.collectAsState()

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .testTag("intranet_screen"),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        item {
            // Header Banner
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = if (warState.isEmergencyModeActive) Color(0x33EF4444) else MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.35f)
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
                            .size(52.dp)
                            .clip(CircleShape)
                            .background(if (warState.isEmergencyModeActive) Color(0x33EF4444) else MaterialTheme.colorScheme.primary.copy(alpha = 0.2f)),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            imageVector = if (warState.isEmergencyModeActive) Icons.Default.Warning else Icons.Default.Hub,
                            contentDescription = null,
                            tint = if (warState.isEmergencyModeActive) Color(0xFFEF4444) else MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(32.dp)
                        )
                    }

                    Spacer(modifier = Modifier.width(14.dp))

                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = "War-Time Intranet Survival Mesh",
                            fontSize = 16.sp,
                            fontWeight = FontWeight.Bold
                        )
                        Text(
                            text = "حالت بقای شبکه ملی در زمان قطع کامل اینترنت بین‌الملل (بلک‌اوت و خاموشی کامل)",
                            fontSize = 11.sp,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
            }
        }

        item {
            // Emergency Mode Toggle Card
            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(16.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text("Emergency Blackout Mode (حالت اضطراری)", fontSize = 14.sp, fontWeight = FontWeight.Bold)
                            Text(
                                text = "مسیریابی خودکار ترافیک از طریق رله‌های امن داخل کشور در شرایط قطعی ۱۰۰٪ اینترنت جهانی",
                                fontSize = 11.sp,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }

                        Switch(
                            checked = warState.isEmergencyModeActive,
                            onCheckedChange = {
                                if (it) warEngine.activateEmergencySuite("Manual Trigger") else warEngine.deactivate()
                            },
                            modifier = Modifier.testTag("emergency_mode_switch")
                        )
                    }

                    if (warState.isEmergencyModeActive) {
                        Spacer(modifier = Modifier.height(10.dp))
                        Surface(
                            color = Color(0x1AEF4444),
                            shape = RoundedCornerShape(8.dp),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Row(
                                modifier = Modifier.padding(10.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Icon(Icons.Default.Bolt, contentDescription = null, tint = Color(0xFFEF4444), modifier = Modifier.size(18.dp))
                                Spacer(modifier = Modifier.width(8.dp))
                                Text("رله دامنه سرورهای علی‌بابا کلود با موفقیت فعال شد (${warState.frontingDomain})", fontSize = 11.sp, color = Color(0xFFEF4444))
                            }
                        }
                    }
                }
            }
        }

        item {
            // Dedicated Iran Intranet Relay & Mesh Diagnostic Panel
            IranIntranetRelayPanel()
        }

        item {
            // Mesh & Covert Channels Card
            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(16.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text("P2P Mesh & Covert Transports (کانال‌های مخفی)", fontSize = 14.sp, fontWeight = FontWeight.Bold)
                    Spacer(modifier = Modifier.height(8.dp))

                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text("Bluetooth Low Energy (BLE) Mesh", fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                            Text("اشتراک‌گذاری آدرس سرورها بین گوشی‌های اطراف بدون نیاز به اینترنت", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        Switch(
                            checked = warState.isBleMeshActive,
                            onCheckedChange = { warEngine.toggleBleMesh(it) },
                            modifier = Modifier.testTag("ble_mesh_switch")
                        )
                    }

                    Divider(modifier = Modifier.padding(vertical = 6.dp), color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f))

                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text("Wi-Fi Aware (NAN) Direct Relay", fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                            Text("ارتباط مستقیم وای‌فای با پهنای باند بالا جهت تبادل کلید با گره‌های مجاور", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        Switch(
                            checked = warState.isWifiAwareActive,
                            onCheckedChange = { warEngine.toggleWifiAware(it) },
                            modifier = Modifier.testTag("wifi_aware_switch")
                        )
                    }

                    Divider(modifier = Modifier.padding(vertical = 6.dp), color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f))

                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text("Covert-NTP & Acoustic Bootstrap", fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                            Text("دریافت پیکربندی سرور از طریق بسته‌های رمزدار زمان و امواج صوتی اولتراسونیک", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        Switch(
                            checked = warState.isNtpCovertActive,
                            onCheckedChange = { warEngine.toggleNtpCovert(it) },
                            modifier = Modifier.testTag("ntp_covert_switch")
                        )
                    }
                }
            }
        }
    }
}
