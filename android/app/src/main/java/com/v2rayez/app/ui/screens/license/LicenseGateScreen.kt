package com.v2rayez.app.ui.screens.license

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
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
 * Ed25519 public key (or the paired reseller key, v1.5.0); only a VALID
 * token unlocks the app. Expired licenses show their expiry; invalid ones
 * show a generic error (no oracle).
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

    /** Non-null while a pairing attempt just failed/succeeded (UI feedback). */
    private val _pairState = MutableStateFlow<PairState?>(null)
    val pairState: StateFlow<PairState?> = _pairState

    /** True when a persisted crash report exists (share offer on the gate). */
    private val _crashDetected = MutableStateFlow(false)
    val crashDetected: StateFlow<Boolean> = _crashDetected

    sealed class PairState {
        /** Key imported; fingerprint for confirmation display. */
        data class Paired(val fingerprintHex: String) : PairState()

        /** Removed again. */
        object Removed : PairState()

        /** Code malformed / storage refused. */
        object Failed : PairState()
    }

    init {
        // Crash evidence probe is file I/O — off the main thread.
        viewModelScope.launch {
            _crashDetected.value = runCatching {
                com.v2rayez.app.data.diagnostics.VorCrashEvidence
                    .latestFile(repository.applicationContextForDiagnostics()) != null
            }.getOrDefault(false)
        }
    }

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

    /** Import a VORP1 pairing code (v1.5.0). Never throws. */
    fun pairPublisher(code: String) {
        viewModelScope.launch {
            val pairing = runCatching { repository.pairPublisher(code) }.getOrNull()
            _pairState.value = when (pairing) {
                null -> PairState.Failed
                else -> PairState.Paired(pairing!!.fingerprintHex)
            }
        }
    }

    /** Remove the paired key. */
    fun unpairPublisher() {
        viewModelScope.launch {
            runCatching { repository.unpairPublisher() }
            _pairState.value = PairState.Removed
        }
    }

    /** Clear the transient pairing feedback. */
    fun clearPairState() {
        _pairState.value = null
    }

    /** Drop the stored crash report (after sharing or ignoring). */
    fun dismissCrashReport() {
        viewModelScope.launch {
            runCatching {
                com.v2rayez.app.data.diagnostics.VorCrashEvidence
                    .clear(repository.applicationContextForDiagnostics())
            }
            _crashDetected.value = false
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
    val pairState by viewModel.pairState.collectAsState()
    val crashDetected by viewModel.crashDetected.collectAsState()
    var token by remember { mutableStateOf("") }
    var showPairDialog by remember { mutableStateOf(false) }
    val context = androidx.compose.ui.platform.LocalContext.current

    // Auto-fade the transient pairing feedback after a few seconds.
    LaunchedEffect(pairState) {
        if (pairState != null) {
            kotlinx.coroutines.delay(6_000)
            viewModel.clearPairState()
        }
    }

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
        // Real Vor brand emblem — the first thing a buyer of the app sees,
        // identical artwork to the launcher icon (anti-phishing trust cue).
        Image(
            painter = painterResource(R.drawable.vor_logo),
            contentDescription = null,
            contentScale = ContentScale.Fit,
            modifier = Modifier
                .size(104.dp)
                .clip(CircleShape)
        )
        Spacer(Modifier.height(20.dp))
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

        // ---- Publisher key pairing (v1.5.0) ------------------------------
        // Shown to EVERY buyer (a fresh install has no pairing yet), so the
        // flow is self-explanatory even before the seller explains it.
        Spacer(Modifier.height(12.dp))
        when (val fp = state.publisherFingerprint) {
            null -> androidx.compose.material3.TextButton(
                onClick = { showPairDialog = true },
                modifier = Modifier.widthIn(max = 520.dp),
            ) {
                Text(stringResource(R.string.pair_action))
            }
            else -> Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.widthIn(max = 520.dp),
            ) {
                Text(
                    text = stringResource(R.string.pair_active, fp),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.weight(1f),
                )
                androidx.compose.material3.TextButton(onClick = { viewModel.unpairPublisher() }) {
                    Text(stringResource(R.string.pair_active_remove))
                }
            }
        }
        // Transient pairing feedback (auto-clears with the next gate emission).
        when (pairState) {
            is LicenseGateViewModel.PairState.Paired -> Text(
                text = stringResource(R.string.pair_success),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.primary,
                textAlign = TextAlign.Center,
                modifier = Modifier.widthIn(max = 520.dp),
            )
            is LicenseGateViewModel.PairState.Removed -> Text(
                text = stringResource(R.string.pair_removed),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
                modifier = Modifier.widthIn(max = 520.dp),
            )
            is LicenseGateViewModel.PairState.Failed -> Text(
                text = stringResource(R.string.pair_error_invalid),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                textAlign = TextAlign.Center,
                modifier = Modifier.widthIn(max = 520.dp),
            )
            null -> Unit
        }

        // ---- Crash evidence (v1.5.0) --------------------------------------
        if (crashDetected) {
            Spacer(Modifier.height(20.dp))
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
                        text = stringResource(R.string.crash_report_detected),
                        style = MaterialTheme.typography.titleSmall,
                        color = MaterialTheme.colorScheme.onErrorContainer,
                    )
                    Spacer(Modifier.height(4.dp))
                    Text(
                        text = stringResource(R.string.crash_report_hint),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onErrorContainer,
                    )
                    Spacer(Modifier.height(8.dp))
                    Row {
                        androidx.compose.material3.TextButton(onClick = {
                            runCatching {
                                context.startActivity(
                                    com.v2rayez.app.data.diagnostics.VorCrashEvidence
                                        .shareIntent(context)
                                        .addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK),
                                )
                            }
                        }) {
                            Text(
                                stringResource(R.string.crash_report_share),
                                color = MaterialTheme.colorScheme.onErrorContainer,
                            )
                        }
                        androidx.compose.material3.TextButton(onClick = { viewModel.dismissCrashReport() }) {
                            Text(
                                stringResource(R.string.crash_report_dismiss),
                                color = MaterialTheme.colorScheme.onErrorContainer,
                            )
                        }
                    }
                }
            }
        }
    }

    if (showPairDialog) {
        PublisherPairDialog(
            onConfirm = { code ->
                viewModel.pairPublisher(code)
                showPairDialog = false
            },
            onDismiss = { showPairDialog = false },
        )
    }
}

/**
 * One-shot pairing input: paste the seller's VORP1 code, confirm, done.
 * Validation happens in the ViewModel/repository — the dialog only collects
 * the string, so malformed input degrades to the Failed feedback on the
 * gate, never to a crash.
 */
@Composable
private fun PublisherPairDialog(
    onConfirm: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    var code by remember { mutableStateOf("") }
    androidx.compose.material3.AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.pair_title)) },
        text = {
            Column {
                Text(
                    text = stringResource(R.string.pair_hint),
                    style = MaterialTheme.typography.bodySmall,
                )
                Spacer(Modifier.height(12.dp))
                androidx.compose.material3.OutlinedTextField(
                    value = code,
                    onValueChange = { code = it },
                    label = { Text(stringResource(R.string.pair_field_label)) },
                    minLines = 2,
                    maxLines = 4,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        },
        confirmButton = {
            androidx.compose.material3.TextButton(
                onClick = { onConfirm(code) },
                enabled = code.isNotBlank(),
            ) { Text(stringResource(R.string.pair_confirm)) }
        },
        dismissButton = {
            androidx.compose.material3.TextButton(onClick = onDismiss) {
                Text(stringResource(R.string.action_cancel))
            }
        },
    )
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
