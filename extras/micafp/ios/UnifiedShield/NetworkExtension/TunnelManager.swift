import NetworkExtension
import Foundation

/**
 * Manages the VPN tunnel lifecycle on iOS.
 * Handles NETunnelProviderManager configuration and activation.
 */
class TunnelManager: ObservableObject {

    static let shared = TunnelManager()

    @Published var isConnected = false
    @Published var isConnecting = false
    @Published var currentCore = "xray"
    @Published var downloadSpeed = "0 KB/s"
    @Published var uploadSpeed = "0 KB/s"
    @Published var connectionUptime = "00:00:00"
    @Published var dpiScore: Double = 0.0

    private var tunnelProviderManager: NETunnelProviderManager?
    private var vpnStatus: NEVPNStatus = .invalid
    private var statusObservation: NSKeyValueObservation?
    private var startTime: Date?

    private init() {
        loadTunnelConfiguration()
        observeVpnStatus()
    }

    // MARK: - Public API

    func connect() {
        guard let manager = tunnelProviderManager else {
            // First time - create configuration
            createTunnelConfiguration { [weak self] success in
                if success {
                    self?.startVPN()
                }
            }
            return
        }
        startVPN()
    }

    func disconnect() {
        guard let session = tunnelProviderManager?.connection as? NETunnelProviderSession else {
            return
        }
        session.stopTunnel()
    }

    func switchCore(to core: String, completion: (() -> Void)? = nil) {
        guard let session = tunnelProviderManager?.connection as? NETunnelProviderSession else {
            completion?()
            return
        }

        let message = ["action": "switchCore", "core": core] as [String: Any]
        guard let messageData = try? JSONSerialization.data(withJSONObject: message) else {
            completion?()
            return
        }

        do {
            try session.sendProviderMessage(messageData) { [weak self] _ in
                DispatchQueue.main.async {
                    self?.currentCore = core
                    completion?()
                }
            }
        } catch {
            completion?()
        }
    }

    func getStatus() {
        guard let session = tunnelProviderManager?.connection as? NETunnelProviderSession else {
            return
        }

        let message = ["action": "getStatus"] as [String: Any]
        guard let messageData = try? JSONSerialization.data(withJSONObject: message) else {
            return
        }

        try? session.sendProviderMessage(messageData) { responseData in
            // Parse status response
            if let data = responseData,
               let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
                DispatchQueue.main.async {
                    // Update UI with status data
                }
            }
        }
    }

    // MARK: - Private Methods

    private func loadTunnelConfiguration() {
        NETunnelProviderManager.loadAllFromPreferences { [weak self] managers, error in
            guard let self = self else { return }

            if let error = error {
                print("Failed to load tunnel configurations: \(error)")
                return
            }

            if let existingManager = managers?.first {
                self.tunnelProviderManager = existingManager
            }
        }
    }

    private func createTunnelConfiguration(completion: @escaping (Bool) -> Void) {
        let manager = NETunnelProviderManager()

        manager.protocolConfiguration = NETunnelProviderProtocol(
            providerBundleIdentifier: "com.unifiedshield.packet-tunnel",
            providerConfiguration: [:],
            serverAddress: "UnifiedShield"
        )

        manager.localizedDescription = "UnifiedShield VPN"
        manager.isEnabled = true

        // Enable always-on VPN (kill switch)
        if #available(iOS 14.2, *) {
            manager.protocolConfiguration?.includeAllNetworks = true
            manager.protocolConfiguration?.excludeLocalNetworks = false
        }

        manager.saveToPreferences { [weak self] error in
            guard let self = self else {
                completion(false)
                return
            }

            if let error = error {
                print("Failed to save tunnel configuration: \(error)")
                completion(false)
                return
            }

            self.tunnelProviderManager = manager
            completion(true)
        }
    }

    private func startVPN() {
        guard let manager = tunnelProviderManager,
              let session = manager.connection as? NETunnelProviderSession else {
            return
        }

        isConnecting = true

        do {
            try session.startTunnel()
        } catch {
            print("Failed to start tunnel: \(error)")
            isConnecting = false
        }
    }

    private func observeVpnStatus() {
        statusObservation = tunnelProviderManager?.connection.observe(\.status) { [weak self] connection, _ in
            DispatchQueue.main.async {
                guard let self = self else { return }

                switch connection.status {
                case .connected:
                    self.isConnected = true
                    self.isConnecting = false
                    self.startTime = Date()
                case .disconnected:
                    self.isConnected = false
                    self.isConnecting = false
                    self.startTime = nil
                case .connecting:
                    self.isConnecting = true
                case .disconnecting:
                    self.isConnecting = false
                case .invalid:
                    self.isConnected = false
                    self.isConnecting = false
                case .reasserting:
                    self.isConnecting = true
                @unknown default:
                    break
                }

                self.vpnStatus = connection.status
            }
        }
    }
}
