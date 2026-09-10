import SwiftUI

@main
struct VorApp: App {
    @StateObject private var license = LicenseStore()

    var body: some Scene {
        WindowGroup {
            if license.isUnlocked {
                MainScreen()
            } else {
                LicenseGateView()
            }
        }
    }
}

/// License persistence + launch re-check (UserDefaults — sandbox-safe).
@MainActor
final class LicenseStore: ObservableObject {
    @Published private(set) var isUnlocked = false
    @Published private(set) var lastResult: LicenseVerifier.Result?
    private let tokenKey = "vor.license.token"

    init() {
        recheck()
    }

    /// Re-verified at every launch: expiry re-locks automatically.
    func recheck() {
        let token = UserDefaults.standard.string(forKey: tokenKey) ?? ""
        guard !token.isEmpty else {
            isUnlocked = false
            lastResult = nil
            return
        }
        let result = LicenseVerifier.verify(token)
        lastResult = result
        isUnlocked = result.status == .valid
    }

    /// Activate a pasted token; persists only signature-valid tokens.
    func activate(_ token: String) {
        let result = LicenseVerifier.verify(token)
        lastResult = result
        if result.status != .invalid {
            UserDefaults.standard.set(token.trimmingCharacters(in: .whitespacesAndNewlines), forKey: tokenKey)
        }
        isUnlocked = result.status == .valid
    }
}

struct LicenseGateView: View {
    @EnvironmentObject var license: LicenseStore
    @State private var token: String = ""

    var body: some View {
        VStack(spacing: 18) {
            Image(systemName: "key.fill")
                .font(.system(size: 44))
                .foregroundStyle(Color.accentColor)
            Text("Activate Vor")
                .font(.title2.bold())
            Text("Paste the license token you received. Verification happens offline on this device — no internet connection is used.")
                .font(.footnote)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 28)

            if license.lastResult?.status == .expired, let expires = license.lastResult?.expiresAt {
                Label("License expired \(expires.formatted(date: .abbreviated, time: .omitted)). Obtain a new token.", systemImage: "exclamationmark.triangle")
                    .font(.footnote)
                    .foregroundStyle(.red)
            }

            TextEditor(text: $token)
                .font(.system(size: 12, design: .monospaced))
                .frame(height: 90)
                .overlay(RoundedRectangle(cornerRadius: 8).stroke(Color.gray.opacity(0.4)))
                .padding(.horizontal, 24)

            Button {
                license.activate(token)
            } label: {
                Text("Check / Activate License")
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 6)
            }
            .buttonStyle(.borderedProminent)
            .disabled(token.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            .padding(.horizontal, 24)
        }
        .padding()
    }
}

/// Main screen: engine status + local SOCKS proxy control.
struct MainScreen: View {
    @StateObject private var proxy = SocksProxyEngine()
    @EnvironmentObject var license: LicenseStore

    var body: some View {
        NavigationStack {
            VStack(spacing: 16) {
                Circle()
                    .fill(proxy.isRunning ? Color.blue.gradient : Color.gray.gradient)
                    .frame(width: 110, height: 110)
                    .overlay(
                        Text(proxy.isRunning ? "ACTIVE" : "OFF")
                            .font(.headline.bold())
                            .foregroundStyle(.white)
                    )
                    .onTapGesture { proxy.toggle() }

                Text(proxy.isRunning ? "Local SOCKS proxy: 127.0.0.1:\(proxy.port)" : "Proxy stopped")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)

                // Selectable resolution transport for upstream dials
                // (system resolver or DNS-over-HTTPS).
                Picker("DNS", selection: $proxy.resolution) {
                    ForEach(SocksProxyEngine.ResolutionTransport.allCases) { transport in
                        Text(transport.rawValue).tag(transport)
                    }
                }
                .pickerStyle(.segmented)
                .padding(.horizontal, 24)

                VStack(alignment: .leading, spacing: 6) {
                    Label("License ID: \(license.lastResult?.licenseId ?? "—")", systemImage: "checkmark.seal")
                    Label("Expires: \(license.lastResult?.expiresAt?.formatted(date: .abbreviated, time: .omitted) ?? "—")", systemImage: "calendar")
                }
                .font(.footnote)
                .foregroundStyle(.secondary)

                Spacer()
            }
            .navigationTitle("Vor")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Re-check license") { license.recheck() }
                }
            }
        }
    }
}
