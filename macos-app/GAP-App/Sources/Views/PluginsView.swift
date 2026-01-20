import SwiftUI

/// Plugin management view.
///
/// Displays installed plugins with their names and URL patterns,
/// and provides controls to install, update, and uninstall plugins.
struct PluginsView: View {
    @EnvironmentObject var appState: AppState
    @State private var newPluginRepo: String = ""
    @State private var isInstalling: Bool = false
    @State private var isLoading: Bool = false
    @State private var errorMessage: String?
    @State private var successMessage: String?

    var body: some View {
        VStack(spacing: 0) {
            // Install plugin section
            VStack(alignment: .leading, spacing: 8) {
                Text("Install a Plugin")
                    .font(.headline)
                    .foregroundColor(.secondary)

                HStack {
                    TextField("owner/repo (e.g., mikekelly/exa-gap)", text: $newPluginRepo)
                        .textFieldStyle(.roundedBorder)
                        .frame(maxWidth: 300)

                    Button(action: installPlugin) {
                        if isInstalling {
                            ProgressView()
                                .scaleEffect(0.7)
                        } else {
                            Text("Install")
                        }
                    }
                    .disabled(newPluginRepo.isEmpty || isInstalling)
                }

                if let error = errorMessage {
                    Text(error)
                        .foregroundColor(.red)
                        .font(.caption)
                }

                if let success = successMessage {
                    Text(success)
                        .foregroundColor(.green)
                        .font(.caption)
                }
            }
            .padding()
            .background(Color(NSColor.controlBackgroundColor))

            Divider()

            // Installed plugins header
            HStack {
                Text("Installed Plugins")
                    .font(.title2)
                    .fontWeight(.semibold)

                Spacer()

                Button(action: { Task { await refresh() } }) {
                    Image(systemName: "arrow.clockwise")
                }
                .disabled(isLoading)
            }
            .padding(.horizontal)
            .padding(.vertical, 12)

            Divider()

            // Plugin list
            if isLoading && appState.plugins.isEmpty {
                Spacer()
                ProgressView("Loading plugins...")
                Spacer()
            } else if appState.plugins.isEmpty {
                Spacer()
                Text("No plugins installed")
                    .foregroundColor(.secondary)
                Spacer()
            } else {
                List(appState.plugins) { plugin in
                    PluginRow(
                        plugin: plugin,
                        onUpdate: { updatePlugin(plugin.name) },
                        onUninstall: { uninstallPlugin(plugin.name) }
                    )
                }
            }
        }
        .task { await refresh() }
    }

    private func refresh() async {
        isLoading = true
        errorMessage = nil
        do {
            try await appState.refreshPlugins()
        } catch {
            errorMessage = error.localizedDescription
        }
        isLoading = false
    }

    private func installPlugin() {
        guard !newPluginRepo.isEmpty else { return }
        isInstalling = true
        errorMessage = nil
        successMessage = nil

        Task {
            do {
                guard let hash = appState.passwordHash else { return }
                let response = try await appState.client.installPlugin(repo: newPluginRepo, passwordHash: hash)
                successMessage = "Installed \(response.name)"
                newPluginRepo = ""
                try await appState.refreshPlugins()
            } catch {
                errorMessage = error.localizedDescription
            }
            isInstalling = false
        }
    }

    private func updatePlugin(_ name: String) {
        errorMessage = nil
        successMessage = nil

        Task {
            do {
                guard let hash = appState.passwordHash else { return }
                let response = try await appState.client.updatePlugin(name: name, passwordHash: hash)
                successMessage = "Updated \(response.name)"
                try await appState.refreshPlugins()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    private func uninstallPlugin(_ name: String) {
        errorMessage = nil
        successMessage = nil

        Task {
            do {
                guard let hash = appState.passwordHash else { return }
                _ = try await appState.client.uninstallPlugin(name: name, passwordHash: hash)
                successMessage = "Uninstalled \(name)"
                try await appState.refreshPlugins()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }
}

struct PluginRow: View {
    @EnvironmentObject var appState: AppState
    let plugin: Plugin
    let onUpdate: () -> Void
    let onUninstall: () -> Void

    @State private var showingCredentials: Bool = false
    @State private var credentialValues: [String: String] = [:]
    @State private var savingCredential: String? = nil
    @State private var credentialError: String?
    @State private var credentialSuccess: String?

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                Text(plugin.name)
                    .font(.headline)
                Text(plugin.matchPatterns.joined(separator: ", "))
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            Spacer()

            if !plugin.credentialSchema.isEmpty {
                Button(action: { showingCredentials.toggle() }) {
                    Image(systemName: "key")
                }
                .buttonStyle(.bordered)
                .help("Set credentials")
                .popover(isPresented: $showingCredentials, arrowEdge: .trailing) {
                    credentialsPopover
                }
            }

            Button(action: onUpdate) {
                Image(systemName: "arrow.clockwise")
            }
            .buttonStyle(.bordered)
            .help("Update plugin")

            Button(action: onUninstall) {
                Image(systemName: "trash")
            }
            .buttonStyle(.bordered)
            .tint(.red)
            .help("Uninstall plugin")
        }
        .padding(.vertical, 4)
    }

    private var credentialsPopover: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Set Credentials")
                .font(.headline)

            Text(plugin.name)
                .font(.caption)
                .foregroundColor(.secondary)

            Divider()

            ForEach(plugin.credentialSchema, id: \.self) { key in
                HStack {
                    Text(key)
                        .frame(width: 100, alignment: .trailing)

                    SecureField("Enter value", text: binding(for: key))
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 180)

                    Button(action: { saveCredential(key: key) }) {
                        if savingCredential == key {
                            ProgressView()
                                .scaleEffect(0.6)
                        } else {
                            Text("Set")
                        }
                    }
                    .disabled(credentialValues[key]?.isEmpty ?? true || savingCredential != nil)
                    .frame(width: 50)
                }
            }

            if let error = credentialError {
                Text(error)
                    .foregroundColor(.red)
                    .font(.caption)
            }

            if let success = credentialSuccess {
                Text(success)
                    .foregroundColor(.green)
                    .font(.caption)
            }
        }
        .padding()
        .frame(width: 400)
    }

    private func binding(for key: String) -> Binding<String> {
        Binding(
            get: { credentialValues[key] ?? "" },
            set: { credentialValues[key] = $0 }
        )
    }

    private func saveCredential(key: String) {
        guard let value = credentialValues[key], !value.isEmpty else { return }
        savingCredential = key
        credentialError = nil
        credentialSuccess = nil

        Task {
            do {
                guard let hash = appState.passwordHash else { return }
                _ = try await appState.client.setCredential(
                    plugin: plugin.name,
                    key: key,
                    value: value,
                    passwordHash: hash
                )
                credentialSuccess = "\(key) saved"
                credentialValues[key] = ""  // Clear after success
            } catch {
                credentialError = error.localizedDescription
            }
            savingCredential = nil
        }
    }
}
