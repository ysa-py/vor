package com.vor.licensemanager

import android.content.Intent
import android.graphics.BitmapFactory
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.vor.license.LicenseResult
import com.vor.license.LicenseStatus
import com.vor.license.LicenseVerifier

/**
 * Vor License Manager — the offline companion app for license holders and
 * maintainers (see license/SPEC.md).
 *
 * v1.0.2 capabilities (all fully offline — no network dependency, no new
 * permissions):
 * - paste/share a token and verify it locally (same :core-license code as the
 *   main Vor app — one verification routine, not two reimplementations);
 * - store MULTIPLE imported licenses, each with a live valid/expired/invalid
 *   status recomputed against the device clock, expiry shown in local time,
 *   and a countdown for licenses nearing expiry;
 * - QR-code import (system photo picker — no storage permission) and export
 *   (on-screen QR bitmap) of a license token;
 * - copy-token and clear-all actions.
 */
class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        // Accept a shared token (ACTION_SEND text/plain) — auto-verified and stored.
        val sharedText: String? = if (intent?.action == Intent.ACTION_SEND) {
            @Suppress("DEPRECATION")
            intent.getCharSequenceExtra(Intent.EXTRA_TEXT)?.toString()
        } else {
            null
        }

        setContent {
            MaterialTheme {
                Surface(modifier = Modifier.fillMaxSize()) {
                    LicenseManagerScreen(initialToken = sharedText)
                }
            }
        }
    }
}

@Composable
private fun LicenseManagerScreen(initialToken: String?) {
    val context = LocalContext.current
    val clipboard = LocalClipboardManager.current
    var token by remember { mutableStateOf(initialToken ?: "") }
    var result by remember { mutableStateOf<LicenseResult?>(null) }
    var stored by remember { mutableStateOf(LicenseStore.load(context)) }
    // Live clock for statuses/countdowns — re-verified every minute so an
    // expiring license flips to EXPIRED on its own while the screen is open.
    var nowEpoch by remember { mutableStateOf(System.currentTimeMillis() / 1000) }
    LaunchedEffect(Unit) {
        while (true) {
            kotlinx.coroutines.delay(60_000)
            nowEpoch = System.currentTimeMillis() / 1000
        }
    }
    var qrToken by remember { mutableStateOf<String?>(null) }
    var confirmClear by remember { mutableStateOf(false) }

    // QR import via the system photo picker — no permission required.
    val qrPickLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.GetContent()
    ) { uri ->
        if (uri != null) {
            runCatching {
                context.contentResolver.openInputStream(uri)?.use { input ->
                    BitmapFactory.decodeStream(input)
                }
            }.getOrNull()?.let { bitmap ->
                val decoded = QrCodec.decode(bitmap)
                if (decoded != null) {
                    token = decoded
                    result = verifyNow(decoded)
                    if (result?.status != LicenseStatus.INVALID) {
                        stored = LicenseStore.upsert(context, decoded)
                    }
                } else {
                    result = LicenseResult(LicenseStatus.INVALID, null)
                }
            }
        }
    }

    // Auto-check + store a shared token on arrival.
    LaunchedEffect(initialToken) {
        if (!initialToken.isNullOrBlank()) {
            result = verifyNow(initialToken)
            if (result?.status != LicenseStatus.INVALID) {
                stored = LicenseStore.upsert(context, initialToken)
            }
        }
    }

    fun checkAndStore(raw: String) {
        val checked = verifyNow(raw)
        result = checked
        if (checked.status != LicenseStatus.INVALID) {
            // Store valid AND expired-signed tokens: an expired license is still
            // authentic — the maintainer wants to see it in the list with its expiry.
            stored = LicenseStore.upsert(context, raw)
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(
            text = stringResource(R.string.app_title),
            style = MaterialTheme.typography.headlineMedium,
            fontWeight = FontWeight.Bold,
            textAlign = TextAlign.Center,
        )
        Text(
            text = stringResource(R.string.subtitle),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
            modifier = Modifier.widthIn(max = 420.dp),
        )
        Spacer(Modifier.height(24.dp))

        OutlinedTextField(
            value = token,
            onValueChange = {
                token = it
                result = null
            },
            label = { Text(stringResource(R.string.token_label)) },
            minLines = 3,
            maxLines = 6,
            modifier = Modifier
                .fillMaxWidth()
                .widthIn(max = 520.dp),
        )
        Spacer(Modifier.height(12.dp))

        Row(modifier = Modifier.widthIn(max = 520.dp).fillMaxWidth()) {
            Button(
                onClick = { checkAndStore(token) },
                enabled = token.isNotBlank(),
                modifier = Modifier.weight(1f),
            ) {
                Text(stringResource(R.string.check_activate))
            }
            Spacer(Modifier.size(8.dp))
            TextButton(onClick = { qrPickLauncher.launch("image/*") }) {
                Text(stringResource(R.string.import_qr))
            }
        }

        result?.let { checked ->
            Spacer(Modifier.height(16.dp))
            StatusCard(result = checked)
        }

        if (stored.isNotEmpty()) {
            Spacer(Modifier.height(24.dp))
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.widthIn(max = 520.dp).fillMaxWidth(),
            ) {
                Text(
                    text = stringResource(R.string.stored_count, stored.size),
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.weight(1f),
                )
                TextButton(onClick = { confirmClear = true }) {
                    Text(stringResource(R.string.clear_all), color = MaterialTheme.colorScheme.error)
                }
            }
            Spacer(Modifier.height(8.dp))
            stored.forEach { item ->
                val view = LicenseStore.view(
                    item.token, item.addedAtMs, nowEpoch, publicKey(),
                )
                StoredLicenseCard(
                    view = view,
                    onCopy = {
                        clipboard.setText(AnnotatedString(item.token))
                    },
                    onShowQr = { qrToken = item.token },
                    onDelete = { stored = LicenseStore.remove(context, item.token) },
                )
                Spacer(Modifier.height(8.dp))
            }
        }
    }

    qrToken?.let { tokenValue ->
        AlertDialog(
            onDismissRequest = { qrToken = null },
            title = { Text(stringResource(R.string.qr_export_title)) },
            text = {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    QrCodec.encode(tokenValue)?.let { bitmap ->
                        Image(
                            bitmap = bitmap.asImageBitmap(),
                            contentDescription = stringResource(R.string.qr_export_title),
                            modifier = Modifier.size(280.dp),
                        )
                    } ?: Text(stringResource(R.string.qr_error))
                    Spacer(Modifier.height(8.dp))
                    Text(
                        text = stringResource(R.string.qr_export_hint),
                        style = MaterialTheme.typography.bodySmall,
                        textAlign = TextAlign.Center,
                    )
                }
            },
            confirmButton = {
                TextButton(onClick = { qrToken = null }) { Text(stringResource(R.string.close)) }
            },
        )
    }

    if (confirmClear) {
        AlertDialog(
            onDismissRequest = { confirmClear = false },
            title = { Text(stringResource(R.string.clear_all_confirm_title)) },
            text = { Text(stringResource(R.string.clear_all_confirm_body)) },
            confirmButton = {
                TextButton(onClick = {
                    stored = LicenseStore.clearAll(context)
                    confirmClear = false
                }) {
                    Text(stringResource(R.string.clear_all), color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmClear = false }) { Text(stringResource(R.string.close)) }
            },
        )
    }
}

private fun publicKey(): String = BuildConfig.VOR_LICENSE_PUBLIC_KEY

private fun verifyNow(token: String): LicenseResult =
    LicenseVerifier.verify(publicKey(), token, System.currentTimeMillis() / 1000)

@Composable
private fun StatusCard(result: LicenseResult) {
    val (container, onContainer) = when (result.status) {
        LicenseStatus.VALID -> MaterialTheme.colorScheme.primaryContainer to MaterialTheme.colorScheme.onPrimaryContainer
        LicenseStatus.EXPIRED -> MaterialTheme.colorScheme.errorContainer to MaterialTheme.colorScheme.onErrorContainer
        LicenseStatus.INVALID -> MaterialTheme.colorScheme.surfaceVariant to MaterialTheme.colorScheme.onSurfaceVariant
    }
    Card(
        colors = CardDefaults.cardColors(containerColor = container, contentColor = onContainer),
        modifier = Modifier
            .fillMaxWidth()
            .widthIn(max = 520.dp),
    ) {
        Column(Modifier.padding(16.dp)) {
            Text(
                text = when (result.status) {
                    LicenseStatus.VALID -> stringResource(R.string.status_valid)
                    LicenseStatus.EXPIRED -> stringResource(R.string.status_expired)
                    LicenseStatus.INVALID -> stringResource(R.string.status_invalid)
                },
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
            )
            result.payload?.let { payload ->
                Spacer(Modifier.height(8.dp))
                DetailRow(stringResource(R.string.detail_id), payload.id)
                DetailRow(stringResource(R.string.detail_issued), payload.issuedAt)
                DetailRow(stringResource(R.string.detail_expires), payload.expiresAt)
                val tier = payload.tier
                if (tier != null) DetailRow(stringResource(R.string.detail_tier), tier)
            }
        }
    }
}

@Composable
private fun StoredLicenseCard(
    view: LicenseStore.LicenseView,
    onCopy: () -> Unit,
    onShowQr: () -> Unit,
    onDelete: () -> Unit,
) {
    val (container, onContainer) = when (view.result.status) {
        LicenseStatus.VALID -> MaterialTheme.colorScheme.primaryContainer to MaterialTheme.colorScheme.onPrimaryContainer
        LicenseStatus.EXPIRED -> MaterialTheme.colorScheme.errorContainer to MaterialTheme.colorScheme.onErrorContainer
        LicenseStatus.INVALID -> MaterialTheme.colorScheme.surfaceVariant to MaterialTheme.colorScheme.onSurfaceVariant
    }
    val countdown = LicenseStore.countdownText(view.remainingMs)
    Card(
        colors = CardDefaults.cardColors(containerColor = container, contentColor = onContainer),
        modifier = Modifier
            .fillMaxWidth()
            .widthIn(max = 520.dp),
    ) {
        Column(Modifier.padding(16.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = when (view.result.status) {
                        LicenseStatus.VALID -> stringResource(R.string.status_valid)
                        LicenseStatus.EXPIRED -> stringResource(R.string.status_expired)
                        LicenseStatus.INVALID -> stringResource(R.string.status_invalid)
                    },
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.weight(1f),
                )
                if (countdown != null && view.result.status == LicenseStatus.VALID) {
                    Text(
                        text = if (LicenseStore.isExpiringSoon(view.remainingMs)) {
                            stringResource(R.string.countdown_soon, countdown)
                        } else {
                            stringResource(R.string.countdown_left, countdown)
                        },
                        style = MaterialTheme.typography.labelMedium,
                        fontWeight = if (LicenseStore.isExpiringSoon(view.remainingMs)) FontWeight.Bold else FontWeight.Normal,
                        color = if (LicenseStore.isExpiringSoon(view.remainingMs)) {
                            MaterialTheme.colorScheme.error
                        } else {
                            MaterialTheme.colorScheme.onSurfaceVariant
                        },
                    )
                }
            }
            view.result.payload?.let { payload ->
                Spacer(Modifier.height(6.dp))
                DetailRow(stringResource(R.string.detail_id), payload.id)
                val tier = payload.tier
                if (tier != null) DetailRow(stringResource(R.string.detail_tier), tier)
                view.expiryLocal?.let {
                    DetailRow(stringResource(R.string.detail_expires_local), it)
                }
            }
            Spacer(Modifier.height(8.dp))
            Row {
                TextButton(onClick = onCopy) { Text(stringResource(R.string.copy_token)) }
                TextButton(onClick = onShowQr) { Text(stringResource(R.string.show_qr)) }
                TextButton(onClick = onDelete) {
                    Text(stringResource(R.string.delete), color = MaterialTheme.colorScheme.error)
                }
            }
        }
    }
}

@Composable
private fun DetailRow(label: String, value: String) {
    Text(
        text = "$label: $value",
        style = MaterialTheme.typography.bodySmall,
    )
}
