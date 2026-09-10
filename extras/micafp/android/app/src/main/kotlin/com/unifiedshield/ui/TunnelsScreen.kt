package com.unifiedshield.ui

import androidx.compose.animation.*
import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.profile.ProfileManager
import com.unifiedshield.tunnel.*

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TunnelsScreen() {
    val context = LocalContext.current
    val profileManager = remember { ProfileManager.getInstance(context) }
    val profileState by profileManager.state.collectAsState()

    var selectedTunnelTypeFilter by remember { mutableStateOf<TunnelType?>(null) }
    var editingProfile by remember { mutableStateOf<TunnelProfile?>(null) }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .testTag("tunnels_screen"),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        // Banner info
        item {
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.35f)
                ),
                shape = RoundedCornerShape(18.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
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
                                imageVector = Icons.Default.Dns,
                                contentDescription = null,
                                tint = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.size(26.dp)
                            )
                        }
                        Spacer(modifier = Modifier.width(12.dp))
                        Column {
                            Text(
                                text = "Tunnel Engine & Protocols (موتور تونل و پروتکل‌ها)",
                                fontSize = 16.sp,
                                fontWeight = FontWeight.Bold
                            )
                            Text(
                                text = "DNSTT, NoizDNS, VayDNS, Slipstream, SSH, NaiveProxy, DOH, Tor",
                                fontSize = 11.sp,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(10.dp))

                    Surface(
                        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.7f),
                        shape = RoundedCornerShape(12.dp)
                    ) {
                        Text(
                            text = "نکته: پروتکل DNSTT گزینه پیش‌فرض و پیشنهادی است. NoizDNS مقاومت در برابر DPI را ارتقا می‌دهد. VayDNS قالب سیم بهینه و قابل تنظیم دارد. انواع SSH رمزنگاری دولایه و جلوگیری از نشت DNS را تضمین می‌کنند.",
                            fontSize = 11.sp,
                            color = MaterialTheme.colorScheme.onSurface,
                            modifier = Modifier.padding(10.dp)
                        )
                    }
                }
            }
        }

        // Filter / Selector Chips for Tunnel Types
        item {
            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text(
                    text = "انتخاب پروتکل تونل (${TunnelType.values().size} نوع فعال):",
                    fontSize = 13.sp,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.primary
                )

                androidx.compose.foundation.lazy.LazyRow(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(6.dp)
                ) {
                    item {
                        FilterChip(
                            selected = selectedTunnelTypeFilter == null,
                            onClick = { selectedTunnelTypeFilter = null },
                            label = { Text("همه (${profileState.profiles.size})", fontSize = 10.sp) },
                            modifier = Modifier.testTag("filter_all_tunnels")
                        )
                    }
                    item {
                        FilterChip(
                            selected = selectedTunnelTypeFilter == TunnelType.STORM_DNS,
                            onClick = { selectedTunnelTypeFilter = if (selectedTunnelTypeFilter == TunnelType.STORM_DNS) null else TunnelType.STORM_DNS },
                            label = { Text("StormDNS", fontSize = 10.sp, fontWeight = FontWeight.Bold) }
                        )
                    }
                    item {
                        FilterChip(
                            selected = selectedTunnelTypeFilter == TunnelType.COTTEN_DNS,
                            onClick = { selectedTunnelTypeFilter = if (selectedTunnelTypeFilter == TunnelType.COTTEN_DNS) null else TunnelType.COTTEN_DNS },
                            label = { Text("CottenDNS", fontSize = 10.sp, fontWeight = FontWeight.Bold) }
                        )
                    }
                    item {
                        FilterChip(
                            selected = selectedTunnelTypeFilter == TunnelType.MASTER_DNS,
                            onClick = { selectedTunnelTypeFilter = if (selectedTunnelTypeFilter == TunnelType.MASTER_DNS) null else TunnelType.MASTER_DNS },
                            label = { Text("MasterDns (9X)", fontSize = 10.sp, fontWeight = FontWeight.Bold) }
                        )
                    }
                    item {
                        FilterChip(
                            selected = selectedTunnelTypeFilter == TunnelType.VLESS_REALITY,
                            onClick = { selectedTunnelTypeFilter = if (selectedTunnelTypeFilter == TunnelType.VLESS_REALITY) null else TunnelType.VLESS_REALITY },
                            label = { Text("VLESS Reality", fontSize = 10.sp) }
                        )
                    }
                    item {
                        FilterChip(
                            selected = selectedTunnelTypeFilter == TunnelType.HYSTERIA_2,
                            onClick = { selectedTunnelTypeFilter = if (selectedTunnelTypeFilter == TunnelType.HYSTERIA_2) null else TunnelType.HYSTERIA_2 },
                            label = { Text("Hysteria 2", fontSize = 10.sp) }
                        )
                    }
                    item {
                        FilterChip(
                            selected = selectedTunnelTypeFilter == TunnelType.TUIC_V5,
                            onClick = { selectedTunnelTypeFilter = if (selectedTunnelTypeFilter == TunnelType.TUIC_V5) null else TunnelType.TUIC_V5 },
                            label = { Text("TUIC v5", fontSize = 10.sp) }
                        )
                    }
                    item {
                        FilterChip(
                            selected = selectedTunnelTypeFilter == TunnelType.SHADOWSOCKS_2022,
                            onClick = { selectedTunnelTypeFilter = if (selectedTunnelTypeFilter == TunnelType.SHADOWSOCKS_2022) null else TunnelType.SHADOWSOCKS_2022 },
                            label = { Text("Shadowsocks 2022", fontSize = 10.sp) }
                        )
                    }
                    item {
                        FilterChip(
                            selected = selectedTunnelTypeFilter == TunnelType.WIREGUARD_AMNEZIA,
                            onClick = { selectedTunnelTypeFilter = if (selectedTunnelTypeFilter == TunnelType.WIREGUARD_AMNEZIA) null else TunnelType.WIREGUARD_AMNEZIA },
                            label = { Text("AmneziaWG", fontSize = 10.sp) }
                        )
                    }
                    item {
                        FilterChip(
                            selected = selectedTunnelTypeFilter == TunnelType.DNSTT,
                            onClick = { selectedTunnelTypeFilter = if (selectedTunnelTypeFilter == TunnelType.DNSTT) null else TunnelType.DNSTT },
                            label = { Text("DNSTT", fontSize = 10.sp) }
                        )
                    }
                    item {
                        FilterChip(
                            selected = selectedTunnelTypeFilter == TunnelType.VAY_DNS,
                            onClick = { selectedTunnelTypeFilter = if (selectedTunnelTypeFilter == TunnelType.VAY_DNS) null else TunnelType.VAY_DNS },
                            label = { Text("VayDNS", fontSize = 10.sp) }
                        )
                    }
                }
            }
        }

        // Active Profile Card Showcase
        item {
            val activeProf = profileState.profiles.find { it.isActive } ?: profileState.profiles.first()
            Card(
                colors = CardDefaults.cardColors(containerColor = Color(0x1510B981)),
                border = androidx.compose.foundation.BorderStroke(1.5.dp, Color(0xFF10B981)),
                shape = RoundedCornerShape(16.dp),
                modifier = Modifier.fillMaxWidth().testTag("active_profile_card")
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(Icons.Default.CheckCircle, contentDescription = null, tint = Color(0xFF10B981), modifier = Modifier.size(20.dp))
                            Spacer(modifier = Modifier.width(8.dp))
                            Text("پروفایل فعال جاری", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFF10B981))
                        }
                        Badge(containerColor = Color(0xFF10B981), contentColor = Color.White) {
                            Text(activeProf.tunnelType.protocol)
                        }
                    }

                    Spacer(modifier = Modifier.height(6.dp))
                    Text(activeProf.namePersian, fontSize = 15.sp, fontWeight = FontWeight.ExtraBold)
                    Text(activeProf.tunnelType.descriptionPersian, fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)

                    Spacer(modifier = Modifier.height(10.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Text("پینگ: ${activeProf.pingMs}ms", fontSize = 11.sp, fontWeight = FontWeight.SemiBold)
                        Text("سرعت: ${activeProf.throughputRating}", fontSize = 11.sp, fontWeight = FontWeight.Bold, color = Color(0xFF10B981))
                        Text(if (activeProf.tunnelType.isDpiResistant) "ضد فیلتر DPI" else "استاندارد", fontSize = 11.sp, color = Color(0xFF6366F1))
                    }

                    Spacer(modifier = Modifier.height(8.dp))

                    Button(
                        onClick = { editingProfile = activeProf },
                        modifier = Modifier.fillMaxWidth().testTag("edit_active_profile_btn"),
                        colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.primary),
                        shape = RoundedCornerShape(10.dp)
                    ) {
                        Icon(Icons.Default.Tune, contentDescription = null, modifier = Modifier.size(18.dp))
                        Spacer(modifier = Modifier.width(6.dp))
                        Text("تنظیم پارامترهای پیشرفته این تونل (Ciphers / Wire / Wrappers)", fontSize = 12.sp)
                    }
                }
            }
        }

        // Section Title
        item {
            Text(
                text = "لیست پروفایل‌های آماده و سفارشی",
                fontSize = 14.sp,
                fontWeight = FontWeight.Bold,
                modifier = Modifier.padding(horizontal = 4.dp)
            )
        }

        // Profile Items List
        val filteredList = profileState.profiles.filter {
            selectedTunnelTypeFilter == null || it.tunnelType == selectedTunnelTypeFilter
        }

        items(filteredList, key = { it.id }) { item ->
            val isCurrentActive = item.isActive

            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(
                        width = if (isCurrentActive) 2.dp else 1.dp,
                        color = if (isCurrentActive) Color(0xFF10B981) else MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.4f),
                        shape = RoundedCornerShape(14.dp)
                    )
                    .clickable { profileManager.selectProfile(item.id) }
                    .testTag("profile_item_${item.id}"),
                colors = CardDefaults.cardColors(
                    containerColor = if (isCurrentActive) Color(0x0F10B981) else MaterialTheme.colorScheme.surface
                ),
                shape = RoundedCornerShape(14.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
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
                                    .background(if (isCurrentActive) Color(0xFF10B981) else Color.Gray)
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(item.namePersian, fontSize = 13.sp, fontWeight = FontWeight.Bold)
                        }

                        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                            if (item.tunnelType.isDefaultRecommended) {
                                Badge(containerColor = MaterialTheme.colorScheme.primary, contentColor = Color.White) {
                                    Text("پیشنهادی")
                                }
                            }
                            if (item.tunnelType.hasSshChaining) {
                                Badge(containerColor = Color(0xFF6366F1), contentColor = Color.White) {
                                    Text("+SSH")
                                }
                            }
                        }
                    }

                    Text(
                        text = "${item.tunnelType.title} • ${item.tunnelType.protocol}",
                        fontSize = 11.sp,
                        color = MaterialTheme.colorScheme.primary,
                        fontWeight = FontWeight.Medium,
                        modifier = Modifier.padding(top = 2.dp)
                    )

                    Spacer(modifier = Modifier.height(4.dp))
                    Text(item.tunnelType.descriptionPersian, fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)

                    Spacer(modifier = Modifier.height(8.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                            Text("RTT: ${item.pingMs}ms", fontSize = 11.sp, fontWeight = FontWeight.SemiBold)
                            Text("پهنای‌باند: ${item.throughputRating}", fontSize = 11.sp, color = Color(0xFF10B981), fontWeight = FontWeight.SemiBold)
                        }

                        IconButton(
                            onClick = { editingProfile = item },
                            modifier = Modifier.size(32.dp).testTag("config_profile_${item.id}")
                        ) {
                            Icon(Icons.Default.Settings, contentDescription = "تنظیمات", tint = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.size(18.dp))
                        }
                    }
                }
            }
        }
    }

    // Modal Sheet or Dialog for editing advanced configuration parameters
    editingProfile?.let { prof ->
        EditTunnelProfileDialog(
            profile = prof,
            onDismiss = { editingProfile = null },
            onSave = { updated ->
                profileManager.updateProfile(updated)
                editingProfile = null
            }
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EditTunnelProfileDialog(
    profile: TunnelProfile,
    onDismiss: () -> Unit,
    onSave: (TunnelProfile) -> Unit
) {
    var editedProfile by remember { mutableStateOf(profile) }
    var selectedTab by remember { mutableIntStateOf(0) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Column {
                Text("تنظیمات تخصصی ${profile.tunnelType.title}", fontSize = 16.sp, fontWeight = FontWeight.Bold)
                Text(profile.tunnelType.protocol, fontSize = 12.sp, color = MaterialTheme.colorScheme.primary)
            }
        },
        text = {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .heightIn(max = 420.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                TabRow(selectedTabIndex = selectedTab) {
                    Tab(
                        selected = selectedTab == 0,
                        onClick = { selectedTab = 0 },
                        text = { Text("هسته تونل", fontSize = 11.sp) }
                    )
                    Tab(
                        selected = selectedTab == 1,
                        onClick = { selectedTab = 1 },
                        text = { Text("SSH و رمزها", fontSize = 11.sp) }
                    )
                    Tab(
                        selected = selectedTab == 2,
                        onClick = { selectedTab = 2 },
                        text = { Text("DNS و وایر", fontSize = 11.sp) }
                    )
                }

                Spacer(modifier = Modifier.height(4.dp))

                when (selectedTab) {
                    0 -> {
                        // Protocol Specific settings
                        when (editedProfile.tunnelType) {
                            TunnelType.MASTER_DNS, TunnelType.MASTER_DNS_SSH -> {
                                Text("تنظیمات MasterDnsVPN (ARQ-5B + ۸ رِزولور):", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
                                
                                Text("حالت بالانس رِزولورها (Balancing Mode):", fontSize = 11.sp, fontWeight = FontWeight.SemiBold)
                                MasterDnsBalancingMode.values().take(4).forEach { mode ->
                                    Row(
                                        verticalAlignment = Alignment.CenterVertically,
                                        modifier = Modifier.clickable {
                                            editedProfile = editedProfile.copy(
                                                masterDnsConfig = editedProfile.masterDnsConfig.copy(balancingMode = mode)
                                            )
                                        }
                                    ) {
                                        RadioButton(
                                            selected = editedProfile.masterDnsConfig.balancingMode == mode,
                                            onClick = {
                                                editedProfile = editedProfile.copy(
                                                    masterDnsConfig = editedProfile.masterDnsConfig.copy(balancingMode = mode)
                                                )
                                            }
                                        )
                                        Text(mode.titlePersian, fontSize = 10.sp)
                                    }
                                }

                                Text("الگوریتم رمزنگاری جریان:", fontSize = 11.sp, fontWeight = FontWeight.SemiBold)
                                Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                                    MasterDnsEncryption.values().forEach { enc ->
                                        FilterChip(
                                            selected = editedProfile.masterDnsConfig.encryption == enc,
                                            onClick = {
                                                editedProfile = editedProfile.copy(
                                                    masterDnsConfig = editedProfile.masterDnsConfig.copy(encryption = enc)
                                                )
                                            },
                                            label = { Text(enc.label.split(" ").first(), fontSize = 10.sp) }
                                        )
                                    }
                                }

                                Text("سربار هدر ARQ: ${editedProfile.masterDnsConfig.headerOverheadBytes} بایت (۸۸٪ کمتر از DNSTT)", fontSize = 11.sp, color = Color(0xFF10B981), fontWeight = FontWeight.Bold)

                                Text("اندازه دلخواه MTU: ${editedProfile.masterDnsConfig.customMtu} Bytes", fontSize = 11.sp)
                                Slider(
                                    value = editedProfile.masterDnsConfig.customMtu.toFloat(),
                                    onValueChange = {
                                        editedProfile = editedProfile.copy(
                                            masterDnsConfig = editedProfile.masterDnsConfig.copy(customMtu = it.toInt())
                                        )
                                    },
                                    valueRange = 256f..1400f
                                )

                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Text("تکثیر بسته (Redundancy Duplication)", fontSize = 11.sp)
                                    Switch(
                                        checked = editedProfile.masterDnsConfig.enablePacketDuplication,
                                        onCheckedChange = {
                                            editedProfile = editedProfile.copy(
                                                masterDnsConfig = editedProfile.masterDnsConfig.copy(enablePacketDuplication = it)
                                            )
                                        }
                                    )
                                }
                            }
                            TunnelType.VAY_DNS, TunnelType.VAY_DNS_SSH -> {
                                Text("تنظیمات VayDNS Wire Format:", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                                Text("نوع رکورد DNS (Record Type):", fontSize = 11.sp)
                                Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                                    VayDnsRecordType.values().forEach { rec ->
                                        FilterChip(
                                            selected = editedProfile.vayDnsConfig.recordType == rec,
                                            onClick = {
                                                editedProfile = editedProfile.copy(
                                                    vayDnsConfig = editedProfile.vayDnsConfig.copy(recordType = rec)
                                                )
                                            },
                                            label = { Text(rec.name, fontSize = 10.sp) }
                                        )
                                    }
                                }

                                Text("طول نام کوئری QNAME: ${editedProfile.vayDnsConfig.qnameLength} بایت", fontSize = 11.sp)
                                Slider(
                                    value = editedProfile.vayDnsConfig.qnameLength.toFloat(),
                                    onValueChange = {
                                        editedProfile = editedProfile.copy(
                                            vayDnsConfig = editedProfile.vayDnsConfig.copy(qnameLength = it.toInt())
                                        )
                                    },
                                    valueRange = 32f..255f
                                )

                                Text("نرخ ارسال بسته (Rate Limit): ${editedProfile.vayDnsConfig.rateLimitPps} pps", fontSize = 11.sp)
                                Slider(
                                    value = editedProfile.vayDnsConfig.rateLimitPps.toFloat(),
                                    onValueChange = {
                                        editedProfile = editedProfile.copy(
                                            vayDnsConfig = editedProfile.vayDnsConfig.copy(rateLimitPps = it.toInt())
                                        )
                                    },
                                    valueRange = 10f..200f
                                )
                            }
                            TunnelType.NOIZ_DNS, TunnelType.NOIZ_DNS_SSH -> {
                                Text("تنظیمات NoizDNS Stealth:", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Text("حالت استتار کامل (Stealth Mode)", fontSize = 12.sp)
                                    Switch(
                                        checked = editedProfile.noizDnsConfig.stealthModeEnabled,
                                        onCheckedChange = {
                                            editedProfile = editedProfile.copy(
                                                noizDnsConfig = editedProfile.noizDnsConfig.copy(stealthModeEnabled = it)
                                            )
                                        }
                                    )
                                }
                                Text("نوسان زمان نویز (Noise Jitter): ${editedProfile.noizDnsConfig.jitterMs}ms", fontSize = 11.sp)
                                Slider(
                                    value = editedProfile.noizDnsConfig.jitterMs.toFloat(),
                                    onValueChange = {
                                        editedProfile = editedProfile.copy(
                                            noizDnsConfig = editedProfile.noizDnsConfig.copy(jitterMs = it.toInt())
                                        )
                                    },
                                    valueRange = 5f..50f
                                )
                            }
                            TunnelType.NAIVE_PROXY, TunnelType.NAIVE_PROXY_SSH -> {
                                Text("تنظیمات NaiveProxy (Chromium Layer):", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                                OutlinedTextField(
                                    value = editedProfile.naiveProxyConfig.host,
                                    onValueChange = {
                                        editedProfile = editedProfile.copy(
                                            naiveProxyConfig = editedProfile.naiveProxyConfig.copy(host = it)
                                        )
                                    },
                                    label = { Text("هاست سرور HTTPS") },
                                    modifier = Modifier.fillMaxWidth()
                                )
                                Text("اثرانگشت JA4 کروم: ${editedProfile.naiveProxyConfig.ja4Fingerprint}", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                            TunnelType.TOR -> {
                                Text("تنظیمات پل‌های تور (Tor Bridges):", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                                TorBridgeType.values().forEach { bridge ->
                                    Row(
                                        verticalAlignment = Alignment.CenterVertically,
                                        modifier = Modifier.clickable {
                                            editedProfile = editedProfile.copy(
                                                torConfig = editedProfile.torConfig.copy(bridgeType = bridge)
                                            )
                                        }
                                    ) {
                                        RadioButton(
                                            selected = editedProfile.torConfig.bridgeType == bridge,
                                            onClick = {
                                                editedProfile = editedProfile.copy(
                                                    torConfig = editedProfile.torConfig.copy(bridgeType = bridge)
                                                )
                                            }
                                        )
                                        Text(bridge.labelPersian, fontSize = 11.sp)
                                    }
                                }
                            }
                            else -> {
                                Text("تنظیمات استاندارد پروتکل ${editedProfile.tunnelType.title}", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                                OutlinedTextField(
                                    value = editedProfile.dnsConfig.resolverIpOrUrl,
                                    onValueChange = {
                                        editedProfile = editedProfile.copy(
                                            dnsConfig = editedProfile.dnsConfig.copy(resolverIpOrUrl = it)
                                        )
                                    },
                                    label = { Text("آدرس سرور / رِزولور") },
                                    modifier = Modifier.fillMaxWidth()
                                )
                            }
                        }
                    }
                    1 -> {
                        // SSH Wrappers & Ciphers
                        Text("الگوریتم رمزنگاری SSH (Cipher Selection):", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                        SshCipher.values().forEach { cipher ->
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                modifier = Modifier.clickable {
                                    editedProfile = editedProfile.copy(
                                        sshConfig = editedProfile.sshConfig.copy(cipher = cipher)
                                    )
                                }
                            ) {
                                RadioButton(
                                    selected = editedProfile.sshConfig.cipher == cipher,
                                    onClick = {
                                        editedProfile = editedProfile.copy(
                                            sshConfig = editedProfile.sshConfig.copy(cipher = cipher)
                                        )
                                    }
                                )
                                Text(cipher.label, fontSize = 11.sp)
                            }
                        }

                        Divider(modifier = Modifier.padding(vertical = 4.dp))

                        Text("نوع بسته‌بندی ترافیک SSH (Wrapper):", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                        SshWrapperType.values().forEach { wrap ->
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                modifier = Modifier.clickable {
                                    editedProfile = editedProfile.copy(
                                        sshConfig = editedProfile.sshConfig.copy(wrapperType = wrap)
                                    )
                                }
                            ) {
                                RadioButton(
                                    selected = editedProfile.sshConfig.wrapperType == wrap,
                                    onClick = {
                                        editedProfile = editedProfile.copy(
                                            sshConfig = editedProfile.sshConfig.copy(wrapperType = wrap)
                                        )
                                    }
                                )
                                Text(wrap.labelPersian, fontSize = 11.sp)
                            }
                        }

                        if (editedProfile.sshConfig.wrapperType == SshWrapperType.TLS) {
                            OutlinedTextField(
                                value = editedProfile.sshConfig.customSni,
                                onValueChange = {
                                    editedProfile = editedProfile.copy(
                                        sshConfig = editedProfile.sshConfig.copy(customSni = it)
                                    )
                                },
                                label = { Text("دامنه فرانتینگ SNI دلخواه") },
                                modifier = Modifier.fillMaxWidth()
                            )
                        }
                    }
                    2 -> {
                        // DNS Transport selection
                        Text("انتخاب بستر انتقال DNS (DNS Transport):", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                        DnsTransport.values().forEach { transport ->
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                modifier = Modifier.clickable {
                                    editedProfile = editedProfile.copy(
                                        dnsConfig = editedProfile.dnsConfig.copy(transport = transport)
                                    )
                                }
                            ) {
                                RadioButton(
                                    selected = editedProfile.dnsConfig.transport == transport,
                                    onClick = {
                                        editedProfile = editedProfile.copy(
                                            dnsConfig = editedProfile.dnsConfig.copy(transport = transport)
                                        )
                                    }
                                )
                                Text("${transport.label} (Port ${transport.port})", fontSize = 11.sp)
                            }
                        }

                        OutlinedTextField(
                            value = editedProfile.dnsConfig.serverDomain,
                            onValueChange = {
                                editedProfile = editedProfile.copy(
                                    dnsConfig = editedProfile.dnsConfig.copy(serverDomain = it)
                                )
                            },
                            label = { Text("دامنه اختصاصی تونل DNS (Server Domain)") },
                            modifier = Modifier.fillMaxWidth()
                        )
                    }
                }
            }
        },
        confirmButton = {
            Button(
                onClick = { onSave(editedProfile) },
                colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF10B981))
            ) {
                Text("ذخیره و اعمال")
            }
        },
        dismissButton = {
            OutlinedButton(onClick = onDismiss) {
                Text("انصراف")
            }
        }
    )
}
