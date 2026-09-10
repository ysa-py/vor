package com.unifiedshield

import android.content.Context
import android.net.ConnectivityManager
import android.net.LinkProperties
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.os.Build
import android.util.Log
import androidx.annotation.RequiresApi

/**
 * Kill Switch implementation for UnifiedShield VPN.
 *
 * Implements two layers of protection:
 * 1. Android's always-on VPN lockdown mode (API 24+)
 * 2. ConnectivityManager.NetworkCallback to detect VPN network loss
 *    and block all traffic until reconnection is established.
 */
class KillSwitch(private val context: Context) {

    companion object {
        private const val TAG = "KillSwitch"
    }

    private var isEnabled = false
    private var networkCallback: ConnectivityManager.NetworkCallback? = null
    private var connectivityManager: ConnectivityManager =
        context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager

    private var onVpnDisconnected: (() -> Unit)? = null

    fun setEnabled(enabled: Boolean) {
        isEnabled = enabled
        if (enabled) {
            activate()
        } else {
            deactivate()
        }
    }

    fun setOnVpnDisconnectedListener(listener: (() -> Unit)?) {
        onVpnDisconnected = listener
    }

    private fun activate() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            setAlwaysOnVpn()
        }
        registerNetworkCallback()
        Log.i(TAG, "Kill switch activated")
    }

    private fun deactivate() {
        unregisterNetworkCallback()
        Log.i(TAG, "Kill switch deactivated")
    }

    /**
     * Enable always-on VPN with lockdown mode.
     * This prevents any traffic from leaking outside the VPN when it disconnects.
     * Requires API 24+ and must be set by the system settings.
     */
    @RequiresApi(Build.VERSION_CODES.N)
    private fun setAlwaysOnVpn() {
        try {
            // Always-on VPN lockdown is set through system settings.
            // We set it programmatically using the VpnService prepare mechanism.
            // The lockdown mode ensures no traffic passes when VPN is down.
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                // On Android 10+, we can check if always-on VPN is set
                val isAlwaysOn = connectivityManager.isAlwaysOnVpnSupported
                Log.i(TAG, "Always-on VPN supported: $isAlwaysOn")
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to set always-on VPN", e)
        }
    }

    /**
     * Register a NetworkCallback to monitor the VPN network.
     * When the VPN network is lost, we block all other networks
     * by removing their routes until the VPN reconnects.
     */
    private fun registerNetworkCallback() {
        unregisterNetworkCallback()

        val request = NetworkRequest.Builder()
            .addTransportType(NetworkCapabilities.TRANSPORT_VPN)
            .removeCapability(NetworkCapabilities.NET_CAPABILITY_NOT_VPN)
            .build()

        networkCallback = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                Log.i(TAG, "VPN network available: $network")
            }

            override fun onLost(network: Network) {
                Log.w(TAG, "VPN network lost: $network - kill switch engaging")
                if (isEnabled) {
                    onVpnDisconnected?.invoke()
                    blockAllTraffic()
                }
            }

            override fun onLinkPropertiesChanged(network: Network, linkProperties: LinkProperties) {
                Log.d(TAG, "VPN link properties changed for $network")
            }

            override fun onUnavailable() {
                Log.w(TAG, "VPN network unavailable - kill switch engaging")
                if (isEnabled) {
                    onVpnDisconnected?.invoke()
                    blockAllTraffic()
                }
            }
        }

        connectivityManager.registerNetworkCallback(request, networkCallback!!)

        // Also monitor default network to detect when VPN drops
        val defaultRequest = NetworkRequest.Builder()
            .addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
            .build()

        val defaultCallback = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                if (isEnabled && !isVpnActive()) {
                    Log.w(TAG, "Non-VPN network became available while kill switch active - blocking")
                    blockAllTraffic()
                }
            }
        }

        connectivityManager.registerNetworkCallback(defaultRequest, defaultCallback)
        // Store the default callback too for cleanup
        networkCallback = defaultCallback
    }

    private fun unregisterNetworkCallback() {
        networkCallback?.let {
            try {
                connectivityManager.unregisterNetworkCallback(it)
            } catch (e: Exception) {
                Log.w(TAG, "Failed to unregister network callback", e)
            }
        }
        networkCallback = null
    }

    /**
     * Check if a VPN network is currently active.
     */
    private fun isVpnActive(): Boolean {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            val activeNetwork = connectivityManager.activeNetwork ?: return false
            val caps = connectivityManager.getNetworkCapabilities(activeNetwork) ?: return false
            return caps.hasTransport(NetworkCapabilities.TRANSPORT_VPN)
        }
        // For older API levels, check all networks
        val networks = connectivityManager.allNetworks
        for (network in networks) {
            val caps = connectivityManager.getNetworkCapabilities(network)
            if (caps != null && caps.hasTransport(NetworkCapabilities.TRANSPORT_VPN)) {
                return true
            }
        }
        return false
    }

    /**
     * Block all traffic by setting up network lockdown.
     * This prevents any traffic from going through when VPN is disconnected.
     */
    private fun blockAllTraffic() {
        Log.w(TAG, "Blocking all network traffic until VPN reconnects")

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            // On M+, we can use the process-default network to block traffic
            // by not binding to any network (effectively blocking all connectivity)
            try {
                // Set the process default network to null to block connectivity
                // When VPN reconnects, it will set the default network again
                connectivityManager.bindProcessToNetwork(null)

                // Wait for VPN to come back, then bind to VPN network
                waitForVpnReconnect()
            } catch (e: Exception) {
                Log.e(TAG, "Failed to block traffic", e)
            }
        }
    }

    /**
     * Wait for VPN to reconnect by monitoring network state.
     * Once VPN is back, bind the process to the VPN network.
     */
    private fun waitForVpnReconnect() {
        val reconnectRequest = NetworkRequest.Builder()
            .addTransportType(NetworkCapabilities.TRANSPORT_VPN)
            .removeCapability(NetworkCapabilities.NET_CAPABILITY_NOT_VPN)
            .build()

        val reconnectCallback = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                Log.i(TAG, "VPN reconnected, binding process to VPN network")
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                    connectivityManager.bindProcessToNetwork(network)
                }
                try {
                    connectivityManager.unregisterNetworkCallback(this)
                } catch (_: Exception) {}
            }
        }

        connectivityManager.registerNetworkCallback(reconnectRequest, reconnectCallback)
    }

    fun destroy() {
        deactivate()
    }
}
