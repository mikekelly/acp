import SwiftUI

/// Activity monitoring view.
///
/// Displays recent proxy requests with timestamps, methods, URLs,
/// agent names, and status codes. Supports manual and auto-refresh.
struct ActivityView: View {
    @EnvironmentObject var appState: AppState
    @State private var isLoading: Bool = false
    @State private var autoRefresh: Bool = false
    @State private var errorMessage: String?
    @State private var refreshTask: Task<Void, Never>?

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Activity Log")
                    .font(.title2)
                    .fontWeight(.semibold)

                Spacer()

                Toggle("Auto-refresh", isOn: $autoRefresh)
                    .toggleStyle(.switch)
                    .onChange(of: autoRefresh) { newValue in
                        if newValue {
                            startAutoRefresh()
                        } else {
                            stopAutoRefresh()
                        }
                    }

                Button(action: { Task { await refresh() } }) {
                    Image(systemName: "arrow.clockwise")
                }
                .disabled(isLoading)
            }
            .padding()

            if let error = errorMessage {
                Text(error)
                    .foregroundColor(.red)
                    .font(.caption)
                    .padding(.horizontal)
            }

            Divider()

            // Activity table
            if isLoading && appState.activity.isEmpty {
                Spacer()
                ProgressView("Loading activity...")
                Spacer()
            } else if appState.activity.isEmpty {
                Spacer()
                Text("No activity recorded")
                    .foregroundColor(.secondary)
                Spacer()
            } else {
                Table(appState.activity) {
                    TableColumn("Time") { entry in
                        Text(formatTimestamp(entry.timestamp))
                            .font(.system(.caption, design: .monospaced))
                    }
                    .width(min: 70, ideal: 80)

                    TableColumn("Method") { entry in
                        Text(entry.method)
                            .font(.system(.caption, design: .monospaced))
                            .foregroundColor(methodColor(entry.method))
                    }
                    .width(min: 50, ideal: 60)

                    TableColumn("URL") { entry in
                        Text(entry.url)
                            .lineLimit(1)
                            .truncationMode(.middle)
                            .help(entry.url)
                    }
                    .width(min: 200, ideal: 300)

                    TableColumn("Agent") { entry in
                        Text(entry.agentId ?? "-")
                            .font(.caption)
                    }
                    .width(min: 80, ideal: 100)

                    TableColumn("Status") { entry in
                        Text("\(entry.status)")
                            .font(.system(.caption, design: .monospaced))
                            .foregroundColor(statusColor(entry.status))
                    }
                    .width(min: 45, ideal: 50)
                }
            }
        }
        .task { await refresh() }
        .onDisappear { stopAutoRefresh() }
    }

    private func refresh() async {
        isLoading = true
        errorMessage = nil
        do {
            try await appState.refreshActivity()
        } catch {
            errorMessage = error.localizedDescription
        }
        isLoading = false
    }

    private func startAutoRefresh() {
        refreshTask = Task {
            while !Task.isCancelled {
                await refresh()
                try? await Task.sleep(for: .seconds(5))
            }
        }
    }

    private func stopAutoRefresh() {
        refreshTask?.cancel()
        refreshTask = nil
    }

    private func formatTimestamp(_ ts: String) -> String {
        // Parse ISO8601 and format as HH:MM:SS
        // Simple approach: extract time part
        if let tIndex = ts.firstIndex(of: "T"),
           let dotIndex = ts.firstIndex(of: ".") ?? ts.firstIndex(of: "Z") {
            let timeStart = ts.index(after: tIndex)
            return String(ts[timeStart..<dotIndex])
        }
        return ts
    }

    private func methodColor(_ method: String) -> Color {
        switch method.uppercased() {
        case "GET": return .blue
        case "POST": return .green
        case "PUT": return .orange
        case "DELETE": return .red
        case "PATCH": return .purple
        default: return .primary
        }
    }

    private func statusColor(_ status: Int) -> Color {
        switch status {
        case 200..<300: return .green
        case 300..<400: return .blue
        case 400..<500: return .orange
        case 500..<600: return .red
        default: return .primary
        }
    }
}
