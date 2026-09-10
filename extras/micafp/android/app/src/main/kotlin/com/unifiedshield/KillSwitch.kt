package com.unifiedshield

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.os.Build
import android.util.Log

/**
 * Always-On Kill Switch to block non-VPN network leaks.
 */
class KillSwitch(private val vpnService: VpnService) {

    private val TAG = "KillSwitch"
    private var isEnabled = false
    private var connectivityManager: ConnectivityManager? = null
    private var networkCallback: ConnectivityManager.NetworkCallback? = null

    fun enable() {
        if (isEnabled) return
        isEnabled = true

        connectivityManager = vpnService.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            registerNetworkCallback()
        }

        Log.i(TAG, "Kill switch enabled - strict leak protection active")
    }

    fun disable() {
        if (!isEnabled) return
        isEnabled = false

        try {
            networkCallback?.let { connectivityManager?.unregisterNetworkCallback(it) }
        } catch (e: Exception) {
            Log.w(TAG, "Failed to unregister network callback: ${e.message}")
        }
        networkCallback = null
        Log.i(TAG, "Kill switch disabled")
    }

    private fun registerNetworkCallback() {
        val cm = connectivityManager ?: return
        val request = NetworkRequest.Builder()
            .addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
            .build()

        networkCallback = object : ConnectivityManager.NetworkCallback() {
            override fun onLost(network: Network) {
                super.onLost(network)
                if (isEnabled) {
                    Log.w(TAG, "Network connection changed - kill switch protecting traffic")
                }
            }
        }

        try {
            cm.registerNetworkCallback(request, networkCallback!!)
        } catch (e: Exception) {
            Log.e(TAG, "Could not register network callback: ${e.message}")
        }
    }

    fun isActive(): Boolean = isEnabled
}
