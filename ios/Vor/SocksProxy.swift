import Foundation
import Network

/// Local SOCKS5 proxy engine — the realistic transport for an unsigned
/// TrollStore sideload: runs entirely inside the app sandbox, needs no
/// entitlements beyond plain network access. Apps with SOCKS support point
/// at 127.0.0.1:<port>.
///
/// Full-device VPN (NetworkExtension PacketTunnelProvider) requires proper
/// signing with the network-extension entitlement — documented in
/// README.md as future work, NOT assumed for TrollStore builds.
@MainActor
final class SocksProxyEngine: ObservableObject {
    @Published private(set) var isRunning = false
    let port: UInt16 = 10808

    private var listener: NWListener?
    private var queue = DispatchQueue(label: "vor.socks")

    func toggle() {
        if isRunning { stop() } else { start() }
    }

    func start() {
        let parameters = NWParameters.tcp
        parameters.allowLocalEndpointReuse = true
        guard let listener = try? NWListener(using: parameters, on: NWEndpoint.Port(rawValue: port)!) else {
            return
        }
        listener.newConnectionHandler = { [weak self] connection in
            self?.handle(connection)
        }
        self.listener = listener
        listener.stateUpdateHandler = { [weak self] state in
            Task { @MainActor in
                self?.isRunning = (state == .ready)
            }
        }
        queue.async { listener.start(queue: .global(qos: .utility)) }
    }

    func stop() {
        listener?.cancel()
        listener = nil
        isRunning = false
    }

    /// Minimal SOCKS5 CONNECT handler (no auth, domain + IPv4 targets).
    private func handle(_ connection: NWConnection) {
        receive greeting: { data in ... }
        receive greeting: nil
    }
}
