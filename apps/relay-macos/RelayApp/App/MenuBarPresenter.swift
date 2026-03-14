import Foundation

@MainActor
struct MenuBarPresenter {
    let session: RelayAppModel

    var title: String {
        session.menuBarTitle
    }

    func currentCardNotes(usage: UsageSnapshot?) -> [UsageCardNote] {
        UsageCardNoteResolver.note(usage: usage, usageRefreshError: nil).map { [$0] } ?? []
    }

    func profileStatusText(profile: Profile, usage: UsageSnapshot?, isActive: Bool) -> String {
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

    func profileStatusSeverity(profile: Profile, usage: UsageSnapshot?, isActive: Bool) -> UsageAlertSeverity? {
        guard !isActive, profile.enabled else {
            return nil
        }

        return UsageCardNoteResolver.severity(for: usage)
    }

    func profileFooterText(profile: Profile, usage: UsageSnapshot?) -> String? {
        var parts = [profile.agent.rawValue]

        if let usage {
            parts.append(usage.source.displayName)
            if usage.stale {
                parts.append("Stale")
            }
        } else {
            parts.append("No usage yet")
        }

        parts.append("P\(profile.priority)")
        return parts.joined(separator: " • ")
    }

    func profileSymbolName(profile: Profile, usage: UsageSnapshot?, isActive: Bool) -> String {
        if isActive {
            return "checkmark.circle.fill"
        }

        if !profile.enabled {
            return "slash.circle"
        }

        if let severity = profileStatusSeverity(profile: profile, usage: usage, isActive: isActive) {
            switch severity {
            case .warning:
                return "exclamationmark.triangle.fill"
            }
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

    func usageText(title: String, window: UsageWindow?) -> String? {
        guard let window else {
            return nil
        }

        return "\(title) \(window.menuBarDisplayValue)"
    }
}
