package com.vor.licensemanager.issuer

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.vor.licensemanager.BuildConfig
import com.vor.licensemanager.R

/**
 * Signing-key management: status card, generate/import, passphrase backup +
 * restore, self-test against the embedded public key, and wipe. Every
 * destructive action asks for explicit confirmation; every fact stated here
 * is honest (embedded-key mismatch is a WARNING, not a hidden failure).
 * Action buttons wrap (FlowRow) so long localized labels never clip.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun KeysScreen(
    ops: IssuerOps,
    keyVersion: Int,
    onKeyChanged: () -> Unit,
) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val keyInfo = remember(keyVersion) { ops.vault.keyInfo() }
    val fingerprint = remember(keyVersion) { ops.vault.fingerprint() }
    val matchesEmbedded = remember(keyVersion) { keyInfo?.pubB64Url == BuildConfig.VOR_LICENSE_PUBLIC_KEY }

    var confirmGenerate by remember { mutableStateOf(false) }
    var showImport by remember { mutableStateOf(false) }
    var showWipe by remember { mutableStateOf(false) }
    var selfTestResult by remember { mutableStateOf<IssueEngine.SelfTestResult?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }

    // Backup: file FIRST (SAF), then passphrase, then encrypt+write in one
    // hop — the passphrase never waits around in a dialog-side variable.
    var backupUri by remember { mutableStateOf<android.net.Uri?>(null) }
    val backupPicker = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/json"),
    ) { uri ->
        if (uri != null) backupUri = uri
    }

    // Restore: file FIRST, then passphrase, then decrypt+install in one hop.
    var restoreText by remember { mutableStateOf<String?>(null) }
    val restorePicker = rememberLauncherForActivityResult(
        ActivityResultContracts.GetContent(),
    ) { uri ->
        if (uri != null) {
            restoreText = runCatching {
                context.contentResolver.openInputStream(uri)?.bufferedReader()?.use { it.readText() }
            }.getOrNull()
            if (restoreText == null) {
                error = context.getString(R.string.issuer_restore_error)
            }
        }
    }

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 24.dp, vertical = 8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Header(stringResource(R.string.issuer_keys_header), stringResource(R.string.issuer_keys_sub))
        Spacer(Modifier.height(16.dp))

        Card(
            colors = CardDefaults.cardColors(
                containerColor = if (keyInfo != null && matchesEmbedded) {
                    MaterialTheme.colorScheme.primaryContainer
                } else {
                    MaterialTheme.colorScheme.errorContainer
                },
                contentColor = if (keyInfo != null && matchesEmbedded) {
                    MaterialTheme.colorScheme.onPrimaryContainer
                } else {
                    MaterialTheme.colorScheme.onErrorContainer
                },
            ),
            modifier = Modifier.fillMaxWidth().widthIn(max = 560.dp),
        ) {
            Column(Modifier.padding(16.dp)) {
                if (keyInfo == null) {
                    Text(stringResource(R.string.issuer_keys_none), fontWeight = FontWeight.Bold)
                    Text(stringResource(R.string.issuer_keys_none_hint), style = MaterialTheme.typography.bodySmall)
                } else {
                    Text(stringResource(R.string.issuer_keys_present), fontWeight = FontWeight.Bold)
                    Spacer(Modifier.height(4.dp))
                    DetailLine(stringResource(R.string.issuer_key_pub), keyInfo.pubB64Url)
                    fingerprint?.let { DetailLine(stringResource(R.string.issuer_key_fingerprint), it) }
                    DetailLine(stringResource(R.string.issuer_key_created), keyInfo.createdAt)
                    DetailLine(stringResource(R.string.issuer_key_source), keyInfo.source)
                    Text(
                        if (matchesEmbedded) stringResource(R.string.issuer_key_matches_embedded)
                        else stringResource(R.string.issuer_key_differs_embedded),
                        style = MaterialTheme.typography.bodySmall,
                        fontWeight = FontWeight.Bold,
                    )
                }
            }
        }

        error?.let {
            Spacer(Modifier.height(8.dp))
            Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
        }

        Spacer(Modifier.height(16.dp))
        FlowRow(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.widthIn(max = 560.dp),
        ) {
            Button(onClick = { confirmGenerate = true }) { Text(stringResource(R.string.issuer_generate_key)) }
            OutlinedButton(onClick = { showImport = true }) { Text(stringResource(R.string.issuer_import_key)) }
            OutlinedButton(
                onClick = { backupPicker.launch("vor-issuer-key-backup.json") },
                enabled = keyInfo != null && !busy,
            ) { Text(stringResource(R.string.issuer_backup)) }
            OutlinedButton(
                onClick = { restorePicker.launch("*/*") },
                enabled = !busy,
            ) { Text(stringResource(R.string.issuer_restore)) }
            OutlinedButton(
                onClick = {
                    selfTestResult = null
                    busy = true
                    error = null
                    ops.selfTest { result ->
                        busy = false
                        selfTestResult = result
                    }
                },
                enabled = keyInfo != null && !busy,
            ) { Text(stringResource(R.string.issuer_self_test)) }
            TextButton(onClick = { showWipe = true }, enabled = keyInfo != null && !busy) {
                Text(stringResource(R.string.issuer_wipe), color = MaterialTheme.colorScheme.error)
            }
        }
        Spacer(Modifier.height(24.dp))
    }

    if (confirmGenerate) {
        AlertDialog(
            onDismissRequest = { confirmGenerate = false },
            title = { Text(stringResource(R.string.issuer_generate_confirm_title)) },
            text = {
                Text(
                    if (keyInfo != null) stringResource(R.string.issuer_generate_confirm_overwrite)
                    else stringResource(R.string.issuer_generate_confirm_body),
                )
            },
            confirmButton = {
                TextButton(onClick = {
                    confirmGenerate = false
                    busy = true
                    ops.generateKey { ok ->
                        busy = false
                        if (ok) onKeyChanged() else error = context.getString(R.string.issuer_error_generic)
                    }
                }) { Text(stringResource(R.string.issuer_generate_key)) }
            },
            dismissButton = {
                TextButton(onClick = { confirmGenerate = false }) {
                    Text(stringResource(R.string.issuer_cancel))
                }
            },
        )
    }

    if (showImport) {
        ImportSeedDialog(
            hasExisting = keyInfo != null,
            onDismiss = { showImport = false },
            onImport = { seedText ->
                showImport = false
                busy = true
                val seed = com.vor.license.issuer.SoftwareEd25519.seedFromBase64Url(seedText)
                if (seed == null) {
                    busy = false
                    error = context.getString(R.string.issuer_import_invalid)
                } else {
                    ops.importSeed(seed, "imported") { ok ->
                        busy = false
                        if (ok) onKeyChanged() else error = context.getString(R.string.issuer_error_generic)
                    }
                }
            },
        )
    }

    backupUri?.let { uri ->
        PassphraseDialog(
            title = stringResource(R.string.issuer_backup),
            requireConfirmation = true,
            onDismiss = { backupUri = null },
            onPassphrase = { passphrase ->
                val target = uri
                backupUri = null
                busy = true
                ops.backupTo(target, passphrase) { ok ->
                    busy = false
                    if (!ok) error = context.getString(R.string.issuer_error_generic)
                }
            },
        )
    }

    restoreText?.let { text ->
        PassphraseDialog(
            title = stringResource(R.string.issuer_restore),
            requireConfirmation = false,
            onDismiss = { restoreText = null },
            onPassphrase = { passphrase ->
                restoreText = null
                busy = true
                ops.restoreBackup(text, passphrase) { ok ->
                    busy = false
                    if (ok) onKeyChanged() else error = context.getString(R.string.issuer_restore_failed)
                }
            },
        )
    }

    if (showWipe) {
        AlertDialog(
            onDismissRequest = { showWipe = false },
            title = { Text(stringResource(R.string.issuer_wipe_confirm_title)) },
            text = { Text(stringResource(R.string.issuer_wipe_confirm_body)) },
            confirmButton = {
                TextButton(onClick = {
                    showWipe = false
                    ops.vault.wipeSeed()
                    onKeyChanged()
                }) {
                    Text(stringResource(R.string.issuer_wipe), color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { showWipe = false }) {
                    Text(stringResource(R.string.issuer_cancel))
                }
            },
        )
    }

    selfTestResult?.let { result ->
        AlertDialog(
            onDismissRequest = { selfTestResult = null },
            title = { Text(stringResource(R.string.issuer_self_test)) },
            text = {
                Column {
                    Text(
                        when {
                            result.signsAndVerifies && result.matchesEmbeddedKey ->
                                stringResource(R.string.issuer_self_test_ok)
                            result.signsAndVerifies ->
                                stringResource(R.string.issuer_self_test_partial)
                            else ->
                                stringResource(R.string.issuer_self_test_fail)
                        },
                        fontWeight = FontWeight.Bold,
                    )
                    Spacer(Modifier.height(4.dp))
                    Text(result.detail, style = MaterialTheme.typography.bodySmall)
                }
            },
            confirmButton = {
                TextButton(onClick = { selfTestResult = null }) {
                    Text(stringResource(R.string.issuer_close))
                }
            },
        )
    }
}

/** Import a raw seed (the LICENSE_SIGNING_KEY secret format). */
@Composable
private fun ImportSeedDialog(
    hasExisting: Boolean,
    onDismiss: () -> Unit,
    onImport: (seedText: String) -> Unit,
) {
    var seedText by androidx.compose.runtime.saveable.rememberSaveable { androidx.compose.runtime.mutableStateOf("") }
    val parsed = com.vor.license.issuer.SoftwareEd25519.seedFromBase64Url(seedText)
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.issuer_import_title)) },
        text = {
            Column {
                Text(
                    stringResource(R.string.issuer_import_warning),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
                Spacer(Modifier.height(8.dp))
                OutlinedTextField(
                    value = seedText,
                    onValueChange = { seedText = it },
                    label = { Text(stringResource(R.string.issuer_import_placeholder)) },
                    textStyle = androidx.compose.material3.LocalTextStyle.current.copy(fontFamily = FontFamily.Monospace),
                    singleLine = true,
                )
                if (seedText.isNotBlank() && parsed == null) {
                    Spacer(Modifier.height(4.dp))
                    Text(
                        stringResource(R.string.issuer_import_invalid),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
                if (hasExisting) {
                    Spacer(Modifier.height(8.dp))
                    Text(
                        stringResource(R.string.issuer_generate_confirm_overwrite),
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = { onImport(seedText) },
                enabled = parsed != null,
            ) { Text(stringResource(R.string.issuer_import_confirm)) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.issuer_cancel)) }
        },
    )
}

/** Passphrase entry (optionally twice, for backup creation). */
@Composable
fun PassphraseDialog(
    title: String,
    requireConfirmation: Boolean,
    onDismiss: () -> Unit,
    onPassphrase: (CharArray) -> Unit,
) {
    var passphrase by androidx.compose.runtime.saveable.rememberSaveable { androidx.compose.runtime.mutableStateOf("") }
    var confirmation by androidx.compose.runtime.saveable.rememberSaveable { androidx.compose.runtime.mutableStateOf("") }
    val tooShort = passphrase.isNotEmpty() && passphrase.length < 8
    val matches = !requireConfirmation || passphrase == confirmation
    val canConfirm = passphrase.length >= 8 && matches
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            Column {
                OutlinedTextField(
                    value = passphrase,
                    onValueChange = { passphrase = it },
                    label = { Text(stringResource(R.string.issuer_passphrase)) },
                    singleLine = true,
                    visualTransformation = androidx.compose.ui.text.input.PasswordVisualTransformation(),
                )
                if (tooShort) {
                    Text(
                        stringResource(R.string.issuer_passphrase_too_short),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
                if (requireConfirmation) {
                    Spacer(Modifier.height(8.dp))
                    OutlinedTextField(
                        value = confirmation,
                        onValueChange = { confirmation = it },
                        label = { Text(stringResource(R.string.issuer_passphrase_again)) },
                        singleLine = true,
                        visualTransformation = androidx.compose.ui.text.input.PasswordVisualTransformation(),
                    )
                    if (confirmation.isNotEmpty() && !matches) {
                        Text(
                            stringResource(R.string.issuer_passphrase_mismatch),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                    }
                }
                Spacer(Modifier.height(4.dp))
                Text(
                    stringResource(R.string.issuer_passphrase_hint),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        },
        confirmButton = {
            TextButton(
                onClick = { onPassphrase(passphrase.toCharArray()) },
                enabled = canConfirm,
            ) { Text(stringResource(R.string.issuer_close_ok)) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.issuer_cancel)) }
        },
    )
}
