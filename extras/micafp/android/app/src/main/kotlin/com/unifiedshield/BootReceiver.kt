package com.unifiedshield

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Build
import com.unifiedshield.profile.ProfileManager

class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action == Intent.ACTION_BOOT_COMPLETED) {
            val profileManager = ProfileManager.getInstance(context)
            if (profileManager.state.value.startOnBoot) {
                val serviceIntent = Intent(context, VpnService::class.java).apply {
                    action = VpnService.ACTION_START
                }
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    context.startForegroundService(serviceIntent)
                } else {
                    context.startService(serviceIntent)
                }
            }
        }
    }
}
