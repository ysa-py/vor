package com.vor.licensemanager.issuer

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.vor.licensemanager.R
import java.time.Instant
import java.time.LocalDate
import java.time.LocalTime
import java.time.ZoneId
import java.util.UUID

internal val PLATFORMS = listOf("android", "windows", "linux", "openwrt", "ios")

/**
 * The single-license issue form. Pure UI — every rule (id/tier/expiry
 * validation, canonical bytes, signing) lives in IssueEngine and the vault.
 *
 * Responsive: single column on phones; at >= 600dp width (landscape phones,
 * tablets, foldables) the form splits into two panes — identity left,
 * expiry/platforms right. Chip groups wrap via FlowRow so narrow screens,
 * large font scales and RTL layouts never clip.
 */
@Composable
fun IssueFormScreen(
    localTiers: List<String>,
    onSign: (IssueEngine.IssueRequest, onDone: (IssueEngine.Issued?) -> Unit) -> Unit,
    onOpenTiers: () -> Unit,
) {
    val context = LocalContext.current
    val nowEpoch = remember { System.currentTimeMillis() / 1000 }
    var id by rememberSaveable { mutableStateOf(UUID.randomUUID().toString()) }
    var tier by rememberSaveable { mutableStateOf("standard") }
    var notes by rememberSaveable { mutableStateOf("") }
    var platforms by rememberSaveable { mutableStateOf(PLATFORMS.toSet()) }
    var presetDays by rememberSaveable { mutableStateOf(30) }
    var showDatePicker by rememberSaveable { mutableStateOf(false) }
    var customDateMillis by rememberSaveable { mutableStateOf<Long?>(null) }
    var customTime by rememberSaveable { mutableStateOf("23:59") }
    var signError by remember { mutableStateOf<String?>(null) }
    var issued by remember { mutableStateOf<IssueEngine.Issued?>(null) }

    // Computed expiry (RFC 3339, UTC) from the preset or the custom date+time.
    val expiresAt: String = if (customDateMillis == null) {
        val endOfDay = LocalDate.now(ZoneId.systemDefault()).plusDays(presetDays.toLong())
            .atTime(23, 59, 59).atZone(ZoneId.systemDefault()).toInstant()
        IssuerTime.formatRfc3339Utc(endOfDay)
    } else {
        val date = Instant.ofEpochMilli(customDateMillis!!).atZone(ZoneId.systemDefault()).toLocalDate()
        val time = parseTimeOfDay(customTime) ?: LocalTime.of(23, 59, 59)
        IssuerTime.formatRfc3339Utc(date.atTime(time).atZone(ZoneId.systemDefault()).toInstant())
    }

    val request = IssueEngine.IssueRequest(
        id = id, tier = tier, expiresAt = expiresAt,
        platforms = platforms.sortedBy { PLATFORMS.indexOf(it) }, notes = notes,
    )
    val validation = IssueEngine.validate(request, nowEpoch)

    BoxWithConstraints {
        val twoPane = maxWidth >= 600.dp
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .imePadding()
                .padding(horizontal = 24.dp, vertical = 8.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Header(stringResource(R.string.issuer_form_header), stringResource(R.string.issuer_form_sub))
            Spacer(Modifier.height(16.dp))

            if (twoPane) {
                Row(
                    horizontalArrangement = Arrangement.spacedBy(24.dp),
                    modifier = Modifier.fillMaxWidth().widthIn(max = 900.dp),
                ) {
                    Column(Modifier.weight(1f)) {
                        IdentityPane(
                            id = id, onId = { id = it },
                            tier = tier, onTier = { tier = it },
                            localTiers = localTiers, onOpenTiers = onOpenTiers,
                        )
                    }
                    Column(Modifier.weight(1f)) {
                        ExpiryPane(
                            presetDays = presetDays,
                            onPreset = { presetDays = it; customDateMillis = null },
                            customDateMillis = customDateMillis,
                            onOpenDatePicker = { showDatePicker = true },
                            customTime = customTime,
                            onCustomTime = { customTime = it },
                            expiresAt = expiresAt,
                            platforms = platforms,
                            onPlatforms = { platforms = it },
                        )
                    }
                }
            } else {
                IdentityPane(
                    id = id, onId = { id = it },
                    tier = tier, onTier = { tier = it },
                    localTiers = localTiers, onOpenTiers = onOpenTiers,
                )
                ExpiryPane(
                    presetDays = presetDays,
                    onPreset = { presetDays = it; customDateMillis = null },
                    customDateMillis = customDateMillis,
                    onOpenDatePicker = { showDatePicker = true },
                    customTime = customTime,
                    onCustomTime = { customTime = it },
                    expiresAt = expiresAt,
                    platforms = platforms,
                    onPlatforms = { platforms = it },
                )
            }

            Spacer(Modifier.height(12.dp))
            OutlinedTextField(
                value = notes,
                onValueChange = { notes = it },
                label = { Text(stringResource(R.string.issuer_field_notes)) },
                supportingText = { Text(stringResource(R.string.issuer_notes_hint)) },
                minLines = 2,
                modifier = Modifier.fillMaxWidth().widthIn(max = 560.dp),
            )

            validation?.let { error ->
                Spacer(Modifier.height(8.dp))
                Text(
                    text = stringResource(R.string.issuer_generate_invalid, error.message),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }
            signError?.let { error ->
                Spacer(Modifier.height(8.dp))
                Text(error, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
            }

            Spacer(Modifier.height(16.dp))
            Button(
                onClick = {
                    signError = null
                    onSign(request) { result ->
                        if (result == null) {
                            signError = context.getString(R.string.issuer_error_generic)
                        } else {
                            issued = result
                            id = UUID.randomUUID().toString() // fresh id: no accidental duplicate issuance
                        }
                    }
                },
                enabled = validation == null,
                modifier = Modifier.widthIn(max = 560.dp).fillMaxWidth(),
            ) {
                Text(stringResource(R.string.issuer_generate))
            }
            Spacer(Modifier.height(24.dp))
        }
    }

    issued?.let { result ->
        TokenResultDialog(
            token = result.token,
            licenseId = result.entry.id,
            onDismiss = {
                issued = null
                notes = ""
            },
        )
    }

    if (showDatePicker) {
        DatePickerModal(
            initialMillis = customDateMillis,
            onDismiss = { showDatePicker = false },
            onSelect = { millis ->
                customDateMillis = millis
                showDatePicker = false
            },
        )
    }
}

/** License identity: free-text id + tier, quick tier presets, tier manager. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun IdentityPane(
    id: String,
    onId: (String) -> Unit,
    tier: String,
    onTier: (String) -> Unit,
    localTiers: List<String>,
    onOpenTiers: () -> Unit,
) {
    OutlinedTextField(
        value = id,
        onValueChange = onId,
        label = { Text(stringResource(R.string.issuer_field_id)) },
        supportingText = { Text(stringResource(R.string.issuer_field_id_hint)) },
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
    )
    TextButton(onClick = { onId(UUID.randomUUID().toString()) }) {
        Text(stringResource(R.string.issuer_new_id))
    }

    OutlinedTextField(
        value = tier,
        onValueChange = onTier,
        label = { Text(stringResource(R.string.issuer_field_tier)) },
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
    )
    FlowRow(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        localTiers.take(4).forEach { candidate ->
            FilterChip(
                selected = tier == candidate,
                onClick = { onTier(candidate) },
                label = { Text(candidate) },
            )
        }
        FilterChip(
            selected = false,
            onClick = onOpenTiers,
            label = { Text(stringResource(R.string.issuer_manage_tiers)) },
        )
    }
    Spacer(Modifier.height(12.dp))
}

/** Expiry presets/custom date + platform matrix. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun ExpiryPane(
    presetDays: Int,
    onPreset: (Int) -> Unit,
    customDateMillis: Long?,
    onOpenDatePicker: () -> Unit,
    customTime: String,
    onCustomTime: (String) -> Unit,
    expiresAt: String,
    platforms: Set<String>,
    onPlatforms: (Set<String>) -> Unit,
) {
    Text(stringResource(R.string.issuer_expiry), fontWeight = FontWeight.Bold)
    Spacer(Modifier.height(4.dp))
    FlowRow(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        PresetChip(7, presetDays, customDateMillis != null, onPreset)
        PresetChip(30, presetDays, customDateMillis != null, onPreset)
        PresetChip(90, presetDays, customDateMillis != null, onPreset)
        PresetChip(365, presetDays, customDateMillis != null, onPreset)
        PresetChip(365 * 5, presetDays, customDateMillis != null, onPreset)
        FilterChip(
            selected = customDateMillis != null,
            onClick = onOpenDatePicker,
            label = { Text(stringResource(R.string.issuer_expiry_custom)) },
        )
    }
    if (customDateMillis != null) {
        Spacer(Modifier.height(8.dp))
        OutlinedTextField(
            value = customTime,
            onValueChange = onCustomTime,
            label = { Text(stringResource(R.string.issuer_expiry_time)) },
            supportingText = { Text(stringResource(R.string.issuer_expiry_time_hint)) },
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
        )
    }
    Spacer(Modifier.height(4.dp))
    DetailLine(stringResource(R.string.issuer_expiry_preview), expiresAt)

    Spacer(Modifier.height(12.dp))
    Text(stringResource(R.string.issuer_platforms), fontWeight = FontWeight.Bold)
    Spacer(Modifier.height(4.dp))
    FlowRow(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        PLATFORMS.forEach { platform ->
            PlatformChip(platforms, platform, onPlatforms)
        }
    }
    Spacer(Modifier.height(12.dp))
}

@Composable
internal fun Header(title: String, subtitle: String) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text(
            title,
            style = MaterialTheme.typography.headlineSmall,
            fontWeight = FontWeight.Bold,
            textAlign = TextAlign.Center,
        )
        Text(
            subtitle,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
            modifier = Modifier.widthIn(max = 520.dp),
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun DatePickerModal(initialMillis: Long?, onDismiss: () -> Unit, onSelect: (Long) -> Unit) {
    val state = rememberDatePickerState(initialSelectedDateMillis = initialMillis)
    DatePickerDialog(
        onDismissRequest = onDismiss,
        confirmButton = {
            TextButton(onClick = {
                state.selectedDateMillis?.let(onSelect)
                onDismiss()
            }) { Text(stringResource(R.string.issuer_expiry_pick)) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.issuer_cancel)) }
        },
    ) {
        DatePicker(state = state)
    }
}

@Composable
private fun PresetChip(days: Int, current: Int, custom: Boolean, onSelect: (Int) -> Unit) {
    FilterChip(
        selected = !custom && current == days,
        onClick = { onSelect(days) },
        label = { Text(presetLabel(days)) },
    )
}

@Composable
private fun PlatformChip(selected: Set<String>, platform: String, onChange: (Set<String>) -> Unit) {
    FilterChip(
        selected = platform in selected,
        onClick = { onChange(if (platform in selected) selected - platform else selected + platform) },
        label = { Text(platform) },
    )
}

@Composable
private fun presetLabel(days: Int): String = when (days) {
    7 -> stringResource(R.string.issuer_expiry_7d)
    30 -> stringResource(R.string.issuer_expiry_30d)
    90 -> stringResource(R.string.issuer_expiry_90d)
    365 -> stringResource(R.string.issuer_expiry_1y)
    else -> stringResource(R.string.issuer_expiry_5y)
}

/** "HH:mm" (24h) -> LocalTime, or null. */
internal fun parseTimeOfDay(text: String): LocalTime? {
    val trimmed = text.trim()
    if (!Regex("^\\d{1,2}:\\d{2}$").matches(trimmed)) return null
    val parts = trimmed.split(":")
    val hour = parts[0].toIntOrNull() ?: return null
    val minute = parts[1].toIntOrNull() ?: return null
    if (hour !in 0..23 || minute !in 0..59) return null
    return LocalTime.of(hour, minute)
}
