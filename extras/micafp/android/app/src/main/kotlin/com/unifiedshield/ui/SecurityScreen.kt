package com.unifiedshield.ui

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
import com.unifiedshield.UnifiedShieldStore
import com.unifiedshield.security.SecurityController

@Composable
fun SecurityScreen() {
    val context = LocalContext.current
    val secController = remember { SecurityController.getInstance(context) }
    val secState by secController.state.collectAsState()
    val store = remember { UnifiedShieldStore.getInstance(context) }
    val storeState by store.state.collectAsState()

    var killSwitch by remember { mutableStateOf(true) }
    var splitTunnel by remember { mutableStateOf(true) }
    var pqcKyber by remember { mutableStateOf(true) }
    var decoyMode by remember { mutableStateOf(false) }

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
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
                Icon(
                    imageVector = Icons.Default.EnhancedEncryption,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(36.dp)
                )
                Spacer(modifier = Modifier.width(14.dp))
                Column {
                    Text(
                        text = "Quantum Security & Anti-Forensics",
                        fontSize = 16.sp,
                        fontWeight = FontWeight.Bold
                    )
                    Text(
                        text = "رمزنگاری پساکوانتومی، کیل‌سوییچ لایه سیستم‌عامل و حفاظت در برابر نشت داده",
                        fontSize = 11.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
        }

        // Toggles Card
        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(16.dp)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text("Security Protections (محافظت‌های امنیتی)", fontSize = 14.sp, fontWeight = FontWeight.Bold)
                Spacer(modifier = Modifier.height(8.dp))

                Row(
                    modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text("Post-Quantum PQC Kyber-1024", fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                        Text("رمزنگاری مشبک مقاوم در برابر کامپیوترهای کوانتومی و شنود بلندمدت", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    Switch(checked = pqcKyber, onCheckedChange = { pqcKyber = it }, modifier = Modifier.testTag("pqc_switch"))
                }

                Divider(modifier = Modifier.padding(vertical = 6.dp), color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f))

                Row(
                    modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text("Strict OS Kill Switch (قطع کامل در نشت)", fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                        Text("مسدودسازی ۱۰۰٪ تمام ترافیک دستگاه هنگام قطع ناگهانی تونل", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    Switch(
                        checked = killSwitch,
                        onCheckedChange = {
                            killSwitch = it
                            store.toggleNetworkLock(it)
                        },
                        modifier = Modifier.testTag("kill_switch_toggle")
                    )
                }

                Divider(modifier = Modifier.padding(vertical = 6.dp), color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f))

                Row(
                    modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text("Iranian Domestic Split-Tunnel (تونل تفکیک‌شده)", fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                        Text("هدایت مستقیم سایت‌ها و اپ‌های بانکی و دولتی بدون عبور از پروکسی", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    Switch(checked = splitTunnel, onCheckedChange = { splitTunnel = it }, modifier = Modifier.testTag("split_tunnel_toggle"))
                }

                Divider(modifier = Modifier.padding(vertical = 6.dp), color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f))

                Row(
                    modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text("Decoy Camouflage Icon (استتار آیکون)", fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                        Text("تغییر ظاهر و نام برنامه به ماشین‌حساب ساده در صفحه اصلی", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    Switch(checked = decoyMode, onCheckedChange = { decoyMode = it }, modifier = Modifier.testTag("decoy_mode_toggle"))
                }
            }
        }

        // Emergency Wipe Card & Security Controller Hook
        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = Color(0xFF3F1212)),
            shape = RoundedCornerShape(16.dp)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text("Emergency Memory Wipe (پاکسازی آنی و ضد ردپا)", fontSize = 14.sp, fontWeight = FontWeight.Bold, color = Color(0xFFFCA5A5))
                    Badge(containerColor = Color(0xFFDC2626), contentColor = Color.White) {
                        Text("${secState.totalWipesPerformed} پاکسازی")
                    }
                }

                Text(
                    text = "هوک فعال SecurityController: امحای خودکار کلیدها در زمان رفتن برنامه به پس‌زمینه (Background Suspension) یا فشردن دکمه Panic.",
                    fontSize = 11.sp,
                    color = Color(0xFFFECACA),
                    modifier = Modifier.padding(top = 4.dp)
                )

                Spacer(modifier = Modifier.height(8.dp))
                Surface(
                    color = Color(0x33000000),
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(modifier = Modifier.padding(10.dp)) {
                        Text("هویت ناشناس جاری: ${secState.ephemeralIdentityId}", fontSize = 11.sp, color = Color(0xFF4ADE80), fontWeight = FontWeight.Bold)
                        Text("آخرین رویداد پاکسازی: ${secState.lastWipeReason}", fontSize = 10.sp, color = Color(0xFFE2E8F0))
                    }
                }

                Spacer(modifier = Modifier.height(10.dp))

                Button(
                    onClick = {
                        secController.triggerEmergencyRamWipe("Manual User Wipe Trigger")
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = Color(0xFFDC2626)),
                    shape = RoundedCornerShape(12.dp),
                    modifier = Modifier.fillMaxWidth().testTag("emergency_wipe_button")
                ) {
                    Icon(Icons.Default.DeleteForever, contentDescription = null)
                    Spacer(modifier = Modifier.width(8.dp))
                    Text("اجرای پاکسازی اضطراری (Emergency Wipe)", color = Color.White, fontWeight = FontWeight.Bold)
                }
            }
        }
    }
}
