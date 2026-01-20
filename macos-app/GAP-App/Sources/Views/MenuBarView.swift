import SwiftUI

/// Menu bar dropdown view showing server and connection status with quick actions.
///
/// This view appears when the user clicks the menu bar icon and provides:
/// - Server status indicator
/// - Server control buttons (start/stop)
/// - Connection status indicator
/// - Open main window button
/// - Lock/logout button (when authenticated)
/// - Quit button
struct MenuBarView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.openWindow) var openWindow

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            // Server status
            HStack {
                Circle()
                    .fill(appState.serverRunning ? Color.green : Color.red)
                    .frame(width: 8, height: 8)
                Text(appState.serverRunning ? "Server Running" : "Server Stopped")
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 4)

            // Server controls
            if !appState.serverInstalled {
                Button("Install Server") {
                    try? "MenuBar Install clicked at \(Date())".write(toFile: "/tmp/gap-menubar-install.txt", atomically: true, encoding: .utf8)
                    appState.installServer()
                }
                .padding(.horizontal, 8)
            } else if appState.serverRunning {
                Button("Stop Server") {
                    appState.stopServer()
                }
                .padding(.horizontal, 8)
            } else {
                Button("Start Server") {
                    appState.startServer()
                }
                .padding(.horizontal, 8)
            }

            Divider()

            Button("Open GAP") {
                openWindow(id: "main")
                NSApp.activate(ignoringOtherApps: true)
            }
            .padding(.horizontal, 8)

            if appState.isAuthenticated {
                Button("Lock") {
                    appState.logout()
                }
                .padding(.horizontal, 8)
            }

            Divider()

            Button("Quit") {
                NSApplication.shared.terminate(nil)
            }
            .padding(.horizontal, 8)
        }
        .padding(.vertical, 8)
        .frame(width: 180)
    }
}
