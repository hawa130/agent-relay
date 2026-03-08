import AppKit
import SwiftUI

public struct MenuBarView: View {
    @ObservedObject var model: RelayAppModel
    @Environment(\.openWindow) private var openWindow

    public init(model: RelayAppModel) {
        self.model = model
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            header
            controls
            Divider()
            profileList
            Divider()
            usagePanel
            footer
            Divider()
            appActions
        }
        .padding(16)
        .frame(width: 420)
        .task {
            await model.refreshForMenuOpen()
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
                    await model.refreshEnabledUsage()
                }
            }
            .disabled(model.isRefreshing)

            Button("Refresh Selected") {
                guard let profileID = model.selectedProfile?.id else {
                    return
                }
                Task {
                    await model.refreshUsage(profileID: profileID)
                }
            }
            .disabled(model.selectedProfile == nil)

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
                VStack(alignment: .leading, spacing: 6) {
                    ForEach(model.profiles) { profile in
                        Button {
                            model.selectProfile(profile.id)
                        } label: {
                            HStack(alignment: .top, spacing: 10) {
                                Circle()
                                    .fill(profile.enabled ? Color.green : Color.gray.opacity(0.6))
                                    .frame(width: 8, height: 8)
                                    .padding(.top, 5)

                                VStack(alignment: .leading, spacing: 2) {
                                    Text(profile.nickname)
                                        .font(.subheadline.weight(model.selectedProfileID == profile.id ? .semibold : .regular))
                                        .foregroundStyle(.primary)
                                    Text(profileSubtitle(profile))
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                        .lineLimit(1)
                                }

                                Spacer(minLength: 8)

                                if model.activeProfileID == profile.id {
                                    Image(systemName: "checkmark.square.fill")
                                        .foregroundStyle(.tint)
                                }
                            }
                            .padding(.horizontal, 10)
                            .padding(.vertical, 8)
                            .background(
                                RoundedRectangle(cornerRadius: 10, style: .continuous)
                                    .fill(model.selectedProfileID == profile.id ? Color.accentColor.opacity(0.14) : Color.gray.opacity(0.08))
                            )
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
        }
    }

    private var usagePanel: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(model.selectedProfile?.nickname ?? "Usage")
                    .font(.headline)
                Spacer()
                if let profile = model.selectedProfile, profile.enabled {
                    Button("Switch") {
                        Task {
                            await model.switchToProfile(profile.id)
                        }
                    }
                    .disabled(model.activeProfileID == profile.id || model.isSwitching)
                }
            }

            if let usage = model.selectedUsage {
                UsageRow(title: "Session", window: usage.session)
                UsageRow(title: "Weekly", window: usage.weekly)

                Text(sourceLine(for: usage))
                    .font(.caption)
                    .foregroundStyle(.secondary)

                if let resetAt = usage.nextResetAt {
                    Text("Next reset: \(resetAt.formatted(date: .abbreviated, time: .shortened))")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                if usage.stale || usage.message != nil {
                    Text(usage.message ?? "Usage data is stale.")
                        .font(.caption)
                        .foregroundStyle(.orange)
                }
            } else if let profile = model.selectedProfile, !profile.enabled {
                Text("Disabled profile. Refresh manually to inspect usage.")
                    .foregroundStyle(.secondary)
            } else {
                Text("Usage data unavailable.")
                    .foregroundStyle(.secondary)
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
            if let settings = model.status?.settings {
                Text("Source mode: \(settings.usageSourceMode.displayName)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            if let lastRefresh = model.lastRefresh {
                Text("Last refresh: \(lastRefresh.formatted(date: .omitted, time: .standard))")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private func profileSubtitle(_ profile: Profile) -> String {
        if let usage = model.usageSnapshot(for: profile.id) {
            if usage.message == "usage not fetched yet" {
                return profile.enabled ? "usage not fetched yet" : "Disabled"
            }
            let freshness = usage.stale ? "stale" : "fresh"
            return "\(usage.source.rawValue) • \(freshness)"
        }
        return profile.enabled ? "usage not fetched yet" : "Disabled"
    }

    private func sourceLine(for usage: UsageSnapshot) -> String {
        var line = "Source: \(usage.source.rawValue) • \(usage.confidence.rawValue)"
        if usage.stale {
            line += " • stale"
        }
        return line
    }
}

private struct UsageRow: View {
    let title: String
    let window: UsageWindow

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(title)
                    .font(.subheadline.weight(.medium))
                Spacer()
                Text(valueLabel)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            ProgressView(value: progressValue)
                .progressViewStyle(.linear)
                .tint(tint)

            if let resetAt = window.resetAt {
                Text("Resets \(resetAt.formatted(date: .omitted, time: .shortened))")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var progressValue: Double {
        guard let usedPercent = window.usedPercent else {
            return window.status == .exhausted ? 1 : 0
        }
        return min(max(usedPercent / 100, 0), 1)
    }

    private var tint: Color {
        switch window.status {
        case .healthy:
            return .green
        case .warning:
            return .orange
        case .exhausted:
            return .red
        case .unknown:
            return .gray
        }
    }

    private var valueLabel: String {
        if let usedPercent = window.usedPercent {
            return String(format: "%.0f%%", usedPercent)
        }
        return window.status.rawValue
    }
}
