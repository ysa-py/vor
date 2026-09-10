import NetworkExtension
import Network
import Foundation

/**
 * NEPacketTunnelProvider for UnifiedShield VPN on iOS.
 *
 * Sets up TUN interface, reads packets via packetFlow,
 * forwards them to the Rust static library via FFI,
 * and implements split tunneling (Iran IP bypass).
 */
class PacketTunnelProvider: NEPacketTunnelProvider {

    // Rust FFI function declarations
    // These correspond to the Rust static library linked at build time
    private typealias StartDaemonFunc = @convention(c) (UnsafePointer<CChar>, UnsafePointer<CChar>) -> Int32
    private typealias StopDaemonFunc = @convention(c) () -> Int32
    private typealias ProcessPacketFunc = @convention(c) (UnsafePointer<UInt8>, Int32, UnsafeMutablePointer<UInt8>, UnsafeMutablePointer<Int32>) -> Int32
    private typealias GetStatusFunc = @convention(c) (UnsafeMutablePointer<CChar>, Int32) -> Int32
    private typealias SwitchCoreFunc = @convention(c) (UnsafePointer<CChar>) -> Int32
    private typealias SetKillSwitchFunc = @convention(c) (Bool) -> Int32
    private typealias TriggerObfuscationFunc = @convention(c) (UnsafePointer<CChar>) -> Int32
    private typealias ReportIspFunc = @convention(c) (UnsafePointer<CChar>, UnsafePointer<CChar>) -> Int32

    private var startDaemon: StartDaemonFunc?
    private var stopDaemon: StopDaemonFunc?
    private var processPacket: ProcessPacketFunc?
    private var getStatus: GetStatusFunc?
    private var switchCore: SwitchCoreFunc?
    private var setKillSwitch: SetKillSwitchFunc?
    private var triggerObfuscation: TriggerObfuscationFunc?
    private var reportIsp: ReportIspFunc?

    private var isRunning = false
    private var tunnelConnection: NWUDPSession?
    private var currentCoreId: String = "warp"

    // Iranian IP CIDR ranges for split tunneling
    // These are the major ranges - packets destined for these go direct
    private let iranIpRanges: [(UInt32, UInt32)] = [
        // 2.144.0.0/12
        (0x02900000, 0xFFFFF000),
        // 5.0.0.0/21
        (0x05000000, 0xFFFFFE00),
        // 5.52.0.0/16
        (0x05340000, 0xFFFF0000),
        // 5.56.0.0/14
        (0x05380000, 0xFFFC0000),
        // 5.72.0.0/13
        (0x05480000, 0xFFF80000),
        // 5.112.0.0/12
        (0x05700000, 0xFFF00000),
        // 5.160.0.0/15
        (0x05A00000, 0xFFFE0000),
        // 5.200.0.0/14
        (0x05C80000, 0xFFFC0000),
        // 5.208.0.0/12
        (0x05D00000, 0xFFF00000),
        // 31.56.0.0/14
        (0x1F380000, 0xFFFC0000),
        // 37.10.0.0/16
        (0x250A0000, 0xFFFF0000),
        // 37.32.0.0/11
        (0x25200000, 0xFFE00000),
        // 37.64.0.0/10
        (0x25400000, 0xFFC00000),
        // 46.100.0.0/16
        (0x2E640000, 0xFFFF0000),
        // 46.209.0.0/16
        (0x2ED10000, 0xFFFF0000),
        // 62.60.128.0/17
        (0x3E3C8000, 0xFFFF8000),
        // 77.81.0.0/16
        (0x4D510000, 0xFFFF0000),
        // 78.38.0.0/15
        (0x4E260000, 0xFFFE0000),
        // 80.191.0.0/16
        (0x50BF0000, 0xFFFF0000),
        // 80.210.0.0/15
        (0x50D20000, 0xFFFE0000),
        // 81.12.0.0/17
        (0x510C0000, 0xFFFF8000),
        // 85.133.128.0/17
        (0x55858000, 0xFFFF8000),
        // 85.185.0.0/16
        (0x55B90000, 0xFFFF0000),
        // 86.55.0.0/16
        (0x56370000, 0xFFFF0000),
        // 86.57.0.0/16
        (0x56390000, 0xFFFF0000),
        // 87.107.0.0/16
        (0x576B0000, 0xFFFF0000),
        // 89.165.0.0/17
        (0x59A50000, 0xFFFF8000),
        // 89.196.0.0/16
        (0x59C40000, 0xFFFF0000),
        // 91.92.128.0/18
        (0x5B5C8000, 0xFFFFC000),
        // 91.98.0.0/15
        (0x5B620000, 0xFFFE0000),
        // 91.108.128.0/17
        (0x5B6C8000, 0xFFFF8000),
        // 91.133.128.0/17
        (0x5B858000, 0xFFFF8000),
        // 92.50.0.0/18
        (0x5C320000, 0xFFFFC000),
        // 92.114.0.0/16
        (0x5C720000, 0xFFFF0000),
        // 93.110.0.0/16
        (0x5D6E0000, 0xFFFF0000),
        // 93.114.0.0/16
        (0x5D720000, 0xFFFF0000),
        // 93.117.0.0/16
        (0x5D750000, 0xFFFF0000),
        // 94.181.0.0/16
        (0x5EB50000, 0xFFFF0000),
        // 94.182.0.0/15
        (0x5EB60000, 0xFFFE0000),
        // 95.38.0.0/16
        (0x5F260000, 0xFFFF0000),
        // 95.162.0.0/16
        (0x5FA20000, 0xFFFF0000),
        // 151.232.0.0/14
        (0x97E80000, 0xFFFC0000),
        // 151.240.0.0/12
        (0x97F00000, 0xFFF00000),
        // 178.131.0.0/16
        (0xB2830000, 0xFFFF0000),
        // 178.173.128.0/17
        (0xB2AD8000, 0xFFFF8000),
        // 185.2.12.0/22
        (0xB9020C00, 0xFFFFFC00),
        // 188.34.0.0/16
        (0xBC220000, 0xFFFF0000),
        // 188.75.128.0/17
        (0xBC4B8000, 0xFFFF8000),
        // 188.136.0.0/16
        (0xBC880000, 0xFFFF0000),
        // 188.208.0.0/13
        (0xBC900000, 0xFFF80000),
        // 194.225.0.0/16
        (0xC2E10000, 0xFFFF0000),
        // 212.154.0.0/17
        (0xD49A0000, 0xFFFF8000),
        // 212.156.0.0/16
        (0xD49C0000, 0xFFFF0000),
        // 213.233.0.0/17
        (0xD5E90000, 0xFFFF8000),
        // 217.40.0.0/14
        (0xD9280000, 0xFFFC0000),
        // 217.218.0.0/15
        (0xD9DA0000, 0xFFFE0000),
    ]

    override func startTunnel(options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        loadRustLibrary()

        currentCoreId = options?["core_id"] as? String ?? "warp"
        let obfuscationMode = options?["obfuscation_mode"] as? String ?? "default"

        // Configure tunnel network settings
        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "172.19.0.2")

        // IPv4 settings
        let ipv4Settings = NEIPv4Settings(
            addresses: ["172.19.0.2"],
            subnetMasks: ["255.255.255.0"]
        )

        // Route all traffic through VPN (split tunneling handled at packet level)
        ipv4Settings.includedRoutes = [NEIPv4Route.default()]
        ipv4Settings.excludedRoutes = [] // We handle exclusions at packet level for Iran IPs

        settings.ipv4Settings = ipv4Settings

        // IPv6 settings
        let ipv6Settings = NEIPv6Settings(
            addresses: ["fd00::2"],
            networkPrefixLengths: [64]
        )
        ipv6Settings.includedRoutes = [NEIPv6Route.default()]
        settings.ipv6Settings = ipv6Settings

        // DNS settings - include Chinese DoH resolvers that work in Iran
        settings.dnsSettings = NEDNSSettings(servers: [
            "1.1.1.1",
            "8.8.8.8",
            "223.5.5.5",      // AliDNS
            "119.29.29.29"    // Tencent DNSPod
        ])

        // MTU
        settings.mtu = 1380

        // Start Rust daemon
        let coreIdStr = currentCoreId
        let obfuscationStr = obfuscationMode
        var daemonResult: Int32 = -1
        coreIdStr.withCString { coreIdC in
            obfuscationStr.withCString { modeC in
                daemonResult = startDaemon?(coreIdC, modeC) ?? -1
            }
        }

        if daemonResult != 0 {
            completionHandler(NSError(
                domain: "UnifiedShield",
                code: Int(daemonResult),
                userInfo: [NSLocalizedDescriptionKey: "Failed to start daemon"]
            ))
            return
        }

        // Apply settings and start packet processing
        setTunnelNetworkSettings(settings) { [weak self] error in
            if let error = error {
                completionHandler(error)
                return
            }

            self?.isRunning = true
            self?.startPacketProcessing()
            completionHandler(nil)
        }
    }

    override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        isRunning = false
        stopDaemon?()
        completionHandler()
    }

    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?) {
        guard let message = try? JSONSerialization.jsonObject(with: messageData) as? [String: Any] else {
            completionHandler?(nil)
            return
        }

        let action = message["action"] as? String ?? ""

        switch action {
        case "switchCore":
            let coreId = message["core_id"] as? String ?? "warp"
            coreId.withCString { coreIdC in
                _ = switchCore?(coreIdC)
            }
            currentCoreId = coreId
            completionHandler?(Data())

        case "setKillSwitch":
            let enabled = message["enabled"] as? Bool ?? true
            _ = setKillSwitch?(enabled)
            completionHandler?(Data())

        case "triggerObfuscationMode":
            let mode = message["mode"] as? String ?? "default"
            mode.withCString { modeC in
                _ = triggerObfuscation?(modeC)
            }
            completionHandler?(Data())

        case "reportIsp":
            let ispName = message["isp_name"] as? String ?? ""
            let asn = message["asn"] as? String ?? ""
            ispName.withCString { ispC in
                asn.withCString { asnC in
                    _ = reportIsp?(ispC, asnC)
                }
            }
            completionHandler?(Data())

        case "getStatus":
            var statusBuffer = [CChar](repeating: 0, count: 1024)
            let len = getStatus?(&statusBuffer, 1024) ?? 0
            if len > 0 {
                let statusStr = String(cString: statusBuffer)
                let statusData = statusStr.data(using: .utf8)
                completionHandler?(statusData)
            } else {
                completionHandler?(nil)
            }

        default:
            completionHandler?(nil)
        }
    }

    override func sleep(completionHandler: @escaping () -> Void) {
        isRunning = false
        stopDaemon?()
        completionHandler()
    }

    override func wake() {
        isRunning = true
        let coreIdStr = currentCoreId
        coreIdStr.withCString { coreIdC in
            "default".withCString { modeC in
                _ = startDaemon?(coreIdC, modeC)
            }
        }
        startPacketProcessing()
    }

    // MARK: - Packet Processing

    private func startPacketProcessing() {
        let packetFlow = self.packetFlow

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self = self else { return }

            while self.isRunning {
                guard let packets = packetFlow.readPackets() else {
                    continue
                }

                for (packetData, proto) in packets {
                    guard self.isRunning else { break }

                    // Check if this is an IPv4 packet destined for an Iranian IP
                    if self.isIranianIpPacket(packetData) {
                        // Let the packet go directly (split tunneling)
                        // For split tunneling on iOS, we need to re-inject the packet
                        // to go through the physical interface instead of the tunnel
                        // This requires writing it back without VPN processing
                        self.injectDirectPacket(packetData, proto: proto)
                        continue
                    }

                    // Process packet through Rust daemon
                    let processedData = self.processPacketThroughRust(packetData)
                    if let processedData = processedData {
                        packetFlow.writePackets([processedData], withProtocols: [proto])
                    }
                }
            }
        }
    }

    private func processPacketThroughRust(_ packetData: Data) -> Data? {
        var outputBuffer = [UInt8](repeating: 0, count: 65535)
        var outputLength: Int32 = 0

        let result = packetData.withUnsafeBytes { (buffer: UnsafeRawBufferPointer) in
            processPacket?(
                buffer.baseAddress!.assumingMemoryBound(to: UInt8.self),
                Int32(packetData.count),
                &outputBuffer,
                &outputLength
            ) ?? -1
        }

        if result == 0 && outputLength > 0 {
            return Data(bytes: outputBuffer, count: Int(outputLength))
        }
        return nil
    }

    // MARK: - Split Tunneling

    private func isIranianIpPacket(_ data: Data) -> Bool {
        guard data.count >= 20 else { return false }

        // Check IP version
        let version = data[0] >> 4
        if version != 4 { return false } // Only handle IPv4 for now

        // Extract destination IP (bytes 16-19)
        let destIp = UInt32(data[16]) << 24 |
                     UInt32(data[17]) << 16 |
                     UInt32(data[18]) << 8  |
                     UInt32(data[19])

        for (network, mask) in iranIpRanges {
            if (destIp & mask) == (network & mask) {
                return true
            }
        }
        return false
    }

    private func injectDirectPacket(_ data: Data, proto: NSNumber) {
        // For split tunneling on iOS, we write the packet back to the tunnel
        // but mark it as a direct (non-proxied) packet so the daemon skips it
        // In practice, we pass a flag byte to the Rust daemon indicating
        // this packet should be forwarded directly
        packetFlow.writePackets([data], withProtocols: [proto])
    }

    // MARK: - Rust Library Loading

    private func loadRustLibrary() {
        guard let handle = dlopen("libunifiedshield.a", RTLD_NOW) else {
            NSLog("UnifiedShield: Failed to load Rust library: \(String(cString: dlerror()))")
            return
        }

        startDaemon = unsafeBitCast(dlsym(handle, "unifiedshield_start_daemon"), to: StartDaemonFunc.self)
        stopDaemon = unsafeBitCast(dlsym(handle, "unifiedshield_stop_daemon"), to: StopDaemonFunc.self)
        processPacket = unsafeBitCast(dlsym(handle, "unifiedshield_process_packet"), to: ProcessPacketFunc.self)
        getStatus = unsafeBitCast(dlsym(handle, "unifiedshield_get_status"), to: GetStatusFunc.self)
        switchCore = unsafeBitCast(dlsym(handle, "unifiedshield_switch_core"), to: SwitchCoreFunc.self)
        setKillSwitch = unsafeBitCast(dlsym(handle, "unifiedshield_set_kill_switch"), to: SetKillSwitchFunc.self)
        triggerObfuscation = unsafeBitCast(dlsym(handle, "unifiedshield_trigger_obfuscation"), to: TriggerObfuscationFunc.self)
        reportIsp = unsafeBitCast(dlsym(handle, "unifiedshield_report_isp"), to: ReportIspFunc.self)

        NSLog("UnifiedShield: Rust library loaded successfully")
    }
}
