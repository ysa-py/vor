package com.unifiedshield.killswitch

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

// =============================================================================
// MICAFP Directive v6 — B3.1 additive global kill-switch state holder.
// READ-ONLY mirror for UI. The authoritative enable/disable logic remains
// inside the existing KillSwitch class (zero modification — Directive A1).
// The VpnService-side wiring publishes its state here via publish(); until
// the gated service commit lands, the UI shows the persisted preference.
// =============================================================================

object KillSwitchState {

    private val _isArmed = MutableStateFlow(false)
    val isArmed: Boolean get() = _isArmed.value
    val armedFlow: StateFlow<Boolean> = _isArmed.asStateFlow()

    /** Called by the (gated) service wiring to publish real armed state. */
    fun publish(isArmed: Boolean) {
        _isArmed.value = isArmed
    }
}
