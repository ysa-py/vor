/*
 * MICAFP-UnifiedShield-6.0
 * QuickSettingsTile.kt — Quick Settings tile for one-tap VPN connect/disconnect
 *
 * Provides a Quick Settings tile that shows the current VPN connection state
 * and allows the user to toggle the connection with a single tap.
 *
 * Features:
 *   - Shows connected/disconnected state with appropriate icon
 *   - One-tap toggle: tap to connect, tap again to disconnect
 *   - Updates tile state based on VPN service status
 *   - Uses foreground service for tile operations
 *   - TILE_SERVICE permission
 *
 * No root required. Cloudflare is NOT used.
 */

package org.micafp.unifiedshield.ui

import android.annotation.SuppressLint
import android.content.BroadcastReceiver
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.ServiceConnection
import android.graphics.drawable.Icon
import android.os.Build
import android.os.IBinder
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import android.util.Log
import org.micafp.unifiedshield.vpn.ShieldVpnService

/**
 * Quick Settings tile for one-tap Shield VPN connect/disconnect.
 *
 * States:
 *   - STATE_INACTIVE: Shield is disconnected (tile greyed out)
 *   - STATE_ACTIVE: Shield is connected and protecting traffic (tile highlighted)
 *   - STATE_UNAVAILABLE: Shield cannot be started (e.g., no VPN permission)
 */
class QuickSettingsTile : TileService() {

    companion object {
        private const val TAG = "Shield/QuickTile"
    }

    // Connection to VPN service
    private var vpnService: ShieldVpnService? = null
    private var isBound = false

    // Service connection for binding to VPN service
    private val serviceConnection = object : ServiceConnection {
        override fun onServiceConnected(name: ComponentName, service: IBinder) {
            val binder = service as ShieldVpnService.ShieldVpnBinder
            vpnService = binder.getService()
            isBound = true
            updateTileState()
            Log.d(TAG, "Bound to VPN service")
        }

        override fun onServiceDisconnected(name: ComponentName) {
            vpnService = null
            isBound = false
            Log.d(TAG, "Unbound from VPN service")
        }
    }

    // Receiver for VPN status updates
    private val vpnStatusReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            if (intent.action == ShieldVpnService.ACTION_VPN_STATUS) {
                val status = intent.getStringExtra(ShieldVpnService.EXTRA_STATUS)
                Log.d(TAG, "VPN status update: $status")
                updateTileForStatus(status)
            }
        }
    }

    // ============================================================
    // Tile Service Lifecycle
    // ============================================================

    override fun onCreate() {
        super.onCreate()
        Log.d(TAG, "QuickSettingsTile created")
    }

    override fun onDestroy() {
        try {
            unregisterReceiver(vpnStatusReceiver)
        } catch (_: Exception) { }
        unbindVpnService()
        super.onDestroy()
        Log.d(TAG, "QuickSettingsTile destroyed")
    }

    override fun onTileAdded() {
        super.onTileAdded()
        Log.i(TAG, "Quick Settings tile added")
        updateTileState()
    }

    override fun onTileRemoved() {
        super.onTileRemoved()
        Log.i(TAG, "Quick Settings tile removed")
    }

    override fun onStartListening() {
        super.onStartListening()
        Log.d(TAG, "Tile start listening")

        // Register for VPN status updates
        val filter = IntentFilter(ShieldVpnService.ACTION_VPN_STATUS)
        registerReceiver(vpnStatusReceiver, filter)

        // Bind to VPN service
        bindVpnService()

        // Update tile immediately
        updateTileState()
    }

    override fun onStopListening() {
        super.onStopListening()
        Log.d(TAG, "Tile stop listening")

        try {
            unregisterReceiver(vpnStatusReceiver)
        } catch (_: Exception) { }

        unbindVpnService()
    }

    // ============================================================
    // Tile Click Handler
    // ============================================================

    @SuppressLint("StartActivityAndCollapseDeprecated")
    override fun onClick() {
        super.onClick()

        Log.i(TAG, "Quick Settings tile clicked")

        // Check if we need VPN permission first
        if (!hasVpnPermission()) {
            Log.w(TAG, "VPN permission not granted, requesting...")
            requestVpnPermission()
            return
        }

        val currentState = qsTile?.state ?: Tile.STATE_INACTIVE

        when (currentState) {
            Tile.STATE_INACTIVE -> {
                // Currently disconnected — connect
                Log.i(TAG, "Starting Shield VPN via tile")
                startShieldVpn()
            }
            Tile.STATE_ACTIVE -> {
                // Currently connected — disconnect
                Log.i(TAG, "Stopping Shield VPN via tile")
                stopShieldVpn()
            }
            Tile.STATE_UNAVAILABLE -> {
                // Unavailable — try to request permission
                requestVpnPermission()
            }
        }
    }

    // ============================================================
    // VPN Control
    // ============================================================

    /**
     * Start the Shield VPN service.
     */
    private fun startShieldVpn() {
        // Update tile to show connecting state
        updateTileForStatus(ShieldVpnService.STATUS_CONNECTING)

        // Start the VPN service as a foreground service
        val intent = Intent(this, ShieldVpnService::class.java).apply {
            action = ShieldVpnService.ACTION_START_VPN
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            startForegroundService(intent)
        } else {
            startService(intent)
        }

        // Also bind for status updates
        bindVpnService()
    }

    /**
     * Stop the Shield VPN service.
     */
    private fun stopShieldVpn() {
        // Update tile to show disconnecting state
        updateTileForStatus(ShieldVpnService.STATUS_DISCONNECTED)

        val intent = Intent(this, ShieldVpnService::class.java).apply {
            action = ShieldVpnService.ACTION_STOP_VPN
        }
        startService(intent)
    }

    // ============================================================
    // Service Binding
    // ============================================================

    private fun bindVpnService() {
        if (isBound) return

        try {
            val intent = Intent(this, ShieldVpnService::class.java).apply {
                action = "org.micafp.unifiedshield.VPN_BINDER"
            }
            isBound = bindService(intent, serviceConnection, Context.BIND_AUTO_CREATE)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to bind to VPN service", e)
        }
    }

    private fun unbindVpnService() {
        if (!isBound) return

        try {
            unbindService(serviceConnection)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to unbind from VPN service", e)
        }
        isBound = false
        vpnService = null
    }

    // ============================================================
    // Tile State Management
    // ============================================================

    /**
     * Update the tile state based on the VPN service's current status.
     */
    private fun updateTileState() {
        val status = if (isBound && vpnService != null) {
            vpnService?.getConnectionStatus() ?: ShieldVpnService.STATUS_DISCONNECTED
        } else {
            // If not bound, check based on whether VPN service is running
            ShieldVpnService.STATUS_DISCONNECTED
        }
        updateTileForStatus(status)
    }

    /**
     * Update the tile to reflect the given VPN status.
     */
    private fun updateTileForStatus(status: String?) {
        val tile = qsTile ?: return

        when (status) {
            ShieldVpnService.STATUS_CONNECTED -> {
                tile.state = Tile.STATE_ACTIVE
                tile.label = "Shield"
                tile.subtitle = "Connected"
                tile.icon = Icon.createWithResource(this, android.R.drawable.ic_lock_lock)
                tile.contentDescription = "Shield is active and protecting your connection"
            }
            ShieldVpnService.STATUS_CONNECTING -> {
                tile.state = Tile.STATE_ACTIVE
                tile.label = "Shield"
                tile.subtitle = "Connecting..."
                tile.icon = Icon.createWithResource(this, android.R.drawable.ic_partial_secure)
                tile.contentDescription = "Shield is connecting"
            }
            ShieldVpnService.STATUS_DISCONNECTED -> {
                tile.state = Tile.STATE_INACTIVE
                tile.label = "Shield"
                tile.subtitle = "Disconnected"
                tile.icon = Icon.createWithResource(this, android.R.drawable.ic_lock_idle_lock)
                tile.contentDescription = "Shield is disconnected. Tap to connect."
            }
            ShieldVpnService.STATUS_ERROR -> {
                tile.state = Tile.STATE_UNAVAILABLE
                tile.label = "Shield"
                tile.subtitle = "Error"
                tile.icon = Icon.createWithResource(this, android.R.drawable.ic_dialog_alert)
                tile.contentDescription = "Shield encountered an error"
            }
            else -> {
                tile.state = Tile.STATE_INACTIVE
                tile.label = "Shield"
                tile.subtitle = null
                tile.icon = Icon.createWithResource(this, android.R.drawable.ic_lock_idle_lock)
            }
        }

        tile.updateTile()
    }

    // ============================================================
    // VPN Permission Handling
    // ============================================================

    /**
     * Check if the user has granted VPN permission.
     */
    private fun hasVpnPermission(): Boolean {
        // VpnService.prepare() returns null if permission is already granted
        return android.net.VpnService.prepare(this) == null
    }

    /**
     * Request VPN permission by launching the system VPN consent dialog.
     */
    @SuppressLint("StartActivityAndCollapseDeprecated")
    private fun requestVpnPermission() {
        val intent = android.net.VpnService.prepare(this)
        if (intent != null) {
            // Need to show the VPN permission dialog
            // For TileService, we need to use startActivityAndCollapse
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                // Android 14+: Use startActivityAndCollapse with PendingIntent
                val pendingIntent = android.app.PendingIntent.getActivity(
                    this, 0, intent,
                    android.app.PendingIntent.FLAG_IMMUTABLE or android.app.PendingIntent.FLAG_UPDATE_CURRENT
                )
                startActivityAndCollapse(pendingIntent)
            } else {
                @Suppress("DEPRECATION")
                startActivityAndCollapse(intent)
            }
        }
    }
}
