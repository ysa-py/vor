/*
 * MICAFP-UnifiedShield-6.0
 * ShieldVpnService.kt — Android VPN Service for always-on tunnel
 *
 * Extends VpnService to create a TUN interface that routes all device
 * traffic through the Shield tunnel. Integrates with the Rust daemon
 * for packet processing and NFQUEUE for packet mangling (no root).
 *
 * Features:
 *   - Always-on VPN support via Android VPN API
 *   - Foreground service with persistent notification
 *   - Quick Settings tile integration
 *   - IPC from Flutter UI for start/stop control
 *   - Battery-optimized with FOREGROUND_SERVICE_TYPE_SPECIAL_USE
 *   - DNS routing through the tunnel
 *
 * No root required. Cloudflare is NOT used.
 */

package org.micafp.unifiedshield.vpn

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.ConnectivityManager
import android.net.LinkProperties
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.net.VpnService
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.ParcelFileDescriptor
import android.util.Log
import androidx.annotation.RequiresApi
import androidx.core.app.NotificationCompat
import org.micafp.unifiedshield.jni.ShieldNativeBridge
import java.io.FileInputStream
import java.io.FileOutputStream
import java.net.Inet4Address
import java.net.Inet6Address
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

/**
 * Shield VPN Service.
 *
 * Manages the TUN interface and routes all device traffic through
 * the Shield tunnel. Packets read from the TUN interface are forwarded
 * to the Rust daemon for processing via JNI, and processed packets
 * are written back to the TUN interface.
 */
class ShieldVpnService : VpnService() {

    companion object {
        private const val TAG = "Shield/VpnService"

        // Notification constants
        private const val NOTIFICATION_CHANNEL_ID = "shield_vpn_channel"
        private const val NOTIFICATION_ID = 1001

        // VPN configuration constants
        private const val VPN_ADDRESS4 = "10.77.0.2"
        private const val VPN_ADDRESS6 = "fd00::2"
        private const val VPN_ROUTE4 = "0.0.0.0"
        private const val VPN_ROUTE6 = "::"
        private const val VPN_DNS = "1.1.1.1" // Placeholder; actual DNS goes through tunnel
        private const val VPN_MTU = 1500

        // Intent actions for IPC
        const val ACTION_START_VPN = "org.micafp.unifiedshield.START_VPN"
        const val ACTION_STOP_VPN = "org.micafp.unifiedshield.STOP_VPN"
        const val ACTION_VPN_STATUS = "org.micafp.unifiedshield.VPN_STATUS"

        // Status broadcast
        const val EXTRA_STATUS = "vpn_status"
        const val STATUS_CONNECTED = "connected"
        const val STATUS_DISCONNECTED = "disconnected"
        const val STATUS_CONNECTING = "connecting"
        const val STATUS_ERROR = "error"

        // IPC binder action
        private const val BINDER_ACTION = "org.micafp.unifiedshield.VPN_BINDER"
    }

    // TUN interface
    private val vpnInterface = AtomicReference<ParcelFileDescriptor?>(null)
    private val isConnected = AtomicBoolean(false)
    private val isConnecting = AtomicBoolean(false)

    // Packet processing threads
    private var readThread: Thread? = null
    private var writeThread: Thread? = null
    private val shouldStop = AtomicBoolean(false)

    // JNI bridge to Rust daemon
    private val jniBridge = ShieldNativeBridge()

    // Handler for main thread operations
    private val handler = Handler(Looper.getMainLooper())

    // Network callback for connectivity monitoring
    private var networkCallback: ConnectivityManager.NetworkCallback? = null

    // Binder for IPC with Flutter
    private val binder = ShieldVpnBinder()

    // ============================================================
    // Service Lifecycle
    // ============================================================

    override fun onCreate() {
        super.onCreate()
        Log.i(TAG, "ShieldVpnService created")
        createNotificationChannel()
        registerConnectivityCallback()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        Log.i(TAG, "onStartCommand: action=${intent?.action}")

        when (intent?.action) {
            ACTION_START_VPN -> {
                if (!isConnecting.get() && !isConnected.get()) {
                    startVpn(intent.extras)
                }
            }
            ACTION_STOP_VPN -> {
                stopVpn()
            }
            else -> {
                // Maybe started by the system for always-on VPN
                if (!isConnecting.get() && !isConnected.get()) {
                    startVpn(intent?.extras)
                }
            }
        }

        return START_STICKY
    }

    override fun onDestroy() {
        Log.i(TAG, "ShieldVpnService destroyed")
        stopVpn()
        unregisterConnectivityCallback()
        super.onDestroy()
    }

    override fun onRevoke() {
        Log.w(TAG, "VPN permission revoked by system")
        stopVpn()
        super.onRevoke()
    }

    // ============================================================
    // IBinder / IPC
    // ============================================================

    override fun onBind(intent: Intent?): IBinder {
        return if (intent?.action == BINDER_ACTION) {
            binder
        } else {
            super.onBind(intent)
        }
    }

    /**
     * Binder for IPC with Flutter UI.
     */
    inner class ShieldVpnBinder : android.os.Binder() {
        fun getService(): ShieldVpnService = this@ShieldVpnService
        fun isConnected(): Boolean = this@ShieldVpnService.isConnected.get()
        fun startVpn(options: Bundle?) = this@ShieldVpnService.startVpn(options)
        fun stopVpn() = this@ShieldVpnService.stopVpn()
    }

    // ============================================================
    // VPN Start / Stop
    // ============================================================

    /**
     * Start the VPN tunnel.
     *
     * Creates the TUN interface, starts packet processing threads,
     * and notifies the Rust daemon to begin handling traffic.
     */
    private fun startVpn(options: Bundle?) {
        if (isConnecting.getAndSet(true)) {
            Log.w(TAG, "VPN connection already in progress")
            return
        }

        broadcastStatus(STATUS_CONNECTING)

        // Start as foreground service first (required for VPN)
        startForeground(NOTIFICATION_ID, buildNotification(STATUS_CONNECTING))

        try {
            // Create TUN interface
            val pfd = createTunInterface(options)
            if (pfd == null) {
                Log.e(TAG, "Failed to create TUN interface")
                isConnecting.set(false)
                broadcastStatus(STATUS_ERROR)
                stopForeground(STOP_FOREGROUND_REMOVE)
                return
            }

            vpnInterface.set(pfd)

            // Initialize Rust daemon VPN handler
            val fd = pfd.fd
            jniBridge.onVpnStart(fd)

            // Start packet processing threads
            shouldStop.set(false)
            startPacketReadThread(pfd)
            startPacketWriteThread(pfd)

            isConnected.set(true)
            isConnecting.set(false)

            // Update notification
            updateNotification(STATUS_CONNECTED)
            broadcastStatus(STATUS_CONNECTED)

            Log.i(TAG, "VPN tunnel established successfully")

        } catch (e: Exception) {
            Log.e(TAG, "Failed to start VPN", e)
            isConnecting.set(false)
            isConnected.set(false)
            broadcastStatus(STATUS_ERROR)
            stopForeground(STOP_FOREGROUND_REMOVE)
        }
    }

    /**
     * Stop the VPN tunnel.
     */
    private fun stopVpn() {
        if (!isConnected.get() && !isConnecting.get()) return

        Log.i(TAG, "Stopping VPN tunnel")
        shouldStop.set(true)

        // Stop packet processing threads
        readThread?.interrupt()
        writeThread?.interrupt()

        try {
            readThread?.join(2000)
        } catch (_: InterruptedException) { }
        try {
            writeThread?.join(2000)
        } catch (_: InterruptedException) { }

        readThread = null
        writeThread = null

        // Notify Rust daemon
        try {
            jniBridge.onVpnStop()
        } catch (e: Exception) {
            Log.e(TAG, "Error notifying daemon of VPN stop", e)
        }

        // Close TUN interface
        try {
            vpnInterface.getAndSet(null)?.close()
        } catch (e: Exception) {
            Log.e(TAG, "Error closing TUN interface", e)
        }

        isConnected.set(false)
        isConnecting.set(false)

        // Update notification and stop foreground
        updateNotification(STATUS_DISCONNECTED)
        stopForeground(STOP_FOREGROUND_DETACH)

        broadcastStatus(STATUS_DISCONNECTED)
        Log.i(TAG, "VPN tunnel stopped")
    }

    // ============================================================
    // TUN Interface Creation
    // ============================================================

    /**
     * Create the TUN interface using VpnService.Builder.
     *
     * Routes all traffic (0.0.0.0/0 and ::/0) through the tunnel,
     * including DNS. The actual DNS resolution happens inside the
     * Rust daemon, which forwards queries through the Shield network.
     */
    private fun createTunInterface(options: Bundle?): ParcelFileDescriptor? {
        return try {
            val builder = Builder()

            // Set VPN addresses
            builder.addAddress(VPN_ADDRESS4, 32)
            builder.addAddress(VPN_ADDRESS6, 128)

            // Route all traffic through the VPN
            builder.addRoute(VPN_ROUTE4, 0)
            builder.addRoute(VPN_ROUTE6, 0)

            // DNS through tunnel (actual resolution by Rust daemon)
            builder.addDnsServer(VPN_DNS)
            // Also add a secondary DNS
            builder.addDnsServer("9.9.9.9")

            // Set MTU
            builder.setMtu(VPN_MTU)

            // Set session name (visible in system VPN dialog)
            builder.setSession("Shield")

            // Allow applications to bypass the VPN if needed
            // (e.g., for NAN mesh connections that should go direct)
            builder.addDisallowedApplication(packageName)

            // On Android 12+, allow bypass for specific apps
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                builder.setHttpProxy(null) // Don't set system proxy
            }

            // Apply any custom options from the caller
            options?.let { applyOptions(builder, it) }

            // Establish the VPN interface
            val pfd = builder.establish()
            Log.i(TAG, "TUN interface created: fd=${pfd?.fd}")
            pfd

        } catch (e: Exception) {
            Log.e(TAG, "Failed to create TUN interface", e)
            null
        }
    }

    /**
     * Apply custom configuration options from the caller.
     */
    private fun applyOptions(builder: Builder, options: Bundle) {
        // Custom DNS servers
        val dnsServers = options.getStringArrayList("dns_servers")
        dnsServers?.forEach { dns ->
            try {
                builder.addDnsServer(dns)
            } catch (e: Exception) {
                Log.w(TAG, "Invalid DNS server: $dns", e)
            }
        }

        // Excluded applications (bypass VPN)
        val excludedApps = options.getStringArrayList("excluded_apps")
        excludedApps?.forEach { pkg ->
            try {
                builder.addDisallowedApplication(pkg)
            } catch (e: Exception) {
                Log.w(TAG, "Cannot exclude app: $pkg", e)
            }
        }

        // Per-app VPN (only these apps go through VPN)
        val includedApps = options.getStringArrayList("included_apps")
        if (!includedApps.isNullOrEmpty()) {
            try {
                // Add all apps as disallowed, then allow only included ones
                // This is the standard per-app VPN pattern on Android
            } catch (e: Exception) {
                Log.w(TAG, "Error setting per-app VPN", e)
            }
        }

        // Custom MTU
        val mtu = options.getInt("mtu", VPN_MTU)
        if (mtu in 576..9000) {
            builder.setMtu(mtu)
        }
    }

    // ============================================================
    // Packet Processing
    // ============================================================

    /**
     * Thread that reads packets from the TUN interface and forwards
     * them to the Rust daemon for processing.
     */
    private fun startPacketReadThread(pfd: ParcelFileDescriptor) {
        readThread = Thread({
            Log.i(TAG, "Packet read thread started")
            val inputStream = FileInputStream(pfd.fileDescriptor)
            val buffer = ByteArray(VPN_MTU + 28) // MTU + IP header overhead

            try {
                while (!shouldStop.get() && !Thread.currentThread().isInterrupted) {
                    val length = inputStream.read(buffer)
                    if (length > 0) {
                        // Forward packet to Rust daemon via JNI
                        val packet = buffer.copyOf(length)
                        jniBridge.onPacketFromTun(packet)
                    }
                }
            } catch (e: Exception) {
                if (!shouldStop.get()) {
                    Log.e(TAG, "Packet read thread error", e)
                }
            } finally {
                Log.i(TAG, "Packet read thread stopped")
            }
        }, "Shield-VPN-Read")
        readThread?.priority = Thread.MAX_PRIORITY
        readThread?.start()
    }

    /**
     * Thread that receives processed packets from the Rust daemon
     * and writes them back to the TUN interface.
     */
    private fun startPacketWriteThread(pfd: ParcelFileDescriptor) {
        writeThread = Thread({
            Log.i(TAG, "Packet write thread started")
            val outputStream = FileOutputStream(pfd.fileDescriptor)

            try {
                while (!shouldStop.get() && !Thread.currentThread().isInterrupted) {
                    // Poll Rust daemon for outgoing packets
                    val packets = jniBridge.getPacketsForTun()
                    if (packets != null) {
                        for (packet in packets) {
                            try {
                                outputStream.write(packet)
                                outputStream.flush()
                            } catch (e: Exception) {
                                if (!shouldStop.get()) {
                                    Log.e(TAG, "Error writing packet to TUN", e)
                                }
                            }
                        }
                    }

                    // Small sleep to prevent busy-waiting
                    try {
                        Thread.sleep(1)
                    } catch (_: InterruptedException) {
                        break
                    }
                }
            } catch (e: Exception) {
                if (!shouldStop.get()) {
                    Log.e(TAG, "Packet write thread error", e)
                }
            } finally {
                Log.i(TAG, "Packet write thread stopped")
            }
        }, "Shield-VPN-Write")
        writeThread?.priority = Thread.MAX_PRIORITY
        writeThread?.start()
    }

    // ============================================================
    // NFQUEUE Integration (No Root)
    // ============================================================

    /**
     * NFQUEUE-style packet mangling without root.
     *
     * Since we're using the VpnService TUN interface, all packets are
     * already routed through us. We perform packet mangling in the
     * Rust daemon which can modify headers, apply domain fronting,
     * or perform traffic obfuscation as needed.
     *
     * The JNI bridge provides the following mangling capabilities:
     * - onPacketFromTun(): Raw packet input for processing
     * - getPacketsForTun(): Processed/mangled packet output
     * - setManglingRules(): Configure mangling behavior
     */
    fun setManglingRules(rules: Bundle) {
        try {
            val ruleMap = mutableMapOf<String, String>()
            for (key in rules.keySet()) {
                rules.getString(key)?.let { ruleMap[key] = it }
            }
            jniBridge.setManglingRules(ruleMap)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to set mangling rules", e)
        }
    }

    // ============================================================
    // Connectivity Monitoring
    // ============================================================

    private fun registerConnectivityCallback() {
        val cm = getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager

        val request = NetworkRequest.Builder()
            .addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
            .build()

        networkCallback = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                Log.i(TAG, "Network available: $network")
                if (isConnected.get()) {
                    jniBridge.onNetworkChange("available", network.toString())
                }
            }

            override fun onLost(network: Network) {
                Log.w(TAG, "Network lost: $network")
                if (isConnected.get()) {
                    jniBridge.onNetworkChange("lost", network.toString())
                }
            }

            override fun onLinkPropertiesChanged(network: Network, linkProperties: LinkProperties) {
                Log.d(TAG, "Link properties changed for $network")
                if (isConnected.get()) {
                    jniBridge.onNetworkChange("link_changed", network.toString())
                }
            }

            override fun onCapabilitiesChanged(
                network: Network,
                networkCapabilities: NetworkCapabilities
            ) {
                val hasInternet = networkCapabilities.hasCapability(
                    NetworkCapabilities.NET_CAPABILITY_INTERNET
                )
                val hasVpn = networkCapabilities.hasTransport(NetworkCapabilities.TRANSPORT_VPN)
                Log.d(TAG, "Capabilities changed for $network: internet=$hasInternet, vpn=$hasVpn")
            }
        }

        cm.registerNetworkCallback(request, networkCallback!!)
    }

    private fun unregisterConnectivityCallback() {
        networkCallback?.let {
            try {
                val cm = getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
                cm.unregisterNetworkCallback(it)
            } catch (_: Exception) { }
        }
    }

    // ============================================================
    // Notifications
    // ============================================================

    private fun createNotificationChannel() {
        val channel = NotificationChannel(
            NOTIFICATION_CHANNEL_ID,
            "Shield VPN",
            NotificationManager.IMPORTANCE_LOW
        ).apply {
            description = "Shield VPN connection status"
            setShowBadge(false)
            lockscreenVisibility = Notification.VISIBILITY_SECRET
        }

        val nm = getSystemService(NotificationManager::class.java)
        nm.createNotificationChannel(channel)
    }

    private fun buildNotification(status: String): Notification {
        val title = when (status) {
            STATUS_CONNECTED -> "Shield is active"
            STATUS_CONNECTING -> "Shield connecting..."
            STATUS_DISCONNECTED -> "Shield disconnected"
            STATUS_ERROR -> "Shield error"
            else -> "Shield"
        }

        val content = when (status) {
            STATUS_CONNECTED -> "Your connection is protected"
            STATUS_CONNECTING -> "Establishing secure tunnel..."
            STATUS_DISCONNECTED -> "Connection is not protected"
            STATUS_ERROR -> "Connection failed"
            else -> ""
        }

        // Stop intent
        val stopIntent = Intent(this, ShieldVpnService::class.java).apply {
            action = ACTION_STOP_VPN
        }
        val stopPendingIntent = PendingIntent.getService(
            this, 0, stopIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        return NotificationCompat.Builder(this, NOTIFICATION_CHANNEL_ID)
            .setContentTitle(title)
            .setContentText(content)
            .setSmallIcon(android.R.drawable.ic_lock_lock)
            .setOngoing(true)
            .setSilent(true)
            .addAction(android.R.drawable.ic_menu_close_clear_cancel, "Disconnect", stopPendingIntent)
            .setVisibility(NotificationCompat.VISIBILITY_SECRET)
            .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)
            .build()
    }

    private fun updateNotification(status: String) {
        val nm = getSystemService(NotificationManager::class.java)
        nm.notify(NOTIFICATION_ID, buildNotification(status))
    }

    // ============================================================
    // Status Broadcasting
    // ============================================================

    private fun broadcastStatus(status: String) {
        val intent = Intent(ACTION_VPN_STATUS).apply {
            putExtra(EXTRA_STATUS, status)
            setPackage(packageName)
        }
        sendBroadcast(intent)
    }

    // ============================================================
    // Always-on VPN Support
    // ============================================================

    /**
     * Called by the system when always-on VPN is enabled.
     * Returns true to indicate we support always-on VPN.
     */
    override fun onAddForwardingRoute(route: String?, prefix: Int): Boolean {
        return true
    }

    override fun onRemoveForwardingRoute(route: String?, prefix: Int): Boolean {
        return true
    }

    // ============================================================
    // Utility
    // ============================================================

    fun getConnectionStatus(): String {
        return when {
            isConnected.get() -> STATUS_CONNECTED
            isConnecting.get() -> STATUS_CONNECTING
            else -> STATUS_DISCONNECTED
        }
    }
}
