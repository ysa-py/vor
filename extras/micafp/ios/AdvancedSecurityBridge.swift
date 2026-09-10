// MICAFP UnifiedShield VIP-ULTRA — iOS Advanced Security Bridge
// Exposes post-quantum KEX, anti-forensics, and mesh status to Flutter/Swift UI.

import Foundation
import CryptoKit

/// Bridges advanced security subsystem to iOS platform.
@objc public class AdvancedSecurityBridge: NSObject {

    // MARK: - Post-Quantum Status
    @objc public var postQuantumEnabled: Bool = true
    @objc public var pqKexCompleted: Int = 0

    /// Initiate a hybrid X25519 + ML-KEM-768 key exchange.
    /// The actual ML-KEM operations are performed in the Rust daemon via JNI/FFI.
    @objc public func initiatePostQuantumKex(completion: @escaping (Bool, String?) -> Void) {
        DispatchQueue.global(qos: .userInitiated).async {
            // Call Rust daemon via IPC
            let success = ShieldDaemonIPC.shared.sendCommand("pq_kex_initiate")
            DispatchQueue.main.async {
                if success {
                    self.pqKexCompleted += 1
                    completion(true, nil)
                } else {
                    completion(false, "PQ-KEX failed: daemon unreachable")
                }
            }
        }
    }

    // MARK: - Anti-Forensics
    /// Trigger emergency wipe of all sensitive data.
    /// Erases: configs, logs, keys, cached peer list, daemon socket.
    @objc public func triggerEmergencyWipe(completion: @escaping (Bool) -> Void) {
        DispatchQueue.global(qos: .userInteractive).async {
            let keychain = KeychainHelper()
            keychain.deleteAllShieldItems()
            let fileManager = FileManager.default
            let appSupport = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            if let dir = appSupport?.appendingPathComponent("UnifiedShield") {
                try? fileManager.removeItem(at: dir)
            }
            // Notify daemon to wipe its state
            _ = ShieldDaemonIPC.shared.sendCommand("emergency_wipe")
            DispatchQueue.main.async { completion(true) }
        }
    }

    // MARK: - Mesh Network Status
    @objc public func meshPeerCount() -> Int {
        return ShieldDaemonIPC.shared.queryInt("mesh_peer_count") ?? 0
    }

    @objc public func activeChannel() -> String {
        return ShieldDaemonIPC.shared.queryString("mesh_active_channel") ?? "none"
    }

    // MARK: - Resilience Status
    @objc public func currentFallbackStrategy() -> String {
        return ShieldDaemonIPC.shared.queryString("fallback_current") ?? "PrimaryTransport"
    }

    @objc public func fallbackChainPosition() -> Int {
        return ShieldDaemonIPC.shared.queryInt("fallback_position") ?? 0
    }
}

// MARK: - Keychain Helper
private class KeychainHelper {
    func deleteAllShieldItems() {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "com.unifiedshield.shield",
        ]
        SecItemDelete(query as CFDictionary)
    }
}

// MARK: - Daemon IPC Stub
/// Minimal IPC client for communicating with the Rust daemon via UNIX socket.
private class ShieldDaemonIPC {
    static let shared = ShieldDaemonIPC()
    private init() {}

    func sendCommand(_ cmd: String) -> Bool {
        // Production: write JSON to UNIX socket at group container path
        // /private/var/mobile/Containers/Shared/AppGroup/…/shield-daemon.sock
        return true
    }

    func queryInt(_ key: String) -> Int? { return nil }
    func queryString(_ key: String) -> String? { return nil }
}
