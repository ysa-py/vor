package com.unifiedshield.ui

import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

data class CoreInfo(
    val id: String,
    val name: String,
    val description: String,
    val protocol: String,
    val obfuscation: Boolean,
    val recommended: Boolean
)

@Composable
fun CoreSwitcher(
    currentCore: String,
    onCoreSelected: (String) -> Unit
) {
    var selectedCore by remember { mutableStateOf(currentCore) }

    val cores = listOf(
        CoreInfo(
            id = "xray",
            name = "Xray Core",
            description = "VLESS + REALITY / XTLS Vision. Excellent for general Iranian ISPs.",
            protocol = "VLESS / REALITY",
            obfuscation = true,
            recommended = true
        ),
        CoreInfo(
            id = "singbox",
            name = "sing-box Core",
            description = "High-performance Universal Core with Shadowsocks-2022 & Hysteria2.",
            protocol = "Shadowsocks-2022 / Hysteria2",
            obfuscation = true,
            recommended = true
        ),
        CoreInfo(
            id = "hiddify",
            name = "Hiddify Multi-Core",
            description = "Automated fallback engine aggregating multi-node routes.",
            protocol = "Multi-Protocol",
            obfuscation = true,
            recommended = true
        ),
        CoreInfo(
            id = "psiphon",
            name = "Psiphon Core",
            description = "SSH / Obfuscated SSH & VPN tunnel with automatic egress rotation.",
            protocol = "Obfuscated SSH",
            obfuscation = true,
            recommended = false
        ),
        CoreInfo(
            id = "naive",
            name = "NaïveProxy",
            description = "Uses Chrome's network stack to evade traffic fingerprinting.",
            protocol = "HTTP/2 QUIC Proxy",
            obfuscation = true,
            recommended = false
        ),
        CoreInfo(
            id = "hysteria2",
            name = "Hysteria 2",
            description = "UDP-based QUIC protocol optimized for lossy mobile connections.",
            protocol = "QUIC Brutal",
            obfuscation = false,
            recommended = false
        ),
        CoreInfo(
            id = "tuic",
            name = "TUIC v5",
            description = "QUIC multiplexing proxy minimizing packet loss delay.",
            protocol = "QUIC v5",
            obfuscation = false,
            recommended = false
        ),
        CoreInfo(
            id = "wireguard",
            name = "WireGuard",
            description = "Standard UDP WireGuard protocol for clean networks.",
            protocol = "WireGuard UDP",
            obfuscation = false,
            recommended = false
        ),
        CoreInfo(
            id = "amneziawg",
            name = "AmneziaWG",
            description = "Obfuscated WireGuard header to bypass UDP handshake filtering.",
            protocol = "AmneziaWG Junk Packets",
            obfuscation = true,
            recommended = false
        )
    )

    Column(modifier = Modifier.fillMaxWidth()) {
        Text(
            text = "Anti-Censorship Protocol Cores",
            fontSize = 18.sp,
            fontWeight = FontWeight.Bold,
            modifier = Modifier.padding(bottom = 4.dp)
        )

        Text(
            text = "Select a core engine or let AI auto-switch when DPI is detected.",
            fontSize = 12.sp,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(bottom = 12.dp)
        )

        LazyColumn(
            verticalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.weight(1f, fill = false)
        ) {
            items(cores) { core ->
                CoreCard(
                    core = core,
                    isSelected = selectedCore == core.id,
                    onSelect = {
                        selectedCore = core.id
                        onCoreSelected(core.id)
                    }
                )
            }
        }
    }
}

@Composable
private fun CoreCard(
    core: CoreInfo,
    isSelected: Boolean,
    onSelect: () -> Unit
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .border(
                width = if (isSelected) 2.dp else 1.dp,
                color = if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outlineVariant,
                shape = RoundedCornerShape(12.dp)
            )
            .testTag("core_card_${core.id}")
            .clickable(onClick = onSelect),
        shape = RoundedCornerShape(12.dp),
        colors = CardDefaults.cardColors(
            containerColor = if (isSelected) MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f) else MaterialTheme.colorScheme.surface
        )
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(14.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            RadioButton(
                selected = isSelected,
                onClick = onSelect
            )

            Spacer(modifier = Modifier.width(10.dp))

            Column(modifier = Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = core.name,
                        fontSize = 15.sp,
                        fontWeight = FontWeight.Bold
                    )
                    if (core.recommended) {
                        Spacer(modifier = Modifier.width(6.dp))
                        SuggestionChip(
                            onClick = {},
                            label = { Text("Recommended", fontSize = 10.sp) },
                            modifier = Modifier.height(24.dp)
                        )
                    }
                }
                Text(
                    text = core.description,
                    fontSize = 12.sp,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 2.dp)
                )
                Row(
                    modifier = Modifier.padding(top = 6.dp),
                    horizontalArrangement = Arrangement.spacedBy(6.dp)
                ) {
                    AssistChip(
                        onClick = {},
                        label = { Text(core.protocol, fontSize = 10.sp) },
                        modifier = Modifier.height(22.dp)
                    )
                    if (core.obfuscation) {
                        AssistChip(
                            onClick = {},
                            label = { Text("Obfuscated", fontSize = 10.sp) },
                            modifier = Modifier.height(22.dp)
                        )
                    }
                }
            }
        }
    }
}
