package com.v2rayez.app.data.dns

import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.ByteArrayOutputStream
import java.net.DatagramPacket
import java.net.DatagramSocket
import kotlin.concurrent.thread

/**
 * Wire-format + scanner conformance tests. A scripted in-process UDP DNS
 * server plays healthy / EDNS / hijacker resolvers so the full probe path
 * (query build, send, parse, score) runs for real — no mocks.
 */
class ResolverScannerTest {

    /** Starts a scripted UDP resolver on an ephemeral port; returns it. */
    private fun startServer(edns: Boolean, hijack: Boolean): DatagramSocket {
        val server = DatagramSocket(0)
        thread(isDaemon = true) {
            val buffer = ByteArray(1500)
            while (true) {
                val packet = DatagramPacket(buffer, buffer.size)
                try {
                    server.receive(packet)
                } catch (e: Exception) {
                    break
                }
                val query = buffer.copyOf(packet.length)
                val reply = composeReply(query, edns, hijack)
                server.send(DatagramPacket(reply, reply.size, packet.address, packet.port))
            }
        }
        return server
    }

    /** Builds a DNS reply for the parsed query (question echoed, one A). */
    private fun composeReply(query: ByteArray, edns: Boolean, hijack: Boolean): ByteArray {
        // Decode the question name (labels until NUL).
        val labels = ArrayList<String>()
        var offset = 12
        while (query[offset].toInt() != 0) {
            val length = query[offset].toInt() and 0xFF
            if (length and 0xC0 == 0xC0) break
            labels.add(String(query, offset + 1, length, Charsets.US_ASCII))
            offset += 1 + length
        }
        val name = query.copyOfRange(12, offset + 1) // includes trailing NUL
        val questionEnd = offset + 5                  // NUL + QTYPE + QCLASS
        val domain = labels.joinToString(".")
        val isInvalid = domain.endsWith(".invalid")

        val answer: ByteArray? = when {
            isInvalid && hijack -> record(name, "146.112.61.106")
            isInvalid -> null
            else -> record(name, "203.0.113.7")
        }

        val out = ByteArrayOutputStream()
        out.write(query.copyOf(2))                                // ID
        out.write(byteArrayOf(0x81.toByte(), 0x80.toByte()))      // QR + RD + RA
        out.write(byteArrayOf(0x00, 0x01))                        // QDCOUNT
        out.write(if (answer != null) byteArrayOf(0x00, 0x01) else byteArrayOf(0x00, 0x00))
        out.write(byteArrayOf(0x00, 0x00))                        // NSCOUNT
        out.write(if (edns) byteArrayOf(0x00, 0x01) else byteArrayOf(0x00, 0x00))
        out.write(name)                                           // question name
        out.write(query.copyOfRange(questionEnd - 4, questionEnd)) // QTYPE + QCLASS
        answer?.let { out.write(it) }
        if (edns) {
            out.write(
                byteArrayOf(
                    0x00,        // root name
                    0x00, 0x29,  // OPT
                    0x04, 0xD0.toByte(),  // payload 1232
                    0x00, 0x00, 0x00, 0x00, // TTL
                    0x00, 0x00,  // RDLENGTH
                )
            )
        }
        val reply = out.toByteArray()
        if (isInvalid && !hijack) {
            reply[3] = 3 // RCODE = NXDOMAIN
        }
        return reply
    }

    private fun record(name: ByteArray, ip: String): ByteArray {
        val out = ByteArrayOutputStream()
        out.write(name)
        out.write(byteArrayOf(0x00, 0x01))             // A
        out.write(byteArrayOf(0x00, 0x01))             // IN
        out.write(byteArrayOf(0x00, 0x00, 0x00, 0x3C)) // TTL 60
        out.write(byteArrayOf(0x00, 0x04))             // RDLENGTH
        ip.split(".").forEach { out.write(it.toInt()) }
        return out.toByteArray()
    }

    private fun targetFor(server: DatagramSocket): ResolverScanner.Target =
        ResolverScanner.Target("127.0.0.1", ResolverScanner.Transport.UDP, server.localPort)

    @Test
    fun `healthy resolver scores high with edns`() = runBlocking {
        val server = startServer(edns = true, hijack = false)
        val result = ResolverScanner.probe(targetFor(server), "example.com")
        assertEquals(null, result.error)
        assertTrue("EDNS should be detected", result.edns)
        assertTrue("healthy resolver must be honest", result.honest)
        assertTrue("score ${result.score} should be >= 80", result.score >= 80)
    }

    @Test
    fun `hijacker is detected and penalized`() = runBlocking {
        val server = startServer(edns = false, hijack = true)
        val result = ResolverScanner.probe(targetFor(server), "example.com")
        assertTrue("hijacker must be flagged", !result.honest)
        org.junit.Assert.assertEquals("146.112.61.106", result.hijackIp)
        assertTrue("score ${result.score} must be heavily penalized", result.score <= 30)
    }

    @Test
    fun `unreachable resolver errors with zero score`() = runBlocking {
        // Port 1 on localhost: nothing listens there.
        val result = ResolverScanner.probe(
            ResolverScanner.Target("127.0.0.1", ResolverScanner.Transport.UDP, 1),
            "example.com",
        )
        assertNotNull(result.error, "error expected")
        assertEquals(0.0, result.score, 1e-9)
    }

    @Test
    fun `score math is stable`() {
        assertEquals(100.0, ResolverScanner.score(0.0, edns = true, honest = true), 1e-9)
        assertEquals(0.0, ResolverScanner.score(500.0, edns = false, honest = false), 1e-9)
        // 55% latency penalty + no-EDNS 10: 100 - 27.5 - 10 = 62.5
        assertEquals(62.5, ResolverScanner.score(250.0, edns = false, honest = true), 1e-9)
    }
}
