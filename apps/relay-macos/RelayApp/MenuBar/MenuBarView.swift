import AppKit
import SwiftUI

struct MenuBarView: View {
    @ObservedObject var model: RelayAppModel
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            header
            controls
            Divider()
            profileList
            Divider()
            appActions
            Divider()
            footer
        }
        .padding(16)
        .frame(width: 360)
        .task {
            await model.refresh()
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Current Profile")
                .font(.caption)
                .foregroundStyle(.secondary)

            Text(model.activeProfile?.nickname ?? "No Active Profile")
                .font(.headline)

            if let status = model.status {
                Text("Auto-switch: \(status.settings.autoSwitchEnabled ? "On" : "Off")")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                Text("Relay home: \(status.relayHome)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            } else {
                Text("Waiting for relay status")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var controls: some View {
        HStack(spacing: 10) {
            Button("Refresh") {
                Task {
                    await model.refresh()
                }
            }
            .disabled(model.isRefreshing)

            Button("Settings") {
                NSApplication.shared.activate(ignoringOtherApps: true)
                openWindow(id: "settings")
            }
        }
    }

    private var profileList: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Profiles")
                .font(.headline)

            if model.profiles.isEmpty {
                Text("No profiles configured.")
                    .foregroundStyle(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Selected")
                        .font(.caption)
                        .foregroundStyle(.secondary)

                    Text(model.activeProfile?.nickname ?? "No Active Profile")
                        .font(.subheadline.weight(.medium))
                        .lineLimit(1)
                }

                Picker("Switch Profile", selection: profileSelection) {
                    ForEach(model.profiles) { profile in
                        Text(profile.enabled ? profile.nickname : "\(profile.nickname) (Disabled)")
                            .tag(profile.id)
                    }
                }
                .pickerStyle(.menu)
                .disabled(model.isSwitching || model.profiles.isEmpty)
            }
        }
    }

    private var appActions: some View {
        VStack(alignment: .leading, spacing: 8) {
            Button("Open Relay Home") {
                guard let relayHome = model.status?.relayHome else {
                    return
                }
                NSWorkspace.shared.open(URL(fileURLWithPath: relayHome))
            }
            .disabled(model.status?.relayHome == nil)
            .frame(maxWidth: .infinity, alignment: .leading)

            Button("Quit Relay") {
                NSApplication.shared.terminate(nil)
            }
            .keyboardShortcut("q")
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private var footer: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let lastRefresh = model.lastRefresh {
                Text("Last refresh: \(lastRefresh.formatted(date: .omitted, time: .standard))")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }
    
    private var profileSelection: Binding<String> {
        Binding(
            get: { model.activeProfileID ?? model.profiles.first?.id ?? "" },
            set: { profileID in
                guard !profileID.isEmpty, profileID != model.activeProfileID else {
                    return
                }
                Task {
                    await model.switchToProfile(profileID)
                }
            }
        )
    }
}
