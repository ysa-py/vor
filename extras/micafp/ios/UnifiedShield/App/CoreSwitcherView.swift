import SwiftUI

struct CoreSwitcherView: View {
    @EnvironmentObject var tunnelManager: TunnelManager
    @State private var isSwitching = false

    private let cores = [
        CoreInfo(
            id: "xray",
            name: "Xray",
            description: "VLESS/VMess with XTLS. Best for general use.",
            protocol: "VLESS/VMess",
            obfuscation: true,
            recommended: true
        ),
        CoreInfo(
            id: "naive",
            name: "NaïveProxy",
            description: "Chrome network stack. Anti-DPI with domain fronting.",
            protocol: "HTTP/2",
            obfuscation: true,
            recommended: false
        ),
        CoreInfo(
            id: "hysteria2",
            name: "Hysteria2",
            description: "QUIC-based. Fast on unstable connections.",
            protocol: "QUIC",
            obfuscation: false,
            recommended: false
        ),
        CoreInfo(
            id: "tuic",
            name: "TUIC",
            description: "QUIC proxy with multiplexing. Low overhead.",
            protocol: "QUIC",
            obfuscation: false,
            recommended: false
        )
    ]

    var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 12) {
                    // DPI warning
                    HStack {
                        Image(systemName: "exclamationmark.triangle.fill")
                            .foregroundColor(.orange)
                        Text("Auto-switch triggers when DPI score > 0.72")
                            .font(.caption)
                            .foregroundColor(.orange)
                        Spacer()
                    }
                    .padding(12)
                    .background(Color.orange.opacity(0.1))
                    .cornerRadius(10)

                    // Core cards
                    ForEach(cores) { core in
                        CoreCard(
                            core: core,
                            isSelected: tunnelManager.currentCore == core.id,
                            isSwitching: isSwitching && tunnelManager.currentCore == core.id,
                            onSelect: {
                                switchCore(to: core.id)
                            }
                        )
                    }
                }
                .padding()
            }
            .navigationTitle("Protocol Core")
        }
    }

    private func switchCore(to coreId: String) {
        guard tunnelManager.currentCore != coreId else { return }
        isSwitching = true
        tunnelManager.switchCore(to: coreId) {
            isSwitching = false
        }
    }
}

struct CoreInfo: Identifiable {
    let id: String
    let name: String
    let description: String
    let protocol: String
    let obfuscation: Bool
    let recommended: Bool
}

struct CoreCard: View {
    let core: CoreInfo
    let isSelected: Bool
    let isSwitching: Bool
    let onSelect: () -> Void

    var body: some View {
        Button(action: onSelect) {
            HStack(spacing: 12) {
                // Radio indicator
                ZStack {
                    Circle()
                        .stroke(isSelected ? Color.blue : Color.gray.opacity(0.3), lineWidth: 2)
                        .frame(width: 24, height: 24)
                    if isSelected {
                        Circle()
                            .fill(Color.blue)
                            .frame(width: 14, height: 14)
                    }
                }

                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text(core.name)
                            .font(.headline)
                        if core.recommended {
                            Text("Recommended")
                                .font(.caption2)
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(Color.blue.opacity(0.15))
                                .foregroundColor(.blue)
                                .cornerRadius(4)
                        }
                    }
                    Text(core.description)
                        .font(.caption)
                        .foregroundColor(.secondary)

                    HStack(spacing: 6) {
                        Text(core.protocol)
                            .font(.caption2)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Color.gray.opacity(0.1))
                            .cornerRadius(4)

                        if core.obfuscation {
                            Text("Obfuscation")
                                .font(.caption2)
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(Color.green.opacity(0.1))
                                .foregroundColor(.green)
                                .cornerRadius(4)
                        }
                    }
                }

                Spacer()

                if isSwitching {
                    ProgressView()
                }
            }
            .padding()
            .background(
                RoundedRectangle(cornerRadius: 12)
                    .fill(Color(.systemBackground))
                    .shadow(color: .black.opacity(0.05), radius: 2, x: 0, y: 1)
                    .overlay(
                        RoundedRectangle(cornerRadius: 12)
                            .stroke(isSelected ? Color.blue : Color.gray.opacity(0.2), lineWidth: isSelected ? 2 : 1)
                    )
            )
        }
        .buttonStyle(PlainButtonStyle())
    }
}

#Preview {
    CoreSwitcherView()
        .environmentObject(TunnelManager.shared)
}
