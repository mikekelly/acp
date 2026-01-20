import SwiftUI
import AppKit

/// View for token management.
///
/// Displays agent tokens with names, prefixes, and creation dates,
/// and provides controls to create and revoke tokens.
struct TokensView: View {
    @EnvironmentObject var appState: AppState
    @State private var newTokenName: String = ""
    @State private var isCreating: Bool = false
    @State private var isLoading: Bool = false
    @State private var createdToken: String?
    @State private var errorMessage: String?

    var body: some View {
        VStack(spacing: 0) {
            // Header with create form
            VStack(alignment: .leading, spacing: 12) {
                Text("Agent Tokens")
                    .font(.title2)
                    .fontWeight(.semibold)

                HStack {
                    TextField("Token name (e.g., claude-code)", text: $newTokenName)
                        .textFieldStyle(.roundedBorder)
                        .frame(maxWidth: 250)

                    Button(action: createToken) {
                        if isCreating {
                            ProgressView()
                                .scaleEffect(0.7)
                        } else {
                            Text("Create")
                        }
                    }
                    .disabled(newTokenName.isEmpty || isCreating)

                    Spacer()

                    Button(action: { Task { await refresh() } }) {
                        Image(systemName: "arrow.clockwise")
                    }
                    .disabled(isLoading)
                }

                // Show created token (only visible once!)
                if let token = createdToken {
                    HStack {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundColor(.green)
                        Text("Token created:")
                        Text(token)
                            .font(.system(.body, design: .monospaced))
                            .textSelection(.enabled)
                        Button(action: { copyToClipboard(token) }) {
                            Image(systemName: "doc.on.doc")
                        }
                        .help("Copy to clipboard")
                        Button("Dismiss") {
                            createdToken = nil
                        }
                    }
                    .padding(8)
                    .background(Color.green.opacity(0.1))
                    .cornerRadius(8)
                }

                if let error = errorMessage {
                    Text(error)
                        .foregroundColor(.red)
                        .font(.caption)
                }
            }
            .padding()

            Divider()

            // Token list
            if isLoading && appState.tokens.isEmpty {
                Spacer()
                ProgressView("Loading tokens...")
                Spacer()
            } else if appState.tokens.isEmpty {
                Spacer()
                Text("No tokens created")
                    .foregroundColor(.secondary)
                Spacer()
            } else {
                List(appState.tokens) { token in
                    TokenRow(token: token, onRevoke: { revokeToken(token.id) })
                }
            }
        }
        .task { await refresh() }
    }

    private func refresh() async {
        isLoading = true
        errorMessage = nil
        do {
            try await appState.refreshTokens()
        } catch {
            errorMessage = error.localizedDescription
        }
        isLoading = false
    }

    private func createToken() {
        guard !newTokenName.isEmpty else { return }
        isCreating = true
        errorMessage = nil
        createdToken = nil

        Task {
            do {
                guard let hash = appState.passwordHash else { return }
                let response = try await appState.client.createToken(name: newTokenName, passwordHash: hash)
                createdToken = response.token  // Show full token ONCE
                newTokenName = ""
                try await appState.refreshTokens()
            } catch {
                errorMessage = error.localizedDescription
            }
            isCreating = false
        }
    }

    private func revokeToken(_ id: String) {
        errorMessage = nil

        Task {
            do {
                guard let hash = appState.passwordHash else { return }
                _ = try await appState.client.revokeToken(id: id, passwordHash: hash)
                try await appState.refreshTokens()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    private func copyToClipboard(_ text: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }
}

struct TokenRow: View {
    let token: Token
    let onRevoke: () -> Void

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                Text(token.name)
                    .font(.headline)
                HStack(spacing: 8) {
                    Text(token.prefix + "...")
                        .font(.system(.caption, design: .monospaced))
                        .foregroundColor(.secondary)
                    Text("Created: \(formatDate(token.createdAt))")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }

            Spacer()

            Button("Revoke", action: onRevoke)
                .buttonStyle(.bordered)
                .tint(.red)
        }
        .padding(.vertical, 4)
    }

    private func formatDate(_ isoString: String) -> String {
        // Simple date formatting - just show date part
        if let range = isoString.range(of: "T") {
            return String(isoString[..<range.lowerBound])
        }
        return isoString
    }
}
