package com.unifiedshield.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowForward
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
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.profile.ProfileManager
import com.unifiedshield.tunnel.*

@Composable
fun MasterDnsScreen(
    profileManager: ProfileManager,
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current
    val masterDnsEngine = remember { MasterDnsEngine.getInstance(context) }
    val profileState by profileManager.state.collectAsState()
    val liveMetrics by masterDnsEngine.liveMetrics.collectAsState()

    val activeProfile = profileState.profiles.find { it.isActive } ?: profileState.profiles.first()
    var currentConfig by remember(activeProfile) { mutableStateOf(activeProfile.masterDnsConfig) }

    var selectedTab by remember { mutableIntStateOf(0) } // 0: Overview & Benchmarks, 1: Multi-Resolver, 2: ARQ & Protocol

    LazyColumn(
        modifier = modifier
            .fillMaxSize()
            .padding(horizontal = 16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
        contentPadding = PaddingValues(top = 12.dp, bottom = 80.dp)
    ) {
        item {
            MasterDnsHeroCard(liveMetrics = liveMetrics)
        }

        item {
            TabRow(
                selectedTabIndex = selectedTab,
                containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                contentColor = MaterialTheme.colorScheme.primary,
                modifier = Modifier
                    .clip(RoundedCornerShape(12.dp))
                    .testTag("masterdns_tab_row")
            ) {
                Tab(
                    selected = selectedTab == 0,
                    onClick = { selectedTab = 0 },
                    text = { Text("معماری و بنچمارک", fontWeight = FontWeight.Bold) },
                    icon = { Icon(Icons.Default.Speed, contentDescription = null, modifier = Modifier.size(18.dp)) }
                )
                Tab(
                    selected = selectedTab == 1,
                    onClick = { selectedTab = 1 },
                    text = { Text("۸ رِزولور و توازن", fontWeight = FontWeight.Bold) },
                    icon = { Icon(Icons.Default.Hub, contentDescription = null, modifier = Modifier.size(18.dp)) }
                )
                Tab(
                    selected = selectedTab == 2,
                    onClick = { selectedTab = 2 },
                    text = { Text("پروتکل و ARQ", fontWeight = FontWeight.Bold) },
                    icon = { Icon(Icons.Default.Tune, contentDescription = null, modifier = Modifier.size(18.dp)) }
                )
            }
        }

        when (selectedTab) {
            0 -> {
                // Overview & Comparison Benchmarks
                item {
                    MasterDnsComparisonBenchmarkCard(liveMetrics = liveMetrics)
                }

                item {
                    LiveArqTelemetryCard(
                        liveMetrics = liveMetrics,
                        config = currentConfig
                    )
                }

                item {
                    TcpForwardingCarrierCard(
                        config = currentConfig,
                        onCarrierChanged = { newCarrier ->
                            currentConfig = currentConfig.copy(tcpCarrier = newCarrier)
                            activeProfile.let { prof ->
                                profileManager.updateProfile(prof.copy(masterDnsConfig = currentConfig))
                            }
                            masterDnsEngine.setTcpCarrier(newCarrier)
                        }
                    )
                }
            }

            1 -> {
                // 8 Balancing Modes & Resolver Cluster
                item {
                    BalancingModeSelectorCard(
                        currentMode = currentConfig.balancingMode,
                        onModeSelected = { newMode ->
                            currentConfig = currentConfig.copy(
                                balancingMode = newMode,
                                enablePacketDuplication = (newMode == MasterDnsBalancingMode.DUPLICATE_BROADCAST)
                            )
                            activeProfile.let { prof ->
                                profileManager.updateProfile(prof.copy(masterDnsConfig = currentConfig))
                            }
                            masterDnsEngine.setBalancingMode(newMode)
                        }
                    )
                }

                item {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = "خوشه رِزولورهای همزمان (۸ نود فعال)",
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold
                        )
                        Button(
                            onClick = {
                                val probed = masterDnsEngine.probeResolvers(currentConfig)
                                currentConfig = probed
                                profileManager.updateProfile(activeProfile.copy(masterDnsConfig = probed))
                            },
                            contentPadding = PaddingValues(horizontal = 12.dp, vertical = 6.dp),
                            modifier = Modifier.testTag("probe_resolvers_button")
                        ) {
                            Icon(Icons.Default.Refresh, contentDescription = null, modifier = Modifier.size(16.dp))
                            Spacer(Modifier.width(4.dp))
                            Text("تست پروب پینگ", fontSize = 12.sp)
                        }
                    }
                }

                items(currentConfig.resolvers) { resolver ->
                    ResolverNodeCard(resolver = resolver)
                }
            }

            2 -> {
                // ARQ, MTU, Encryption, SOCKS5 Optimization
                item {
                    EncryptionAndMtuCard(
                        config = currentConfig,
                        onConfigChanged = { updated ->
                            currentConfig = updated
                            profileManager.updateProfile(activeProfile.copy(masterDnsConfig = updated))
                        }
                    )
                }

                item {
                    AdvancedProtocolFeaturesCard(
                        config = currentConfig,
                        onConfigChanged = { updated ->
                            currentConfig = updated
                            profileManager.updateProfile(activeProfile.copy(masterDnsConfig = updated))
                        }
                    )
                }
            }
        }
    }
}

@Composable
fun MasterDnsHeroCard(liveMetrics: MasterDnsLiveMetrics) {
    Card(
        shape = RoundedCornerShape(20.dp),
        colors = CardDefaults.cardColors(
            containerColor = Color.Transparent
        ),
        modifier = Modifier
            .fillMaxWidth()
            .border(
                1.dp,
                Brush.horizontalGradient(listOf(Color(0xFF00E5FF), Color(0xFF7C4DFF))),
                RoundedCornerShape(20.dp)
            )
            .background(
                Brush.linearGradient(
                    listOf(
                        Color(0xFF0F172A),
                        Color(0xFF1E1B4B)
                    )
                ),
                shape = RoundedCornerShape(20.dp)
            )
    ) {
        Column(
            modifier = Modifier.padding(20.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp)
        ) {
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
                            .size(44.dp)
                            .clip(CircleShape)
                            .background(Color(0xFF00E5FF).copy(alpha = 0.2f)),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            Icons.Default.Bolt,
                            contentDescription = null,
                            tint = Color(0xFF00E5FF),
                            modifier = Modifier.size(26.dp)
                        )
                    }
                    Column {
                        Text(
                            text = "MasterDnsVPN Protocol",
                            style = MaterialTheme.typography.titleLarge,
                            fontWeight = FontWeight.Black,
                            color = Color.White
                        )
                        Text(
                            text = "پروتکل اختصاصی ARQ با هدر فوق‌سبک ۵ بایت",
                            fontSize = 12.sp,
                            color = Color(0xFF94A3B8)
                        )
                    }
                }

                Surface(
                    shape = RoundedCornerShape(12.dp),
                    color = Color(0xFF10B981).copy(alpha = 0.2f),
                    border = BorderStroke(1.dp, Color(0xFF10B981))
                ) {
                    Row(
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp)
                    ) {
                        Box(
                            modifier = Modifier
                                .size(8.dp)
                                .clip(CircleShape)
                                .background(Color(0xFF10B981))
                        )
                        Text("Active 9X", fontSize = 11.sp, fontWeight = FontWeight.Bold, color = Color(0xFF10B981))
                    }
                }
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                MetricChip(label = "سربار هدر", value = "5-7 Bytes", sub = "۸۸٪ کمتر از DNSTT", color = Color(0xFF00E5FF))
                MetricChip(label = "هندشیک با کش", value = "0.270s", sub = "در برابر 1.746s عادی", color = Color(0xFF10B981))
                MetricChip(label = "شتاب سرعت", value = "Up to 9x", sub = "۳.۶ برابر Slipstream", color = Color(0xFFF59E0B))
            }
        }
    }
}

@Composable
fun MetricChip(label: String, value: String, sub: String, color: Color) {
    Card(
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(containerColor = Color.White.copy(alpha = 0.06f)),
        modifier = Modifier.width(102.dp)
    ) {
        Column(
            modifier = Modifier.padding(8.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            Text(label, fontSize = 10.sp, color = Color(0xFF94A3B8), textAlign = TextAlign.Center)
            Text(value, fontSize = 14.sp, fontWeight = FontWeight.Black, color = color, textAlign = TextAlign.Center)
            Text(sub, fontSize = 9.sp, color = Color(0xFFCBD5E1), textAlign = TextAlign.Center, maxLines = 1)
        }
    }
}

@Composable
fun MasterDnsComparisonBenchmarkCard(liveMetrics: MasterDnsLiveMetrics) {
    Card(
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.45f))
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Text(
                text = "مقایسه عملکردی MasterDnsVPN با سایر پروتکل‌ها",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold
            )

            // Comparison row 1: MasterDns vs DNSTT
            BenchmarkComparisonRow(
                protocolName = "MasterDnsVPN",
                protocolSecondary = "DNSTT (KCP/Noise)",
                metricName = "سربار هدر پروتکل (Overhead)",
                currentValue = "5-7 Bytes",
                benchmarkValue = "42 Bytes",
                improvementText = "۸۸٪ سربار کمتر",
                progress = 0.88f,
                accentColor = Color(0xFF10B981)
            )

            // Comparison row 2: Speed vs Slipstream
            BenchmarkComparisonRow(
                protocolName = "MasterDnsVPN",
                protocolSecondary = "Slipstream (QUIC)",
                metricName = "کارایی و سرعت در شرایط اختلال شدید",
                currentValue = "94.6 Mbps",
                benchmarkValue = "26.2 Mbps",
                improvementText = "۳.۶ برابر سریع‌تر (Up to 3.6x)",
                progress = 0.78f,
                accentColor = Color(0xFF00E5FF)
            )

            // Comparison row 3: Handshake speed with DNS cache
            BenchmarkComparisonRow(
                protocolName = "MasterDns (L1/L2 Cache)",
                protocolSecondary = "Standard DNS Handshake",
                metricName = "زمان برقراری اتصال اولیه (Initial Handshake)",
                currentValue = "0.270 ثانیه",
                benchmarkValue = "1.746 ثانیه",
                improvementText = "۶.۵ برابر سرعت اتصال بالاتر",
                progress = 0.85f,
                accentColor = Color(0xFFF59E0B)
            )
        }
    }
}

@Composable
fun BenchmarkComparisonRow(
    protocolName: String,
    protocolSecondary: String,
    metricName: String,
    currentValue: String,
    benchmarkValue: String,
    improvementText: String,
    progress: Float,
    accentColor: Color
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(12.dp))
            .background(MaterialTheme.colorScheme.surface.copy(alpha = 0.6f))
            .padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp)
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Text(metricName, fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
            Text(improvementText, fontSize = 11.sp, fontWeight = FontWeight.Bold, color = accentColor)
        }

        LinearProgressIndicator(
            progress = { progress },
            modifier = Modifier
                .fillMaxWidth()
                .height(8.dp)
                .clip(RoundedCornerShape(4.dp)),
            color = accentColor,
            trackColor = MaterialTheme.colorScheme.surfaceVariant
        )

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Text("$protocolName: $currentValue", fontSize = 11.sp, color = accentColor, fontWeight = FontWeight.Bold)
            Text("$protocolSecondary: $benchmarkValue", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@Composable
fun LiveArqTelemetryCard(
    liveMetrics: MasterDnsLiveMetrics,
    config: MasterDnsConfig
) {
    Card(
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.45f))
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "وضعیت زنده ماشین حالت ARQ و مالتی‌پث",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold
                )
                Icon(Icons.Default.Sensors, contentDescription = null, tint = Color(0xFF10B981))
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                TelemetryStatBox(
                    title = "بسته‌های ارسالی",
                    value = "${liveMetrics.packetsTransmitted}",
                    icon = Icons.Default.ArrowUpward,
                    color = Color(0xFF00E5FF)
                )
                TelemetryStatBox(
                    title = "بسته‌های دریافتی",
                    value = "${liveMetrics.packetsReceived}",
                    icon = Icons.Default.ArrowDownward,
                    color = Color(0xFF10B981)
                )
                TelemetryStatBox(
                    title = "نرخ کش DNS",
                    value = "${liveMetrics.cacheHitRatioPct}%",
                    icon = Icons.Default.Memory,
                    color = Color(0xFFF59E0B)
                )
            }

            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(8.dp))
                    .background(MaterialTheme.colorScheme.surface.copy(alpha = 0.5f))
                    .padding(10.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text("حالت بالانس فعال:", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(
                    liveMetrics.activeBalancingMode.titlePersian,
                    fontSize = 12.sp,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.primary
                )
            }
        }
    }
}

@Composable
fun TelemetryStatBox(title: String, value: String, icon: androidx.compose.ui.graphics.vector.ImageVector, color: Color) {
    Surface(
        shape = RoundedCornerShape(10.dp),
        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.7f),
        modifier = Modifier.width(100.dp)
    ) {
        Column(
            modifier = Modifier.padding(8.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Icon(icon, contentDescription = null, tint = color, modifier = Modifier.size(16.dp))
            Spacer(Modifier.height(4.dp))
            Text(value, fontSize = 13.sp, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace)
            Text(title, fontSize = 9.sp, color = MaterialTheme.colorScheme.onSurfaceVariant, textAlign = TextAlign.Center)
        }
    }
}

@Composable
fun TcpForwardingCarrierCard(
    config: MasterDnsConfig,
    onCarrierChanged: (MasterDnsTcpCarrier) -> Unit
) {
    Card(
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.45f))
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp)
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Icon(Icons.Default.Layers, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                Text(
                    text = "حالت حمل ترافیک TCP (TCP Forwarding Mode)",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold
                )
            }

            Text(
                text = "حمل نامحسوس پروتکل‌های TCP مانند Shadowsocks، VLESS/VMess، Trojan و OpenVPN بر بستر بسته‌های سبک MasterDns بدون قطعی.",
                fontSize = 12.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )

            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                MasterDnsTcpCarrier.values().forEach { carrier ->
                    val isSelected = config.tcpCarrier == carrier
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(10.dp))
                            .background(
                                if (isSelected) MaterialTheme.colorScheme.primary.copy(alpha = 0.15f)
                                else MaterialTheme.colorScheme.surface.copy(alpha = 0.4f)
                            )
                            .border(
                                width = if (isSelected) 1.dp else 0.dp,
                                color = if (isSelected) MaterialTheme.colorScheme.primary else Color.Transparent,
                                shape = RoundedCornerShape(10.dp)
                            )
                            .clickable { onCarrierChanged(carrier) }
                            .padding(12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Column {
                            Text(carrier.titlePersian, fontWeight = FontWeight.Bold, fontSize = 13.sp)
                            Text(carrier.protocolName, fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                        RadioButton(
                            selected = isSelected,
                            onClick = { onCarrierChanged(carrier) }
                        )
                    }
                }
            }
        }
    }
}

@Composable
fun BalancingModeSelectorCard(
    currentMode: MasterDnsBalancingMode,
    onModeSelected: (MasterDnsBalancingMode) -> Unit
) {
    Card(
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.45f))
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp)
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Icon(Icons.Default.AltRoute, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
                Text(
                    text = "۸ حالت متعادل‌سازی بار رِزولورها (Balancing Modes)",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold
                )
            }

            Text(
                text = "انتخاب استراتژی هوشمند برای هدایت، تکثیر یا توازن کوئری‌های DNS میان سرورها:",
                fontSize = 12.sp,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )

            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                MasterDnsBalancingMode.values().forEach { mode ->
                    val isSelected = currentMode == mode
                    Card(
                        shape = RoundedCornerShape(10.dp),
                        colors = CardDefaults.cardColors(
                            containerColor = if (isSelected) MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.6f)
                            else MaterialTheme.colorScheme.surface.copy(alpha = 0.5f)
                        ),
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { onModeSelected(mode) }
                    ) {
                        Row(
                            modifier = Modifier.padding(12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text(
                                    mode.titlePersian,
                                    fontWeight = FontWeight.Bold,
                                    fontSize = 13.sp,
                                    color = if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface
                                )
                                Text(
                                    mode.modeName,
                                    fontSize = 11.sp,
                                    fontFamily = FontFamily.Monospace,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                                Text(
                                    mode.descriptionPersian,
                                    fontSize = 11.sp,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                            RadioButton(
                                selected = isSelected,
                                onClick = { onModeSelected(mode) }
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun ResolverNodeCard(resolver: MasterDnsResolverNode) {
    Card(
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface.copy(alpha = 0.7f)),
        modifier = Modifier.fillMaxWidth()
    ) {
        Row(
            modifier = Modifier.padding(12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                Box(
                    modifier = Modifier
                        .size(10.dp)
                        .clip(CircleShape)
                        .background(if (resolver.isAlive) Color(0xFF10B981) else Color.Red)
                )
                Column {
                    Text(resolver.name, fontWeight = FontWeight.Bold, fontSize = 13.sp)
                    Text(
                        "${resolver.address}:${resolver.port} (${resolver.transport.label})",
                        fontSize = 11.sp,
                        fontFamily = FontFamily.Monospace,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            Column(horizontalAlignment = Alignment.End) {
                Text(
                    "${resolver.pingMs} ms",
                    fontWeight = FontWeight.Bold,
                    fontSize = 13.sp,
                    color = if (resolver.pingMs < 20) Color(0xFF10B981) else Color(0xFFF59E0B)
                )
                Text(
                    "افت: ${resolver.packetLossPct}%",
                    fontSize = 10.sp,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}

@Composable
fun EncryptionAndMtuCard(
    config: MasterDnsConfig,
    onConfigChanged: (MasterDnsConfig) -> Unit
) {
    Card(
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.45f))
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Text(
                text = "رمزنگاری و تنظیم MTU اختصاصی",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold
            )

            Text(
                text = "الگوریتم رمزنگاری جریان:",
                fontSize = 12.sp,
                fontWeight = FontWeight.SemiBold
            )

            MasterDnsEncryption.values().forEach { enc ->
                val isSelected = config.encryption == enc
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(8.dp))
                        .background(
                            if (isSelected) MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.5f)
                            else MaterialTheme.colorScheme.surface.copy(alpha = 0.4f)
                        )
                        .clickable { onConfigChanged(config.copy(encryption = enc)) }
                        .padding(10.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Column {
                        Text(enc.label, fontWeight = FontWeight.Bold, fontSize = 13.sp)
                        Text(enc.labelPersian, fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    RadioButton(
                        selected = isSelected,
                        onClick = { onConfigChanged(config.copy(encryption = enc)) }
                    )
                }
            }

            Spacer(Modifier.height(8.dp))

            Text(
                text = "اندازه MTU دلخواه: ${config.customMtu} Bytes (پشتیبانی حتی از MTUهای بسیار ریز)",
                fontSize = 12.sp,
                fontWeight = FontWeight.SemiBold
            )

            Slider(
                value = config.customMtu.toFloat(),
                onValueChange = { onConfigChanged(config.copy(customMtu = it.toInt())) },
                valueRange = 256f..1400f,
                steps = 10,
                modifier = Modifier.testTag("mtu_slider")
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text("۲۵۶ بایت (حداقل)", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text("۵۱۲ بایت (استاندارد)", fontSize = 10.sp, color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.Bold)
                Text("۱۴۰۰ بایت (حداکثر)", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}

@Composable
fun AdvancedProtocolFeaturesCard(
    config: MasterDnsConfig,
    onConfigChanged: (MasterDnsConfig) -> Unit
) {
    Card(
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.45f))
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp)
        ) {
            Text(
                text = "بهینه‌سازی‌های پیشرفته MasterDns",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text("کاهش سربار SOCKS5 / SOCKS4", fontWeight = FontWeight.Bold, fontSize = 13.sp)
                    Text("فشرده‌سازی هدرهای پراکسی محلی جهت افزایش راندمان", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Switch(
                    checked = config.enableSocksOptimization,
                    onCheckedChange = { onConfigChanged(config.copy(enableSocksOptimization = it)) }
                )
            }

            HorizontalDivider()

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text("کش چند لایه DNS (اتصال 0.270s)", fontWeight = FontWeight.Bold, fontSize = 13.sp)
                    Text("پیش‌بارگذاری و کش هوشمند پاسخ‌های DNS برای هندشیک آنی", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Switch(
                    checked = config.enableStrongDnsCache,
                    onCheckedChange = { onConfigChanged(config.copy(enableStrongDnsCache = it)) }
                )
            }

            HorizontalDivider()

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text("موتور هسته (Core Engine)", fontWeight = FontWeight.Bold, fontSize = 13.sp)
                    Text(config.coreEngine.label, fontSize = 11.sp, color = MaterialTheme.colorScheme.primary)
                }
                Text(config.coreEngine.language, fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.secondary)
            }
        }
    }
}
