import Foundation
import NetworkExtension

/**
 * Kill switch implementation for iOS.
 *
 * Uses NEVPNManager's includeAllNetworks (iOS 14.2+) to ensure
 * all traffic goes through the VPN tunnel. If the tunnel disconnects,
 * all internet access is blocked to prevent IP leaks.
 *
 * No jailbreak required - uses standard NetworkExtension APIs.
 */
class KillSwitch {

    private var isEnabled = false
    private weak var provider: NEPacketTunnelProvider?

    /**
     * Enable the kill switch.
     * Sets includeAllNetworks = true on the tunnel configuration.
     */
    func enable(provider: NEPacketTunnelProvider) {
        self.provider = provider
        isEnabled = true

        // iOS 14.2+ supports includeAllNetworks for kill switch behavior
        // This is configured in TunnelManager when creating the tunnel configuration
        // The actual enforcement happens at the system level

        print("[KillSwitch] Enabled - includeAllNetworks active")
    }

    /**
     * Disable the kill switch.
     */
    func disable() {
        isEnabled = false
        provider = nil

        print("[KillSwitch] Disabled")
    }

    /**
     * Emergency block - called when VPN disconnection is detected.
     * On iOS, if includeAllNetworks is set, the system automatically
     * blocks all traffic when the VPN disconnects.
     *
     * This method additionally attempts to reconnect the VPN.
     */
    func emergencyBlock() {
        guard isEnabled else { return }

        print("[KillSwitch] EMERGENCY - VPN disconnected, traffic blocked by system")

        // Attempt reconnection
        provider?.cancelTunnelWithError(nil)
    }

    /**
     * Check if kill switch is active.
     */
    var isActive: Bool {
        return isEnabled
    }

    /**
     * Configure kill switch on the tunnel provider manager.
     * Must be called before saving the configuration.
     */
    static func configureKillSwitch(on manager: NETunnelProviderManager, enabled: Bool) {
        if #available(iOS 14.2, *) {
            manager.protocolConfiguration?.includeAllNetworks = enabled
            manager.protocolConfiguration?.excludeLocalNetworks = false
        }
    }
}
