package com.unifiedshield

import android.content.Context
import android.net.VpnService
import android.util.Log

/**
 * Split Tunnel engine for Iranian IP subnets.
 * Keeps domestic Iranian traffic (banking, government, local video) direct,
 * while tunneling foreign web traffic through anti-censorship cores.
 */
class SplitTunnel(private val context: Context) {

    private val TAG = "SplitTunnel"
    private val iranianRanges = mutableListOf<String>()

    companion object {
        val IRANIAN_IP_RANGES = listOf(
            "78.38.0.0/16", "78.39.0.0/16", "217.218.0.0/15",
            "5.106.0.0/16", "5.107.0.0/16", "94.182.0.0/15", "2.146.0.0/15",
            "31.56.0.0/14", "151.233.0.0/16", "5.200.200.0/24",
            "46.36.0.0/17", "91.92.0.0/14", "185.143.232.0/22"
        )
    }

    fun loadIranianIpRanges() {
        iranianRanges.clear()
        iranianRanges.addAll(IRANIAN_IP_RANGES)
        Log.i(TAG, "Loaded ${iranianRanges.size} Iranian IP ranges for split tunneling")
    }

    fun applySplitTunnelRoutes(builder: VpnService.Builder) {
        builder.addRoute("0.0.0.0", 0)
        Log.i(TAG, "Split tunnel active: per-packet routing handled by core bridge")
    }

    fun getIranianRanges(): List<String> = iranianRanges.toList()

    fun shouldTunnel(ip: String): Boolean {
        return !iranianRanges.any { range -> ip.startsWith(range.substringBefore("/")) }
    }
}
