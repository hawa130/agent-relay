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
            profilePickerSection
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
                guard let profileId = model.selectedProfile?.id else {
                    return
                }
                Task {
                    await model.refreshUsage(profileId: profileId)
                }
            }
            .disabled(model.selectedProfile == nil)

            Button("Settings") {
                NSApplication.shared.activate(ignoringOtherApps: true)
                openWindow(id: "settings")
            }
        }
    }

    private var profilePickerSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Profiles")
                .font(.headline)

            if model.profiles.isEmpty {
                Text("No profiles configured.")
                    .foregroundStyle(.secondary)
            } else {
                Menu {
                    ForEach(model.profiles) { profile in
                        Button {
                            model.selectProfile(profile.id)
                        } label: {
                            HStack(spacing: 10) {
                                VStack(alignment: .leading, spacing: 3) {
                                    Text(profile.nickname)
                                    Text(profileSubtitle(profile))
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                                Spacer(minLength: 12)
                                if let usage = model.usageSnapshot(for: profile.id) {
                                    UsageBadgeRow(usage: usage)
                                }
                                if model.activeProfileId == profile.id {
                                    Image(systemName: "checkmark.square.fill")
                                        .foregroundStyle(.tint)
                                }
                            }
                        }
                    }
                } label: {
                    HStack(alignment: .center, spacing: 10) {
                        VStack(alignment: .leading, spacing: 3) {
                            Text(model.selectedProfile?.nickname ?? "Select a profile")
                                .font(.subheadline.weight(.semibold))
                                .foregroundStyle(.primary)
                            if let profile = model.selectedProfile {
                                Text(profileSubtitle(profile))
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .lineLimit(1)
                            }
                        }
                        Spacer(minLength: 8)
                        if let usage = model.selectedUsage {
                            UsageBadgeRow(usage: usage)
                        }
                        Image(systemName: "chevron.down")
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(.secondary)
                    }
                    .padding(.horizontal, 10)
                    .padding(.vertical, 10)
                    .background(
                        RoundedRectangle(cornerRadius: 12, style: .continuous)
                            .fill(Color.gray.opacity(0.08))
                    )
                }
                .menuStyle(.borderlessButton)
            }
        }
    }

    private var usagePanel: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("Usage")
                    .font(.headline)
                Spacer()
                if let profile = model.selectedProfile, profile.enabled {
                    Button("Switch") {
                        Task {
                            await model.switchToProfile(profile.id)
                        }
                    }
                    .disabled(model.activeProfileId == profile.id || model.isSwitching)
                }
            }

            if let usage = model.selectedUsage {
                UsageBadgeRow(usage: usage)
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
            return "\(profile.enabled ? "Enabled" : "Disabled") • \(usage.source.rawValue) • \(freshness)"
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
