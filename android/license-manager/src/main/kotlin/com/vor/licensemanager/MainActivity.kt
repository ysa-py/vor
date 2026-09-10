package com.vor.licensemanager

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.vor.license.LicenseResult
import com.vor.license.LicenseStatus
import com.vor.license.LicenseVerifier

/**
 * Vor License Manager — the minimal companion app (see license/SPEC.md and
 * the engineering task: "a separate, minimal companion Android app whose
 * only job is import/paste a license token, verify its signature and expiry
 * locally, and display status").
 *
 * One primary action button: "Check / Activate License".
 * Shares the exact same verification code as the main Vor app via the
 * :core-license module, and verifies fully offline.
 */
class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        // Accept a shared token (ACTION_SEND text/plain).
        val sharedText: String? = if (intent?.action == android.content.Intent.ACTION_SEND) {
            @Suppress("DEPRECATION")
            intent.getCharSequenceExtra(android.content.Intent.EXTRA_TEXT)?.toString()
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
    var token by remember { mutableStateOf(initialToken ?: "") }
    var result by remember { mutableStateOf<LicenseResult?>(null) }

    // Auto-check a shared token on arrival.
    LaunchedEffect(initialToken) {
        if (!initialToken.isNullOrBlank()) {
            result = verifyNow(initialToken)
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
        Spacer(Modifier.height(12.dp))
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
        Spacer(Modifier.height(16.dp))

        Button(
            onClick = { result = verifyNow(token) },
            enabled = token.isNotBlank(),
            modifier = Modifier
                .fillMaxWidth()
                .widthIn(max = 520.dp),
        ) {
            Text(stringResource(R.string.check_activate))
        }

        result?.let { checked ->
            Spacer(Modifier.height(24.dp))
            StatusCard(result = checked)
        }
    }
}

private fun verifyNow(token: String): LicenseResult =
    LicenseVerifier.verify(BuildConfig.VOR_LICENSE_PUBLIC_KEY, token, System.currentTimeMillis() / 1000)

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
private fun DetailRow(label: String, value: String) {
    Text(
        text = "$label: $value",
        style = MaterialTheme.typography.bodySmall,
    )
}
