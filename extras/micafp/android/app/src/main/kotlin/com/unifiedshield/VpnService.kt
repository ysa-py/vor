package com.unifiedshield

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.net.VpnService as AndroidVpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import androidx.core.app.NotificationCompat
import androidx.lifecycle.LifecycleService
import kotlinx.coroutines.*

class VpnService : AndroidVpnService() {

    private var vpnInterface: ParcelFileDescriptor? = null
    private var tunnelJob: Job? = null
    private val serviceScope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private val coreBridge = CoreBridge()
    private val splitTunnel by lazy { SplitTunnel(this) }
    private val killSwitch by lazy { KillSwitch(this) }
    private val dpiDetector = DpiDetector()
    private val ispDetector by lazy { IspDetector(this) }

    private var currentCore = "xray"
    private var isRunning = false

    companion object {
        const val ACTION_START = "com.unifiedshield.START"
        const val ACTION_STOP = "com.unifiedshield.STOP"
        const val NOTIFICATION_CHANNEL_ID = "unifiedshield_vpn"
        const val NOTIFICATION_CHANNEL_CRITICAL_ID = "unifiedshield_critical_alerts"
        const val NOTIFICATION_ID = 1
        const val CRITICAL_NOTIFICATION_ID = 999
        const val TUN_MTU = 1380
        const val TUN_ADDRESS = "172.19.0.1"
        const val TUN_PREFIX = 24
        const val UNIX_SOCKET = "unifiedshield-tun"
    }

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        super.onStartCommand(intent, flags, startId)

        when (intent?.action) {
            ACTION_START -> startVpn()
            ACTION_STOP -> stopVpn()
        }

        return START_STICKY
    }

    private fun startVpn() {
        if (isRunning) return

        // Explicit permission check before starting background operations
        val prepareIntent = AndroidVpnService.prepare(this)
        if (prepareIntent != null) {
            android.util.Log.w("VpnService", "VPN permission not granted by user. Aborting start.")
            stopSelf()
            return
        }

        val notification = buildNotification("Connecting to anti-censorship engine...")
        startForeground(NOTIFICATION_ID, notification)

        serviceScope.launch(Dispatchers.IO) {
            try {
                val detectionResult = ispDetector.detect()
                splitTunnel.loadIranianIpRanges()

                val builder = Builder()
                    .setMtu(TUN_MTU)
                    .addAddress(TUN_ADDRESS, TUN_PREFIX)
                    .addRoute("0.0.0.0", 0)
                    .addRoute("::", 0)
                    .setSession("UnifiedShield")
                    .setBlocking(true)

                runCatching { builder.addDisallowedApplication(packageName) }

                splitTunnel.applySplitTunnelRoutes(builder)

                // Chinese CDN Primary DNS servers
                builder.addDnsServer("223.5.5.5")      // Alibaba DNS
                builder.addDnsServer("119.29.29.29")    // Tencent DNS
                builder.addDnsServer("1.12.12.12")      // Tencent DNS Backup

                val pfd = builder.establish()
                vpnInterface = pfd

                if (pfd != null && pfd.fd > 0) {
                    val nativeTunFd = pfd.fd
                    coreBridge.startDaemonSafe(
                        tunFd = nativeTunFd,
                        core = currentCore,
                        unixSocket = UNIX_SOCKET,
                        isp = detectionResult.isp.code
                    )

                    killSwitch.enable()

                    dpiDetector.startMonitoring { score ->
                        if (score > 0.72) {
                            runCatching { coreBridge.triggerObfuscationMode() }
                            currentCore = if (currentCore == "xray") "naive" else "xray"
                            coreBridge.switchCoreSafe(currentCore)
                        }
                    }

                    isRunning = true
                    TunnelManager.getInstance(applicationContext).updateConnectionState(
                        connected = true,
                        core = currentCore,
                        isp = detectionResult.isp.name
                    )
                    updateNotification("Protected via $currentCore (${detectionResult.isp.name})")
                } else {
                    android.util.Log.e("VpnService", "Failed to establish VPN interface TUN descriptor.")
                    stopSelf()
                }
            } catch (e: Exception) {
                android.util.Log.e("VpnService", "Error during VPN start: ${e.message}", e)
                updateNotification("Error: ${e.message}")
                stopVpn()
            }
        }
    }

    private fun stopVpn() {
        if (!isRunning) return

        runCatching { dpiDetector.stopMonitoring() }
        runCatching { coreBridge.stopDaemonSafe() }
        runCatching { killSwitch.disable() }
        runCatching { vpnInterface?.close() }

        vpnInterface = null
        isRunning = false
        tunnelJob?.cancel()

        TunnelManager.getInstance(applicationContext).updateConnectionState(connected = false)

        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    override fun onDestroy() {
        stopVpn()
        serviceScope.cancel()
        super.onDestroy()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val manager = getSystemService(NotificationManager::class.java)

            val channel = NotificationChannel(
                NOTIFICATION_CHANNEL_ID,
                "UnifiedShield VPN",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Anti-censorship connection status"
                setShowBadge(false)
            }
            manager.createNotificationChannel(channel)

            // High Priority Always-On Emergency Channel for Iran Blackout / ISP Disruptions
            val criticalChannel = NotificationChannel(
                NOTIFICATION_CHANNEL_CRITICAL_ID,
                "Critical Censorship & Blackout Alerts",
                NotificationManager.IMPORTANCE_HIGH
            ).apply {
                description = "High-priority warnings for severe ISP filtering or national blackout"
                enableVibration(true)
                setShowBadge(true)
            }
            manager.createNotificationChannel(criticalChannel)
        }
    }

    fun showCriticalEmergencyAlert(title: String, message: String) {
        val pendingIntent = PendingIntent.getActivity(
            this, 0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val builder = NotificationCompat.Builder(this, NOTIFICATION_CHANNEL_CRITICAL_ID)
            .setContentTitle("⚠️ $title")
            .setContentText(message)
            .setStyle(NotificationCompat.BigTextStyle().bigText(message))
            .setSmallIcon(android.R.drawable.ic_dialog_alert)
            .setPriority(NotificationCompat.PRIORITY_MAX)
            .setCategory(NotificationCompat.CATEGORY_ALARM)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .setDefaults(NotificationCompat.DEFAULT_ALL)

        val manager = getSystemService(NotificationManager::class.java)
        manager.notify(CRITICAL_NOTIFICATION_ID, builder.build())
    }

    private fun buildNotification(text: String): Notification {
        val pendingIntent = PendingIntent.getActivity(
            this, 0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE
        )

        return NotificationCompat.Builder(this, NOTIFICATION_CHANNEL_ID)
            .setContentTitle("UnifiedShield")
            .setContentText(text)
            .setSmallIcon(android.R.drawable.ic_lock_lock)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .build()
    }

    private fun updateNotification(text: String) {
        val notification = buildNotification(text)
        val manager = getSystemService(NotificationManager::class.java)
        manager.notify(NOTIFICATION_ID, notification)
    }
}
