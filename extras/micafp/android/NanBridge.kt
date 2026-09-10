/*
 * MICAFP-UnifiedShield-6.0
 * NanBridge.kt — WiFi Aware / NAN mesh bridge for peer-to-peer connectivity
 *
 * Uses WifiAwareManager for publishing and subscribing to create a local mesh
 * without requiring a traditional access point. Peers within ~100m can discover
 * each other and establish direct socket connections.
 *
 * BATTERY: Scanning only occurs when screen is ON or NAIN CompleteBlackout is
 * active. When the app is backgrounded (unless NAIN is active), scanning is
 * paused to conserve battery.
 *
 * No root required. Cloudflare is NOT used.
 */

package org.micafp.unifiedshield.nearby

import android.annotation.SuppressLint
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.net.wifi.aware.AttachCallback
import android.net.wifi.aware.Characteristics
import android.net.wifi.aware.DiscoverySessionCallback
import android.net.wifi.aware.PeerHandle
import android.net.wifi.aware.PublishConfig
import android.net.wifi.aware.PublishDiscoverySession
import android.net.wifi.aware.ServiceSpecificInfo
import android.net.wifi.aware.SubscribeConfig
import android.net.wifi.aware.SubscribeDiscoverySession
import android.net.wifi.aware.WifiAwareManager
import android.net.wifi.aware.WifiAwareNetworkInfo
import android.net.wifi.aware.WifiAwareNetworkSpecifier
import android.net.wifi.aware.WifiAwareSession
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.Log
import androidx.annotation.RequiresApi
import org.micafp.unifiedshield.jni.ShieldNativeBridge
import org.micafp.unifiedshield.nain.NainController
import org.micafp.unifiedshield.nain.NainState
import java.nio.ByteBuffer
import java.nio.charset.StandardCharsets
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicReference

/**
 * WiFi Aware (NAN) mesh bridge for peer-to-peer connectivity.
 *
 * Manages both publishing (advertising this device) and subscribing
 * (discovering nearby devices) through the WiFi Aware API. Discovered
 * peers are reported to the Rust daemon via JNI callbacks.
 */
class NanBridge(
    private val context: Context,
    private val nainController: NainController
) {
    companion object {
        private const val TAG = "Shield/NanBridge"
        private const val SERVICE_NAME = "UnifiedShield"
        private const val MAX_SERVICE_INFO_LENGTH = 255
        private const val SUBSCRIBE_TIMEOUT_MS = 120_000L
        private const val SCAN_INTERVAL_SCREEN_ON_MS = 15_000L
        private const val SCAN_INTERVAL_BLACKOUT_MS = 10_000L
        private const val PEER_STALE_TIMEOUT_MS = 180_000L

        // Service info format markers
        private const val INFO_VERSION = 0x01
        private const val INFO_PUBKEY_LEN = 64 // Ed25519 public key length
    }

    // WiFi Aware session management
    private var awareSession: AtomicReference<WifiAwareSession?> = AtomicReference(null)
    private var publishSession: AtomicReference<PublishDiscoverySession?> = AtomicReference(null)
    private var subscribeSession: AtomicReference<SubscribeDiscoverySession?> = AtomicReference(null)

    // Lifecycle state
    private val isScanning = AtomicBoolean(false)
    private val isAppForegrounded = AtomicBoolean(true)
    private val isScreenOn = AtomicBoolean(true)
    private val isPublishing = AtomicBoolean(false)

    // Peer tracking
    private data class PeerInfo(
        val peerHandle: PeerHandle,
        val pubkey: ByteArray,
        val endpoints: List<String>,
        val discoveredAt: Long,
        val network: Network?
    )
    private val discoveredPeers = ConcurrentHashMap<PeerHandle, PeerInfo>()

    // Handler for periodic scan scheduling
    private val handler = Handler(Looper.getMainLooper())
    private val scanRunnable = object : Runnable {
        override fun run() {
            if (shouldScan()) {
                performScanCycle()
            }
            val interval = if (nainController.state == NainState.COMPLETE_BLACKOUT) {
                SCAN_INTERVAL_BLACKOUT_MS
            } else {
                SCAN_INTERVAL_SCREEN_ON_MS
            }
            handler.postDelayed(this, interval)
        }
    }

    // JNI bridge to Rust daemon
    private val jniBridge = ShieldNativeBridge()

    // WiFi Aware availability receiver
    private val awareAvailableReceiver = object : BroadcastReceiver() {
        override fun onReceive(ctx: Context, intent: Intent) {
            when (intent.action) {
                WifiAwareManager.ACTION_WIFI_AWARE_STATE_CHANGED -> {
                    if (isWifiAwareAvailable()) {
                        Log.i(TAG, "WiFi Aware became available, attaching session")
                        attachAwareSession()
                    } else {
                        Log.w(TAG, "WiFi Aware became unavailable, cleaning up")
                        cleanupSessions()
                    }
                }
            }
        }
    }

    // Screen state receiver
    private val screenReceiver = object : BroadcastReceiver() {
        override fun onReceive(ctx: Context, intent: Intent) {
            when (intent.action) {
                Intent.ACTION_SCREEN_ON -> {
                    isScreenOn.set(true)
                    Log.d(TAG, "Screen turned ON, resuming NAN scanning")
                    startScanning()
                }
                Intent.ACTION_SCREEN_OFF -> {
                    isScreenOn.set(false)
                    Log.d(TAG, "Screen turned OFF, checking if scanning should pause")
                    if (nainController.state != NainState.COMPLETE_BLACKOUT) {
                        stopScanning()
                    }
                }
            }
        }
    }

    // Network callback for established NAN connections
    private var networkCallback: ConnectivityManager.NetworkCallback? = null

    init {
        registerReceivers()
    }

    // ============================================================
    // Public API
    // ============================================================

    /**
     * Start the NAN bridge: attach to WiFi Aware and begin publishing/subscribing.
     */
    fun start(yggdrasilPubkey: ByteArray, availableEndpoints: List<String>) {
        if (!isWifiAwareAvailable()) {
            Log.w(TAG, "WiFi Aware not available on this device")
            return
        }
        Log.i(TAG, "Starting NAN bridge with pubkey=${yggdrasilPubkey.take(8).toHexString()}...")
        attachAwareSession()
        startPublishing(yggdrasilPubkey, availableEndpoints)
        startScanning()
    }

    /**
     * Stop the NAN bridge and release all resources.
     */
    fun stop() {
        Log.i(TAG, "Stopping NAN bridge")
        stopScanning()
        stopPublishing()
        cleanupSessions()
        handler.removeCallbacksAndMessages(null)
    }

    /**
     * Notify the bridge that the app has moved to the foreground.
     */
    fun onAppForegrounded() {
        isAppForegrounded.set(true)
        if (isScreenOn.get() || nainController.state == NainState.COMPLETE_BLACKOUT) {
            startScanning()
        }
    }

    /**
     * Notify the bridge that the app has moved to the background.
     */
    fun onAppBackgrounded() {
        isAppForegrounded.set(false)
        if (nainController.state != NainState.COMPLETE_BLACKOUT) {
            stopScanning()
        }
    }

    /**
     * Update the published service info (e.g., when endpoints change).
     */
    fun updatePublishInfo(yggdrasilPubkey: ByteArray, availableEndpoints: List<String>) {
        val session = publishSession.get() ?: run {
            Log.w(TAG, "No publish session to update")
            return
        }
        val serviceInfo = buildServiceInfo(yggdrasilPubkey, availableEndpoints)
        val config = PublishConfig.Builder()
            .setServiceName(SERVICE_NAME)
            .setServiceSpecificInfo(serviceInfo)
            .build()
        session.updatePublish(config)
        Log.d(TAG, "Updated publish info with ${availableEndpoints.size} endpoints")
    }

    /**
     * Get list of currently known peer public keys.
     */
    fun getDiscoveredPeers(): List<ByteArray> {
        pruneStalePeers()
        return discoveredPeers.values.map { it.pubkey }
    }

    // ============================================================
    // Session Management
    // ============================================================

    private fun attachAwareSession() {
        val manager = getAwareManager() ?: return

        manager.attach(object : AttachCallback() {
            override fun onAttached(session: WifiAwareSession) {
                Log.i(TAG, "WiFi Aware session attached successfully")
                awareSession.set(session)
            }

            override fun onAttachFailed() {
                Log.e(TAG, "Failed to attach WiFi Aware session")
                awareSession.set(null)
                // Retry after delay
                handler.postDelayed({ attachAwareSession() }, 30_000L)
            }
        }, handler)
    }

    // ============================================================
    // Publishing
    // ============================================================

    private fun startPublishing(yggdrasilPubkey: ByteArray, availableEndpoints: List<String>) {
        val session = awareSession.get() ?: run {
            Log.w(TAG, "Cannot publish: no Aware session")
            return
        }

        val serviceInfo = buildServiceInfo(yggdrasilPubkey, availableEndpoints)
        val config = PublishConfig.Builder()
            .setServiceName(SERVICE_NAME)
            .setServiceSpecificInfo(serviceInfo)
            .build()

        session.publish(config, object : DiscoverySessionCallback() {
            override fun onPublishStarted(session: PublishDiscoverySession) {
                Log.i(TAG, "NAN publish started successfully")
                publishSession.set(session)
                isPublishing.set(true)
            }

            override fun onSessionTerminated(reason: Int) {
                Log.w(TAG, "Publish session terminated: reason=$reason")
                publishSession.set(null)
                isPublishing.set(false)
                // Re-publish if still active
                if (isScanning.get() || isPublishing.get()) {
                    handler.postDelayed({
                        startPublishing(yggdrasilPubkey, availableEndpoints)
                    }, 10_000L)
                }
            }

            override fun onMessageReceived(peerHandle: PeerHandle, message: ByteArray) {
                Log.d(TAG, "Received message from peer: ${message.size} bytes")
                handlePeerMessage(peerHandle, message)
            }
        }, handler)
    }

    private fun stopPublishing() {
        publishSession.get()?.close()
        publishSession.set(null)
        isPublishing.set(false)
    }

    // ============================================================
    // Subscribing / Discovery
    // ============================================================

    private fun startScanning() {
        if (isScanning.getAndSet(true)) return
        Log.i(TAG, "Starting NAN scanning")

        // Start periodic scan cycle
        handler.post(scanRunnable)
    }

    private fun stopScanning() {
        if (!isScanning.getAndSet(false)) return
        Log.i(TAG, "Stopping NAN scanning")
        handler.removeCallbacks(scanRunnable)

        subscribeSession.get()?.close()
        subscribeSession.set(null)
    }

    private fun performScanCycle() {
        val session = awareSession.get() ?: return
        if (!shouldScan()) return

        // Close existing subscribe session to start fresh
        subscribeSession.get()?.close()
        subscribeSession.set(null)

        val config = SubscribeConfig.Builder()
            .setServiceName(SERVICE_NAME)
            .setSubscribeType(SubscribeConfig.SUBSCRIBE_TYPE_PASSIVE)
            .setTerminateNotificationEnabled(true)
            .build()

        session.subscribe(config, object : DiscoverySessionCallback() {
            override fun onSubscribeStarted(session: SubscribeDiscoverySession) {
                Log.d(TAG, "NAN subscribe session started")
                subscribeSession.set(session)
            }

            override fun onSessionTerminated(reason: Int) {
                Log.d(TAG, "Subscribe session terminated: reason=$reason")
                subscribeSession.set(null)
            }

            override fun onServiceDiscovered(
                peerHandle: PeerHandle,
                serviceSpecificInfo: ByteArray?,
                matchFilter: List<ServiceSpecificInfo>?
            ) {
                if (serviceSpecificInfo == null) {
                    Log.w(TAG, "Service discovered but no service-specific info")
                    return
                }
                Log.i(TAG, "NAN service discovered from peer, info=${serviceSpecificInfo.size} bytes")
                val peerData = parseServiceInfo(serviceSpecificInfo) ?: return
                addDiscoveredPeer(peerHandle, peerData.pubkey, peerData.endpoints)
            }

            override fun onServiceDiscovered(
                peerHandle: PeerHandle,
                serviceSpecificInfo: ByteArray?,
                matchFilter: List<ServiceSpecificInfo>?,
                characteristics: Characteristics?
            ) {
                // API 33+ overload with characteristics
                if (serviceSpecificInfo == null) return
                val peerData = parseServiceInfo(serviceSpecificInfo) ?: return
                addDiscoveredPeer(peerHandle, peerData.pubkey, peerData.endpoints)

                // Request a direct network connection
                requestNetworkToPeer(peerHandle)
            }

            override fun onMessageReceived(peerHandle: PeerHandle, message: ByteArray) {
                handlePeerMessage(peerHandle, message)
            }
        }, handler)
    }

    // ============================================================
    // Network Connection to Peer
    // ============================================================

    @SuppressLint("MissingPermission")
    private fun requestNetworkToPeer(peerHandle: PeerHandle) {
        val pubSession = publishSession.get()
        val subSession = subscribeSession.get()
        if (pubSession == null && subSession == null) {
            Log.w(TAG, "No active session for network request to peer")
            return
        }

        val builder = WifiAwareNetworkSpecifier.Builder(pubSession, peerHandle)
            .setPskPassphrase("UnifiedShield2024!")

        // Port is for the Rust daemon's listening socket on the NAN interface
        val specifier = builder.build()

        val request = NetworkRequest.Builder()
            .addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
            .setNetworkSpecifier(specifier)
            .build()

        val connectivityManager = context.getSystemService(Context.CONNECTIVITY_SERVICE)
            as ConnectivityManager

        // Close any existing callback for this peer
        networkCallback?.let { connectivityManager.unregisterNetworkCallback(it) }

        networkCallback = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                Log.i(TAG, "NAN network available to peer")
                updatePeerNetwork(peerHandle, network)
                // Notify Rust daemon about the new network path
                val peerInfo = discoveredPeers[peerHandle]
                if (peerInfo != null) {
                    jniBridge.onNanPeerConnected(
                        peerInfo.pubkey,
                        getIpv6LinkLocalFromNetwork(network)
                    )
                }
            }

            override fun onLost(network: Network) {
                Log.w(TAG, "NAN network lost to peer")
                updatePeerNetwork(peerHandle, null)
                val peerInfo = discoveredPeers[peerHandle]
                if (peerInfo != null) {
                    jniBridge.onNanPeerDisconnected(peerInfo.pubkey)
                }
            }

            override fun onUnavailable() {
                Log.w(TAG, "NAN network request unavailable for peer")
            }
        }

        connectivityManager.requestNetwork(request, networkCallback!!, handler, SUBSCRIBE_TIMEOUT_MS)
    }

    // ============================================================
    // Service Info Encoding / Decoding
    // ============================================================

    /**
     * Build the service-specific info payload for publishing.
     * Format:
     *   [1 byte version] [1 byte pubkey_len] [pubkey_len bytes pubkey] [remaining: endpoints as NUL-separated UTF-8]
     */
    private fun buildServiceInfo(yggdrasilPubkey: ByteArray, availableEndpoints: List<String>): ByteArray {
        val buffer = ByteBuffer.allocate(MAX_SERVICE_INFO_LENGTH)
        buffer.put(INFO_VERSION)
        buffer.put(yggdrasilPubkey.size.toByte())
        buffer.put(yggdrasilPubkey)

        // Encode endpoints as NUL-separated string
        val endpointsStr = availableEndpoints.joinToString("\u0000")
        val endpointsBytes = endpointsStr.toByteArray(StandardCharsets.UTF_8)
        val remaining = MAX_SERVICE_INFO_LENGTH - buffer.position()
        if (endpointsBytes.size <= remaining) {
            buffer.put(endpointsBytes)
        } else {
            buffer.put(endpointsBytes, 0, remaining)
        }

        val result = ByteArray(buffer.position())
        buffer.flip()
        buffer.get(result)
        return result
    }

    private data class ParsedServiceInfo(
        val pubkey: ByteArray,
        val endpoints: List<String>
    )

    private fun parseServiceInfo(data: ByteArray): ParsedServiceInfo? {
        if (data.size < 3) {
            Log.w(TAG, "Service info too short: ${data.size} bytes")
            return null
        }

        val buffer = ByteBuffer.wrap(data)
        val version = buffer.get().toInt()
        if (version != INFO_VERSION) {
            Log.w(TAG, "Unknown service info version: $version")
            return null
        }

        val pubkeyLen = buffer.get().toInt() and 0xFF
        if (pubkeyLen > INFO_PUBKEY_LEN + 32) { // Allow some flexibility
            Log.w(TAG, "Unreasonable pubkey length: $pubkeyLen")
            return null
        }

        if (buffer.remaining() < pubkeyLen) {
            Log.w(TAG, "Not enough data for pubkey: remaining=${buffer.remaining()}, needed=$pubkeyLen")
            return null
        }

        val pubkey = ByteArray(pubkeyLen)
        buffer.get(pubkey)

        val endpoints = if (buffer.hasRemaining()) {
            val remaining = ByteArray(buffer.remaining())
            buffer.get(remaining)
            String(remaining, StandardCharsets.UTF_8)
                .split('\u0000')
                .filter { it.isNotBlank() }
        } else {
            emptyList()
        }

        return ParsedServiceInfo(pubkey, endpoints)
    }

    // ============================================================
    // Peer Management
    // ============================================================

    private fun addDiscoveredPeer(peerHandle: PeerHandle, pubkey: ByteArray, endpoints: List<String>) {
        val now = System.currentTimeMillis()
        val existing = discoveredPeers[peerHandle]
        val network = existing?.network

        discoveredPeers[peerHandle] = PeerInfo(
            peerHandle = peerHandle,
            pubkey = pubkey,
            endpoints = endpoints,
            discoveredAt = now,
            network = network
        )

        Log.i(TAG, "Discovered NAN peer: pubkey=${pubkey.take(8).toHexString()}, " +
                "endpoints=${endpoints.size}, total_peers=${discoveredPeers.size}")

        // Report to Rust daemon via JNI
        jniBridge.onNanPeerDiscovered(pubkey, endpoints.toTypedArray())
    }

    private fun updatePeerNetwork(peerHandle: PeerHandle, network: Network?) {
        val existing = discoveredPeers[peerHandle] ?: return
        discoveredPeers[peerHandle] = existing.copy(network = network)
    }

    private fun pruneStalePeers() {
        val now = System.currentTimeMillis()
        val stalePeers = discoveredPeers.entries.filter {
            now - it.value.discoveredAt > PEER_STALE_TIMEOUT_MS
        }
        stalePeers.forEach { (handle, info) ->
            Log.d(TAG, "Pruning stale peer: ${info.pubkey.take(8).toHexString()}")
            discoveredPeers.remove(handle)
            jniBridge.onNanPeerLost(info.pubkey)
        }
    }

    // ============================================================
    // Message Handling
    // ============================================================

    private fun handlePeerMessage(peerHandle: PeerHandle, message: ByteArray) {
        val peerInfo = discoveredPeers[peerHandle]
        if (peerInfo == null) {
            Log.w(TAG, "Message from unknown peer handle")
            return
        }
        Log.d(TAG, "Handling message from peer: ${message.size} bytes")
        jniBridge.onNanPeerMessage(peerInfo.pubkey, message)
    }

    /**
     * Send a message to a discovered peer.
     */
    fun sendToPeer(peerHandle: PeerHandle, message: ByteArray): Boolean {
        val session = publishSession.get() ?: subscribeSession.get() ?: return false
        val peerInfo = discoveredPeers[peerHandle] ?: return false

        val messageId = System.currentTimeMillis().toInt() and 0xFFFF
        session.sendMessage(peerHandle, messageId, message)
        return true
    }

    // ============================================================
    // Scanning Logic & Battery Optimization
    // ============================================================

    private fun shouldScan(): Boolean {
        // Always scan during CompleteBlackout regardless of screen state
        if (nainController.state == NainState.COMPLETE_BLACKOUT) return true
        // Otherwise, only scan when screen is on and app is foregrounded
        return isScreenOn.get() && isAppForegrounded.get()
    }

    // ============================================================
    // Utility Methods
    // ============================================================

    @SuppressLint("MissingPermission")
    private fun isWifiAwareAvailable(): Boolean {
        val manager = getAwareManager() ?: return false
        return manager.isAvailable
    }

    private fun getAwareManager(): WifiAwareManager? {
        return context.getSystemService(Context.WIFI_AWARE_SERVICE) as? WifiAwareManager
    }

    private fun getIpv6LinkLocalFromNetwork(network: Network): String {
        // Try to get link-local address from the NAN network interface
        try {
            val linkProperties = (context.getSystemService(Context.CONNECTIVITY_SERVICE)
                as ConnectivityManager).getLinkProperties(network)
            linkProperties?.linkAddresses?.forEach { addr ->
                if (addr.address.isLinkLocalAddress) {
                    return addr.address.hostAddress ?: ""
                }
            }
        } catch (e: Exception) {
            Log.w(TAG, "Failed to get IPv6 link-local from network", e)
        }
        return ""
    }

    private fun ByteArray.toHexString(): String =
        joinToString("") { "%02x".format(it) }

    private fun ByteArray.take(n: Int): ByteArray = copyOfRange(0, minOf(n, size))

    // ============================================================
    // Cleanup
    // ============================================================

    private fun cleanupSessions() {
        publishSession.getAndSet(null)?.close()
        subscribeSession.getAndSet(null)?.close()
        awareSession.getAndSet(null)?.close()

        // Unregister network callback
        networkCallback?.let {
            try {
                val cm = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
                cm.unregisterNetworkCallback(it)
            } catch (_: Exception) { }
            networkCallback = null
        }

        discoveredPeers.clear()
    }

    private fun registerReceivers() {
        // WiFi Aware availability changes
        context.registerReceiver(awareAvailableReceiver,
            IntentFilter(WifiAwareManager.ACTION_WIFI_AWARE_STATE_CHANGED))

        // Screen on/off for battery optimization
        val screenFilter = IntentFilter().apply {
            addAction(Intent.ACTION_SCREEN_ON)
            addAction(Intent.ACTION_SCREEN_OFF)
        }
        context.registerReceiver(screenReceiver, screenFilter)
    }

    fun destroy() {
        stop()
        try {
            context.unregisterReceiver(awareAvailableReceiver)
        } catch (_: Exception) { }
        try {
            context.unregisterReceiver(screenReceiver)
        } catch (_: Exception) { }
    }
}
