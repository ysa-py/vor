import Foundation
import NetworkExtension
import Network
import UIKit

/**
 * PacketTunnelProvider — iOS Network Extension
 *
 * Implements the NEPacketTunnelProvider protocol to create a full-tunnel VPN
 * on iOS. Configures the TUN interface with IPv4/IPv6, DNS, and MTU settings,
 * handles packet flow from the NEPacketTunnelProvider, routes packets through
 * the Rust daemon via FFI, and manages tunnel lifecycle including system
 * sleep/wake events.
 *
 * Integrates BGTaskScheduler for periodic updates and heavy background processing,
 * AVAudioEngine for acoustic channel receive, and battery-aware operation modes.
 * Provides a "Paste Config Code" UI for SMS fallback since iOS cannot read SMS
 * programmatically.
 */
class PacketTunnelProvider: NEPacketTunnelProvider {

    // MARK: - Configuration Constants

    private let tunnelAddress4 = "10.0.0.2"
    private let tunnelAddress6 = "fd00::2"
    private let subnetMask4 = "255.255.255.255"
    private let dnsServers = ["1.1.1.1", "1.0.0.1"]
    private let dnsServers6 = ["2606:4700:4700::1111", "2606:4700:4700::1001"]
    private let mtu: Int = 1500
    private let includedRoutes4 = "0.0.0.0"
    private let includedRoutes4Prefix: Int = 0
    private let includedRoutes6 = "::"
    private let includedRoutes6Prefix: Int = 0

    // MARK: - State

    private var isRunning = false
    private var rustTunnelHandle: Int32 = -1
    private var packetProcessingTimer: DispatchSourceTimer?
    private var lastWakeTime: Date = Date()
    private var acousticReceiver: AcousticReceiver?
    private var batteryManager: BatteryManager?

    // MARK: - Rust FFI Bindings

    private typealias RustStartTunnel = @convention(c) (Int32) -> Int32
    private typealias RustStopTunnel = @convention(c) (Int32) -> Void
    private typealias RustProcessPacket = @convention(c) (Int32, UnsafePointer<UInt8>, Int) -> Void
    private typealias RustGetStatus = @convention(c) () -> Int32
    private typealias RustSetPowerState = @convention(c) (Int32) -> Void
    private typealias RustUpdateEndpoints = @convention(c) (UnsafePointer<CChar>) -> Int32
    private typealias RustTriggerWipe = @convention(c) () -> Void

    private var rustStartTunnel: RustStartTunnel?
    private var rustStopTunnel: RustStopTunnel?
    private var rustProcessPacket: RustProcessPacket?
    private var rustGetStatus: RustGetStatus?
    private var rustSetPowerState: RustSetPowerState?
    private var rustUpdateEndpoints: RustUpdateEndpoints?
    private var rustTriggerWipe: RustTriggerWipe?

    // MARK: - Tunnel Lifecycle

    override func startTunnel(options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        NSLog("[Shield] PacketTunnelProvider: startTunnel called")

        loadRustLibrary()

        // Configure the TUN interface
        let tunnelNetworkSettings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "127.0.0.1")

        // IPv4 settings
        let ipv4Settings = NEIPv4Settings(
            addresses: [tunnelAddress4],
            subnetMasks: [subnetMask4]
        )
        ipv4Settings.includedRoutes = [NEIPv4Route.default()]
        ipv4Settings.excludedRoutes = []

        // IPv6 settings
        let ipv6Settings = NEIPv6Settings(
            addresses: [tunnelAddress6],
            networkPrefixLengths: [128]
        )
        ipv6Settings.includedRoutes = [NEIPv6Route.default()]
        ipv6Settings.excludedRoutes = []

        // DNS settings
        let dnsSettings = NEDNSSettings(servers: dnsServers + dnsServers6)
        dnsSettings.matchDomains = [""]  // Route all DNS through tunnel
        dnsSettings.matchDomainsNoSearch = true

        // Apply settings
        tunnelNetworkSettings.ipv4Settings = ipv4Settings
        tunnelNetworkSettings.ipv6Settings = ipv6Settings
        tunnelNetworkSettings.dnsSettings = dnsSettings
        tunnelNetworkSettings.mtu = NSNumber(value: mtu)

        // Check for pasted config code (iOS SMS fallback)
        if let configCode = options?["config_code"] as? String {
            NSLog("[Shield] Config code received from paste, updating endpoints")
            updateEndpointsFromConfigCode(configCode)
        }

        // Set the tunnel settings
        setTunnelNetworkSettings(tunnelNetworkSettings) { [weak self] error in
            guard let self = self else { return }

            if let error = error {
                NSLog("[Shield] Failed to set tunnel network settings: \(error)")
                completionHandler(error)
                return
            }

            NSLog("[Shield] Tunnel network settings applied successfully")

            // Start the Rust daemon tunnel
            self.startRustTunnel()

            // Start packet processing loop
            self.startPacketProcessing()

            // Initialize battery manager
            self.batteryManager = BatteryManager()
            self.batteryManager?.delegate = self
            self.batteryManager?.startMonitoring()

            // Initialize acoustic receiver
            self.acousticReceiver = AcousticReceiver()
            self.acousticReceiver?.delegate = self
            self.acousticReceiver?.startListening()

            // Register background tasks
            self.registerBackgroundTasks()

            // Set up system sleep/wake notifications
            self.registerSleepWakeNotifications()

            self.isRunning = true
            completionHandler(nil)
        }
    }

    override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        NSLog("[Shield] PacketTunnelProvider: stopTunnel called with reason: \(reason.rawValue)")

        isRunning = false

        // Stop packet processing
        packetProcessingTimer?.cancel()
        packetProcessingTimer = nil

        // Stop Rust tunnel
        if rustTunnelHandle >= 0, let stopFn = rustStopTunnel {
            stopFn(rustTunnelHandle)
            rustTunnelHandle = -1
        }

        // Stop acoustic receiver
        acousticReceiver?.stopListening()
        acousticReceiver = nil

        // Stop battery monitoring
        batteryManager?.stopMonitoring()
        batteryManager = nil

        // Cancel background tasks
        cancelBackgroundTasks()

        // Remove sleep/wake notifications
        removeSleepWakeNotifications()

        completionHandler()
    }

    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?) {
        // Handle messages from the container app
        guard let message = try? JSONSerialization.jsonObject(with: messageData) as? [String: Any] else {
            completionHandler?(nil)
            return
        }

        let action = message["action"] as? String ?? ""

        switch action {
        case "get_status":
            let status = rustGetStatus?() ?? 0
            let statusData = try? JSONSerialization.data(withJSONObject: [
                "status": status,
                "connected": status == 2,
                "transport": "shield"
            ])
            completionHandler?(statusData)

        case "update_endpoints":
            if let endpoints = message["endpoints"] as? String {
                let result = rustUpdateEndpoints?(endpoints) ?? -1
                let responseData = try? JSONSerialization.data(withJSONObject: ["result": result])
                completionHandler?(responseData)
            } else {
                completionHandler?(nil)
            }

        case "trigger_wipe":
            rustTriggerWipe?()
            completionHandler?(Data())

        case "paste_config_code":
            if let code = message["config_code"] as? String {
                updateEndpointsFromConfigCode(code)
                completionHandler?(Data())
            } else {
                completionHandler?(nil)
            }

        default:
            completionHandler?(nil)
        }
    }

    override func sleep(completionHandler: @escaping () -> Void) {
        NSLog("[Shield] System going to sleep")

        // Reduce tunnel activity
        rustSetPowerState?(2) // SCREEN_OFF_DEEP

        // Stop acoustic listening (saves battery during sleep)
        acousticReceiver?.stopListening()

        completionHandler()
    }

    override func wake() {
        NSLog("[Shield] System waking up from sleep")
        lastWakeTime = Date()

        // Resume tunnel activity based on battery state
        if let battery = batteryManager {
            if battery.isCharging {
                rustSetPowerState?(3) // CHARGING
            } else if battery.batteryLevel > 15 {
                rustSetPowerState?(0) // SCREEN_ON
            } else {
                rustSetPowerState?(2) // SCREEN_OFF_DEEP
            }
        } else {
            rustSetPowerState?(0) // SCREEN_ON
        }

        // Resume acoustic listening if battery permits
        if batteryManager?.shouldEnableAcousticChannel() == true {
            acousticReceiver?.startListening()
        }

        // Process any queued packets
        readPackets()
    }

    // MARK: - Packet Processing

    private func startPacketProcessing() {
        let timer = DispatchSource.makeTimerSource(queue: DispatchQueue(label: "shield.packet-processing"))

        timer.schedule(deadline: .now(), repeating: .milliseconds(10))

        timer.setEventHandler { [weak self] in
            self?.readPackets()
        }

        timer.resume()
        packetProcessingTimer = timer
    }

    private func readPackets() {
        guard isRunning else { return }

        packetFlow.readPackets { [weak self] packets, protocols in
            guard let self = self, self.isRunning else { return }

            for (index, packet) in packets.enumerated() {
                let protocolFamily = protocols[index]

                // Pass packet to Rust daemon for processing
                packet.withUnsafeBytes { rawBufferPointer in
                    if let baseAddress = rawBufferPointer.baseAddress {
                        let pointer = baseAddress.assumingMemoryBound(to: UInt8.self)
                        self.rustProcessPacket?(self.rustTunnelHandle, pointer, packet.count)
                    }
                }
            }
        }
    }

    /// Write a processed packet back to the TUN interface.
    /// Called by the Rust daemon via FFI callback.
    func writePacket(_ data: Data, protocolFamily: NSNumber) {
        guard isRunning else { return }
        packetFlow.writePackets([data], withProtocols: [protocolFamily])
    }

    // MARK: - Rust FFI Integration

    private func loadRustLibrary() {
        // Load the Rust shared library
        let libraryPath = Bundle.main.bundlePath + "/Frameworks/libshield_native.dylib"

        guard let handle = dlopen(libraryPath, RTLD_NOW) else {
            NSLog("[Shield] Failed to load Rust library: \(String(cString: dlerror()))")
            return
        }

        // Load function pointers
        rustStartTunnel = unsafeBitCast(dlsym(handle, "shield_start_tunnel"), to: RustStartTunnel.self)
        rustStopTunnel = unsafeBitCast(dlsym(handle, "shield_stop_tunnel"), to: RustStopTunnel.self)
        rustProcessPacket = unsafeBitCast(dlsym(handle, "shield_process_packet"), to: RustProcessPacket.self)
        rustGetStatus = unsafeBitCast(dlsym(handle, "shield_get_status"), to: RustGetStatus.self)
        rustSetPowerState = unsafeBitCast(dlsym(handle, "shield_set_power_state"), to: RustSetPowerState.self)
        rustUpdateEndpoints = unsafeBitCast(dlsym(handle, "shield_update_endpoints"), to: RustUpdateEndpoints.self)
        rustTriggerWipe = unsafeBitCast(dlsym(handle, "shield_trigger_wipe"), to: RustTriggerWipe.self)

        NSLog("[Shield] Rust library loaded and FFI functions bound")
    }

    private func startRustTunnel() {
        guard let startFn = rustStartTunnel else {
            NSLog("[Shield] Rust start function not available")
            return
        }

        // The TUN file descriptor is available via packetFlow
        let handle = startFn(rustTunnelHandle)
        if handle < 0 {
            NSLog("[Shield] Rust tunnel failed to start (error: \(handle))")
            return
        }

        rustTunnelHandle = handle
        NSLog("[Shield] Rust tunnel started with handle: \(handle)")
    }

    // MARK: - Config Code Processing (iOS SMS Fallback)

    private func updateEndpointsFromConfigCode(_ code: String) {
        // The config code is a base64-encoded, AES-GCM encrypted endpoint list
        // Similar to the Android SMS bootstrap, but manually pasted by the user
        guard let data = Data(base64Encoded: code) else {
            NSLog("[Shield] Invalid base64 in config code")
            return
        }

        // Pass to Rust daemon for decryption and processing
        let result = code.withCString { cString in
            return rustUpdateEndpoints?(cString) ?? -1
        }

        if result == 0 {
            NSLog("[Shield] Endpoints updated from pasted config code")
        } else {
            NSLog("[Shield] Failed to update endpoints from config code (error: \(result))")
        }
    }

    // MARK: - Background Task Scheduling

    private func registerBackgroundTasks() {
        // Register for BGAppRefreshTask — periodic lightweight updates
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: "com.shield.refresh",
            using: nil
        ) { [weak self] task in
            self?.handleAppRefresh(task: task as! BGAppRefreshTask)
        }

        // Register for BGProcessingTask — heavy operations when charging
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: "com.shield.processing",
            using: nil
        ) { [weak self] task in
            self?.handleProcessing(task: task as! BGProcessingTask)
        }

        scheduleBackgroundTasks()
    }

    private func scheduleBackgroundTasks() {
        // Schedule app refresh
        let refreshRequest = BGAppRefreshTaskRequest(identifier: "com.shield.refresh")
        refreshRequest.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60) // 15 min minimum

        do {
            try BGTaskScheduler.shared.submit(refreshRequest)
            NSLog("[Shield] BGAppRefreshTask scheduled")
        } catch {
            NSLog("[Shield] Failed to schedule BGAppRefreshTask: \(error)")
        }

        // Schedule processing task (requires charging + idle)
        let processingRequest = BGProcessingTaskRequest(identifier: "com.shield.processing")
        processingRequest.earliestBeginDate = Date(timeIntervalSinceNow: 60 * 60) // 1 hour minimum
        processingRequest.requiresNetworkConnectivity = true
        processingRequest.requiresExternalPower = true
        processingRequest.requiresDeviceConnected = true

        do {
            try BGTaskScheduler.shared.submit(processingRequest)
            NSLog("[Shield] BGProcessingTask scheduled")
        } catch {
            NSLog("[Shield] Failed to schedule BGProcessingTask: \(error)")
        }
    }

    private func handleAppRefresh(task: BGAppRefreshTask) {
        // Schedule the next refresh
        scheduleBackgroundTasks()

        // Set expiration handler
        task.expirationHandler = {
            NSLog("[Shield] BGAppRefreshTask expired")
        }

        // Perform lightweight update: check endpoint health
        if rustGetStatus?() == 2 { // Connected
            // Probe endpoints
            rustSetPowerState?(1) // Briefly increase activity
            DispatchQueue.global().asyncAfter(deadline: .now() + 5) { [weak self] in
                self?.rustSetPowerState?(2) // Back to minimal
                task.setTaskCompleted(success: true)
            }
        } else {
            task.setTaskCompleted(success: false)
        }
    }

    private func handleProcessing(task: BGProcessingTask) {
        scheduleBackgroundTasks()

        task.expirationHandler = {
            NSLog("[Shield] BGProcessingTask expired")
        }

        // Heavy operation: endpoint discovery, key rotation, acoustic listening
        rustSetPowerState?(3) // CHARGING mode

        // Start acoustic channel briefly for config discovery
        acousticReceiver?.startListening()

        DispatchQueue.global().asyncAfter(deadline: .now() + 30) { [weak self] in
            self?.acousticReceiver?.stopListening()
            self?.rustSetPowerState?(2)
            task.setTaskCompleted(success: true)
        }
    }

    private func cancelBackgroundTasks() {
        BGTaskScheduler.shared.cancelAllTaskRequests()
    }

    // MARK: - Sleep/Wake Notifications

    private func registerSleepWakeNotifications() {
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(systemWillSleep),
            name: NSNotification.Name.NSProcessInfoPowerStateDidChange,
            object: nil
        )

        NotificationCenter.default.addObserver(
            self,
            selector: #selector(systemDidWake),
            name: UIApplication.willEnterForegroundNotification,
            object: nil
        )
    }

    private func removeSleepWakeNotifications() {
        NotificationCenter.default.removeObserver(self)
    }

    @objc private func systemWillSleep() {
        NSLog("[Shield] System will sleep notification received")
        rustSetPowerState?(2) // SCREEN_OFF_DEEP
        acousticReceiver?.stopListening()
    }

    @objc private func systemDidWake() {
        NSLog("[Shield] System did wake notification received")
        lastWakeTime = Date()
        if batteryManager?.shouldEnableAcousticChannel() == true {
            acousticReceiver?.startListening()
        }
        rustSetPowerState?(0)
    }

    deinit {
        removeSleepWakeNotifications()
        packetProcessingTimer?.cancel()
    }
}

// MARK: - BatteryManagerDelegate

extension PacketTunnelProvider: BatteryManagerDelegate {
    func batteryStateDidChange(level: Int, isCharging: Bool, powerState: Int) {
        rustSetPowerState?(Int32(powerState))

        // Adjust acoustic channel based on battery
        if powerState == 3 || (powerState == 0 && level > 15) {
            acousticReceiver?.startListening()
        } else {
            acousticReceiver?.stopListening()
        }
    }
}

// MARK: - AcousticReceiverDelegate

extension PacketTunnelProvider: AcousticReceiverDelegate {
    func didReceiveAcousticData(_ data: Data) {
        // Forward acoustic data to Rust daemon for OFDM demodulation
        data.withUnsafeBytes { rawBufferPointer in
            if let baseAddress = rawBufferPointer.baseAddress {
                let pointer = baseAddress.assumingMemoryBound(to: UInt8.self)
                // Call Rust FFI for OFDM demodulation
                // The Rust function will decode the acoustic signal into endpoint data
                if let updateFn = rustUpdateEndpoints {
                    // Convert decoded data to string for endpoint update
                    let decodedStr = String(data: data, encoding: .utf8) ?? ""
                    _ = updateFn(decodedStr)
                }
            }
        }
        NSLog("[Shield] Acoustic data received and forwarded to Rust daemon")
    }

    func acousticReceiverDidEncounterError(_ error: Error) {
        NSLog("[Shield] Acoustic receiver error: \(error.localizedDescription)")
    }
}

// MARK: - Protocol Definitions

protocol BatteryManagerDelegate: AnyObject {
    func batteryStateDidChange(level: Int, isCharging: Bool, powerState: Int)
}

protocol AcousticReceiverDelegate: AnyObject {
    func didReceiveAcousticData(_ data: Data)
    func acousticReceiverDidEncounterError(_ error: Error)
}
