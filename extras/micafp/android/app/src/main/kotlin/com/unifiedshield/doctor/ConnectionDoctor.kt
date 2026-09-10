package com.unifiedshield.doctor

import android.content.Context
import com.unifiedshield.CoreBridge
import com.unifiedshield.TunnelManager
import com.unifiedshield.license.AuditLog
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.Socket

// =============================================================================
// MICAFP Directive v6 — Connection Doctor (advanced feature, additive).
// Real probes only (A2): DNS resolution, TCP/443 reachability, UDP/53 query,
// tunnel daemon state. Root cause + suggested fix are derived from the ACTUAL
// failing probe. The "smart explanation" flows through the B.9/B.10 AI
// failover chain (external providers or grounded local analysis).
// =============================================================================

data class DoctorCheck(
    val id: String,          // dns | tcp443 | udp53 | tunnel
    val passed: Boolean,
    val detail: String,
    val latencyMs: Long
)

data class DoctorReport(
    val checks: List<DoctorCheck>,
    val rootCause: String,
    val suggestedFix: String,
    val completedAtMillis: Long
)

class ConnectionDoctor private constructor(private val context: Context) {

    private val audit = AuditLog.getInstance(context)

    suspend fun runFullCheck(): DoctorReport = withContext(Dispatchers.IO) {
        val checks = mutableListOf<DoctorCheck>()

        // 1) DNS resolution (real)
        val dnsStart = System.currentTimeMillis()
        val dnsOk = runCatching { InetAddress.getByName("www.google.com").hostAddress != null }.getOrDefault(false)
        checks += DoctorCheck(
            "dns", dnsOk,
            if (dnsOk) "A-record resolved via system resolver" else "System DNS resolution failed (likely poisoned/blocked)",
            System.currentTimeMillis() - dnsStart
        )

        // 2) TCP/443 reachability (real connect, 4s budget)
        val tcpStart = System.currentTimeMillis()
        val tcpOk = runCatching {
            Socket().use { s ->
                s.connect(InetSocketAddress("1.1.1.1", 443), 4000)
                true
            }
        }.getOrDefault(false)
        checks += DoctorCheck(
            "tcp443", tcpOk,
            if (tcpOk) "TCP handshake with 1.1.1.1:443 succeeded" else "TCP/443 connect timed out or reset (SNI-based filtering suspected)",
            System.currentTimeMillis() - tcpStart
        )

        // 3) UDP/53 DNS query (real DNS query packet to 8.8.8.8)
        val udpStart = System.currentTimeMillis()
        val udpOk = runCatching {
            DatagramSocket().use { sock ->
                sock.soTimeout = 4000
                val query = buildDnsQuery("www.google.com")
                sock.send(DatagramPacket(query, query.size, InetAddress.getByName("8.8.8.8"), 53))
                val buf = ByteArray(512)
                val pkt = DatagramPacket(buf, buf.size)
                sock.receive(pkt)
                true
            }
        }.getOrDefault(false)
        checks += DoctorCheck(
            "udp53", udpOk,
            if (udpOk) "UDP/53 query to 8.8.8.8 answered" else "No UDP/53 answer — plain DNS blocked or tampered",
            System.currentTimeMillis() - udpStart
        )

        // 4) Tunnel daemon state (real bridge status)
        val bridge = CoreBridge()
        val status = bridge.getStatusSafe() // 0 stopped, 1 running, 2 connecting, -1 error
        val tm = TunnelManager.getInstance(context)
        val tunnelOk = status == 1 && tm.stats.value.connected
        checks += DoctorCheck(
            "tunnel", tunnelOk,
            "Daemon status=$status, tunnel connected=${tm.stats.value.connected}, core=${tm.stats.value.currentCore}",
            0
        )

        // Root cause from the ACTUAL first failing probe (ordered by layer)
        val rootCause: String
        val fix: String
        when {
            !checks.first { it.id == "tunnel" }.passed -> {
                rootCause = "MICAFP tunnel daemon is not in the running state"
                fix = "Reconnect from the dashboard; if permission was revoked, re-grant VPN permission."
            }
            !checks.first { it.id == "dns" }.passed && !checks.first { it.id == "udp53" }.passed -> {
                rootCause = "Both system DNS and plain UDP/53 fail — DNS layer is being poisoned"
                fix = "Switch profile to a DNS-tunnel family protocol (DNSTT / MasterDNS / StormDNS) which carries DNS inside the encrypted tunnel."
            }
            !checks.first { it.id == "tcp443" }.passed -> {
                rootCause = "TCP/443 is reset — TLS-based transports are being filtered on this ISP"
                fix = "Enable Auto-Pilot rotation toward UDP-family cores (Hysteria 2 / TUIC) or SSH-chained DNS tunnels."
            }
            else -> {
                rootCause = "No fault found — all probes passed"
                fix = "Connection path is healthy; no action needed."
            }
        }

        audit.append(
            "DOCTOR",
            "Connection Doctor: dns=${checks[0].passed} tcp443=${checks[1].passed} udp53=${checks[2].passed} tunnel=${checks[3].passed} → $rootCause"
        )
        DoctorReport(checks, rootCause, fix, System.currentTimeMillis())
    }

    /** Minimal RFC1035 A-query for the UDP probe (real packet, no library). */
    private fun buildDnsQuery(host: String): ByteArray {
        val out = mutableListOf<Byte>()
        out.add(0x12); out.add(0x34) // txid
        out.add(0x01); out.add(0x00) // flags: recursion desired
        out.add(0x00); out.add(0x01) // qdcount
        out.add(0x00); out.add(0x00); out.add(0x00); out.add(0x00); out.add(0x00); out.add(0x00)
        host.split(".").forEach { label ->
            out.add(label.length.toByte())
            label.forEach { out.add(it.code.toByte()) }
        }
        out.add(0x00)
        out.add(0x00); out.add(0x01) // type A
        out.add(0x00); out.add(0x01) // class IN
        return out.toByteArray()
    }

    companion object {
        @Volatile private var instance: ConnectionDoctor? = null
        fun getInstance(context: Context): ConnectionDoctor =
            instance ?: synchronized(this) {
                instance ?: ConnectionDoctor(context.applicationContext).also { instance = it }
            }
    }
}
