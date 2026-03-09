import AppKit
import SwiftUI

public struct MenuBarView: View {
    @ObservedObject var model: RelayAppModel
    @Environment(\.openWindow) private var openWindow

    public init(model: RelayAppModel) {
        self.model = model
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            currentProfileSection

            Divider()

            profilesPickerSection

            Divider()

            actionsSection
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .frame(width: 330, alignment: .leading)
        .task {
            await model.refreshForMenuOpen()
        }
    }

    private var currentProfileSection: some View {
        Group {
            if let profile = model.activeProfile {
                let usage = model.usageSnapshot(for: profile.id)
                MenuBarCurrentProfileCard(model: currentCardModel(profile: profile, usage: usage))
            } else {
                Text("No active profile")
                    .font(.subheadline)
                    .foregroundStyle(Color(nsColor: .secondaryLabelColor))
            }
        }
    }

    private var profilesPickerSection: some View {
        VStack(alignment: .leading, spacing: 6) {
            if model.profiles.isEmpty {
                MenuBarActionRow(
                    title: "Profiles",
                    systemImage: "person.2",
                    showsChevron: true
                )
                .foregroundStyle(.secondary)
            } else {
                Picker(
                    selection: profilePickerSelection,
                    label: MenuBarActionRow(
                        title: "Profiles",
                        systemImage: "person.2",
                        showsChevron: true
                    )
                ) {
                    ForEach(model.profiles) { profile in
                        let usage = model.usageSnapshot(for: profile.id)
                        MenuBarProfilePickerItem(
                            profileName: profile.nickname,
                            statusText: profileStatusText(profile, usage: usage, isActive: model.activeProfileId == profile.id),
                            sessionText: usageText(title: "Session", window: usage?.session),
                            sessionResetText: usage?.session.resetAt.map { "Resets \(relativeResetDescription(for: $0))" },
                            weeklyText: usageText(title: "Weekly", window: usage?.weekly),
                            weeklyResetText: usage?.weekly.resetAt.map { "Resets \(relativeResetDescription(for: $0))" },
                            footerText: profileSubtitle(profile, usage: usage, isActive: model.activeProfileId == profile.id),
                            symbolName: profileSymbolName(profile, usage: usage, isActive: model.activeProfileId == profile.id),
                            isDimmed: !profile.enabled
                        )
                        .tag(profile.id)
                    }
                }
                .pickerStyle(.menu)
                .disabled(model.isSwitching)
            }
        }
    }

    private var actionsSection: some View {
        VStack(alignment: .leading, spacing: 0) {
            Button {
                Task {
                    await model.refreshEnabledUsage()
                }
            } label: {
                MenuBarActionRow(title: "Refresh", systemImage: "arrow.clockwise")
            }
            .buttonStyle(.plain)
            .disabled(model.isRefreshing)

            Divider()
                .padding(.leading, 22)

            Button {
                NSApplication.shared.activate(ignoringOtherApps: true)
                openWindow(id: "settings")
            } label: {
                MenuBarActionRow(title: "Settings...", systemImage: "gearshape")
            }
            .buttonStyle(.plain)

            Divider()
                .padding(.leading, 22)

            Button {
                NSApplication.shared.terminate(nil)
            } label: {
                MenuBarActionRow(title: "Quit Relay", systemImage: "power")
            }
            .buttonStyle(.plain)
            .keyboardShortcut("q")
        }
    }

    private func activate(_ profile: Profile) async {
        guard profile.enabled, model.activeProfileId != profile.id else {
            return
        }

        await model.switchToProfile(profile.id)
    }

    private var profilePickerSelection: Binding<String> {
        Binding(
            get: { model.selectedProfileId ?? model.activeProfileId ?? model.profiles.first?.id ?? "" },
            set: { newValue in
                model.selectProfile(newValue)

                guard let profile = model.profiles.first(where: { $0.id == newValue }) else {
                    return
                }

                Task {
                    await activate(profile)
                }
            }
        )
    }

    private func profileSubtitle(_ profile: Profile, usage: UsageSnapshot?, isActive: Bool) -> String {
        var parts: [String] = []

        parts.append(profile.agent.rawValue)

        if let usage {
            parts.append(usage.source.rawValue)
            if usage.stale {
                parts.append("Stale")
            }
        } else {
            parts.append("No usage yet")
        }

        parts.append("P\(profile.priority)")

        return parts.joined(separator: " • ")
    }

    private func profileStatusText(_ profile: Profile, usage: UsageSnapshot?, isActive: Bool) -> String {
        if isActive {
            return "Active"
        }

        if !profile.enabled {
            return "Disabled"
        }

        if usage?.stale == true {
            return "Stale"
        }

        return "Ready"
    }

    private func usageText(title: String, window: UsageWindow?) -> String? {
        guard let window else {
            return nil
        }

        return "\(title) \(window.menuBarDisplayValue)"
    }

    private func profileSymbolName(_ profile: Profile, usage: UsageSnapshot?, isActive: Bool) -> String {
        if isActive {
            return "checkmark.circle.fill"
        }

        if !profile.enabled {
            return "slash.circle"
        }

        switch (usage?.weekly.status ?? usage?.session.status) ?? .unknown {
        case .warning:
            return "exclamationmark.circle"
        case .exhausted:
            return "xmark.circle"
        default:
            return "circle"
        }
    }

    private func currentCardModel(profile: Profile, usage: UsageSnapshot?) -> MenuBarCurrentCardModel {
        MenuBarCurrentCardModel(
            providerName: profile.agent.rawValue,
            email: profile.nickname,
            subtitleText: currentCardSubtitle(usage: usage),
            planText: currentCardPlanText(usage: usage),
            metrics: currentMetricRows(usage: usage),
            placeholder: currentCardPlaceholder(usage: usage),
            usageNotes: currentCardNotes(usage: usage)
        )
    }

    private func currentMetricRows(usage: UsageSnapshot?) -> [MenuBarMetricRowModel] {
        guard let usage else {
            return []
        }

        return [
            metricRowModel(id: "session", title: "Session", window: usage.session),
            metricRowModel(id: "weekly", title: "Weekly", window: usage.weekly)
        ]
    }

    private func metricRowModel(id: String, title: String, window: UsageWindow) -> MenuBarMetricRowModel {
        MenuBarMetricRowModel(
            id: id,
            title: title,
            percent: window.menuBarProgressPercent,
            percentLabel: "\(window.menuBarDisplayValue) used",
            resetText: window.resetAt.map { "Resets \(relativeResetDescription(for: $0))" },
            detailLeftText: detailLeftText(for: window),
            detailRightText: detailRightText(for: window),
            tint: window.status.menuBarTint
        )
    }

    private func currentCardSubtitle(usage: UsageSnapshot?) -> String {
        if model.isRefreshing {
            return "Refreshing…"
        }

        if let lastRefresh = model.lastRefresh {
            return "Updated \(relativeTimestamp(for: lastRefresh))"
        }

        return "Waiting for refresh"
    }

    private func currentCardPlanText(usage: UsageSnapshot?) -> String? {
        usage?.source.rawValue
    }

    private func currentCardPlaceholder(usage: UsageSnapshot?) -> String? {
        usage == nil ? "No usage yet" : nil
    }

    private func currentCardNotes(usage: UsageSnapshot?) -> [String] {
        var notes: [String] = []

        if let usage {
            if usage.stale {
                notes.append(usage.message ?? "Usage data is stale.")
            } else if let message = usage.message, message != "usage not fetched yet" {
                notes.append(message)
            }
        }

        return notes
    }

    private func detailLeftText(for window: UsageWindow) -> String? {
        guard window.status == .exhausted else {
            return nil
        }

        return "Unavailable"
    }

    private func detailRightText(for window: UsageWindow) -> String? {
        guard window.status == .exhausted, let resetAt = window.resetAt else {
            return nil
        }

        return "Back \(relativeResetDescription(for: resetAt))"
    }

    private func relativeTimestamp(for date: Date) -> String {
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .short
        return formatter.localizedString(for: date, relativeTo: Date())
    }

    private func relativeResetDescription(for date: Date) -> String {
        let interval = date.timeIntervalSinceNow

        if interval <= 0 {
            return "now"
        }

        let totalMinutes = max(1, Int(ceil(interval / 60)))
        let days = totalMinutes / (24 * 60)
        let hours = (totalMinutes % (24 * 60)) / 60
        let minutes = totalMinutes % 60

        var parts: [String] = []
        if days > 0 {
            parts.append("\(days)d")
        }
        if hours > 0 || !parts.isEmpty {
            parts.append("\(hours)h")
        }
        parts.append("\(minutes)m")

        return "in \(parts.joined(separator: " "))"
    }
}
