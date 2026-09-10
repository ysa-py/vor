package com.unifiedshield.ui

import android.app.Activity
import android.content.Intent
import android.speech.RecognizerIntent
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.core.*
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyRow
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
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.unifiedshield.AiStealthEngine
import com.unifiedshield.TunnelManager
import com.unifiedshield.aiorchestrator.AiCoreOrchestrator
import com.unifiedshield.logging.DebugLogger
import com.unifiedshield.ui.components.EnterpriseStatusPill
import com.unifiedshield.ui.components.StatusPillType
import com.unifiedshield.ui.theme.EnterpriseColors

@Composable
fun VoiceCommandDialog(
    onDismissRequest: () -> Unit,
    onConnectRequested: () -> Unit = {},
    onDisconnectRequested: () -> Unit = {}
) {
    val context = LocalContext.current
    val tunnelManager = remember { TunnelManager.getInstance(context) }
    val orchestrator = remember { AiCoreOrchestrator.getInstance() }
    val logger = remember { DebugLogger.getInstance() }

    var lastRecognizedText by remember { mutableStateOf<String?>(null) }
    var executionStatus by remember { mutableStateOf("برای گفتن دستور روی میکروفون ضربه بزنید یا از میانبرها استفاده کنید") }
    var isListening by remember { mutableStateOf(false) }

    val infiniteTransition = rememberInfiniteTransition(label = "voice_wave")
    val pulseScale by infiniteTransition.animateFloat(
        initialValue = 1.0f,
        targetValue = 1.25f,
        animationSpec = infiniteRepeatable(
            animation = tween(600, easing = LinearEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "pulseScale"
    )

    fun processVoiceCommand(commandRaw: String) {
        val command = commandRaw.trim().lowercase()
        lastRecognizedText = commandRaw
        logger.info("VoiceCommand", "Received voice command: '$commandRaw'")

        when {
            command.contains("connect") || command.contains("start") || command.contains("اتصال") || command.contains("وصل") || command.contains("روشن") -> {
                tunnelManager.updateConnectionState(true)
                onConnectRequested()
                executionStatus = "✅ دستور '$commandRaw' اجرا شد: اتصال تانل برقرار گردید!"
                logger.info("VoiceCommand", "Voice command triggered VPN CONNECT")
            }
            command.contains("disconnect") || command.contains("stop") || command.contains("off") || command.contains("قطع") || command.contains("خاموش") -> {
                tunnelManager.updateConnectionState(false)
                onDisconnectRequested()
                executionStatus = "🛑 دستور '$commandRaw' اجرا شد: اتصال تانل قطع شد!"
                logger.info("VoiceCommand", "Voice command triggered VPN DISCONNECT")
            }
            command.contains("switch") || command.contains("core") || command.contains("تغییر") || command.contains("هسته") || command.contains("عوض") -> {
                val newCore = tunnelManager.performHotSwap("Voice Command Request")
                orchestrator.autoShiftToBestCore("Voice Command Request")
                executionStatus = "🔀 دستور '$commandRaw' اجرا شد: هسته فعال به $newCore تغییر یافت!"
                logger.info("VoiceCommand", "Voice command triggered CORE SWITCH -> $newCore")
            }
            command.contains("stealth") || command.contains("استتار") || command.contains("مخفی") -> {
                AiStealthEngine.getInstance().evaluateTrafficSignal(256, 12, true, 45, "VOICE_STEALTH_BOOST")
                executionStatus = "🛡️ دستور '$commandRaw' اجرا شد: حالت حداکثر استتار فعال گردید!"
                logger.info("VoiceCommand", "Voice command triggered STEALTH BOOST")
            }
            else -> {
                executionStatus = "❓ دستور '$commandRaw' شناسایی نشد. می‌توانید از عبارت‌های 'Connect VPN'، 'Disconnect' یا 'Switch Core' استفاده کنید."
            }
        }
    }

    val speechLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.StartActivityForResult()
    ) { result ->
        isListening = false
        if (result.resultCode == Activity.RESULT_OK) {
            val spokenText = result.data
                ?.getStringArrayListExtra(RecognizerIntent.EXTRA_RESULTS)
                ?.firstOrNull()
            if (!spokenText.isNullOrBlank()) {
                processVoiceCommand(spokenText)
            } else {
                executionStatus = "صدایی دریافت نشد. لطفاً دوباره تلاش کنید."
            }
        } else {
            executionStatus = "تشخیص گفتار لغو شد یا در دسترس نیست."
        }
    }

    fun launchSpeechRecognizer() {
        isListening = true
        executionStatus = "در حال گوش دادن..."
        val intent = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
            putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            putExtra(RecognizerIntent.EXTRA_PROMPT, "دستور صوتی را بگوئید (مثلاً 'Connect VPN', 'Switch Core', 'اتصال')")
        }
        try {
            speechLauncher.launch(intent)
        } catch (e: Exception) {
            isListening = false
            executionStatus = "سرویس گفتار گوگل در این دستگاه در دسترس نیست. می‌توانید از میانبرهای زیر استفاده کنید."
        }
    }

    AlertDialog(
        onDismissRequest = onDismissRequest,
        shape = RoundedCornerShape(24.dp),
        containerColor = EnterpriseColors.CyberSurfaceDark,
        title = {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(
                        modifier = Modifier
                            .size(36.dp)
                            .clip(RoundedCornerShape(10.dp))
                            .background(EnterpriseColors.EmeraldPrimaryDark.copy(alpha = 0.2f))
                            .border(1.dp, EnterpriseColors.EmeraldNeon.copy(alpha = 0.6f), RoundedCornerShape(10.dp)),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            imageVector = Icons.Default.Mic,
                            contentDescription = null,
                            tint = EnterpriseColors.EmeraldNeon,
                            modifier = Modifier.size(20.dp)
                        )
                    }
                    Spacer(modifier = Modifier.width(10.dp))
                    Text("دستیار صوتی سایبری (AI Voice Cockpit)", fontSize = 15.sp, fontWeight = FontWeight.Black, color = Color.White)
                }
            }
        },
        text = {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(14.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                // Mic Pulse Visualizer
                Box(
                    modifier = Modifier
                        .size(88.dp)
                        .scale(if (isListening) pulseScale else 1.0f)
                        .clip(CircleShape)
                        .background(
                            brush = if (isListening) {
                                Brush.radialGradient(listOf(EnterpriseColors.EmeraldNeon, EnterpriseColors.CyanQuantum))
                            } else {
                                Brush.verticalGradient(listOf(EnterpriseColors.CyberSurfaceElevated, EnterpriseColors.CyberSurfaceCard))
                            }
                        )
                        .border(
                            2.5.dp,
                            if (isListening) EnterpriseColors.EmeraldGlow else EnterpriseColors.CyberBorderSubtle,
                            CircleShape
                        )
                        .clickable { launchSpeechRecognizer() }
                        .testTag("voice_mic_button"),
                    contentAlignment = Alignment.Center
                ) {
                    Icon(
                        imageVector = Icons.Default.Mic,
                        contentDescription = "میکروفون",
                        tint = if (isListening) Color.White else EnterpriseColors.EmeraldNeon,
                        modifier = Modifier.size(42.dp)
                    )
                }

                Text(
                    text = if (isListening) "در حال شنیدن و تحلیل فرکانس صوتی..." else "جهت ضبط فرمان صوتی روی میکروفون ضربه بزنید",
                    fontSize = 12.sp,
                    fontWeight = FontWeight.Bold,
                    color = if (isListening) EnterpriseColors.EmeraldGlow else MaterialTheme.colorScheme.onSurfaceVariant
                )

                if (lastRecognizedText != null) {
                    Surface(
                        color = EnterpriseColors.CyberSurfaceCard,
                        shape = RoundedCornerShape(12.dp),
                        border = androidx.compose.foundation.BorderStroke(1.dp, EnterpriseColors.CyanNeon.copy(alpha = 0.4f)),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text(
                            text = "🗣️ فرمان شناسایی شده: \"${lastRecognizedText}\"",
                            fontSize = 12.sp,
                            fontWeight = FontWeight.Bold,
                            color = Color.White,
                            modifier = Modifier.padding(10.dp),
                            textAlign = TextAlign.Center
                        )
                    }
                }

                Surface(
                    color = EnterpriseColors.EmeraldDeep.copy(alpha = 0.35f),
                    shape = RoundedCornerShape(12.dp),
                    border = androidx.compose.foundation.BorderStroke(1.dp, EnterpriseColors.EmeraldNeon.copy(alpha = 0.4f)),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text(
                        text = executionStatus,
                        fontSize = 12.sp,
                        fontWeight = FontWeight.SemiBold,
                        color = Color(0xFFA7F3D0),
                        modifier = Modifier.padding(12.dp),
                        textAlign = TextAlign.Center
                    )
                }

                HorizontalDivider(color = EnterpriseColors.CyberBorderSubtle.copy(alpha = 0.5f))

                Text(
                    text = "فرمان‌های صوتی سریع (Quick AI Directives):",
                    fontSize = 11.sp,
                    fontWeight = FontWeight.ExtraBold,
                    color = EnterpriseColors.EmeraldNeon,
                    modifier = Modifier.fillMaxWidth()
                )

                val quickCommands = listOf("Connect VPN", "Disconnect", "Switch Core", "Enable Stealth", "اتصال به شبکه")

                LazyRow(
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    items(quickCommands) { cmd ->
                        Surface(
                            color = EnterpriseColors.CyberSurfaceElevated,
                            shape = RoundedCornerShape(8.dp),
                            border = androidx.compose.foundation.BorderStroke(1.dp, EnterpriseColors.CyberBorderSubtle),
                            modifier = Modifier.clickable { processVoiceCommand(cmd) }
                        ) {
                            Text(
                                text = cmd,
                                fontSize = 11.sp,
                                fontWeight = FontWeight.Bold,
                                color = Color.White,
                                modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp)
                            )
                        }
                    }
                }
            }
        },
        confirmButton = {
            Button(
                onClick = onDismissRequest,
                colors = ButtonDefaults.buttonColors(containerColor = EnterpriseColors.EmeraldPrimaryDark),
                shape = RoundedCornerShape(10.dp)
            ) {
                Text("بستن", fontWeight = FontWeight.Bold, color = Color.White)
            }
        }
    )
}
