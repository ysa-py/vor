package com.v2rayez.app.ui.screens.license

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Key
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
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
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.v2rayez.app.R
import com.vor.license.LicensePayload
import com.vor.license.LicenseStatus
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import javax.inject.Inject

/**
 * The license-entry gate — the FIRST screen of Vor (before onboarding and
 * the main UI). Pasting a token verifies it locally against the embedded
 * Ed25519 public key; only a VALID token unlocks the app. Expired licenses
 * show their expiry; invalid ones show a generic error (no oracle).
 */
@HiltViewModel
class LicenseGateViewModel @Inject constructor(
    private val repository: com.v2rayez.app.data.license.LicenseRepository,
) : ViewModel() {

    val gateState: StateFlow<com.v2rayez.app.data.license.LicenseRepository.GateState> =
        repository.gateState.stateIn(
            scope = viewModelScope,
            started = SharingStarted.Eagerly,
            initialValue = com.v2rayez.app.data.license.LicenseRepository.GateState(),
        )

    private val _busy = MutableStateFlow(false)
    val busy: StateFlow<Boolean> = _busy

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error

    fun activate(token: String, onUnlocked: () -> Unit) {
        if (token.isBlank()) return
        viewModelScope.launch {
            _busy.value = true
            _error.value = null
            try {
                when (repository.activate(token.trim()).status) {
                    LicenseStatus.VALID -> onUnlocked()
                    LicenseStatus.EXPIRED -> _error.value = "expired"
                    LicenseStatus.INVALID -> _error.value = "invalid"
                }
            } catch (t: Throwable) {
                // 2026-09 crash hardening (device report: tapping "Check / Activate
                // License" with a well-formed token crashed the app). This scope
                // has no CoroutineExceptionHandler, so ANY escaped throwable used
                // to kill the process. Now every failure — device-specific or
                // otherwise — degrades to the standard invalid-token UI.
                _error.value = "invalid"
            } finally {
                _busy.value = false
            }
        }
    }
}

@Composable
fun LicenseGateScreen(
    onUnlocked: () -> Unit,
    viewModel: LicenseGateViewModel = hiltViewModel(),
) {
    val state by viewModel.gateState.collectAsState()
    val busy by viewModel.busy.collectAsState()
    val error by viewModel.error.collectAsState()
    var token by remember { mutableStateOf("") }

    // Unlock immediately when a stored license is (still) valid.
    LaunchedEffect(state.status, state.hydrated) {
        if (state.hydrated && state.status == LicenseStatus.VALID) {
            onUnlocked()
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
        Icon(
            imageVector = Icons.Filled.Key,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.primary,
            modifier = Modifier.height(48.dp),
        )
        Spacer(Modifier.height(16.dp))
        Text(
            text = stringResource(R.string.license_gate_title),
            style = MaterialTheme.typography.headlineMedium,
            fontWeight = FontWeight.Bold,
            textAlign = TextAlign.Center,
        )
        Spacer(Modifier.height(8.dp))
        Text(
            text = stringResource(R.string.license_gate_subtitle),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
            modifier = Modifier.widthIn(max = 420.dp),
        )
        Spacer(Modifier.height(24.dp))

        // Status of a stored-but-expired token.
        val storedPayload = state.payload
        if (state.hydrated && state.status == LicenseStatus.EXPIRED && storedPayload != null) {
            LicenseExpiredCard(payload = storedPayload)
            Spacer(Modifier.height(16.dp))
        }

        OutlinedTextField(
            value = token,
            onValueChange = { token = it },
            label = { Text(stringResource(R.string.license_gate_token_label)) },
            placeholder = { Text(stringResource(R.string.license_gate_token_hint)) },
            minLines = 3,
            maxLines = 6,
            modifier = Modifier
                .fillMaxWidth()
                .widthIn(max = 520.dp),
            isError = error != null,
            supportingText = {
                when (error) {
                    "expired" -> Text(stringResource(R.string.license_gate_error_expired))
                    "invalid" -> Text(stringResource(R.string.license_gate_error_invalid))
                    else -> Text(stringResource(R.string.license_gate_offline_note))
                }
            },
        )
        Spacer(Modifier.height(16.dp))
        Button(
            onClick = { viewModel.activate(token, onUnlocked) },
            enabled = token.isNotBlank() && !busy,
            modifier = Modifier
                .fillMaxWidth()
                .widthIn(max = 520.dp),
        ) {
            if (busy) {
                CircularProgressIndicator(
                    modifier = Modifier.height(20.dp),
                    strokeWidth = 2.dp,
                    color = MaterialTheme.colorScheme.onPrimary,
                )
                Spacer(Modifier.width(8.dp))
            }
            Text(stringResource(R.string.license_gate_activate))
        }
    }
}

@Composable
private fun LicenseExpiredCard(payload: LicensePayload) {
    Card(
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.errorContainer,
        ),
        modifier = Modifier
            .fillMaxWidth()
            .widthIn(max = 520.dp),
    ) {
        Column(Modifier.padding(16.dp)) {
            Text(
                text = stringResource(R.string.license_gate_expired_title),
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.onErrorContainer,
            )
            Spacer(Modifier.height(4.dp))
            Text(
                text = stringResource(R.string.license_gate_expired_body, payload.expiresAt),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onErrorContainer,
            )
        }
    }
}
