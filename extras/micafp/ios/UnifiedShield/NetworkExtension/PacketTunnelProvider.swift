import NetworkExtension
import Foundation

/**
 * PacketTunnelProvider for UnifiedShield VPN on iOS.
 *
 * Implements NEPacketTunnelProvider for VPN functionality without jailbreak.
 * Uses Rust FFI core for protocol handling and packet routing.
 *
 * Configuration:
 * - TUN address: 172.19.0.2/24
 * - MTU: 1380
 * - Split tunnel: Iranian IPs excluded
 * - DNS: Alibaba/Tencent (Cloudflare blocked in Iran)
 * - Routes: 0.0.0.0/0 and ::/0
 */
class PacketTunnelProvider: NEPacketTunnelProvider {

    private var tunnelConnection: NWUDPSession?
    private var lastPathUpdate: Date?
    private let coreBridge = CoreBridge()
    private let dpiDetector = DpiDetector()
    private let killSwitch = KillSwitch()
    private var isRunning = false
    private var currentCore = "xray"

    // Rust FFI function pointers
    private var rustContext: UnsafeMutableRawPointer?

    override func startTunnel(options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        // Configure tunnel settings
        let tunnelSettings = createTunnelSettings()

        // Set up the tunnel
        setTunnelNetworkSettings(tunnelSettings) { [weak self] error in
            guard let self = self else {
                completionHandler(PacketTunnelError.tunnelSetupFailed)
                return
            }

            if let error = error {
                completionHandler(error)
                return
            }

            // Start the Rust core daemon via FFI
            self.startCoreDaemon { coreError in
                if let coreError = coreError {
                    completionHandler(coreError)
                    return
                }

                // Start DPI monitoring
                self.dpiDetector.startMonitoring { score in
                    if score > 0.72 {
                        // DPI detected - trigger core switch
                        self.triggerDpiEvasion()
                    }
                }

                // Enable kill switch
                self.killSwitch.enable(provider: self)

                self.isRunning = true

                // Start reading packets from the tunnel
                self.readPackets()

                completionHandler(nil)
            }
        }
    }

    override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        isRunning = false
        dpiDetector.stopMonitoring()
        killSwitch.disable()
        coreBridge.stopDaemon()

        completionHandler()
    }

    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?) {
        guard let message = try? JSONSerialization.jsonObject(with: messageData) as? [String: Any],
              let action = message["action"] as? String else {
            completionHandler(nil)
            return
        }

        switch action {
        case "switchCore":
            if let core = message["core"] as? String {
                switchCore(to: core)
                completionHandler(nil)
            }
        case "getStatus":
            let status = coreBridge.getStatus()
            let responseData = try? JSONSerialization.data(withJSONObject: ["status": status])
            completionHandler(responseData)
        case "setKillSwitch":
            if let enabled = message["enabled"] as? Bool {
                if enabled {
                    killSwitch.enable(provider: self)
                } else {
                    killSwitch.disable()
                }
                completionHandler(nil)
            }
        default:
            completionHandler(nil)
        }
    }

    override func wake() {
        // Handle wake from sleep
        if isRunning {
            coreBridge.reconnect()
        }
    }

    override func sleep(completionHandler: @escaping () -> Void) {
        // Handle sleep - keep connection alive if possible
        completionHandler()
    }

    // MARK: - Tunnel Configuration

    private func createTunnelSettings() -> NEPacketTunnelNetworkSettings {
        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "172.19.0.2")

        // IPv4 settings
        let ipv4Settings = NEIPv4Settings(
            addresses: ["172.19.0.2"],
            subnetMasks: ["255.255.255.0"]
        )
        ipv4Settings.includedRoutes = [NEIPv4Route.default()]
        ipv4Settings.excludedRoutes = buildExcludedRoutes()
        settings.ipv4Settings = ipv4Settings

        // IPv6 settings
        let ipv6Settings = NEIPv6Settings(
            addresses: ["fd00::2"],
            networkPrefixLengths: [64]
        )
        ipv6Settings.includedRoutes = [NEIPv6Route.default()]
        settings.ipv6Settings = ipv6Settings

        // DNS settings (Chinese CDN - Cloudflare blocked in Iran)
        let dnsSettings = NEDNSSettings(servers: [
            "223.5.5.5",      // Alibaba DNS
            "119.29.29.29",   // Tencent DNS
            "1.12.12.12"      // Tencent DNS backup
        ])
        dnsSettings.searchDomains = ["local"]
        dnsSettings.matchDomains = [""] // Route all DNS through tunnel
        settings.dnsSettings = dnsSettings

        // Proxy settings - none (we handle at packet level)
        settings.proxySettings = nil

        // MTU
        settings.mtu = 1380

        return settings
    }

    /**
     * Build excluded routes for split tunnel.
     * Iranian IP ranges are excluded to allow direct access
     * to local banking and government services.
     */
    private func buildExcludedRoutes() -> [NEIPv4Route] {
        let iranianRanges = SplitTunnelHelper.iranianIpRanges()
        return iranianRanges.compactMap { cidr -> NEIPv4Route? in
            let parts = cidr.split(separator: "/")
            guard parts.count == 2,
                  let prefixLength = Int(parts[1]) else { return nil }
            return NEIPv4Route(
                destinationAddress: String(parts[0]),
                subnetMask: prefixLengthToSubnetMask(prefixLength)
            )
        }
    }

    private func prefixLengthToSubnetMask(_ prefix: Int) -> String {
        let mask = prefix == 0 ? 0 : (~0 << (32 - prefix))
        return [
            (mask >> 24) & 0xFF,
            (mask >> 16) & 0xFF,
            (mask >> 8) & 0xFF,
            mask & 0xFF
        ].map { String($0) }.joined(separator: ".")
    }

    // MARK: - Packet Handling

    private func readPackets() {
        packetFlow.readPackets { [weak self] packets, protocols in
            guard let self = self, self.isRunning else { return }

            // Forward packets to Rust core via FFI
            for (index, packet) in packets.enumerated() {
                let protocolFamily = protocols[index]
                self.coreBridge.processPacket(packet, protocolFamily: protocolFamily)
            }

            // Continue reading
            self.readPackets()
        }
    }

    // MARK: - Core Management

    private func startCoreDaemon(completion: @escaping (Error?) -> Void) {
        // Initialize Rust FFI bridge
        coreBridge.startDaemon(
            core: currentCore,
            tunAddress: "172.19.0.2",
            dnsServers: ["223.5.5.5", "119.29.29.29"]
        ) { result in
            switch result {
            case .success:
                completion(nil)
            case .failure(let error):
                completion(error)
            }
        }
    }

    private func switchCore(to core: String) {
        currentCore = core
        coreBridge.switchCore(to: core)
    }

    private func triggerDpiEvasion() {
        // Switch to alternative core
        let newCore = currentCore == "xray" ? "naive" : "xray"
        switchCore(to: newCore)
        coreBridge.triggerObfuscationMode()
        dpiDetector.reset()
    }
}

enum PacketTunnelError: Error {
    case tunnelSetupFailed
    case coreStartFailed
}

// Split tunnel helper for Iranian IP ranges
struct SplitTunnelHelper {
    static func iranianIpRanges() -> [String] {
        return [
            "78.38.0.0/16",
            "78.39.0.0/16",
            "217.218.0.0/15",
            "5.106.0.0/16",
            "5.107.0.0/16",
            "94.182.0.0/15",
            "2.146.0.0/15",
            "31.56.0.0/14",
            "151.233.0.0/16",
            "5.200.200.0/24",
            "46.36.0.0/17",
            "91.92.0.0/14",
            "185.143.232.0/22",
            "62.60.128.0/17",
            "80.191.0.0/16",
            "81.12.0.0/17"
        ]
    }
}
