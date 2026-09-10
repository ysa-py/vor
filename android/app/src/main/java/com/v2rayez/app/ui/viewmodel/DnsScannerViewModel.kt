package com.v2rayez.app.ui.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.v2rayez.app.data.dns.ResolverScanner
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import javax.inject.Inject

data class DnsScannerUiState(
    val running: Boolean = false,
    val results: List<ResolverScanner.Result> = emptyList(),
    val error: String? = null,
)

/**
 * DNS resolver scanner/scorer — surfaces resolver health (latency, EDNS,
 * NXDOMAIN-hijack detection) inside the tools flow; the server-profile
 * setup links here so users pick a resolver before configuring tunnels.
 */
@HiltViewModel
class DnsScannerViewModel @Inject constructor() : ViewModel() {

    private val _state = MutableStateFlow(DnsScannerUiState())
    val state: StateFlow<DnsScannerUiState> = _state.asStateFlow()

    fun scan(targets: List<ResolverScanner.Target> = ResolverScanner.defaultTargets()) {
        if (_state.value.running) return
        _state.update { it.copy(running = true, error = null) }
        viewModelScope.launch {
            runCatching { ResolverScanner.scan(targets, "example.com") }
                .onSuccess { results -> _state.update { it.copy(running = false, results = results) } }
                .onFailure { failure ->
                    _state.update { it.copy(running = false, error = failure.message) }
                }
        }
    }
}
