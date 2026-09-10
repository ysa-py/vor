package com.shield.vpn

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import android.util.Log
import java.io.FileInputStream
import java.io.FileOutputStream
import java.net.DatagramSocket
import java.net.Socket
import kotlin.concurrent.thread

/**
 * ShieldVpnService — Android VPN Service
 *
 * Implements a full-tunnel VPN using Android's VpnService API. Routes all device
 * traffic through the TUN interface, protects sockets from routing loops, supports
 * always-on VPN, provides a Quick Settings Tile for one-tap connect/disconnect,
 * and integrates with the Rust daemon via JNI for all tunnel/encryption logic.
 */
class ShieldVpnService : VpnService() {

    companion object {
        private const val TAG = "ShieldVpnService"

        // VPN configuration
        private const val VPN_ADDRESS = "10.0.0.2"
        private const val VPN_ROUTE = "0.0.0.0"
        private const val VPN_ROUTE_PREFIX = 0
        private const val VPN_DNS = "1.1.1.1"
        private const val VPN_DNS_SECONDARY = "1.0.0.1"
        private const val VPN_MTU = 1500
        private const val VPN_ADDRESS_V6 = "fd00::2"
        private const val VPN_ROUTE_V6 = "::"
        private const val VPN_ROUTE_PREFIX_V6 = 0

        // Notification
        private const val NOTIFICATION_CHANNEL_ID = "shield_vpn_channel"
        private const val NOTIFICATION_ID = 3001

        // Intent actions
        const val ACTION_CONNECT = "com.shield.vpn.CONNECT"
        const val ACTION_DISCONNECT = "com.shield.vpn.DISCONNECT"

        // Service state
        @Volatile
        var isConnected: Boolean = false
            private set

        // JNI native methods for Rust daemon communication
        init {
            System.loadLibrary("shield_native")
        }

        @JvmStatic
        external fun nativeStartVpn(tunFd: Int): Int
        @JvmStatic
        external fun nativeStopVpn()
        @JvmStatic
        external fun nativeProtectSocket(fd: Int): Boolean
        @JvmStatic
        external fun nativeGetConnectionStatus(): Int  // 0=disconnected, 1=connecting, 2=connected
        @JvmStatic
        external fun nativeGetTrafficStats(): LongArray // [bytesIn, bytesOut]
        @JvmStatic
        external fun nativeGetCurrentTransport(): String
    }

    private var vpnInterface: ParcelFileDescriptor? = null
    private var tunInputThread: Thread? = null
    private var tunOutputThread: Thread? = null
    private var isRunning = false
    private var configEndpoint: String = ""
    private var configTransport: String = ""

    // ---------------------------------------------------------------
    // Service Lifecycle
    // ---------------------------------------------------------------

    override fun onCreate() {
        super.onCreate()
        Log.d(TAG, "ShieldVpnService created")
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_CONNECT -> {
                configEndpoint = intent.getStringExtra("endpoint") ?: ""
                configTransport = intent.getStringExtra("transport") ?: ""
                connect()
            }
            ACTION_DISCONNECT -> {
                disconnect()
            }
            else -> {
                // Auto-connect on service start (always-on VPN)
                if (!isRunning) {
                    connect()
                }
            }
        }
        return START_STICKY
    }

    override fun onDestroy() {
        disconnect()
        super.onDestroy()
        Log.d(TAG, "ShieldVpnService destroyed")
    }

    override fun onRevoke() {
        // Called when the user revokes VPN permission
        Log.w(TAG, "VPN permission revoked by user")
        disconnect()
    }

    // ---------------------------------------------------------------
    // VPN Connection
    // ---------------------------------------------------------------

    private fun connect() {
        if (isRunning) {
            Log.d(TAG, "VPN already running, skipping connect")
            return
        }

        Log.d(TAG, "Establishing VPN tunnel...")
        isConnected = false

        // Build the TUN interface
        val builder = Builder()
        configureTunInterface(builder)

        try {
            vpnInterface = builder.establish()
            if (vpnInterface == null) {
                Log.e(TAG, "Failed to establish TUN interface (user may have denied permission)")
                stopSelf()
                return
            }

            isRunning = true
            isConnected = true

            // Start foreground service with notification
            val notification = buildNotification("Connecting...")
            startForeground(NOTIFICATION_ID, notification)

            // Pass TUN file descriptor to Rust daemon
            val tunFd = vpnInterface!!.fd
            val result = nativeStartVpn(tunFd)
            if (result != 0) {
                Log.e(TAG, "Rust daemon failed to start VPN tunnel (error: $result)")
                disconnect()
                return
            }

            // Start TUN I/O threads
            startTunThreads()

            // Update notification
            updateNotification("Connected — Shield Active")
            notifyTileState(true)

            Log.d(TAG, "VPN tunnel established successfully")

        } catch (e: Exception) {
            Log.e(TAG, "Failed to establish VPN tunnel: ${e.message}")
            disconnect()
        }
    }

    private fun disconnect() {
        if (!isRunning) return

        Log.d(TAG, "Disconnecting VPN tunnel...")
        isRunning = false
        isConnected = false

        // Signal Rust daemon to stop
        try {
            nativeStopVpn()
        } catch (e: Exception) {
            Log.e(TAG, "Error stopping Rust VPN daemon: ${e.message}")
        }

        // Interrupt I/O threads
        tunInputThread?.interrupt()
        tunOutputThread?.interrupt()
        tunInputThread = null
        tunOutputThread = null

        // Close TUN interface
        try {
            vpnInterface?.close()
        } catch (e: Exception) {
            Log.e(TAG, "Error closing TUN interface: ${e.message}")
        }
        vpnInterface = null

        // Stop foreground service
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()

        // Update Quick Settings Tile
        notifyTileState(false)

        Log.d(TAG, "VPN tunnel disconnected")
    }

    // ---------------------------------------------------------------
    // TUN Interface Configuration
    // ---------------------------------------------------------------

    private fun configureTunInterface(builder: Builder) {
        builder.apply {
            // Session name
            setSession("Shield")

            // MTU
            setMtu(VPN_MTU)

            // IPv4 address and route (all traffic)
            addAddress(VPN_ADDRESS, 32)
            addRoute(VPN_ROUTE, VPN_ROUTE_PREFIX)

            // IPv6 address and route
            addAddress(VPN_ADDRESS_V6, 128)
            addRoute(VPN_ROUTE_V6, VPN_ROUTE_PREFIX_V6)

            // DNS servers
            addDnsServer(VPN_DNS)
            addDnsServer(VPN_DNS_SECONDARY)

            // DNS search domain
            addSearchDomain("local")

            // Allow application to bypass VPN (for testing)
            addDisallowedApplication(packageName)

            // Set underlying network if available
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP_MR1) {
                // Don't set underlying network to avoid routing loops
                // The Rust daemon will handle network selection
            }
        }

        Log.d(TAG, "TUN interface configured: $VPN_ADDRESS/$VPN_MTU, routes: $VPN_ROUTE/$VPN_ROUTE_PREFIX")
    }

    // ---------------------------------------------------------------
    // TUN I/O Threads
    // ---------------------------------------------------------------

    private fun startTunThreads() {
        val pfd = vpnInterface ?: return

        // Input thread: reads packets from TUN → Rust daemon
        tunInputThread = thread(name = "tun-input") {
            val inputStream = FileInputStream(pfd.fileDescriptor)
            val buffer = ByteArray(VPN_MTU + 28) // MTU + header overhead

            try {
                while (!Thread.currentThread().isInterrupted && isRunning) {
                    val length = inputStream.read(buffer)
                    if (length > 0) {
                        // Packet read from TUN — Rust daemon processes it via the fd
                        // The native side reads directly from the fd, so this is a fallback
                    }
                }
            } catch (e: java.io.IOException) {
                if (isRunning) {
                    Log.e(TAG, "TUN input stream error: ${e.message}")
                }
            } catch (e: InterruptedException) {
                // Normal shutdown
            }
        }

        // Output thread: writes packets from Rust daemon → TUN
        tunOutputThread = thread(name = "tun-output") {
            val outputStream = FileOutputStream(pfd.fileDescriptor)

            try {
                while (!Thread.currentThread().isInterrupted && isRunning) {
                    // The Rust daemon writes directly to the fd
                    // This thread can be used for monitoring or fallback
                    Thread.sleep(100)
                }
            } catch (e: java.io.IOException) {
                if (isRunning) {
                    Log.e(TAG, "TUN output stream error: ${e.message}")
                }
            } catch (e: InterruptedException) {
                // Normal shutdown
            }
        }

        Log.d(TAG, "TUN I/O threads started")
    }

    // ---------------------------------------------------------------
    // Socket Protection
    // ---------------------------------------------------------------

    /**
     * Protect a socket from the VPN routing loop.
     * This prevents packets sent through this socket from being routed
     * back through the VPN tunnel.
     */
    override fun protect(socket: Socket): Boolean {
        val result = super.protect(socket)
        if (result) {
            Log.d(TAG, "TCP socket protected from VPN routing loop")
        } else {
            Log.w(TAG, "Failed to protect TCP socket")
        }
        return result
    }

    override fun protect(socket: DatagramSocket): Boolean {
        val result = super.protect(socket)
        if (result) {
            Log.d(TAG, "UDP socket protected from VPN routing loop")
        } else {
            Log.w(TAG, "Failed to protect UDP socket")
        }
        return result
    }

    /**
     * Protect a socket file descriptor via JNI.
     * The Rust daemon calls this through the JNI bridge to protect its sockets.
     */
    fun protectSocketFd(fd: Int): Boolean {
        return try {
            nativeProtectSocket(fd)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to protect socket fd $fd: ${e.message}")
            false
        }
    }

    // ---------------------------------------------------------------
    // Always-on VPN Support
    // ---------------------------------------------------------------

    /**
     * Called by the system when always-on VPN is enabled and the service
     * is expected to maintain the connection.
     */
    override fun onTimeout(timeout: Int) {
        Log.w(TAG, "VPN onTimeout called with timeout=$timeout")
        // Reconnect the VPN
        disconnect()
        connect()
    }

    // ---------------------------------------------------------------
    // Notifications
    // ---------------------------------------------------------------

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                NOTIFICATION_CHANNEL_ID,
                "Shield VPN Service",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "VPN tunnel status"
                setShowBadge(false)
                lockscreenVisibility = Notification.VISIBILITY_SECRET
            }

            val nm = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            nm.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(text: String): Notification {
        val pendingIntent = PendingIntent.getActivity(
            this, 0,
            packageManager.getLaunchIntentForPackage(packageName),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val disconnectIntent = PendingIntent.getService(
            this, 0,
            Intent(this, ShieldVpnService::class.java).apply {
                action = ACTION_DISCONNECT
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        return androidx.core.app.NotificationCompat.Builder(this, NOTIFICATION_CHANNEL_ID)
            .setContentTitle("Shield")
            .setContentText(text)
            .setSmallIcon(android.R.drawable.ic_lock_lock)
            .setContentIntent(pendingIntent)
            .addAction(
                android.R.drawable.ic_menu_close_clear_cancel,
                "Disconnect",
                disconnectIntent
            )
            .setPriority(androidx.core.app.NotificationCompat.PRIORITY_LOW)
            .setOngoing(true)
            .setShowWhen(false)
            .setVisibility(androidx.core.app.NotificationCompat.VISIBILITY_SECRET)
            .build()
    }

    private fun updateNotification(text: String) {
        val nm = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        nm.notify(NOTIFICATION_ID, buildNotification(text))
    }

    // ---------------------------------------------------------------
    // Quick Settings Tile
    // ---------------------------------------------------------------

    private fun notifyTileState(connected: Boolean) {
        // Trigger tile update via broadcast
        val intent = Intent("com.shield.vpn.TILE_UPDATE").apply {
            putExtra("connected", connected)
        }
        sendBroadcast(intent)
    }

    // ---------------------------------------------------------------
    // Public API
    // ---------------------------------------------------------------

    fun getConnectionStatus(): Int = nativeGetConnectionStatus()

    fun getTrafficStats(): LongArray = nativeGetTrafficStats()

    fun getCurrentTransport(): String {
        return try {
            nativeGetCurrentTransport()
        } catch (e: Exception) {
            "unknown"
        }
    }
}

/**
 * ShieldTileService — Quick Settings Tile for one-tap VPN connect/disconnect.
 */
class ShieldTileService : TileService() {

    companion object {
        private const val TAG = "ShieldTileService"
    }

    override fun onStartListening() {
        super.onStartListening()
        updateTileState()
    }

    override fun onStopListening() {
        super.onStopListening()
    }

    override fun onClick() {
        super.onClick()

        val tile = qsTile ?: return

        if (ShieldVpnService.isConnected) {
            // Disconnect
            val intent = Intent(this, ShieldVpnService::class.java).apply {
                action = ShieldVpnService.ACTION_DISCONNECT
            }
            startService(intent)
            tile.state = Tile.STATE_INACTIVE
        } else {
            // Connect
            val intent = Intent(this, ShieldVpnService::class.java).apply {
                action = ShieldVpnService.ACTION_CONNECT
            }
            startService(intent)
            tile.state = Tile.STATE_ACTIVE
        }

        tile.updateTile()
    }

    private fun updateTileState() {
        val tile = qsTile ?: return
        tile.state = if (ShieldVpnService.isConnected) Tile.STATE_ACTIVE else Tile.STATE_INACTIVE
        tile.label = "Shield"
        tile.updateTile()
    }
}
