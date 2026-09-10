import SwiftUI

struct SettingsView: View {
    @AppStorage("killSwitchEnabled") private var killSwitchEnabled = true
    @AppStorage("splitTunnelEnabled") private var splitTunnelEnabled = true
    @AppStorage("autoCoreSwitchEnabled") private var autoCoreSwitchEnabled = true
    @AppStorage("autoUpdateEnabled") private var autoUpdateEnabled = true
    @AppStorage("startOnBootEnabled") private var startOnBootEnabled = false
    @AppStorage("dnsProvider") private var dnsProvider = "alibaba"
    @AppStorage("obfuscationLevel") private var obfuscationLevel = 1

    var body: some View {
        NavigationView {
            Form {
                // Security Section
                Section(header: Text("Security")) {
                    Toggle("Kill Switch", isOn: $killSwitchEnabled)
                    Toggle("Split Tunnel", isOn: $splitTunnelEnabled)
                    Toggle("Auto Core Switch", isOn: $autoCoreSwitchEnabled)

                    VStack(alignment: .leading) {
                        Text("Obfuscation Level")
                            .font(.subheadline)
                        Text("Higher = more resistant but slower")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        HStack {
                            Text("Low")
                                .font(.caption)
                            Slider(value: Binding(
                                get: { Double(obfuscationLevel) },
                                set: { obfuscationLevel = Int($0) }
                            ), in: 0...3, step: 1)
                            Text("Max")
                                .font(.caption)
                        }
                    }
                }

                // DNS Section
                Section(header: Text("DNS")) {
                    Picker("DNS Provider", selection: $dnsProvider) {
                        Text("Alibaba DNS (223.5.5.5)").tag("alibaba")
                        Text("Tencent DNS (119.29.29.29)").tag("tencent")
                        Text("Tencent Backup (1.12.12.12)").tag("tencent-backup")
                    }
                    .pickerStyle(.menu)

                    Text("Chinese CDN primary (Cloudflare blocked in Iran)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                // General Section
                Section(header: Text("General")) {
                    Toggle("Auto Update (6h interval)", isOn: $autoUpdateEnabled)
                    Toggle("Start on Boot", isOn: $startOnBootEnabled)
                }

                // About Section
                Section(header: Text("About")) {
                    HStack {
                        Text("Version")
                        Spacer()
                        Text("1.0.0")
                            .foregroundColor(.secondary)
                    }
                    Text("Next-gen anti-censorship VPN for Iran")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text("No jailbreak required • Split tunnel • DPI evasion")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
            .navigationTitle("Settings")
        }
    }
}

#Preview {
    SettingsView()
}
