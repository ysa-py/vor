package com.vor.licensemanager.issuer

import android.content.Intent
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.vor.licensemanager.QrCodec
import com.vor.licensemanager.R

/**
 * Shared result dialog for a freshly issued (or recalled) token:
 * selectable token text, copy, on-screen QR, share sheet, and PNG export
 * through the system file picker. Zero network, zero permissions.
 *
 * Responsive: the QR scales to the available dialog width (capped at
 * 280dp), the action buttons wrap via FlowRow, and the QR reveal animates.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun TokenResultDialog(
    token: String,
    licenseId: String,
    onDismiss: () -> Unit,
) {
    val context = LocalContext.current
    val clipboard = LocalClipboardManager.current
    var showQr by remember { mutableStateOf(false) }

    val pngSaver = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("image/png"),
    ) { uri ->
        if (uri != null) {
            runCatching {
                val bitmap = QrCodec.encode(token) ?: return@runCatching false
                context.contentResolver.openOutputStream(uri)?.use { QrExport.writePng(bitmap, it) } ?: false
            }
        }
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.issuer_token_title)) },
        text = {
            Column(Modifier.animateContentSize()) {
                Text(
                    stringResource(R.string.issuer_token_hint),
                    style = MaterialTheme.typography.bodySmall,
                )
                Spacer(Modifier.height(8.dp))
                SelectionContainer {
                    Text(
                        token,
                        fontFamily = FontFamily.Monospace,
                        style = MaterialTheme.typography.bodySmall,
                        maxLines = 6,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                if (showQr) {
                    Spacer(Modifier.height(12.dp))
                    Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.fillMaxWidth()) {
                        QrCodec.encode(token)?.let { bitmap ->
                            // Fit the QR to the dialog: never wider than the
                            // available width, capped at 280dp so it stays
                            // comfortably scannable on larger screens.
                            BoxWithConstraints(Modifier.fillMaxWidth()) {
                                val qrSize = maxWidth.coerceAtMost(280.dp)
                                Image(
                                    bitmap = bitmap.asImageBitmap(),
                                    contentDescription = stringResource(R.string.issuer_show_qr),
                                    modifier = Modifier.size(qrSize),
                                )
                            }
                        } ?: Text(
                            stringResource(R.string.issuer_qr_error),
                            style = MaterialTheme.typography.bodySmall,
                        )
                        Text(
                            stringResource(R.string.issuer_qr_scan_hint),
                            style = MaterialTheme.typography.bodySmall,
                            textAlign = TextAlign.Center,
                        )
                    }
                }
            }
        },
        confirmButton = {
            TextButton(onClick = { showQr = !showQr }) {
                Text(if (showQr) stringResource(R.string.issuer_hide_qr) else stringResource(R.string.issuer_show_qr))
            }
        },
        dismissButton = {
            FlowRow(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                TextButton(onClick = { clipboard.setText(AnnotatedString(token)) }) {
                    Text(stringResource(R.string.issuer_copy))
                }
                TextButton(onClick = {
                    val send = Intent(Intent.ACTION_SEND).apply {
                        type = "text/plain"
                        putExtra(Intent.EXTRA_TEXT, token)
                    }
                    context.startActivity(Intent.createChooser(send, null))
                }) { Text(stringResource(R.string.issuer_share)) }
                OutlinedButton(onClick = { pngSaver.launch("vor-license-$licenseId.png") }) {
                    Text(stringResource(R.string.issuer_save_png))
                }
            }
        },
    )
}

/** Small labeled detail row used across the issuer screens. */
@Composable
fun DetailLine(label: String, value: String) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(label, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(
            value,
            style = MaterialTheme.typography.bodySmall,
            textAlign = TextAlign.End,
            modifier = Modifier.padding(start = 12.dp),
        )
    }
}
