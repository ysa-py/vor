package com.v2rayez.app.data.dns

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.withContext
import java.io.ByteArrayOutputStream
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.net.InetSocketAddress
import java.nio.ByteBuffer
import javax.net.ssl.SSLSocketFactory
import kotlin.random.Random

/**
 * DNS resolver scanner/scorer — the Android port of the reference
 * implementation (desktop internal/dnsprobe; semantics documented in
 * ENGINES.md). Implemented from scratch against RFC 1035 (wire format),
 * RFC 6891 (EDNS0) and RFC 7858 (DoT).
 *
 * Scanner semantics (identical across ports):
 *  - latency: median of 3 A-queries against [probeDomain]
 *  - EDNS0: a query carrying an OPT RR gets an OPT RR back
 *  - NXDOMAIN honesty: a random label under .invalid must return RCODE 3;
 *    an A answer is flagged as hijacking (DNS poisoning / redirect farms)
 *  - score: 0..100 composite (latency penalty, EDNS bonus, hijack penalty)
 *
 * Nothing here derives from any third-party application code.
 */
object ResolverScanner {

    /** Transport used for the probe. */
    enum class Transport(val wire: String) { UDP("udp"), DOT("dot") }

    /** One resolver to probe. */
    data class Target(
        val server: String,
        val transport: Transport = Transport.UDP,
        val port: Int = if (transport == Transport.DOT) 853 else 53,
    )

    /** Probe outcome for one resolver. */
    data class Result(
        val server: String,
        val transport: String,
        val latencyMs: Double = 0.0,
        val edns: Boolean = false,
        val honest: Boolean = true,
        val hijackIp: String? = null,
        val score: Double = 0.0,
        val error: String? = null,
    )

    private data class DnsReply(val rcode: Int, val answerIps: List<String>, val hasEdns: Boolean)

    /** Public starter set across transports. */
    fun defaultTargets(): List<Target> = listOf(
        Target("1.1.1.1"), Target("8.8.8.8"), Target("9.9.9.9"),
        Target("1.1.1.1", Transport.DOT), Target("8.8.8.8", Transport.DOT),
    )

    /** Probe one resolver. */
    suspend fun probe(target: Target, probeDomain: String): Result = withContext(Dispatchers.IO) {
        val samples = ArrayList<Double>(3)
        repeat(3) {
            val start = System.nanoTime()
            val reply = runCatching { exchange(target, probeDomain, 1, false) }.getOrElse {
                return@withContext Result(
                    target.server, target.transport.wire, error = it.message ?: "failed"
                )
            }
            samples.add((System.nanoTime() - start) / 1_000_000.0)
        }
        val edns = runCatching { exchange(target, probeDomain, 1, true) }.getOrNull()?.hasEdns == true
        val testLabel = randomLabel(12) + ".invalid"
        var honest = true
        var hijackIp: String? = null
        runCatching { exchange(target, testLabel, 1, false) }.getOrNull()?.let { reply ->
            if (reply.rcode != 3 && reply.answerIps.isNotEmpty()) {
                honest = false
                hijackIp = reply.answerIps.firstOrNull()
            }
        }
        val latency = samples.sorted()[samples.size / 2]
        Result(
            server = target.server,
            transport = target.transport.wire,
            latencyMs = latency,
            edns = edns,
            honest = honest,
            hijackIp = hijackIp,
            score = score(latency, edns, honest),
        )
    }

    /** Probe many resolvers concurrently; results sorted by score. */
    suspend fun scan(targets: List<Target>, probeDomain: String): List<Result> = coroutineScope {
        targets.map { target -> async { probe(target, probeDomain) } }.awaitAll()
            .sortedByDescending { it.score }
    }

    /** 0..100 composite (mirrors the reference scoring). */
    fun score(latencyMs: Double, edns: Boolean, honest: Boolean): Double {
        var value = 100.0
        value -= 55.0 * (latencyMs / 500.0).coerceIn(0.0, 1.0)
        if (!edns) value -= 10.0
        if (!honest) value -= 70.0
        return value.coerceIn(0.0, 100.0)
    }

    // ------------------------------------------------------------ wire format

    private fun buildQuery(id: Int, name: String, qtype: Int, edns: Boolean): ByteArray {
        val buffer = ByteBuffer.allocate(512)
        buffer.putShort(id.toShort())
        buffer.putShort(0x0100) // RD
        buffer.putShort(1)      // QDCOUNT
        buffer.putShort(0)      // ANCOUNT
        buffer.putShort(0)      // NSCOUNT
        buffer.putShort(if (edns) 1 else 0) // ARCOUNT
        for (label in name.trim('.').split('.')) {
            if (label.isEmpty()) continue
            buffer.put(label.length.toByte())
            buffer.put(label.toByteArray(Charsets.US_ASCII))
        }
        buffer.put(0)
        buffer.putShort(qtype.toShort())
        buffer.putShort(1) // IN
        if (edns) {
            buffer.put(0)               // root name
            buffer.putShort(41)         // OPT
            buffer.putShort(1232)       // UDP payload size
            buffer.putInt(0)            // TTL
            buffer.putShort(0)          // RDLENGTH
        }
        return buffer.array().copyOf(buffer.position())
    }

    private fun parseReply(data: ByteArray): DnsReply {
        val buffer = ByteBuffer.wrap(data)
        buffer.short // ID
        val flags = buffer.short.toInt() and 0xFFFF
        val rcode = flags and 0x000F
        val questions = buffer.short.toInt() and 0xFFFF
        val answers = buffer.short.toInt() and 0xFFFF
        val authority = buffer.short.toInt() and 0xFFFF
        val additional = buffer.short.toInt() and 0xFFFF
        var hasEdns = false
        val ips = ArrayList<String>()
        repeat(questions) { skipName(buffer); buffer.int } // QTYPE + QCLASS
        val counts = intArrayOf(answers, authority, additional)
        for (count in counts) repeat(count) {
            skipName(buffer)
            val rtype = buffer.short.toInt() and 0xFFFF
            buffer.short // class
            buffer.int   // ttl
            val rdLength = buffer.short.toInt() and 0xFFFF
            when {
                rtype == 41 -> hasEdns = true
                rtype == 1 && rdLength == 4 -> {
                    val bytes = ByteArray(4).also { buffer.get(it) }
                    ips.add(bytes.joinToString(".") { (it.toInt() and 0xFF).toString() })
                }
                rtype == 28 && rdLength == 16 -> {
                    val bytes = ByteArray(16).also { buffer.get(it) }
                    ips.add(groupsIPv6(bytes))
                }
                else -> buffer.position(buffer.position() + rdLength)
            }
        }
        return DnsReply(rcode, ips, hasEdns)
    }

    private fun groupsIPv6(bytes: ByteArray): String {
        val groups = (0 until 8).map {
            ((bytes[it * 2].toInt() and 0xFF) shl 8) or (bytes[it * 2 + 1].toInt() and 0xFF)
        }
        return groups.joinToString(":") { Integer.toHexString(it) }
    }

    /** Skips a (possibly compressed) name; pointers are not followed — the
     *  offsets past the name are all the parser needs. */
    private fun skipName(buffer: ByteBuffer) {
        var guard = 0
        while (guard < 128) {
            val length = buffer.get().toInt() and 0xFF
            if (length == 0) return
            if (length and 0xC0 == 0xC0) {
                buffer.get() // second pointer byte
                return
            }
            buffer.position(buffer.position() + length)
            guard++
        }
    }

    /** One query/response exchange over the target transport. */
    private fun exchange(target: Target, name: String, qtype: Int, edns: Boolean): DnsReply {
        val query = buildQuery(Random.nextInt(0xFFFF), name, qtype, edns)
        return when (target.transport) {
            Transport.UDP -> exchangeUdp(target, query)
            Transport.DOT -> exchangeDot(target, query)
        }
    }

    private fun exchangeUdp(target: Target, query: ByteArray): DnsReply {
        DatagramSocket().use { socket ->
            socket.soTimeout = 3000
            val address = InetAddress.getByName(target.server)
            socket.send(DatagramPacket(query, query.size, address, target.port))
            val response = ByteArray(1500)
            val packet = DatagramPacket(response, response.size)
            socket.receive(packet)
            return parseReply(response.copyOf(packet.length))
        }
    }

    private fun exchangeDot(target: Target, query: ByteArray): DnsReply {
        val factory = SSLSocketFactory.getDefault() as SSLSocketFactory
        val socket = factory.createSocket() as javax.net.ssl.SSLSocket
        try {
            socket.connect(InetSocketAddress(target.server, target.port), 3000)
            socket.soTimeout = 3000
            socket.startHandshake()
            val framed = ByteArrayOutputStream()
            framed.write(byteArrayOf((query.size shr 8).toByte(), query.size.toByte()))
            framed.write(query)
            socket.getOutputStream().apply { write(framed.toByteArray()); flush() }
            val input = socket.getInputStream()
            val lengthHeader = ByteArray(2)
            readFully(input, lengthHeader)
            val length = ((lengthHeader[0].toInt() and 0xFF) shl 8) or (lengthHeader[1].toInt() and 0xFF)
            val body = ByteArray(length)
            readFully(input, body)
            return parseReply(body)
        } finally {
            socket.close()
        }
    }

    private fun readFully(input: java.io.InputStream, buffer: ByteArray) {
        var offset = 0
        while (offset < buffer.size) {
            val read = input.read(buffer, offset, buffer.size - offset)
            if (read < 0) throw java.io.EOFException("short read")
            offset += read
        }
    }

    private fun randomLabel(length: Int): String {
        val alphabet = "abcdefghijklmnopqrstuvwxyz0123456789"
        return (0 until length).map { alphabet[Random.nextInt(alphabet.length)] }.joinToString("")
    }
}
