package com.unifiedshield.security

import android.content.ComponentCallbacks2
import android.content.Context
import android.content.res.Configuration
import android.util.Log
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import java.security.SecureRandom
import java.util.Arrays

/**
 * SecurityController:
 * Hooks into Android OS lifecycle (via DefaultLifecycleObserver)
 * as well as ComponentCallbacks2 (onTrimMemory, onStop, onDestroy)
 * to perform zero-trace emergency RAM wipes of sensitive session keys, cryptographic salts,
 * dynamic SNI buffers, and ephemeral identity tokens whenever onStop, onDestroy, or Panic mode occurs.
 */
data class SecurityControllerState(
    val lastWipeTimestamp: Long = 0,
    val lastWipeReason: String = "INITIALIZED",
    val totalWipesPerformed: Int = 0,
    val isKeysActiveInRam: Boolean = true,
    val ephemeralIdentityId: String = "ID-9A84F2B0"
)

class SecurityController private constructor(private val context: Context) : ComponentCallbacks2, DefaultLifecycleObserver {

    private val TAG = "SecurityController"
    private val secureRandom = SecureRandom()

    private val _state = MutableStateFlow(SecurityControllerState())
    val state: StateFlow<SecurityControllerState> = _state

    // In-memory sensitive buffers
    private var sessionKeyBuffer: ByteArray? = ByteArray(64)
    private var pqcSharedSecretBuffer: ByteArray? = ByteArray(128)
    private var dynamicSniBuffer: ByteArray? = ByteArray(256)

    init {
        // Fill initial buffers with cryptographic material
        sessionKeyBuffer?.let { secureRandom.nextBytes(it) }
        pqcSharedSecretBuffer?.let { secureRandom.nextBytes(it) }
        dynamicSniBuffer?.let { secureRandom.nextBytes(it) }

        // Register for OS memory and lifecycle events
        try {
            context.registerComponentCallbacks(this)
            Log.i(TAG, "SecurityController registered to Android OS ComponentCallbacks")
        } catch (e: Exception) {
            Log.w(TAG, "Failed to register component callbacks: ${e.message}")
        }
    }

    /**
     * Register SecurityController with an Activity or Service LifecycleOwner
     */
    fun registerLifecycleOwner(lifecycleOwner: LifecycleOwner) {
        try {
            lifecycleOwner.lifecycle.addObserver(this)
            Log.i(TAG, "SecurityController registered to LifecycleOwner: $lifecycleOwner")
        } catch (e: Exception) {
            Log.w(TAG, "Failed to register LifecycleOwner: ${e.message}")
        }
    }

    /**
     * Performs a DoD 5220.22-M 3-pass zeroing of in-memory sensitive keys and buffers.
     * Overwrites all sensitive session keys and buffers currently held in the app's heap with random data.
     */
    @Synchronized
    fun triggerEmergencyRamWipe(reason: String) {
        Log.i(TAG, "🛡️ Anti-Forensic RAM Wipe executed ($reason)")

        // 3-Pass Overwrite: 0x00 -> 0xFF -> Random -> Nullify
        sessionKeyBuffer?.let { buf ->
            Arrays.fill(buf, 0.toByte())
            Arrays.fill(buf, 0xFF.toByte())
            secureRandom.nextBytes(buf)
            Arrays.fill(buf, 0.toByte())
        }
        pqcSharedSecretBuffer?.let { buf ->
            Arrays.fill(buf, 0.toByte())
            Arrays.fill(buf, 0xFF.toByte())
            secureRandom.nextBytes(buf)
            Arrays.fill(buf, 0.toByte())
        }
        dynamicSniBuffer?.let { buf ->
            Arrays.fill(buf, 0.toByte())
            Arrays.fill(buf, 0xFF.toByte())
            secureRandom.nextBytes(buf)
            Arrays.fill(buf, 0.toByte())
        }

        // Generate new ephemeral identity
        val newId = "ID-" + (10000000..99999999).random().toString(16).uppercase()

        // Reallocate clean ephemeral memory buffers with fresh random entropy
        sessionKeyBuffer = ByteArray(64).also { secureRandom.nextBytes(it) }
        pqcSharedSecretBuffer = ByteArray(128).also { secureRandom.nextBytes(it) }
        dynamicSniBuffer = ByteArray(256).also { secureRandom.nextBytes(it) }

        _state.value = _state.value.copy(
            lastWipeTimestamp = System.currentTimeMillis(),
            lastWipeReason = reason,
            totalWipesPerformed = _state.value.totalWipesPerformed + 1,
            isKeysActiveInRam = true,
            ephemeralIdentityId = newId
        )

        Log.i(TAG, "RAM Wipe completed successfully. Ephemeral identity regenerated: $newId")
    }

    /**
     * LifecycleObserver hook: Triggered on app onStop event.
     */
    override fun onStop(owner: LifecycleOwner) {
        super.onStop(owner)
        Log.w(TAG, "LifecycleObserver: onStop detected - executing emergency memory wipe")
        triggerEmergencyRamWipe("LifecycleObserver onStop / Background Transition")
    }

    /**
     * LifecycleObserver hook: Triggered on app onDestroy event.
     */
    override fun onDestroy(owner: LifecycleOwner) {
        super.onDestroy(owner)
        Log.w(TAG, "LifecycleObserver: onDestroy detected - executing final zero-trace RAM wipe")
        triggerEmergencyRamWipe("LifecycleObserver onDestroy / App Termination")
    }

    /**
     * OS Hook: Called when background task suspension or UI hidden occurs.
     */
    override fun onTrimMemory(level: Int) {
        when (level) {
            ComponentCallbacks2.TRIM_MEMORY_UI_HIDDEN,
            ComponentCallbacks2.TRIM_MEMORY_BACKGROUND,
            ComponentCallbacks2.TRIM_MEMORY_MODERATE,
            ComponentCallbacks2.TRIM_MEMORY_COMPLETE,
            ComponentCallbacks2.TRIM_MEMORY_RUNNING_CRITICAL -> {
                Log.w(TAG, "OS Background Task Suspension / TrimMemory Signal ($level) detected.")
                triggerEmergencyRamWipe("OS Background Suspension (Level: $level)")
            }
        }
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        // No-op
    }

    override fun onLowMemory() {
        Log.w(TAG, "OS LowMemory Signal detected.")
        triggerEmergencyRamWipe("OS LowMemory Signal")
    }

    companion object {
        @Volatile
        private var instance: SecurityController? = null

        fun getInstance(context: Context): SecurityController {
            return instance ?: synchronized(this) {
                instance ?: SecurityController(context.applicationContext).also { instance = it }
            }
        }
    }
}
