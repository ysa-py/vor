import CryptoKit
import Foundation
import Network

/// Local SOCKS5 proxy engine (RFC 1928 subset) — the realistic transport for
/// an unsigned TrollStore sideload: runs entirely inside the app sandbox,
/// needs no entitlements beyond plain network access. Apps with SOCKS
/// support point at 127.0.0.1:<port>.
///
/// Protocol subset: no-auth handshake, CONNECT command only, IPv4 / IPv6 /
/// domain target addresses, bidirectional relay with send-completion
/// back-pressure. Domain targets are resolved through the selectable
/// resolution transport (system resolver or DNS-over-HTTPS) before the
/// upstream connection is dialed.
///
/// Full-device VPN (NetworkExtension PacketTunnelProvider) requires proper
/// signing with the network-extension entitlement — documented in README.md
/// as future work, NOT assumed for TrollStore builds.
@MainActor
final class SocksProxyEngine: ObservableObject {
    @Published private(set) var isRunning = false
    /// Selectable resolution transport for domain-name targets:
    /// `.system` (default) or `.doh` (DNS-over-HTTPS, RFC 8484 JSON API).
    enum ResolutionTransport: String, CaseIterable, Identifiable {
        case system = "System DNS"
        case doh = "DoH (cloudflare-dns.com)"
        var id: String { rawValue }
    }

    @Published var resolution: ResolutionTransport = .system
    let port: UInt16 = 10808

    private var server: SocksServer?

    func toggle() {
        if isRunning { stop() } else { start() }
    }

    func start() {
        guard server == nil else { return }
        let server = SocksServer(port: port, useDoH: resolution == .doh)
        server.onStateChange = { [weak self] running in
            Task { @MainActor in self?.isRunning = running }
        }
        self.server = server
        server.start()
    }

    func stop() {
        server?.stop()
        server = nil
        isRunning = false
    }
}

/// Listener + connection bookkeeping. All mutable state is confined to one
/// private serial DispatchQueue; UI state crosses the boundary only through
/// the `onStateChange` callback (which hops to the main actor).
private final class SocksServer {
    private let port: UInt16
    private let useDoH: Bool
    private let queue = DispatchQueue(label: "vor.socks")
    private var listener: NWListener?
    private var sessions: [ObjectIdentifier: Socks5Session] = [:]
    var onStateChange: ((Bool) -> Void)?

    init(port: UInt16, useDoH: Bool) {
        self.port = port
        self.useDoH = useDoH
    }

    func start() {
        let parameters = NWParameters.tcp
        parameters.allowLocalEndpointReuse = true
        do {
            let listener = try NWListener(using: parameters, on: NWEndpoint.Port(rawValue: port)!)
            self.listener = listener
            listener.newConnectionHandler = { [weak self] connection in
                self?.accept(connection)
            }
            listener.stateUpdateHandler = { [weak self] state in
                switch state {
                case .ready:
                    self?.onStateChange?(true)
                case .failed, .cancelled:
                    self?.onStateChange?(false)
                default:
                    break
                }
            }
            listener.start(queue: queue)
        } catch {
            onStateChange?(false)
        }
    }

    func stop() {
        listener?.cancel()
        listener = nil
        queue.async { [weak self] in
            guard let self else { return }
            for (_, session) in self.sessions {
                session.teardown()
            }
            self.sessions.removeAll()
        }
    }

    private func accept(_ connection: NWConnection) {
        let session = Socks5Session(client: connection, queue: queue, useDoH: useDoH)
        sessions[ObjectIdentifier(connection)] = session
        session.onClose = { [weak self] id in
            self?.sessions.removeValue(forKey: id)
        }
        session.run()
    }
}

/// One client connection driven through the SOCKS5 state machine:
/// greeting -> (no-auth select) -> CONNECT request -> resolve -> dial
/// upstream -> success reply -> bidirectional relay.
private final class Socks5Session {
    private let client: NWConnection
    private let queue: DispatchQueue
    private let useDoH: Bool
    private var upstream: NWConnection?
    private var buffer = Data()
    var onClose: ((ObjectIdentifier) -> Void)?

    private static let successReply = Data([
        0x05, 0x00, 0x00, 0x01, // VER, REP=success, RSV, ATYP=IPv4
        0x00, 0x00, 0x00, 0x00, // BND.ADDR 0.0.0.0
        0x00, 0x00,             // BND.PORT 0
    ])

    private static func failureReply(_ code: UInt8) -> Data {
        Data([0x05, code, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
    }

    init(client: NWConnection, queue: DispatchQueue, useDoH: Bool) {
        self.client = client
        self.queue = queue
        self.useDoH = useDoH
    }

    func run() {
        client.stateUpdateHandler = { [weak self] state in
            switch state {
            case .failed, .cancelled:
                self?.teardown()
            default:
                break
            }
        }
        client.start(queue: queue)
        receiveGreeting()
    }

    /// Cancels both directions and drops the session from the server's table.
    func teardown() {
        upstream?.cancel()
        upstream = nil
        client.cancel()
        onClose?(ObjectIdentifier(client))
    }

    // MARK: - Handshake

    private func receiveGreeting() {
        receive(minimum: 2) { [weak self] in
            guard let self, self.buffer.count >= 2 else { return }
            let version = self.buffer.removeFirst()
            let methodCount = Int(self.buffer.removeFirst())
            guard version == 0x05, methodCount > 0 else {
                self.teardown()
                return
            }
            // Discard the offered method bytes; we always answer NO-AUTH.
            self.receive(minimum: methodCount) { [weak self] in
                guard let self else { return }
                if self.buffer.count < methodCount {
                    self.teardown()
                    return
                }
                self.buffer.removeFirst(methodCount)
                self.client.send(content: Data([0x05, 0x00]), completion: .contentProcessed { [weak self] _ in
                    self?.receiveRequest()
                })
            }
        }
    }

    private func receiveRequest() {
        // VER CMD RSV ATYP + (variable) DST.ADDR + 2-byte DST.PORT
        receive(minimum: 5) { [weak self] in
            guard let self, self.buffer.count >= 5 else { return }
            let version = self.buffer.removeFirst()
            let command = self.buffer.removeFirst()
            _ = self.buffer.removeFirst() // RSV
            let addressType = self.buffer.removeFirst()
            guard version == 0x05 else {
                self.teardown()
                return
            }
            guard command == 0x01 else { // CONNECT only
                self.replyFailure(0x07) // command not supported
                return
            }
            let addressLength: Int
            switch addressType {
            case 0x01: addressLength = 4        // IPv4
            case 0x03: addressLength = -1        // domain (1 length byte + N)
            case 0x04: addressLength = 16       // IPv6
            default:
                self.replyFailure(0x08) // address type not supported
                return
            }
            if addressLength > 0 {
                self.readAddressAndPort(addressType: addressType, addressLength: addressLength)
            } else {
                self.receive(minimum: 1) { [weak self] in
                    guard let self, !self.buffer.isEmpty else { return }
                    let domainLength = Int(self.buffer.removeFirst())
                    guard domainLength > 0, domainLength <= 253 else {
                        self.replyFailure(0x01)
                        return
                    }
                    self.readAddressAndPort(addressType: addressType, addressLength: domainLength)
                }
            }
        }
    }

    private func readAddressAndPort(addressType: UInt8, addressLength: Int) {
        receive(minimum: addressLength + 2) { [weak self] in
            guard let self, self.buffer.count >= addressLength + 2 else { return }
            let addressBytes = self.buffer.prefix(addressLength)
            self.buffer.removeFirst(addressLength)
            let port = UInt16(self.buffer.removeFirst()) << 8 | UInt16(self.buffer.removeFirst())

            let host: String
            switch addressType {
            case 0x01:
                host = Self.ipv4String(Array(addressBytes))
            case 0x04:
                host = Self.ipv6String(Array(addressBytes))
            default:
                host = String(data: Data(addressBytes), encoding: .utf8) ?? ""
            }
            guard !host.isEmpty, port != 0 else {
                self.replyFailure(0x01)
                return
            }
            self.connectUpstream(host: host, port: port)
        }
    }

    // MARK: - Upstream

    private func connectUpstream(host: String, port: UInt16) {
        func dial(_ endpoint: NWEndpoint) {
            let upstream = NWConnection(to: endpoint, using: .tcp)
            self.upstream = upstream
            upstream.stateUpdateHandler = { [weak self] state in
                switch state {
                case .ready:
                    self?.client.send(content: Self.successReply, completion: .contentProcessed { [weak self] _ in
                        guard let self else { return }
                        self.relay(from: self.client, to: self.upstream!)
                        self.relay(from: self.upstream!, to: self.client)
                    })
                case .failed, .cancelled:
                    // Only report failure if we never got connected.
                    if self?.client.state != .cancelled, self?.upstream === upstream {
                        self?.replyFailure(0x04) // host unreachable
                    }
                default:
                    break
                }
            }
            upstream.start(queue: queue)
        }

        if let v4 = IPv4Address(host) {
            dial(NWEndpoint.hostPort(host: .ipv4(v4), port: NWEndpoint.Port(rawValue: port)!))
        } else if let v6 = IPv6Address(host) {
            dial(NWEndpoint.hostPort(host: .ipv6(v6), port: NWEndpoint.Port(rawValue: port)!))
        } else if useDoH {
            DoHResolver.resolve(host: host) { [weak self] addresses in
                guard let self, self.client.state != .cancelled else { return }
                if let first = addresses.first,
                   let v4 = IPv4Address(first) {
                    dial(NWEndpoint.hostPort(host: .ipv4(v4), port: NWEndpoint.Port(rawValue: port)!))
                } else if let first = addresses.dropFirst().first,
                          let v6 = IPv6Address(first) {
                    dial(NWEndpoint.hostPort(host: .ipv6(v6), port: NWEndpoint.Port(rawValue: port)!))
                } else if let v6 = addresses.compactMap({ IPv6Address($0) }).first {
                    dial(NWEndpoint.hostPort(host: .ipv6(v6), port: NWEndpoint.Port(rawValue: port)!))
                } else {
                    // DoH unavailable -> fall back to the system resolver.
                    dial(Self.nameEndpoint(host: host, port: port))
                }
            }
        } else {
            dial(Self.nameEndpoint(host: host, port: port))
        }
    }

    /// Hostname endpoint (Apple-documented Host string initializer).
    private static func nameEndpoint(host: String, port: UInt16) -> NWEndpoint {
        NWEndpoint.hostPort(host: NWEndpoint.Host(host), port: NWEndpoint.Port(rawValue: port)!)
    }

    /// Bidirectional byte pump with back-pressure: the next receive only
    /// starts after the previous send completes.
    private func relay(from: NWConnection, to: NWConnection) {
        from.receive(minimumIncompleteLength: 1, maximumLength: 64 * 1024) { [weak self] data, _, isComplete, error in
            guard let self else { return }
            if let data, !data.isEmpty {
                to.send(content: data, completion: .contentProcessed { [weak self] _ in
                    self?.relay(from: from, to: to)
                })
            } else if error != nil || isComplete {
                // EOF or failure on one side: tear both down.
                self.teardown()
            } else {
                self.relay(from: from, to: to)
            }
        }
    }

    // MARK: - Reply helpers

    private func replyFailure(_ code: UInt8) {
        client.send(content: Self.failureReply(code), completion: .contentProcessed { [weak self] _ in
            self?.teardown()
        })
    }

    /// Accumulates into `buffer` until at least `minimum` bytes are present,
    /// then runs `next` on the session queue.
    private func receive(minimum: Int, then next: @escaping () -> Void) {
        if buffer.count >= minimum {
            next()
            return
        }
        client.receive(minimumIncompleteLength: minimum - buffer.count, maximumLength: 64 * 1024) { [weak self] data, _, isComplete, error in
            guard let self else { return }
            if let data { self.buffer += data }
            if self.buffer.count >= minimum {
                next()
            } else if error != nil || (isComplete && data == nil) {
                self.teardown()
            } else {
                self.receive(minimum: minimum, then: next)
            }
        }
    }

    private static func ipv4String(_ bytes: [UInt8]) -> String {
        guard bytes.count == 4 else { return "" }
        return bytes.map { String($0) }.joined(separator: ".")
    }

    private static func ipv6String(_ bytes: [UInt8]) -> String {
        guard bytes.count == 16 else { return "" }
        var groups: [String] = []
        for index in stride(from: 0, to: 16, by: 2) {
            groups.append(String(format: "%x", (UInt16(bytes[index]) << 8) | UInt16(bytes[index + 1])))
        }
        return groups.joined(separator: ":")
    }
}

/// DNS-over-HTTPS JSON resolver (RFC 8484 `application/dns-json` profile as
/// served by cloudflare-dns.com and dns.google). Returns IPv4 + IPv6 strings
/// for a hostname; used as the selectable resolution transport for upstream
/// dials so target names never leak through the plaintext system resolver.
enum DoHResolver {
    static let defaultEndpoint = URL(string: "https://cloudflare-dns.com/dns-query")!

    static func resolve(host: String, endpoint: URL = defaultEndpoint, completion: @escaping ([String]) -> Void) {
        var components = URLComponents(url: endpoint, resolvingAgainstBaseURL: false)
        components?.queryItems = [URLQueryItem(name: "name", value: host)]
        guard let url = components?.url else {
            completion([])
            return
        }
        var request = URLRequest(url: url)
        request.setValue("application/dns-json", forHTTPHeaderField: "Accept")
        request.timeoutInterval = 5
        URLSession.shared.dataTask(with: request) { data, response, error in
            guard error == nil,
                  let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode),
                  let data,
                  let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let answers = object["Answer"] as? [[String: Any]] else {
                completion([])
                return
            }
            // Type 1 = A, 28 = AAAA.
            let addresses = answers.compactMap { answer -> String? in
                guard let type = answer["type"] as? Int, type == 1 || type == 28 else { return nil }
                return answer["data"] as? String
            }
            completion(addresses)
        }.resume()
    }
}
