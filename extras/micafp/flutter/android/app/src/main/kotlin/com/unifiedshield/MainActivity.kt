package com.unifiedshield

import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

/**
 * Main Activity for UnifiedShield Flutter app.
 *
 * Handles:
 * - Flutter engine initialization
 * - Platform channels for VPN service management
 * - VPN permission requests
 * - Rust daemon lifecycle
 */
class MainActivity : FlutterActivity() {

    companion object {
        private const val CHANNEL = "com.unifiedshield/vpn"
        private const val VPN_REQUEST_CODE = 1001
        private const val DAEMON_SOCKET_PATH = "unifiedshield.sock"
    }

    private var vpnInterface: ParcelFileDescriptor? = null
    private var pendingResult: MethodChannel.Result? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "requestVpnPermission" -> {
                        pendingResult = result
                        requestVpnPermission()
                    }
                    "startVpn" -> {
                        val server = call.argument<String>("server") ?: ""
                        val port = call.argument<Int>("port") ?: 443
                        val coreId = call.argument<String>("coreId") ?: ""
                        startVpn(server, port, coreId, result)
                    }
                    "stopVpn" -> {
                        stopVpn(result)
                    }
                    "isVpnRunning" -> {
                        result.success(vpnInterface != null)
                    }
                    "startDaemon" -> {
                        startDaemon(result)
                    }
                    "stopDaemon" -> {
                        stopDaemon(result)
                    }
                    else -> result.notImplemented()
                }
            }
    }

    private fun requestVpnPermission() {
        val intent = VpnService.prepare(this)
        if (intent != null) {
            startActivityForResult(intent, VPN_REQUEST_CODE)
        } else {
            pendingResult?.success(true)
            pendingResult = null
        }
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == VPN_REQUEST_CODE) {
            pendingResult?.success(resultCode == RESULT_OK)
            pendingResult = null
        }
    }

    private fun startVpn(server: String, port: Int, coreId: String, result: MethodChannel.Result) {
        try {
            val intent = Intent(this, UnifiedShieldVpnService::class.java).apply {
                putExtra("server", server)
                putExtra("port", port)
                putExtra("coreId", coreId)
                action = "START"
            }

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                startForegroundService(intent)
            } else {
                startService(intent)
            }

            result.success(true)
        } catch (e: Exception) {
            result.error("VPN_START_ERROR", e.message, null)
        }
    }

    private fun stopVpn(result: MethodChannel.Result) {
        try {
            val intent = Intent(this, UnifiedShieldVpnService::class.java).apply {
                action = "STOP"
            }
            startService(intent)
            result.success(true)
        } catch (e: Exception) {
            result.error("VPN_STOP_ERROR", e.message, null)
        }
    }

    private fun startDaemon(result: MethodChannel.Result) {
        try {
            // Start the Rust daemon process via JNI or command
            val intent = Intent(this, UnifiedShieldVpnService::class.java).apply {
                action = "START_DAEMON"
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                startForegroundService(intent)
            } else {
                startService(intent)
            }
            result.success(true)
        } catch (e: Exception) {
            result.error("DAEMON_START_ERROR", e.message, null)
        }
    }

    private fun stopDaemon(result: MethodChannel.Result) {
        try {
            val intent = Intent(this, UnifiedShieldVpnService::class.java).apply {
                action = "STOP_DAEMON"
            }
            startService(intent)
            result.success(true)
        } catch (e: Exception) {
            result.error("DAEMON_STOP_ERROR", e.message, null)
        }
    }

    override fun onDestroy() {
        vpnInterface?.close()
        super.onDestroy()
    }
}

/**
 * VPN Service implementation for Android.
 *
 * Creates the TUN interface and passes the file descriptor
 * to the Rust daemon for packet processing.
 */
class UnifiedShieldVpnService : VpnService() {

    private var vpnInterface: ParcelFileDescriptor? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            "START" -> {
                val server = intent.getStringExtra("server") ?: ""
                val port = intent.getIntExtra("port", 443)
                val coreId = intent.getStringExtra("coreId") ?: ""
                setupVpn(server, port, coreId)
            }
            "STOP" -> {
                teardownVpn()
            }
            "START_DAEMON" -> {
                // Initialize Rust daemon
            }
            "STOP_DAEMON" -> {
                teardownVpn()
            }
        }
        return START_STICKY
    }

    private fun setupVpn(server: String, port: Int, coreId: String) {
        if (vpnInterface != null) return

        try {
            val builder = Builder()
                .setSession("UnifiedShield")
                .addAddress("10.0.0.2", 32)
                .addRoute("0.0.0.0", 0)
                .addDnsServer("223.5.5.5")   // AliDNS - Chinese, accessible from Iran
                .addDnsServer("119.29.29.29") // DNSPod - Chinese
                .setMtu(1500)
                .setBlocking(true)

            // IPv6 support
            builder.addAddress("fd00::2", 128)
            builder.addRoute("::", 0)

            vpnInterface = builder.establish()

            // Pass the file descriptor to the Rust daemon via JNI
            val fd = vpnInterface?.fd ?: -1
            if (fd >= 0) {
                // nativeStartVpn(fd, server, port, coreId)
            }
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    private fun teardownVpn() {
        try {
            vpnInterface?.close()
            vpnInterface = null
            stopSelf()
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    override fun onDestroy() {
        teardownVpn()
        super.onDestroy()
    }

    // External function to call Rust daemon via JNI
    // private external fun nativeStartVpn(fd: Int, server: String, port: Int, coreId: String)

    companion object {
        init {
            // System.loadLibrary("unifiedshield_daemon")
        }
    }
}
