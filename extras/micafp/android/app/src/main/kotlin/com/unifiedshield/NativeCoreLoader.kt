package com.unifiedshield

import android.content.Context
import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.io.File

/**
 * NativeCoreLoader singleton object providing thread-safe, non-blocking native library loading
 * with an automatic fallback to Pure-Kotlin Enterprise Mode.
 */
object NativeCoreLoader {
    private const val TAG = "NativeCoreLoader"

    private val _isNativeReady = MutableStateFlow(false)
    val isNativeReady: StateFlow<Boolean> = _isNativeReady.asStateFlow()

    private val _isPureKotlinMode = MutableStateFlow(true)
    val isPureKotlinMode: StateFlow<Boolean> = _isPureKotlinMode.asStateFlow()

    private val _statusMessage = MutableStateFlow("Uninitialized")
    val statusMessage: StateFlow<String> = _statusMessage.asStateFlow()

    @Volatile
    private var isInitialized = false

    var isUnifiedShieldNativeLoaded = false
        private set

    var isMicafpCoreNativeLoaded = false
        private set

    /**
     * Thread-safe asynchronous initialize function.
     * Can be invoked on Dispatchers.IO.
     */
    @Synchronized
    fun initialize(context: Context? = null): Boolean {
        if (isInitialized) {
            return _isNativeReady.value
        }
        isInitialized = true

        Log.i(TAG, "Initializing NativeCoreLoader...")

        // Safe check for libunifiedshield
        isUnifiedShieldNativeLoaded = tryLoadLibrary("unifiedshield")

        // Safe check for libmicafp_transport_core
        isMicafpCoreNativeLoaded = tryLoadLibrary("micafp_transport_core")

        val ready = isUnifiedShieldNativeLoaded || isMicafpCoreNativeLoaded
        _isNativeReady.value = ready
        _isPureKotlinMode.value = !ready

        if (ready) {
            _statusMessage.value = "Native Core Active (JNI Accelerators Enabled)"
            Log.i(TAG, "Native Cores Ready: true (libunifiedshield=$isUnifiedShieldNativeLoaded, libmicafp_transport_core=$isMicafpCoreNativeLoaded)")
        } else {
            _statusMessage.value = "Pure-Kotlin Enterprise Resilient Mode Active"
            Log.i(TAG, "Operating in Pure-Kotlin Enterprise Mode: High-performance coroutine fallback ready without native binary requirement.")
        }

        return ready
    }

    private fun tryLoadLibrary(libName: String): Boolean {
        return try {
            System.loadLibrary(libName)
            Log.i(TAG, "Successfully loaded native library: lib$libName.so")
            true
        } catch (e: UnsatisfiedLinkError) {
            Log.i(TAG, "Native library lib$libName.so not present in runtime path. Fallback to Kotlin engine.")
            false
        } catch (t: Throwable) {
            Log.w(TAG, "Unexpected error loading native library lib$libName.so: ${t.message}")
            false
        }
    }
}
