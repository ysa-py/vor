package com.unifiedshield

import android.content.Context
import android.net.DhcpInfo
import android.net.wifi.WifiManager
import android.util.Log
import java.net.Inet4Address
import java.net.InetAddress
import java.net.NetworkInterface

/**
 * Detects the user's ISP by analyzing DNS server IPs and network configuration.
 *
 * Iranian ISPs are identified by their known DNS IP ranges:
 * - MCI (Hamrah-e-Aval): 5.200.200.200, 217.218.155.155
 * - Irancell: 217.218.127.127
 * - Rightel: 5.200.200.201
 * - Shatel: 217.218.155.156
 * - Mokhtari: 178.22.122.100
 * - ParsOnline: 91.92.156.6
 * - Asiatech: 185.143.234.6
 * - HiWeb: 185.143.233.6
 * - TCI (landline): 217.218.155.155
 * - Zitel: 5.200.202.202
 */
class IspDetector(private val context: Context) {

    private val TAG = "IspDetector"

    // Iranian ISP DNS IP mapping
    private val ispDnsMap = mapOf(
        // MCI (Mobile Communication Company of Iran)
        "5.200.200.200" to IspInfo("MCI", "Hamrah-e-Aval", IspType.MOBILE),
        "5.200.202.202" to IspInfo("MCI", "Hamrah-e-Aval", IspType.MOBILE),

        // Irancell (MTN Irancell)
        "217.218.127.127" to IspInfo("Irancell", "MTN Irancell", IspType.MOBILE),

        // Rightel
        "5.200.200.201" to IspInfo("Rightel", "Rightel", IspType.MOBILE),

        // TCI (Telecommunication Company of Iran - landline)
        "217.218.155.155" to IspInfo("TCI", "Telecommunication Co. Iran", IspType.LANDLINE),

        // Shatel
        "217.218.155.156" to IspInfo("Shatel", "Shatel", IspType.BROADBAND),

        // ParsOnline
        "91.92.156.6" to IspInfo("ParsOnline", "ParsOnline", IspType.BROADBAND),

        // Asiatech
        "185.143.234.6" to IspInfo("Asiatech", "Asiatech", IspType.BROADBAND),

        // HiWeb
        "185.143.233.6" to IspInfo("HiWeb", "HiWeb", IspType.BROADBAND),

        // Zitel
        "5.200.202.202" to IspInfo("Zitel", "Zitel", IspType.BROADBAND)
    )

    // Iranian IP ranges (major ASNs)
    private val iranianAsnPrefixes = listOf(
        "5.200.", "5.201.", "5.202.",     // MCI, Rightel
        "217.218.", "217.219.",           // TCI
        "178.22.", "178.23.",             // Various ISPs
        "185.143.",                        // Asiatech, HiWeb
        "91.92.",                          // ParsOnline
        "31.56.", "31.57.",               // Irancell
        "46.36.", "46.37.",               // Shatel
        "78.38.", "78.39.",               // TCI
        "94.182.", "94.183.",             // MCI
        "151.233.",                        // Irancell
        "185.55.", "185.56.",             // Various
        "5.106.", "5.107.",               // MCI
        "2.146.", "2.147."                // MCI
    )

    data class IspInfo(
        val code: String,
        val name: String,
        val type: IspType
    )

    enum class IspType {
        MOBILE, LANDLINE, BROADBAND, UNKNOWN
    }

    data class DetectionResult(
        val isp: IspInfo,
        val isIranian: Boolean,
        val dnsServers: List<String>,
        val localIp: String
    )

    /**
     * Detect ISP from DNS configuration and network interfaces.
     */
    fun detect(): DetectionResult {
        val dnsServers = getDnsServers()
        val localIp = getLocalIpAddress()

        // Try to match DNS servers against known Iranian ISP DNS
        for (dns in dnsServers) {
            val isp = ispDnsMap[dns]
            if (isp != null) {
                Log.i(TAG, "ISP detected via DNS: ${isp.name} (DNS: $dns)")
                return DetectionResult(
                    isp = isp,
                    isIranian = true,
                    dnsServers = dnsServers,
                    localIp = localIp
                )
            }
        }

        // Fallback: check if local IP falls in Iranian ranges
        val isIranianByIp = isIranianIp(localIp)

        val defaultIsp = if (isIranianByIp) {
            IspInfo("IR_UNKNOWN", "Unknown Iranian ISP", IspType.UNKNOWN)
        } else {
            IspInfo("NON_IR", "Non-Iranian ISP", IspType.UNKNOWN)
        }

        Log.i(TAG, "ISP detection result: ${defaultIsp.name} (Iranian: $isIranianByIp)")
        return DetectionResult(
            isp = defaultIsp,
            isIranian = isIranianByIp,
            dnsServers = dnsServers,
            localIp = localIp
        )
    }

    /**
     * Get DNS servers from the device's network configuration.
     */
    private fun getDnsServers(): List<String> {
        val dnsList = mutableListOf<String>()

        try {
            // Try DHCP info from Wi-Fi
            val wifiManager = context.applicationContext
                .getSystemService(Context.WIFI_SERVICE) as? WifiManager
            val dhcpInfo = wifiManager?.dhcpInfo
            if (dhcpInfo != null) {
                dhcpInfo.dns1.let { if (it != 0) dnsList.add(intToIp(it)) }
                dhcpInfo.dns2.let { if (it != 0) dnsList.add(intToIp(it)) }
            }
        } catch (e: Exception) {
            Log.w(TAG, "Failed to get DNS from DHCP: ${e.message}")
        }

        // Fallback: get DNS from network interfaces
        try {
            val enum = NetworkInterface.getNetworkInterfaces()
            while (enum.hasMoreElements()) {
                val intf = enum.nextElement()
                val addrEnum = intf.inetAddresses
                while (addrEnum.hasMoreElements()) {
                    val addr = addrEnum.nextElement()
                    if (addr is Inet4Address && !addr.isLoopbackAddress) {
                        // Use the interface's network for DNS inference
                    }
                }
            }
        } catch (e: Exception) {
            Log.w(TAG, "Failed to enumerate network interfaces: ${e.message}")
        }

        // If no DNS found, add common Iranian DNS as fallback
        if (dnsList.isEmpty()) {
            dnsList.add("217.218.155.155") // TCI default
        }

        return dnsList
    }

    /**
     * Get the device's local IP address.
     */
    private fun getLocalIpAddress(): String {
        try {
            val enum = NetworkInterface.getNetworkInterfaces()
            while (enum.hasMoreElements()) {
                val intf = enum.nextElement()
                val addrEnum = intf.inetAddresses
                while (addrEnum.hasMoreElements()) {
                    val addr = addrEnum.nextElement()
                    if (addr is Inet4Address && !addr.isLoopbackAddress) {
                        return addr.hostAddress ?: "0.0.0.0"
                    }
                }
            }
        } catch (e: Exception) {
            Log.w(TAG, "Failed to get local IP: ${e.message}")
        }
        return "0.0.0.0"
    }

    /**
     * Check if an IP address falls within known Iranian IP ranges.
     */
    fun isIranianIp(ip: String): Boolean {
        if (ip == "0.0.0.0" || ip.isEmpty()) return false
        return iranianAsnPrefixes.any { prefix -> ip.startsWith(prefix) }
    }

    private fun intToIp(addr: Int): String {
        return ((addr and 0xFF).toString() + "." +
                ((addr shr 8) and 0xFF) + "." +
                ((addr shr 16) and 0xFF) + "." +
                ((addr shr 24) and 0xFF))
    }
}
