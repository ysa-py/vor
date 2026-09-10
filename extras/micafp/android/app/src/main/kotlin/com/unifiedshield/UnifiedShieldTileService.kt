package com.unifiedshield

import android.content.Intent
import android.graphics.drawable.Icon
import android.os.Build
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import androidx.annotation.RequiresApi

@RequiresApi(Build.VERSION_CODES.N)
class UnifiedShieldTileService : TileService() {

    override fun onStartListening() {
        super.onStartListening()
        updateTileState()
    }

    override fun onClick() {
        super.onClick()
        val tile = qsTile ?: return
        val isCurrentlyActive = (tile.state == Tile.STATE_ACTIVE)

        if (isCurrentlyActive) {
            // Stop VPN
            val intent = Intent(this, VpnService::class.java).apply {
                action = VpnService.ACTION_STOP
            }
            startService(intent)
            tile.state = Tile.STATE_INACTIVE
            tile.label = "UnifiedShield (خاموش)"
        } else {
            // Start VPN
            val intent = Intent(this, VpnService::class.java).apply {
                action = VpnService.ACTION_START
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                startForegroundService(intent)
            } else {
                startService(intent)
            }
            tile.state = Tile.STATE_ACTIVE
            tile.label = "UnifiedShield (متصل)"
        }
        tile.updateTile()
    }

    private fun updateTileState() {
        val tile = qsTile ?: return
        val isConnected = TunnelManager.getInstance(applicationContext).stats.value.connected
        tile.state = if (isConnected) Tile.STATE_ACTIVE else Tile.STATE_INACTIVE
        tile.label = if (isConnected) "UnifiedShield (متصل)" else "UnifiedShield"
        tile.updateTile()
    }
}
