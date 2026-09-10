import SwiftUI

struct ContentView: View {
    @StateObject private var tunnelManager = TunnelManager.shared
    @State private var selectedTab = 0

    var body: some View {
        TabView(selection: $selectedTab) {
            StatusView()
                .tabItem {
                    Label("Status", systemImage: "shield.fill")
                }
                .tag(0)

            CoreSwitcherView()
                .tabItem {
                    Label("Cores", systemImage: "arrow.left.arrow.right")
                }
                .tag(1)

            SettingsView()
                .tabItem {
                    Label("Settings", systemImage: "gearshape.fill")
                }
                .tag(2)
        }
        .tint(.blue)
        .environmentObject(tunnelManager)
    }
}

#Preview {
    ContentView()
}
