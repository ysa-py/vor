package com.vor.licensemanager.issuer

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.vor.licensemanager.R

/**
 * The "Issued by this device" ledger. Honest by design: this is a LOCAL
 * history, not a revocation list — the disclaimer sits directly under the
 * header so no maintainer can mistake it for one.
 *
 * Everything lives in ONE LazyColumn (header included) so the list scrolls
 * as a unit and the header never overlaps the entries.
 */
@Composable
fun HistoryScreen(
    history: List<IssuerStore.IssuedEntry>,
    onDelete: (IssuerStore.IssuedEntry) -> Unit,
    onClearAll: () -> Unit,
) {
    var showQr by remember { mutableStateOf<IssuerStore.IssuedEntry?>(null) }
    var confirmClear by remember { mutableStateOf(false) }

    LazyColumn(
        modifier = Modifier.fillMaxWidth(),
        contentPadding = PaddingValues(horizontal = 24.dp, vertical = 8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        // Tablet/foldable: cap every history card at a readable width, centered.
        item {
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Header(stringResource(R.string.issuer_history_header), stringResource(R.string.issuer_history_sub))
                Spacer(Modifier.height(4.dp))
                Text(
                    stringResource(R.string.issuer_history_note),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.widthIn(max = 520.dp),
                )
            }
            Spacer(Modifier.height(12.dp))
        }

        if (history.isEmpty()) {
            item {
                Text(
                    stringResource(R.string.issuer_history_empty),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(vertical = 24.dp),
                )
            }
        } else {
            item {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.widthIn(max = 560.dp).fillMaxWidth(),
                ) {
                    Text(
                        stringResource(R.string.issuer_history_count, history.size),
                        fontWeight = FontWeight.Bold,
                        modifier = Modifier.padding(end = 8.dp).weight(1f),
                    )
                    TextButton(onClick = { confirmClear = true }) {
                        Text(
                            stringResource(R.string.issuer_history_clear),
                            color = MaterialTheme.colorScheme.error,
                        )
                    }
                }
                Spacer(Modifier.height(4.dp))
            }
            items(history.reversed()) { entry ->
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.surfaceVariant,
                        contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
                    ),
                    modifier = Modifier
                        .fillMaxWidth()
                        .widthIn(max = 560.dp)
                        .padding(vertical = 4.dp),
                ) {
                    Column(Modifier.padding(12.dp)) {
                        Text(
                            entry.id,
                            fontWeight = FontWeight.Bold,
                            style = MaterialTheme.typography.titleSmall,
                        )
                        DetailLine(stringResource(R.string.issuer_detail_tier), entry.tier)
                        DetailLine(stringResource(R.string.issuer_detail_issued), entry.issuedAt)
                        DetailLine(stringResource(R.string.issuer_detail_expires), entry.expiresAt)
                        if (entry.notes.isNotBlank()) {
                            DetailLine(stringResource(R.string.issuer_detail_notes), entry.notes)
                        }
                        Spacer(Modifier.height(4.dp))
                        Row {
                            TextButton(onClick = { showQr = entry }) {
                                Text(stringResource(R.string.issuer_show_qr))
                            }
                            TextButton(onClick = { onDelete(entry) }) {
                                Text(
                                    stringResource(R.string.issuer_delete_entry),
                                    color = MaterialTheme.colorScheme.error,
                                )
                            }
                        }
                    }
                }
            }
            item { Spacer(Modifier.height(24.dp)) }
        }
    }

    showQr?.let { entry ->
        TokenResultDialog(
            token = entry.token,
            licenseId = entry.id,
            onDismiss = { showQr = null },
        )
    }

    if (confirmClear) {
        AlertDialog(
            onDismissRequest = { confirmClear = false },
            title = { Text(stringResource(R.string.issuer_history_clear_confirm_title)) },
            text = { Text(stringResource(R.string.issuer_history_clear_confirm_body)) },
            confirmButton = {
                TextButton(onClick = {
                    confirmClear = false
                    onClearAll()
                }) {
                    Text(
                        stringResource(R.string.issuer_history_clear),
                        color = MaterialTheme.colorScheme.error,
                    )
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmClear = false }) {
                    Text(stringResource(R.string.issuer_cancel))
                }
            },
        )
    }
}
