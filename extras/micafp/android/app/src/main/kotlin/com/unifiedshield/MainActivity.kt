package com.unifiedshield

import android.content.Intent
import android.net.VpnService as AndroidVpnService
import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.ui.Modifier
import androidx.lifecycle.lifecycleScope
import com.unifiedshield.security.SecurityController
import com.unifiedshield.ui.MainScreen
import com.unifiedshield.ui.theme.UnifiedShieldTheme
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {

    private val TAG = "MainActivity"

    private val vpnPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == RESULT_OK) {
            startVpnService()
        } else {
            Log.w(TAG, "VPN permission denied by user.")
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Initialize SecurityController and register LifecycleObserver safely
        val securityController = SecurityController.getInstance(applicationContext)
        securityController.registerLifecycleOwner(this)

        // Offload heavy Rust daemon / JNI subsystem pre-warm to Dispatchers.IO to ensure zero main-thread blocking
        lifecycleScope.launch(Dispatchers.IO) {
            try {
                NativeCoreLoader.initialize(applicationContext)
                val isCoreReady = NativeCoreLoader.isNativeReady.value
                Log.i(TAG, "Rust Daemon Subsystem Pre-warm Completed. Native Cores Ready: $isCoreReady (${NativeCoreLoader.statusMessage.value})")
            } catch (e: Throwable) {
                Log.w(TAG, "Async daemon warm-up handled gracefully: ${e.message}")
            }
            // ── MICAFP Directive v6 — start additive background engines (A1: additive only) ──
            try {
                // Auto-Pilot rotation engine (self-starts; C2 Kotlin coordination layer)
                com.unifiedshield.autopilot.AutoPilotEngine.getInstance(applicationContext)
                // Auto Leak Test monitor (B3.3; existing manual run stays untouched)
                com.unifiedshield.doctor.LeakTestMonitor.getInstance(applicationContext).startAutoMonitor()
                Log.i(TAG, "MICAFP additive engines started (AutoPilot + LeakTestMonitor)")
            } catch (e: Throwable) {
                Log.w(TAG, "MICAFP engine start handled gracefully: ${e.message}")
            }
        }

        setContent {
            UnifiedShieldTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    MainScreen(
                        onConnectClick = { requestVpnPermission() },
                        onDisconnectClick = { stopVpnService() }
                    )
                }
            }
        }
    }

    override fun onStop() {
        super.onStop()
    }

    override fun onTrimMemory(level: Int) {
        super.onTrimMemory(level)
        SecurityController.getInstance(applicationContext).onTrimMemory(level)
    }

    private fun requestVpnPermission() {
        try {
            val intent = AndroidVpnService.prepare(this)
            if (intent != null) {
                vpnPermissionLauncher.launch(intent)
            } else {
                startVpnService()
            }
        } catch (e: Exception) {
            Log.e(TAG, "VpnService.prepare() error: ${e.message}")
            startVpnService()
        }
    }

    private fun startVpnService() {
        try {
            val intent = Intent(this, VpnService::class.java).apply {
                action = VpnService.ACTION_START
            }
            startService(intent)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to start VpnService: ${e.message}")
        }
    }

    private fun stopVpnService() {
        try {
            val intent = Intent(this, VpnService::class.java).apply {
                action = VpnService.ACTION_STOP
            }
            startService(intent)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to stop VpnService: ${e.message}")
        }
    }
}

