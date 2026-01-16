import SwiftUI

@main
struct GAPApp: App {
    @StateObject private var appState = AppState()

    var body: some Scene {
        MenuBarExtra {
            MenuBarView()
                .environmentObject(appState)
        } label: {
            Image(systemName: appState.isConnected ? "shield.checkered" : "shield.slash")
        }

        Window("GAP", id: "main") {
            ContentView()
                .environmentObject(appState)
        }
        .defaultSize(width: 700, height: 500)
    }
}

struct ContentView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        if appState.isAuthenticated {
            MainWindow()
        } else {
            PasswordPrompt()
        }
    }
}
