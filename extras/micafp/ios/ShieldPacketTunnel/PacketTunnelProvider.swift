/*
 * MICAFP-UnifiedShield-6.0
 * PacketTunnelProvider.swift — iOS Network Extension packet tunnel
 *
 * NEPacketTunnelProvider subclass that creates a TUN interface for
 * routing all device traffic through the Shield tunnel. Integrates
 * with the Rust daemon via Unix domain socket IPC.
 *
 * Features:
 *   - NEPacketTunnelProvider (no root required)
 *   - NEPacketTunnelNetworkSettings for TUN interface
 *   - Packet flow via packetFlow.readPackets() / writePackets()
 *   - Rust daemon IPC via Unix domain socket
 *   - DNS routing through tunnel
 *   - BGTaskScheduler for periodic background work
 *   - Minimal background activity to avoid iOS killing the extension
 *
 * No root required. Cloudflare is NOT used.
 */

import NetworkExtension
import Foundation
import os.log

// MARK: - Rust Daemon IPC Protocol

/// Protocol for communicating with the Rust daemon via Unix domain socket.
protocol RustDaemonIPC {
    func connect(socketPath: String) -> Bool
    func sendPacket(_ packet: Data) -> Bool
    func receivePackets() -> [Data]
    func disconnect()
    func isConnected() -> Bool
}

// MARK: - Unix Domain Socket IPC

/// Implementation of Rust daemon IPC using Unix domain sockets.
class UnixSocketIPC: RustDaemonIPC {
    private var socketfd: Int32 = -1
    private let queue = DispatchQueue(label: "org.micafp.unifiedshield.ipc", qos: .userInitiated)
    private let logger = Logger(subsystem: "org.micafp.unifiedshield", category: "UnixSocketIPC")

    // IPC message header format:
    // [4 bytes: message_type] [4 bytes: payload_length] [payload_length bytes: payload]
    private let HEADER_SIZE = 8
    private let MSG_TYPE_PACKET_IN: UInt32 = 0x01
    private let MSG_TYPE_PACKET_OUT: UInt32 = 0x02
    private let MSG_TYPE_CONFIG: UInt32 = 0x03
    private let MSG_TYPE_STATUS: UInt32 = 0x04
    private let MSG_TYPE_PING: UInt32 = 0x05
    private let MSG_TYPE_PONG: UInt32 = 0x06

    func connect(socketPath: String) -> Bool {
        return queue.sync {
            // Create Unix domain socket
            socketfd = socket(AF_UNIX, SOCK_STREAM, 0)
            if socketfd < 0 {
                logger.error("Failed to create Unix domain socket")
                return false
            }

            // Connect to the Rust daemon's socket
            var addr = sockaddr_un()
            addr.sun_family = sa_family_t(AF_UNIX)
            let pathData = socketPath.utf8CString
            guard pathData.count <= MemoryLayout.size(ofValue: addr.sun_path) else {
                logger.error("Socket path too long: \(socketPath)")
                close(socketfd)
                socketfd = -1
                return false
            }
            _ = withUnsafeMutableBytes(of: &addr.sun_path) { dest in
                pathData.withUnsafeBytes { src in
                    memcpy(dest.baseAddress, src.baseAddress, min(dest.count, src.count))
                }
            }

            let connectResult = withUnsafePointer(to: &addr) { ptr in
                ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sockaddrPtr in
                    connect(socketfd, sockaddrPtr, socklen_t(MemoryLayout<sockaddr_un>.size))
                }
            }

            if connectResult < 0 {
                logger.error("Failed to connect to Rust daemon at \(socketPath): \(String(cString: strerror(errno)))")
                close(socketfd)
                socketfd = -1
                return false
            }

            logger.info("Connected to Rust daemon at \(socketPath)")
            return true
        }
    }

    func sendPacket(_ packet: Data) -> Bool {
        return queue.sync {
            guard socketfd >= 0 else { return false }

            // Build IPC message: [type: PACKET_IN] [length] [packet data]
            var header = Data(capacity: HEADER_SIZE)
            var msgType: UInt32 = MSG_TYPE_PACKET_IN
            var payloadLen: UInt32 = UInt32(packet.count)
            header.append(contentsOf: withUnsafeBytes(of: &msgType) { Array($0) })
            header.append(contentsOf: withUnsafeBytes(of: &payloadLen) { Array($0) })

            let message = header + packet
            let written = message.withUnsafeBytes { ptr in
                write(socketfd, ptr.baseAddress, message.count)
            }

            if written < 0 {
                logger.error("Failed to send packet to daemon: \(String(cString: strerror(errno)))")
                return false
            }
            return written == message.count
        }
    }

    func receivePackets() -> [Data] {
        return queue.sync {
            guard socketfd >= 0 else { return [] }

            var packets: [Data] = []

            // Non-blocking read: check if data is available
            var pollfd = pollfd(fd: socketfd, events: Int16(POLLIN), revents: 0)
            let pollResult = poll(&pollfd, 1, 0) // 0 = non-blocking

            guard pollResult > 0 && (pollfd.revents & Int16(POLLIN)) != 0 else {
                return packets
            }

            // Read available messages
            while true {
                // Read header
                var headerData = Data(capacity: HEADER_SIZE)
                let headerRead = headerData.withUnsafeMutableBytes { ptr in
                    read(socketfd, ptr.baseAddress, HEADER_SIZE)
                }

                guard headerRead == HEADER_SIZE else {
                    break // No more complete messages
                }

                // Parse header
                let msgType = headerData.withUnsafeBytes { ptr -> UInt32 in
                    ptr.load(as: UInt32.self)
                }
                let payloadLen = headerData.dropFirst(4).withUnsafeBytes { ptr -> UInt32 in
                    ptr.load(as: UInt32.self)
                }

                guard payloadLen > 0 && payloadLen <= 65535 else {
                    logger.error("Invalid payload length: \(payloadLen)")
                    break
                }

                // Read payload
                var payload = Data(capacity: Int(payloadLen))
                let payloadRead = payload.withUnsafeMutableBytes { ptr in
                    read(socketfd, ptr.baseAddress, Int(payloadLen))
                }

                guard payloadRead == Int(payloadLen) else {
                    logger.error("Incomplete payload read: expected \(payloadLen), got \(payloadRead)")
                    break
                }

                switch msgType {
                case MSG_TYPE_PACKET_OUT:
                    packets.append(payload)
                case MSG_TYPE_STATUS:
                    // Handle status messages from daemon
                    handleStatusMessage(payload)
                case MSG_TYPE_PONG:
                    logger.debug("Received pong from daemon")
                default:
                    logger.warning("Unknown message type from daemon: \(msgType)")
                }

                // Don't process too many packets in one batch to avoid blocking
                if packets.count >= 64 {
                    break
                }
            }

            return packets
        }
    }

    func disconnect() {
        queue.sync {
            if socketfd >= 0 {
                close(socketfd)
                socketfd = -1
                logger.info("Disconnected from Rust daemon")
            }
        }
    }

    func isConnected() -> Bool {
        return queue.sync {
            return socketfd >= 0
        }
    }

    /// Send a ping to the Rust daemon to check connectivity.
    func sendPing() -> Bool {
        guard socketfd >= 0 else { return false }

        var header = Data(capacity: HEADER_SIZE)
        var msgType: UInt32 = MSG_TYPE_PING
        var payloadLen: UInt32 = 0
        header.append(contentsOf: withUnsafeBytes(of: &msgType) { Array($0) })
        header.append(contentsOf: withUnsafeBytes(of: &payloadLen) { Array($0) })

        let written = header.withUnsafeBytes { ptr in
            write(socketfd, ptr.baseAddress, header.count)
        }
        return written == header.count
    }

    /// Send configuration to the Rust daemon.
    func sendConfig(_ config: Data) -> Bool {
        guard socketfd >= 0 else { return false }

        var header = Data(capacity: HEADER_SIZE)
        var msgType: UInt32 = MSG_TYPE_CONFIG
        var payloadLen: UInt32 = UInt32(config.count)
        header.append(contentsOf: withUnsafeBytes(of: &msgType) { Array($0) })
        header.append(contentsOf: withUnsafeBytes(of: &payloadLen) { Array($0) })

        let message = header + config
        let written = message.withUnsafeBytes { ptr in
            write(socketfd, ptr.baseAddress, message.count)
        }
        return written == message.count
    }

    private func handleStatusMessage(_ data: Data) {
        guard let statusStr = String(data: data, encoding: .utf8) else { return }
        logger.info("Daemon status: \(statusStr)")
    }
}

// MARK: - PacketTunnelProvider

/// Shield packet tunnel provider for iOS Network Extension.
///
/// Creates a virtual network interface that routes all device traffic
/// through the Shield tunnel. Packets are processed by the Rust daemon
/// via Unix domain socket IPC.
class PacketTunnelProvider: NEPacketTunnelProvider {

    // MARK: - Properties

    private let logger = Logger(subsystem: "org.micafp.unifiedshield", category: "PacketTunnel")
    private let ipc = UnixSocketIPC()

    // Rust daemon socket path (within the extension's container)
    private let daemonSocketPath = "/var/run/shield-daemon.sock"

    // Tunnel configuration
    private var tunnelAddress: String = "10.77.0.1"
    private var tunnelAddress6: String = "fd00::1"
    private var tunnelRemoteAddress: String = "10.77.0.2"
    private var tunnelRemoteAddress6: String = "fd00::2"
    private var dnsServers: [String] = ["1.1.1.1", "9.9.9.9"]
    private var mtu: Int = 1500

    // Packet processing
    private var packetProcessingActive = false
    private let processingQueue = DispatchQueue(
        label: "org.micafp.unifiedshield.packet-processing",
        qos: .userInitiated,
        attributes: .concurrent
    )

    // Background task scheduling
    private var backgroundTaskIdentifier: UIBackgroundTaskIdentifier = .invalid

    // Keepalive timer
    private var keepaliveTimer: DispatchSourceTimer?

    // Reconnect state
    private var shouldReconnect = false
    private var reconnectAttempts = 0
    private let maxReconnectAttempts = 5

    // MARK: - Tunnel Lifecycle

    /// Called when the tunnel is started by the system or the container app.
    override func startTunnel(options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        logger.info("Starting Shield packet tunnel")

        // Parse tunnel options
        if let options = options {
            parseTunnelOptions(options)
        }

        // Connect to the Rust daemon
        guard ipc.connect(socketPath: daemonSocketPath) else {
            logger.error("Failed to connect to Rust daemon")
            completionHandler(PacketTunnelError.daemonConnectionFailed)
            return
        }

        // Send configuration to daemon
        sendTunnelConfigToDaemon()

        // Set up the tunnel network settings
        let networkSettings = createTunnelNetworkSettings()

        setTunnelNetworkSettings(networkSettings) { [weak self] error in
            guard let self = self else { return }

            if let error = error {
                self.logger.error("Failed to set tunnel network settings: \(error.localizedDescription)")
                self.ipc.disconnect()
                completionHandler(error)
                return
            }

            self.logger.info("Tunnel network settings applied successfully")

            // Start packet processing
            self.packetProcessingActive = true
            self.startPacketProcessing()
            self.startKeepaliveTimer()
            self.scheduleBackgroundTasks()

            completionHandler(nil)
        }
    }

    /// Called when the tunnel is stopped by the system or the container app.
    override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        logger.info("Stopping Shield packet tunnel, reason: \(reason.rawValue)")

        packetProcessingActive = false
        stopKeepaliveTimer()
        cancelBackgroundTasks()

        // Notify daemon of tunnel stop
        ipc.disconnect()

        completionHandler()
    }

    /// Called when the tunnel configuration is updated.
    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?) {
        logger.debug("Received app message: \(messageData.count) bytes")

        guard let message = try? JSONSerialization.jsonObject(with: messageData) as? [String: Any] else {
            completionHandler?(nil)
            return
        }

        switch message["command"] as? String {
        case "status":
            let status = getTunnelStatus()
            let responseData = try? JSONSerialization.data(withJSONObject: status)
            completionHandler?(responseData)

        case "reconnect":
            shouldReconnect = true
            reconnectAttempts = 0
            reconnectToDaemon()
            completionHandler?(nil)

        case "config":
            if let config = message["config"] as? [String: Any] {
                applyRuntimeConfig(config)
            }
            completionHandler?(nil)

        case "ping":
            let _ = ipc.sendPing()
            completionHandler?(Data("pong".utf8))

        default:
            completionHandler?(nil)
        }
    }

    /// Called when the system wakes from sleep.
    override func wake() {
        logger.info("System wake — resuming packet processing")

        if !ipc.isConnected() {
            reconnectToDaemon()
        }

        if !packetProcessingActive {
            packetProcessingActive = true
            startPacketProcessing()
        }
    }

    /// Called when the system is going to sleep.
    override func sleep(completionHandler: @escaping () -> Void) {
        logger.info("System sleep — pausing packet processing")

        packetProcessingActive = false
        stopKeepaliveTimer()

        completionHandler()
    }

    // MARK: - Network Settings

    /// Create NEPacketTunnelNetworkSettings for the TUN interface.
    private func createTunnelNetworkSettings() -> NEPacketTunnelNetworkSettings {
        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: tunnelRemoteAddress)

        // IPv4 settings
        let ipv4Settings = NEIPv4Settings(
            addresses: [tunnelAddress],
            subnetMasks: ["255.255.255.0"]
        )
        ipv4Settings.includedRoutes = [NEIPv4Route.default()]
        ipv4Settings.excludedRoutes = []
        settings.ipv4Settings = ipv4Settings

        // IPv6 settings
        let ipv6Settings = NEIPv6Settings(
            addresses: [tunnelAddress6],
            networkPrefixLengths: [64]
        )
        ipv6Settings.includedRoutes = [NEIPv6Route.default()]
        ipv6Settings.excludedRoutes = []
        settings.ipv6Settings = ipv6Settings

        // DNS settings — route all DNS through the tunnel
        let dnsSettings = NEDNSSettings(servers: dnsServers)
        dnsSettings.matchDomains = [""]  // Match all domains
        dnsSettings.matchDomainsNoSearch = true
        settings.dnsSettings = dnsSettings

        // Proxy settings — no proxy (traffic goes through tunnel)
        settings.proxySettings = nil

        // MTU
        settings.mtu = NSNumber(value: mtu)

        return settings
    }

    // MARK: - Packet Processing

    /// Start reading packets from the TUN interface and forwarding to the Rust daemon.
    private func startPacketProcessing() {
        logger.info("Starting packet processing loop")

        processingQueue.async { [weak self] in
            self?.readPacketsLoop()
        }

        processingQueue.async { [weak self] in
            self?.writePacketsLoop()
        }
    }

    /// Loop that reads packets from the TUN interface and sends them to the Rust daemon.
    private func readPacketsLoop() {
        while packetProcessingActive {
            guard let protocolObjects = try? self.packetFlow.readPackets() else {
                if packetProcessingActive {
                    logger.error("Failed to read packets from TUN")
                    // Brief pause before retry
                    Thread.sleep(forTimeInterval: 0.1)
                }
                continue
            }

            let (packets, protocols) = protocolObjects

            for (index, packet) in packets.enumerated() {
                guard packetProcessingActive else { return }

                // Forward packet to Rust daemon via IPC
                if !ipc.sendPacket(packet) {
                    logger.warning("Failed to send packet to daemon (\(packet.count) bytes)")
                    // Attempt to reconnect
                    handleDaemonDisconnection()
                    return
                }
            }
        }
    }

    /// Loop that receives processed packets from the Rust daemon and writes them to the TUN interface.
    private func writePacketsLoop() {
        while packetProcessingActive {
            let packets = ipc.receivePackets()

            if !packets.isEmpty {
                // Write packets back to the TUN interface
                let protocols = [NSNumber](repeating: NSNumber(value: AF_INET), count: packets.count)
                self.packetFlow.writePackets(packets, withProtocols: protocols)
            } else {
                // No packets available — brief sleep to avoid busy-waiting
                Thread.sleep(forTimeInterval: 0.005)
            }
        }
    }

    // MARK: - Daemon Connection Management

    private func handleDaemonDisconnection() {
        logger.warning("Lost connection to Rust daemon")

        if shouldReconnect && reconnectAttempts < maxReconnectAttempts {
            reconnectAttempts += 1
            let delay = min(1.0 * Double(reconnectAttempts), 30.0)

            logger.info("Attempting reconnect in \(delay)s (attempt \(reconnectAttempts)/\(maxReconnectAttempts))")

            DispatchQueue.global().asyncAfter(deadline: .now() + delay) { [weak self] in
                self?.reconnectToDaemon()
            }
        } else {
            // Cancel the tunnel since we can't connect to the daemon
            logger.error("Cannot reconnect to daemon, canceling tunnel")
            cancelTunnelWithError(PacketTunnelError.daemonConnectionLost)
        }
    }

    private func reconnectToDaemon() {
        ipc.disconnect()

        if ipc.connect(socketPath: daemonSocketPath) {
            logger.info("Reconnected to Rust daemon")
            reconnectAttempts = 0
            sendTunnelConfigToDaemon()

            if !packetProcessingActive {
                packetProcessingActive = true
                startPacketProcessing()
            }
        } else {
            handleDaemonDisconnection()
        }
    }

    // MARK: - Configuration

    private func parseTunnelOptions(_ options: [String: NSObject]) {
        if let addr = options["tunnelAddress"] as? NSString as? String {
            tunnelAddress = addr
        }
        if let addr6 = options["tunnelAddress6"] as? NSString as? String {
            tunnelAddress6 = addr6
        }
        if let remoteAddr = options["remoteAddress"] as? NSString as? String {
            tunnelRemoteAddress = remoteAddr
        }
        if let remoteAddr6 = options["remoteAddress6"] as? NSString as? String {
            tunnelRemoteAddress6 = remoteAddr6
        }
        if let dns = options["dnsServers"] as? NSArray as? [String] {
            dnsServers = dns
        }
        if let newMtu = options["mtu"] as? NSNumber as? Int {
            mtu = newMtu
        }
    }

    private func sendTunnelConfigToDaemon() {
        let config: [String: Any] = [
            "tunnel_address": tunnelAddress,
            "tunnel_address6": tunnelAddress6,
            "remote_address": tunnelRemoteAddress,
            "remote_address6": tunnelRemoteAddress6,
            "dns_servers": dnsServers,
            "mtu": mtu
        ]

        guard let configData = try? JSONSerialization.data(withJSONObject: config) else {
            logger.error("Failed to serialize tunnel config")
            return
        }

        if !ipc.sendConfig(configData) {
            logger.warning("Failed to send config to daemon")
        }
    }

    private func applyRuntimeConfig(_ config: [String: Any]) {
        logger.info("Applying runtime configuration update")

        if let nainInterval = config["nain_interval"] as? Int {
            // Forward NAIN configuration to daemon
            let configData: [String: Any] = ["nain_interval": nainInterval]
            if let data = try? JSONSerialization.data(withJSONObject: configData) {
                _ = ipc.sendConfig(data)
            }
        }

        if let scanMode = config["scan_mode"] as? String {
            let configData: [String: Any] = ["scan_mode": scanMode]
            if let data = try? JSONSerialization.data(withJSONObject: configData) {
                _ = ipc.sendConfig(data)
            }
        }
    }

    // MARK: - Keepalive

    private func startKeepaliveTimer() {
        stopKeepaliveTimer()

        let timer = DispatchSource.makeTimerSource(
            queue: DispatchQueue(label: "org.micafp.unifiedshield.keepalive")
        )
        timer.schedule(deadline: .now() + 25, repeating: .seconds(25))
        timer.setEventHandler { [weak self] in
            guard let self = self else { return }
            if !self.ipc.sendPing() {
                self.logger.warning("Keepalive ping failed")
            }
        }
        timer.resume()
        keepaliveTimer = timer
    }

    private func stopKeepaliveTimer() {
        keepaliveTimer?.cancel()
        keepaliveTimer = nil
    }

    // MARK: - Background Task Scheduling

    /// Schedule background tasks for periodic connectivity checks.
    /// Uses BGTaskScheduler for periodic work when the extension is not actively processing packets.
    private func scheduleBackgroundTasks() {
        // Schedule a BGAppRefreshTask for periodic connectivity checks
        let request = BGAppRefreshTaskRequest(identifier: "org.micafp.unifiedshield.connectivity-check")
        request.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60) // 15 minutes minimum

        do {
            try BGTaskScheduler.shared.submit(request)
            logger.debug("Scheduled background app refresh task")
        } catch {
            logger.warning("Failed to schedule background task: \(error.localizedDescription)")
        }
    }

    private func cancelBackgroundTasks() {
        BGTaskScheduler.shared.cancel(taskRequestWithIdentifier: "org.micafp.unifiedshield.connectivity-check")
    }

    // MARK: - Status

    private func getTunnelStatus() -> [String: Any] {
        return [
            "connected": ipc.isConnected(),
            "packetProcessing": packetProcessingActive,
            "reconnectAttempts": reconnectAttempts
        ]
    }
}

// MARK: - Error Types

enum PacketTunnelError: Error, LocalizedError {
    case daemonConnectionFailed
    case daemonConnectionLost
    case tunnelSetupFailed
    case invalidConfiguration

    var errorDescription: String? {
        switch self {
        case .daemonConnectionFailed:
            return "Failed to connect to Shield daemon"
        case .daemonConnectionLost:
            return "Lost connection to Shield daemon"
        case .tunnelSetupFailed:
            return "Failed to set up tunnel interface"
        case .invalidConfiguration:
            return "Invalid tunnel configuration"
        }
    }
}

// MARK: - BGTaskScheduler Helpers

/// Register background task handlers in the main app.
/// This must be called from the app delegate during application launch.
class ShieldBackgroundTaskRegistrar {

    static func registerBackgroundTasks() {
        // Register the BGAppRefreshTask handler
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: "org.micafp.unifiedshield.connectivity-check",
            using: nil
        ) { task in
            handleConnectivityCheck(task: task as! BGAppRefreshTask)
        }

        // Register BGProcessingTask for heavy work when charging
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: "org.micafp.unifiedshield.processing",
            using: nil
        ) { task in
            handleProcessingTask(task: task as! BGProcessingTask)
        }
    }

    private static func handleConnectivityCheck(task: BGAppRefreshTask) {
        // Schedule the next background task
        let nextRequest = BGAppRefreshTaskRequest(identifier: "org.micafp.unifiedshield.connectivity-check")
        nextRequest.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60)
        try? BGTaskScheduler.shared.submit(nextRequest)

        // Perform a lightweight connectivity check
        task.expirationHandler = {
            task.setTaskCompleted(success: false)
        }

        // The actual check goes through the Rust daemon IPC
        // For now, just mark as completed
        task.setTaskCompleted(success: true)
    }

    private static func handleProcessingTask(task: BGProcessingTask) {
        task.expirationHandler = {
            task.setTaskCompleted(success: false)
        }

        // Heavy processing work when device is charging
        task.setTaskCompleted(success: true)
    }
}
