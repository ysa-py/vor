package com.unifiedshield.ui.license

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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.unifiedshield.license.LicenseVerifyState
import com.unifiedshield.license.MicafpLicenseManager
import com.unifiedshield.license.MicafpLicenseState
import com.unifiedshield.localization.MicafpLangManager
import com.unifiedshield.ui.theme.MicafpTokens
import com.unifiedshield.ui.theme.MicafpType
import kotlinx.coroutines.launch
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone

// =============================================================================
// MICAFP Directive v6 — B3.6 Account & License (C1 anti-forgery UI).
// Shows the REAL Ed25519 signature-verification status (never a static
// checkmark), trusted-time status, expiry countdown, and the confirmed
// expiry behaviour: international tunnel only — everything else stays up.
// =============================================================================

@Composable
fun AccountLicenseScreen() {
    val context = LocalContext.current
    val langManager = remember { MicafpLangManager.getInstance(context) }
    val lang by langManager.lang.collectAsState()
    fun t(key: String) = MicafpLangManager.get(lang, key)

    val manager = remember { MicafpLicenseManager.getInstance(context) }
    val state by manager.state.collectAsState()
    var serialInput by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    val utcFmt = remember {
        SimpleDateFormat("yyyy-MM-dd HH:mm 'UTC'", Locale.US).apply {
            timeZone = TimeZone.getTimeZone("UTC")
        }
    }
    val expiry = manager.currentExpiryUtc()
    val trustedNow = state.trustedTimeMillis
    val daysLeft = if (trustedNow > 0) ((expiry - trustedNow) / (1000L * 60 * 60 * 24)).coerceAtLeast(0) else null

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        // ---------- Signature verification status (real) ----------
        Surface(
            shape = RoundedCornerShape(14.dp),
            color = MicafpTokens.SurfaceCard,
            border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(Modifier.padding(16.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        when (state.state) {
                            LicenseVerifyState.VALID -> Icons.Default.Verified
                            LicenseVerifyState.EXPIRED -> Icons.Default.HourglassBottom
                            LicenseVerifyState.INVALID_SIGNATURE,
                            LicenseVerifyState.DEVICE_MISMATCH -> Icons.Default.GppBad
                            LicenseVerifyState.NO_LICENSE -> Icons.Default.Key
                        },
                        contentDescription = null,
                        tint = when (state.state) {
                            LicenseVerifyState.VALID -> MicafpTokens.Ok
                            LicenseVerifyState.EXPIRED -> MicafpTokens.Warn
                            LicenseVerifyState.NO_LICENSE -> MicafpTokens.TextMuted
                            else -> MicafpTokens.Crit
                        },
                        modifier = Modifier.size(22.dp)
                    )
                    Spacer(Modifier.width(10.dp))
                    Text(
                        when (state.state) {
                            LicenseVerifyState.VALID -> t("license.verified")
                            LicenseVerifyState.EXPIRED -> t("license.expired")
                            LicenseVerifyState.INVALID_SIGNATURE -> t("license.invalid")
                            LicenseVerifyState.DEVICE_MISMATCH -> t("license.deviceBound") + " — mismatch"
                            LicenseVerifyState.NO_LICENSE -> t("license.noLicense")
                        },
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = MicafpTokens.TextPrimary
                    )
                }
                if (state.state == LicenseVerifyState.NO_LICENSE && state.message.contains("pending")) {
                    Spacer(Modifier.height(6.dp))
                    Text(t("license.pendingWiring"), style = MaterialTheme.typography.bodySmall,
                        color = MicafpTokens.TextMuted)
                }
                Spacer(Modifier.height(10.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        if (state.trustedTimeSynced) Icons.Default.CloudDone else Icons.Default.CloudOff,
                        contentDescription = null,
                        tint = if (state.trustedTimeSynced) MicafpTokens.Ok else MicafpTokens.Warn,
                        modifier = Modifier.size(15.dp)
                    )
                    Spacer(Modifier.width(6.dp))
                    Text(
                        t("license.trustedTime") + ": " +
                            if (state.trustedTimeSynced) t("license.trustedTimeOk")
                            else t("license.trustedTimeFallback"),
                        style = MicafpType.MonoCaption,
                        color = MicafpTokens.TextSecondary
                    )
                }
            }
        }

        // ---------- Serial entry ----------
        Surface(
            shape = RoundedCornerShape(14.dp),
            color = MicafpTokens.SurfaceCard,
            border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(Modifier.padding(16.dp)) {
                Text(t("license.serial"), style = MaterialTheme.typography.titleSmall,
                    color = MicafpTokens.TextPrimary)
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(
                    value = serialInput,
                    onValueChange = { serialInput = it },
                    placeholder = { Text(t("license.serialHint"), style = MaterialTheme.typography.bodySmall) },
                    modifier = Modifier.fillMaxWidth(),
                    textStyle = MicafpType.MonoCaption.copy(color = MicafpTokens.TextPrimary),
                    minLines = 2,
                    singleLine = false
                )
                Spacer(Modifier.height(10.dp))
                Button(
                    enabled = serialInput.isNotBlank() && !busy,
                    onClick = {
                        busy = true
                        scope.launch {
                            val st = manager.applyLicense(serialInput)
                            manager.enforceExpiryPolicy()
                            busy = false
                        }
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = MicafpTokens.AccentDeep)
                ) {
                    Text(if (busy) t("common.loading") else t("license.apply"))
                }
            }
        }

        // ---------- Expiry (single signed field — displayed honestly) ----------
        Surface(
            shape = RoundedCornerShape(14.dp),
            color = MicafpTokens.SurfaceCard,
            border = androidx.compose.foundation.BorderStroke(1.dp, MicafpTokens.BorderSubtle),
            modifier = Modifier.fillMaxWidth()
        ) {
            Column(Modifier.padding(16.dp)) {
                Text(t("license.expiry"), style = MaterialTheme.typography.titleSmall,
                    color = MicafpTokens.TextPrimary)
                Spacer(Modifier.height(6.dp))
                Text(utcFmt.format(Date(expiry)), style = MicafpType.MonoValueLarge,
                    color = MicafpTokens.TextPrimary)
                daysLeft?.let {
                    Text(
                        "$it ${t("license.daysLeft")}",
                        style = MicafpType.MonoValue,
                        color = if (it > 30) MicafpTokens.Ok
                                else if (it > 7) MicafpTokens.Warn else MicafpTokens.Crit
                    )
                }
                Spacer(Modifier.height(8.dp))
                Text(t("license.expiryNote"), style = MaterialTheme.typography.bodySmall,
                    color = MicafpTokens.TextMuted)
                if (state.payload != null) {
                    Spacer(Modifier.height(8.dp))
                    val payload = state.payload
                    Text("licenseId: ${payload?.licenseId ?: "—"}", style = MicafpType.MonoCaption,
                        color = MicafpTokens.TextSecondary)
                    Text(t("license.deviceBound") + " ✓", style = MicafpType.MonoCaption,
                        color = MicafpTokens.TextSecondary)
                }
            }
        }
    }
}
