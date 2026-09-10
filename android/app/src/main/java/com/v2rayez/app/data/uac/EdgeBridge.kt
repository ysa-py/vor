package com.v2rayez.app.data.uac

import java.io.IOException
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.net.Socket
import java.net.SocketException
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger
import kotlin.concurrent.thread

/**
 * Edge Bridge — pure-Kotlin port of the UAC `MciEdgeBridge`
 * ("external finalmask") loopback relay.
 *
 * Listens on 127.0.0.1:40443. For every accepted TLS flow it:
 *  1. reads the first ClientHello record,
 *  2. splits it into well-formed TLS records according to the selected
 *     strategy (see [HelloFragmenter.splitIntoWellFormedRecords]),
 *  3. writes the fragments to the protected upstream edge socket with the
 *     inter-fragment delay,
 *  4. relays the rest of the session bidirectionally.
 *
 * Because the split happens OUTSIDE the proxy core, ANY core gets
 * finalmask behavior — the stock Xray AAR, sing-box process core, or a
 * plain local proxy — without requiring the patched Xray fork that UAC
 * ships as 153 MB of binaries. (Users who run the forked Xray as a
 * process core keep the native `streamSettings.finalmask` path instead.)
 *
 * Upstream sockets are expected to be protected (routed outside the VPN) by
 * the caller via [socketProtector] — mirroring UAC's `SocketProtector`.
 */
class EdgeBridge(
    private val listenPort: Int = DEFAULT_PORT,
    private val maxSessions: Int = DEFAULT_MAX_SESSIONS,
    private val socketProtector: (Socket) -> Unit = {},
) {
    companion object {
        const val DEFAULT_PORT = 40443
        const val DEFAULT_MAX_SESSIONS = 64
        const val FIRST_RECORD_TIMEOUT_MS = 10_000
        const val RELAY_BUFFER = 256 * 1024
        const val BIND_RETRY_COUNT = 5
        const val BIND_RETRY_DELAY_MS = 200L
    }

    private val running = AtomicBoolean(false)
    private val activeSessions = AtomicInteger(0)
    private var serverSocket: ServerSocket? = null
    private val sessionPool = Executors.newCachedThreadPool()

    /** Fragment strategy + delay applied to the ClientHello. */
    @Volatile
    var strategy: String = "tls_record_frag"

    /** Inter-fragment delay (ms). */
    @Volatile
    var interFragmentDelayMs: Int = 0

    /** Resolves the upstream edge address for an incoming connection. */
    @Volatile
    var upstreamResolver: (Socket) -> InetSocketAddress = {
        InetSocketAddress(InetAddress.getByName("104.18.1.1"), 443)
    }

    /** Whether the bridge is currently accepting sessions. */
    val isRunning: Boolean
        get() = running.get()

    /** Start the loopback listener (idempotent). */
    @Synchronized
    fun start() {
        if (running.get()) return
        var attempt = 0
        while (true) {
            try {
                serverSocket = ServerSocket(listenPort, 50, InetAddress.getLoopbackAddress())
                break
            } catch (_: IOException) {
                attempt += 1
                if (attempt >= BIND_RETRY_COUNT) {
                    throw IOException("EdgeBridge: could not bind 127.0.0.1:$listenPort after $attempt attempts")
                }
                Thread.sleep(BIND_RETRY_DELAY_MS)
            }
        }
        running.set(true)
        thread(name = "vor-edge-bridge-accept") {
            acceptLoop()
        }
    }

    /** Stop the listener and close active sessions. */
    @Synchronized
    fun stop() {
        if (!running.getAndSet(false)) return
        runCatching { serverSocket?.close() }
        serverSocket = null
    }

    private fun acceptLoop() {
        val socket = serverSocket ?: return
        while (running.get()) {
            val client = try {
                socket.accept()
            } catch (_: SocketException) {
                break
            } catch (_: IOException) {
                continue
            }
            if (activeSessions.get() >= maxSessions) {
                runCatching { client.close() }
                continue
            }
            activeSessions.incrementAndGet()
            sessionPool.execute {
                try {
                    serveSession(client)
                } finally {
                    activeSessions.decrementAndGet()
                    runCatching { client.close() }
                }
            }
        }
    }

    private fun serveSession(client: Socket) {
        val upstream = Socket()
        try {
            client.soTimeout = FIRST_RECORD_TIMEOUT_MS
            val firstRecord = readFirstTlsRecord(client) ?: return
            if (!HelloFragmenter.isClientHello(firstRecord)) {
                // Not a ClientHello — relay untouched (passthrough semantics).
                upstream.connect(upstreamResolver(client), FIRST_RECORD_TIMEOUT_MS)
                socketProtector(upstream)
                upstream.getOutputStream().write(firstRecord)
                relay(client, upstream)
                return
            }

            // External finalmask: split into well-formed records, write with
            // the strategy delay, then relay the rest.
            upstream.connect(upstreamResolver(client), FIRST_RECORD_TIMEOUT_MS)
            socketProtector(upstream)
            upstream.tcpNoDelay = true
            val fragments = HelloFragmenter.splitIntoWellFormedRecords(firstRecord)
            val output = upstream.getOutputStream()
            for ((index, fragment) in fragments.withIndex()) {
                if (index > 0 && interFragmentDelayMs > 0) {
                    Thread.sleep(interFragmentDelayMs.toLong())
                }
                output.write(fragment)
                output.flush()
            }
            client.soTimeout = 0
            relay(client, upstream)
        } catch (_: IOException) {
            // Session failure — closing both sides is the whole cleanup.
        } catch (_: InterruptedException) {
            Thread.currentThread().interrupt()
        } finally {
            runCatching { upstream.close() }
        }
    }

    /** Read the first complete TLS record (5-byte header + body). */
    private fun readFirstTlsRecord(client: Socket): ByteArray? {
        val input = client.getInputStream() ?: return null
        val header = ByteArray(5)
        var read = 0
        while (read < header.size) {
            val n = input.read(header, read, header.size - read)
            if (n < 0) return null
            read += n
        }
        if ((header[0].toInt() and 0xFF) != 0x16) {
            // Not a handshake record — return the raw header bytes plus the
            // rest of the stream via the caller (passthrough).
            return header
        }
        val length = ((header[3].toInt() and 0xFF) shl 8) or (header[4].toInt() and 0xFF)
        if (length <= 0 || length > 16_384 * 2) return header
        val body = ByteArray(length)
        read = 0
        while (read < length) {
            val n = input.read(body, read, length - read)
            if (n < 0) return null
            read += n
        }
        return header + body
    }

    private fun relay(client: Socket, upstream: Socket) {
        val a = thread(name = "vor-edge-c2u") { pump(client.getInputStream(), upstream.getOutputStream()) }
        val b = thread(name = "vor-edge-u2c") { pump(upstream.getInputStream(), client.getOutputStream()) }
        a.join()
        b.join()
    }

    private fun pump(from: java.io.InputStream, to: java.io.OutputStream) {
        val buffer = ByteArray(RELAY_BUFFER)
        try {
            while (running.get()) {
                val n = from.read(buffer)
                if (n < 0) break
                to.write(buffer, 0, n)
                to.flush()
            }
        } catch (_: IOException) {
            // Broken pipe closes both directions via the paired pump.
        } finally {
            runCatching { to.close() }
        }
    }
}
