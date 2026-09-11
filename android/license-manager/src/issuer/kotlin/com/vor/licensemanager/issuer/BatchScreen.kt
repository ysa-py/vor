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
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.vor.license.issuer.BatchCsv
import com.vor.licensemanager.R
import java.time.LocalDate
import java.time.ZoneId

/**
 * Batch issuance: paste (or import) `id[,tier][,expires]` lines, preview
 * the parse, then issue everything at once. Outputs: one token per line in
 * a plain text file and/or a ZIP of per-license QR PNGs — both written via
 * the system file picker, never to shared storage, never over any network.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun BatchScreen(
    onSignBatch: (
        rows: List<BatchCsv.Row>,
        onDone: (List<IssueEngine.Issued>?) -> Unit,
    ) -> Unit,
) {
    val context = LocalContext.current
    var csvText by remember { mutableStateOf("") }
    var defaultTier by remember { mutableStateOf("standard") }
    var defaultDays by remember { mutableStateOf(30) }
    var issuedTokens by remember { mutableStateOf<List<IssueEngine.Issued>?>(null) }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    val defaultExpiry = run {
        val endOfDay = LocalDate.now(ZoneId.systemDefault()).plusDays(defaultDays.toLong())
            .atTime(23, 59, 59).atZone(ZoneId.systemDefault()).toInstant()
        IssuerTime.formatRfc3339Utc(endOfDay)
    }
    val parsed = remember(csvText, defaultTier, defaultExpiry) {
        BatchCsv.parse(csvText, defaultTier, defaultExpiry)
    }

    val fileImporter = rememberLauncherForActivityResult(
        ActivityResultContracts.GetContent(),
    ) { uri ->
        if (uri != null) {
            runCatching {
                context.contentResolver.openInputStream(uri)?.bufferedReader()?.use { it.readText() }
            }.getOrNull()?.let { imported ->
                csvText = if (csvText.isBlank()) imported else csvText + "\n" + imported
            }
        }
    }

    val txtSaver = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("text/plain"),
    ) { uri ->
        if (uri != null && issuedTokens != null) {
            runCatching {
                context.contentResolver.openOutputStream(uri)?.use { output ->
                    output.write(
                        BatchCsv.renderTokensText(issuedTokens!!.map { it.token })
                            .toByteArray(Charsets.UTF_8),
                    )
                }
            }
        }
    }

    val zipSaver = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/zip"),
    ) { uri ->
        if (uri != null && issuedTokens != null) {
            runCatching {
                context.contentResolver.openOutputStream(uri)?.use { output ->
                    QrExport.writeQrZip(
                        issuedTokens!!.map { it.entry.id to it.token },
                        output,
                    )
                }
            }
        }
    }

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .verticalScroll(rememberScrollState())
            .imePadding()
            .padding(horizontal = 24.dp, vertical = 8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Header(stringResource(R.string.issuer_batch_header), stringResource(R.string.issuer_batch_hint))
        Spacer(Modifier.height(16.dp))

        OutlinedTextField(
            value = csvText,
            onValueChange = { csvText = it },
            label = { Text(stringResource(R.string.issuer_batch_input)) },
            supportingText = { Text(stringResource(R.string.issuer_batch_format)) },
            minLines = 5,
            maxLines = 14,
            textStyle = androidx.compose.material3.LocalTextStyle.current.copy(fontFamily = FontFamily.Monospace),
            modifier = Modifier.fillMaxWidth().widthIn(max = 560.dp),
        )
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            TextButton(onClick = { fileImporter.launch("*/*") }) {
                Text(stringResource(R.string.issuer_batch_import))
            }
            TextButton(onClick = { csvText = "" }) {
                Text(stringResource(R.string.issuer_batch_clear))
            }
        }

        Spacer(Modifier.height(8.dp))
        OutlinedTextField(
            value = defaultTier,
            onValueChange = { defaultTier = it },
            label = { Text(stringResource(R.string.issuer_batch_default_tier)) },
            singleLine = true,
            modifier = Modifier.widthIn(max = 560.dp),
        )
        FlowRow(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.widthIn(max = 560.dp),
        ) {
            listOf(7, 30, 90, 365).forEach { days ->
                androidx.compose.material3.FilterChip(
                    selected = defaultDays == days,
                    onClick = { defaultDays = days },
                    label = { Text("+$days${stringResource(R.string.issuer_batch_days)}") },
                )
            }
        }
        DetailLine(stringResource(R.string.issuer_expiry_preview), defaultExpiry)

        if (csvText.isNotBlank()) {
            Spacer(Modifier.height(8.dp))
            Text(
                stringResource(
                    R.string.issuer_batch_parsed,
                    parsed.rows.size,
                    parsed.errors.size,
                ),
                fontWeight = FontWeight.Bold,
            )
            parsed.errors.take(5).forEach { lineError ->
                Text(
                    lineError,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }
            if (parsed.errors.size > 5) {
                Text(
                    stringResource(R.string.issuer_batch_more_errors, parsed.errors.size - 5),
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }
        error?.let {
            Spacer(Modifier.height(8.dp))
            Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
        }

        Spacer(Modifier.height(16.dp))
        if (busy) {
            CircularProgressIndicator()
        } else {
            Button(
                onClick = {
                    busy = true
                    error = null
                    onSignBatch(parsed.rows) { result ->
                        busy = false
                        if (result == null) {
                            error = context.getString(R.string.issuer_error_generic)
                        } else {
                            issuedTokens = result
                        }
                    }
                },
                enabled = parsed.rows.isNotEmpty() && !busy,
                modifier = Modifier.widthIn(max = 560.dp).fillMaxWidth(),
            ) {
                Text(stringResource(R.string.issuer_batch_generate, parsed.rows.size))
            }
        }
        Spacer(Modifier.height(24.dp))
    }

    issuedTokens?.let { tokens ->
        AlertDialog(
            onDismissRequest = { issuedTokens = null },
            title = { Text(stringResource(R.string.issuer_batch_done_title)) },
            text = {
                Column {
                    Text(stringResource(R.string.issuer_batch_done_body, tokens.size))
                    Spacer(Modifier.height(8.dp))
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedButton(onClick = {
                            txtSaver.launch("vor-licenses.txt")
                        }) { Text(stringResource(R.string.issuer_batch_save_txt)) }
                        OutlinedButton(onClick = {
                            zipSaver.launch("vor-licenses-qr.zip")
                        }) { Text(stringResource(R.string.issuer_batch_save_zip)) }
                    }
                    Spacer(Modifier.height(8.dp))
                    Text(
                        stringResource(R.string.issuer_history_note),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            },
            confirmButton = {
                TextButton(onClick = { issuedTokens = null }) {
                    Text(stringResource(R.string.issuer_close))
                }
            },
        )
    }
}
