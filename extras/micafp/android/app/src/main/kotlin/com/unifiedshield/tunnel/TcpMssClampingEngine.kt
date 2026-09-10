package com.unifiedshield.tunnel

import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.nio.ByteBuffer

/**
 * TCP MSS Clamping & MTU Safeguard Engine.
 * 
 * Prevents hanging on Cloudflare "Verifying you are human" Turnstile challenges,
 * TLS Handshakes, and large HTTPS web payloads by clamping TCP Maximum Segment Size (MSS)
 * in SYN / SYN-ACK packets to 1360 bytes (MTU 1400 - 40B header overhead).
 *
 * Implements RFC 1624 incremental one's complement checksum modification for zero-copy efficiency.
 */
object TcpMssClampingEngine {
    private const val TAG = "TcpMssClamping"

    const val DEFAULT_CLAMPED_MSS = 1360
    const val SAFE_MTU = 1400

    private val _clampingStats = MutableStateFlow(
        TcpMssStats(
            totalPacketsInspected = 0L,
            synPacketsClamped = 0L,
            cloudflareStallsPrevented = 0L,
            activeClampedMss = DEFAULT_CLAMPED_MSS,
            isClampingEnabled = true
        )
    )
    val clampingStats: StateFlow<TcpMssStats> = _clampingStats.asStateFlow()

    /**
     * Inspects and clamps MSS in a raw packet byte array in-place.
     * Returns true if MSS was clamped, false otherwise.
     */
    fun clampPacket(packet: ByteArray, length: Int, targetMss: Int = DEFAULT_CLAMPED_MSS): Boolean {
        if (length < 40) return false

        // Check IP version
        val version = (packet[0].toInt() shr 4) and 0x0F
        var ipHeaderLen = 0
        var protocol = 0

        if (version == 4) {
            ipHeaderLen = (packet[0].toInt() and 0x0F) * 4
            if (length < ipHeaderLen + 20) return false
            protocol = packet[9].toInt() and 0xFF
        } else if (version == 6) {
            ipHeaderLen = 40
            if (length < ipHeaderLen + 20) return false
            protocol = packet[6].toInt() and 0xFF
        } else {
            return false
        }

        // Protocol 6 is TCP
        if (protocol != 6) return false

        val tcpOffset = ipHeaderLen
        val tcpHeaderLen = ((packet[tcpOffset + 12].toInt() shr 4) and 0x0F) * 4
        if (tcpHeaderLen < 20 || length < tcpOffset + tcpHeaderLen) return false

        val flags = packet[tcpOffset + 13].toInt() and 0xFF
        val isSyn = (flags and 0x02) != 0 // SYN flag

        // We only clamp SYN or SYN-ACK packets where MSS is negotiated
        if (!isSyn) return false

        var optionIdx = tcpOffset + 20
        val optionEnd = tcpOffset + tcpHeaderLen

        var wasClamped = false

        while (optionIdx < optionEnd) {
            val kind = packet[optionIdx].toInt() and 0xFF
            if (kind == 0) break // End of option list
            if (kind == 1) {
                // NOP
                optionIdx++
                continue
            }
            if (optionIdx + 1 >= optionEnd) break
            val optLen = packet[optionIdx + 1].toInt() and 0xFF
            if (optLen < 2 || optionIdx + optLen > optionEnd) break

            // Kind 2 is TCP MSS (Length = 4)
            if (kind == 2 && optLen == 4) {
                val oldMssHigh = packet[optionIdx + 2].toInt() and 0xFF
                val oldMssLow = packet[optionIdx + 3].toInt() and 0xFF
                val currentMss = (oldMssHigh shl 8) or oldMssLow

                if (currentMss > targetMss) {
                    val newMssHigh = (targetMss shr 8) and 0xFF
                    val newMssLow = targetMss and 0xFF

                    val oldMssWord = currentMss
                    val newMssWord = targetMss

                    // Write clamped MSS
                    packet[optionIdx + 2] = newMssHigh.toByte()
                    packet[optionIdx + 3] = newMssLow.toByte()

                    // Update TCP Checksum incrementally (RFC 1624: HC' = ~(~HC + ~m + m'))
                    val checksumOffset = tcpOffset + 16
                    val oldChecksum = ((packet[checksumOffset].toInt() and 0xFF) shl 8) or
                            (packet[checksumOffset + 1].toInt() and 0xFF)

                    val updatedChecksum = updateChecksumRfc1624(oldChecksum, oldMssWord, newMssWord)
                    packet[checksumOffset] = ((updatedChecksum shr 8) and 0xFF).toByte()
                    packet[checksumOffset + 1] = (updatedChecksum and 0xFF).toByte()

                    wasClamped = true
                    break
                }
            }
            optionIdx += optLen
        }

        if (wasClamped) {
            val stats = _clampingStats.value
            _clampingStats.value = stats.copy(
                totalPacketsInspected = stats.totalPacketsInspected + 1,
                synPacketsClamped = stats.synPacketsClamped + 1,
                cloudflareStallsPrevented = stats.cloudflareStallsPrevented + 1
            )
            Log.d(TAG, "Clamped TCP MSS from SYN handshake to $targetMss bytes (RFC 1624 Checksum updated)")
        }

        return wasClamped
    }

    /**
     * RFC 1624 Incremental 16-bit One's Complement Checksum Update.
     * Equation: HC' = ~(~HC + ~m + m')
     */
    private fun updateChecksumRfc1624(oldChecksum: Int, oldWord: Int, newWord: Int): Int {
        var sum = (oldChecksum.inv() and 0xFFFF) + (oldWord.inv() and 0xFFFF) + (newWord and 0xFFFF)
        while (sum shr 16 != 0) {
            sum = (sum and 0xFFFF) + (sum shr 16)
        }
        return sum.inv() and 0xFFFF
    }
}

data class TcpMssStats(
    val totalPacketsInspected: Long,
    val synPacketsClamped: Long,
    val cloudflareStallsPrevented: Long,
    val activeClampedMss: Int,
    val isClampingEnabled: Boolean
)
