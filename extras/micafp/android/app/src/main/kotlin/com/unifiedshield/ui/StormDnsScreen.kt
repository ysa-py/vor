package com.unifiedshield.ui

import androidx.compose.animation.*
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
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
import com.unifiedshield.stormdns.*

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun StormDnsScreen() {
    val context = LocalContext.current
    val engine = remember { StormDnsEngine.getInstance(context) }
    val state by engine.state.collectAsState()

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .testTag("stormdns_screen"),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        // Hero Card
        item {
            Card(
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                border = BorderStroke(1.dp, Color(0xFFF59E0B).copy(alpha = 0.4f)),
                shape = RoundedCornerShape(20.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .background(
                            Brush.linearGradient(
                                colors = listOf(
                                    Color(0xFFD97706).copy(alpha = 0.20f),
                                    Color(0xFF0F172A).copy(alpha = 0.95f)
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
                                        .background(Color(0xFFF59E0B).copy(alpha = 0.25f)),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Icon(
                                        imageVector = Icons.Default.Storm,
                                        contentDescription = null,
                                        tint = Color(0xFFFBBF24),
                                        modifier = Modifier.size(28.dp)
                                    )
                                }
                                Spacer(modifier = Modifier.width(12.dp))
                                Column {
                                    Text(
                                        text = "StormDNS Tunneling Core",
                                        fontSize = 17.sp,
                                        fontWeight = FontWeight.ExtraBold,
                                        color = Color.White
                                    )
                                    Text(
                                        text = "TCP-over-DNS Stream Transport with ARQ",
                                        fontSize = 11.sp,
                                        color = Color(0xFFFDE68A)
                                    )
                                }
                            }

                            Switch(
                                checked = state.isTunnelRunning,
                                onCheckedChange = { engine.toggleTunnel(it) },
                                modifier = Modifier.testTag("stormdns_toggle")
                            )
                        }

                        Spacer(modifier = Modifier.height(12.dp))

                        Text(
                            text = "سامانه انتقال ترافیک TCP در کوئری‌ها و پاسخ‌های DNS (UDP/53) با شنود محلی SOCKS5، پنجره‌های ARQ، فشرده‌سازی ZSTD و تکثیر هوشمند جهتی بسته‌ها برای شکست بن‌بست آپلود در شبکه‌های متخاصم.",
                            fontSize = 12.sp,
                            color = Color(0xFFE2E8F0),
                            lineHeight = 18.sp
                        )

                        Spacer(modifier = Modifier.height(10.dp))

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Badge(containerColor = Color(0xFFF59E0B), contentColor = Color.Black) {
                                Text("SOCKS5 127.0.0.1:${state.localSocks5Port}", fontWeight = FontWeight.Bold)
                            }
                            Badge(containerColor = Color(0xFF10B981), contentColor = Color.White) {
                                Text("ZSTD + AES-128-GCM")
                            }
                        }
                    }
                }
            }
        }

        // Live Telemetry Card
        item {
            Card(
                colors = CardDefaults.cardColors(containerColor = Color(0xFF1E293B)),
                shape = RoundedCornerShape(16.dp),
                border = BorderStroke(1.dp, if (state.isTunnelRunning) Color(0xFF10B981) else Color(0xFF64748B).copy(alpha = 0.5f)),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = if (state.isTunnelRunning) "تونل StormDNS فعال است" else "تونل غیرفعال است",
                            fontSize = 13.sp,
                            fontWeight = FontWeight.Bold,
                            color = if (state.isTunnelRunning) Color(0xFF34D399) else Color(0xFF94A3B8)
                        )
                        Text(
                            text = "RTO: ${state.dynamicRtoMs}ms",
                            fontSize = 12.sp,
                            fontWeight = FontWeight.Bold,
                            color = Color(0xFF38BDF8)
                        )
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Column {
                            Text("ترافیک ارسالی (TX):", fontSize = 11.sp, color = Color(0xFF94A3B8))
                            Text("${state.bytesTransmitted / 1024} KB", fontSize = 13.sp, fontWeight = FontWeight.Bold, color = Color.White)
                        }
                        Column {
                            Text("ترافیک دریافتی (RX):", fontSize = 11.sp, color = Color(0xFF94A3B8))
                            Text("${state.bytesReceived / 1024} KB", fontSize = 13.sp, fontWeight = FontWeight.Bold, color = Color(0xFF34D399))
                        }
                        Column {
                            Text("بازتکرار ARQ:", fontSize = 11.sp, color = Color(0xFF94A3B8))
                            Text("${state.arqRetransmissions}", fontSize = 13.sp, fontWeight = FontWeight.Bold, color = Color(0xFFF59E0B))
                        }
                        Column {
                            Text("جریان‌های فعال:", fontSize = 11.sp, color = Color(0xFF94A3B8))
                            Text("${state.activeStreamsCount} Streams", fontSize = 13.sp, fontWeight = FontWeight.Bold, color = Color.White)
                        }
                    }
                }
            }
        }

        // Directional Duplication Controls (StormDNS Specialty)
        item {
            Card(
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.35f)),
                shape = RoundedCornerShape(18.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Text(
                        text = "کنترل‌های تکثیر جهت‌دار (Directional Duplication):",
                        fontSize = 13.sp,
                        fontWeight = FontWeight.Bold,
                        color = Color(0xFFF59E0B)
                    )
                    Text(
                        text = "جبران پهنای باند آپلود با ضریب ارسال مجدد جداگانه برای بسته‌های دیتا، تاییدیه (ACK)، هندشیک و کنترل.",
                        fontSize = 11.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )

                    // ACK Duplication
                    Column {
                        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                            Text("تکثیر بسته‌های ACK (تثبیت آپلود):", fontSize = 11.sp)
                            Text("${state.duplicationControls.ackDuplication}x Copies", fontSize = 11.sp, fontWeight = FontWeight.Bold, color = Color(0xFFF59E0B))
                        }
                        Slider(
                            value = state.duplicationControls.ackDuplication.toFloat(),
                            onValueChange = {
                                engine.updateDuplication(
                                    data = state.duplicationControls.dataDuplication,
                                    ack = it.toInt(),
                                    setup = state.duplicationControls.setupDuplication,
                                    control = state.duplicationControls.controlDuplication
                                )
                            },
                            valueRange = 1f..4f
                        )
                    }

                    // Setup Handshake Duplication
                    Column {
                        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                            Text("تکثیر بسته‌های راه‌اندازی (Handshake Setup):", fontSize = 11.sp)
                            Text("${state.duplicationControls.setupDuplication}x Copies", fontSize = 11.sp, fontWeight = FontWeight.Bold, color = Color(0xFF10B981))
                        }
                        Slider(
                            value = state.duplicationControls.setupDuplication.toFloat(),
                            onValueChange = {
                                engine.updateDuplication(
                                    data = state.duplicationControls.dataDuplication,
                                    ack = state.duplicationControls.ackDuplication,
                                    setup = it.toInt(),
                                    control = state.duplicationControls.controlDuplication
                                )
                            },
                            valueRange = 2f..5f
                        )
                    }

                    // Data Duplication
                    Column {
                        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                            Text("تکثیر بسته‌های داده (Data Frames):", fontSize = 11.sp)
                            Text("${state.duplicationControls.dataDuplication}x Copies", fontSize = 11.sp, fontWeight = FontWeight.Bold)
                        }
                        Slider(
                            value = state.duplicationControls.dataDuplication.toFloat(),
                            onValueChange = {
                                engine.updateDuplication(
                                    data = it.toInt(),
                                    ack = state.duplicationControls.ackDuplication,
                                    setup = state.duplicationControls.setupDuplication,
                                    control = state.duplicationControls.controlDuplication
                                )
                            },
                            valueRange = 1f..3f
                        )
                    }
                }
            }
        }

        // Balancing & Efficiency Card
        item {
            Card(
                shape = RoundedCornerShape(18.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.35f)),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Text("استراتژی بالانس رِزولورها:", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(6.dp)
                    ) {
                        FilterChip(
                            selected = state.balancing == StormDnsBalancing.LEAST_LOSS,
                            onClick = { engine.updateBalancing(StormDnsBalancing.LEAST_LOSS) },
                            label = { Text("Least Loss", fontSize = 10.sp) }
                        )
                        FilterChip(
                            selected = state.balancing == StormDnsBalancing.LOWEST_LATENCY,
                            onClick = { engine.updateBalancing(StormDnsBalancing.LOWEST_LATENCY) },
                            label = { Text("Fastest RTT", fontSize = 10.sp) }
                        )
                        FilterChip(
                            selected = state.balancing == StormDnsBalancing.ROUND_ROBIN,
                            onClick = { engine.updateBalancing(StormDnsBalancing.ROUND_ROBIN) },
                            label = { Text("Round Robin", fontSize = 10.sp) }
                        )
                    }

                    Text("الگوریتم فشرده‌سازی داده (Compression):", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(6.dp)
                    ) {
                        StormDnsCompression.values().forEach { comp ->
                            FilterChip(
                                selected = state.compression == comp,
                                onClick = { engine.updateCompression(comp) },
                                label = { Text(comp.name, fontSize = 10.sp) }
                            )
                        }
                    }

                    Text("الگوریتم رمزنگاری جریان:", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(4.dp)
                    ) {
                        FilterChip(
                            selected = state.cipher == StormDnsCipher.AES_128_GCM,
                            onClick = { engine.updateCipher(StormDnsCipher.AES_128_GCM) },
                            label = { Text("AES-128", fontSize = 10.sp) }
                        )
                        FilterChip(
                            selected = state.cipher == StormDnsCipher.CHACHA20,
                            onClick = { engine.updateCipher(StormDnsCipher.CHACHA20) },
                            label = { Text("ChaCha20", fontSize = 10.sp) }
                        )
                        FilterChip(
                            selected = state.cipher == StormDnsCipher.XOR,
                            onClick = { engine.updateCipher(StormDnsCipher.XOR) },
                            label = { Text("XOR Zero", fontSize = 10.sp) }
                        )
                    }

                    // MTU Slider
                    Column {
                        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                            Text("کشف و تنظیم MTU فعال:", fontSize = 11.sp)
                            Text("${state.activeMtu} Bytes", fontSize = 11.sp, fontWeight = FontWeight.Bold, color = Color(0xFF0284C7))
                        }
                        Slider(
                            value = state.activeMtu.toFloat(),
                            onValueChange = { engine.updateMtu(it.toInt()) },
                            valueRange = 256f..1400f
                        )
                    }

                    OutlinedTextField(
                        value = state.tunnelDomain,
                        onValueChange = { engine.updateTunnelDomain(it) },
                        label = { Text("دامنه اختصاصی تونل StormDNS") },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true
                    )
                }
            }
        }

        // Resolvers Table Header
        item {
            Text(
                text = "رِزولورهای فعال StormDNS (${state.resolvers.size} سرور):",
                fontSize = 13.sp,
                fontWeight = FontWeight.Bold
            )
        }

        // Resolvers List
        items(state.resolvers, key = { it.id }) { res ->
            Card(
                shape = RoundedCornerShape(12.dp),
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.4f)),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(12.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Box(
                            modifier = Modifier
                                .size(10.dp)
                                .clip(CircleShape)
                                .background(if (res.isActive) Color(0xFF10B981) else Color(0xFFEF4444))
                        )
                        Spacer(modifier = Modifier.width(10.dp))
                        Column {
                            Text(res.address, fontSize = 13.sp, fontWeight = FontWeight.Bold, fontFamily = FontFamily.Monospace)
                            Text("MTU: ${res.discoveredMtu}B • پورت: ${res.port}", fontSize = 11.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }

                    Column(horizontalAlignment = Alignment.End) {
                        Text("${res.latencyMs}ms", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFF38BDF8))
                        Text("افت: ${res.packetLossPct}%", fontSize = 10.sp, color = if (res.packetLossPct < 1.0) Color(0xFF10B981) else Color(0xFFEF4444))
                    }
                }
            }
        }
    }
}
