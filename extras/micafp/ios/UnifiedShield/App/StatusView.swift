import SwiftUI

struct StatusView: View {
    @EnvironmentObject var tunnelManager: TunnelManager
    @State private var pulseAnimation = false

    var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 24) {
                    // Connection indicator
                    ZStack {
                        // Pulse ring
                        if tunnelManager.isConnected {
                            Circle()
                                .stroke(Color.green.opacity(0.3), lineWidth: 3)
                                .frame(width: 160, height: 160)
                                .scaleEffect(pulseAnimation ? 1.1 : 1.0)
                                .opacity(pulseAnimation ? 0.5 : 1.0)
                                .animation(
                                    .easeInOut(duration: 1.5).repeatForever(autoreverses: true),
                                    value: pulseAnimation
                                )
                        }

                        Circle()
                            .fill(statusColor.opacity(0.12))
                            .frame(width: 130, height: 130)
                            .overlay(
                                Circle()
                                    .stroke(statusColor, lineWidth: 3)
                            )

                        Image(systemName: "shield.fill")
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .frame(width: 50, height: 50)
                            .foregroundColor(statusColor)
                    }
                    .onAppear { pulseAnimation = true }
                    .onDisappear { pulseAnimation = false }

                    // Status text
                    Text(tunnelManager.isConnected ? "Connected" : "Disconnected")
                        .font(.title)
                        .fontWeight(.bold)
                        .foregroundColor(statusColor)

                    if tunnelManager.isConnected {
                        Text("Core: \(tunnelManager.currentCore)")
                            .font(.subheadline)
                            .foregroundColor(.secondary)

                        Text("Uptime: \(tunnelManager.connectionUptime)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }

                    // Speed indicators
                    if tunnelManager.isConnected {
                        HStack(spacing: 40) {
                            SpeedIndicator(
                                label: "Download",
                                speed: tunnelManager.downloadSpeed,
                                color: .blue
                            )
                            SpeedIndicator(
                                label: "Upload",
                                speed: tunnelManager.uploadSpeed,
                                color: .orange
                            )
                        }
                        .padding()
                        .background(Color(.systemGray6))
                        .cornerRadius(12)
                    }

                    // DPI score indicator
                    if tunnelManager.isConnected {
                        DpiScoreView(score: tunnelManager.dpiScore)
                    }

                    Spacer(minLength: 20)

                    // Connect/Disconnect button
                    Button(action: {
                        if tunnelManager.isConnected {
                            tunnelManager.disconnect()
                        } else {
                            tunnelManager.connect()
                        }
                    }) {
                        Text(tunnelManager.isConnected ? "DISCONNECT" : "CONNECT")
                            .font(.headline)
                            .foregroundColor(.white)
                            .frame(maxWidth: .infinity)
                            .frame(height: 52)
                            .background(tunnelManager.isConnected ? Color.red : Color.green)
                            .cornerRadius(12)
                    }
                    .padding(.horizontal)
                }
                .padding()
            }
            .navigationTitle("UnifiedShield")
        }
    }

    private var statusColor: Color {
        tunnelManager.isConnected ? .green : .gray
    }
}

struct SpeedIndicator: View {
    let label: String
    let speed: String
    let color: Color

    var body: some View {
        VStack(spacing: 4) {
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
            Text(speed)
                .font(.title3)
                .fontWeight(.semibold)
                .foregroundColor(color)
        }
    }
}

struct DpiScoreView: View {
    let score: Double

    var body: some View {
        VStack(spacing: 8) {
            HStack {
                Text("DPI Score")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                Spacer()
                Text(String(format: "%.2f", score))
                    .font(.subheadline)
                    .fontWeight(.semibold)
                    .foregroundColor(score > 0.72 ? .red : .green)
            }

            ProgressView(value: score, total: 1.0)
                .progressViewStyle(LinearProgressViewStyle(tint: score > 0.72 ? .red : .green))

            if score > 0.72 {
                HStack {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundColor(.orange)
                    Text("DPI detected - auto-switching core")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
            }
        }
        .padding()
        .background(Color(.systemGray6))
        .cornerRadius(12)
    }
}

#Preview {
    StatusView()
        .environmentObject(TunnelManager.shared)
}
