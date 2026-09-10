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
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.NovelProtocol
import com.unifiedshield.NovelProtocolsEngine

@Composable
fun NovelProtocolsScreen() {
    val engine = remember { NovelProtocolsEngine.getInstance() }
    val protocols by engine.protocols.collectAsState()
    val activeId by engine.activeProtocolId.collectAsState()

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .testTag("novel_protocols_screen"),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        item {
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.4f)
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
                            .size(50.dp)
                            .clip(CircleShape)
                            .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.2f)),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            imageVector = Icons.Default.ElectricBolt,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(30.dp)
                        )
                    }

                    Spacer(modifier = Modifier.width(14.dp))

                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = "Next-Gen Novel Protocols (پروتکل‌های نسل جدید)",
                            fontSize = 16.sp,
                            fontWeight = FontWeight.Bold
                        )
                        Text(
                            text = "پروتکل‌های ضد فیلترینگ بدون امضا و پساکوانتومی جهت عبور از سخت‌ترین فیلترها",
                            fontSize = 11.sp,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
            }
        }

        items(protocols, key = { it.id }) { item ->
            val isActive = item.id == activeId

            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(
                        width = if (isActive) 2.dp else 1.dp,
                        color = if (isActive) Color(0xFF10B981) else MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.4f),
                        shape = RoundedCornerShape(16.dp)
                    )
                    .clickable { engine.selectProtocol(item.id) }
                    .testTag("protocol_item_${item.id}"),
                colors = CardDefaults.cardColors(
                    containerColor = if (isActive) Color(0x0F10B981) else MaterialTheme.colorScheme.surface
                ),
                shape = RoundedCornerShape(16.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Box(
                                modifier = Modifier
                                    .size(12.dp)
                                    .clip(CircleShape)
                                    .background(if (isActive) Color(0xFF10B981) else Color.Gray)
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(
                                text = item.namePersian,
                                fontSize = 15.sp,
                                fontWeight = FontWeight.Bold
                            )
                        }

                        if (isActive) {
                            Badge(containerColor = Color(0xFF10B981), contentColor = Color.White) {
                                Text("فعال (ACTIVE)")
                            }
                        } else {
                            Badge(containerColor = MaterialTheme.colorScheme.surfaceVariant) {
                                Text("آماده (READY)")
                            }
                        }
                    }

                    Text(
                        text = item.name,
                        fontSize = 12.sp,
                        color = MaterialTheme.colorScheme.primary,
                        fontWeight = FontWeight.Medium,
                        modifier = Modifier.padding(top = 2.dp)
                    )

                    Spacer(modifier = Modifier.height(6.dp))

                    Text(
                        text = item.descriptionPersian,
                        fontSize = 12.sp,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )

                    Spacer(modifier = Modifier.height(10.dp))

                    // Metrics Strip
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Column {
                            Text("سرعت تخمینی", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text(item.speedRating, fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFF10B981))
                        }

                        Column {
                            Text("پینگ (RTT)", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text("${item.latencyMs} ms", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                        }

                        Column {
                            Text("نمره عبور DPI", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text("${item.iranDpiBypassScore}/100", fontSize = 12.sp, fontWeight = FontWeight.Bold, color = Color(0xFF6366F1))
                        }

                        Column {
                            Text("استتار ترافیک", fontSize = 10.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Text(if (item.isZeroSignature) "فاقد امضا" else "نقاب‌دار", fontSize = 12.sp, fontWeight = FontWeight.Bold)
                        }
                    }

                    Spacer(modifier = Modifier.height(8.dp))

                    // Tags
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(6.dp)
                    ) {
                        item.features.take(3).forEach { feat ->
                            Surface(
                                color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                                shape = RoundedCornerShape(8.dp)
                            ) {
                                Text(
                                    text = feat,
                                    fontSize = 10.sp,
                                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}
