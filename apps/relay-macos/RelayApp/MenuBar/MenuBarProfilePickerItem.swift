import SwiftUI

struct MenuBarProfilePickerItem: View {
    @ObservedObject var session: RelayAppModel
    let profileID: String
    @Environment(\.menuItemHighlighted) private var isHighlighted

    var body: some View {
        Group {
            if let profile {
                HStack(alignment: .top, spacing: 8) {
                    Image(systemName: presenter.profileSymbolName(profile: profile, usage: usage, isActive: isActive))
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(symbolColor)
                        .frame(width: 13, height: 13)
                        .padding(.top, 2)

                    VStack(alignment: .leading, spacing: 3) {
                        HStack(alignment: .firstTextBaseline, spacing: 6) {
                            Text(profile.nickname)
                                .font(.system(size: 12.5, weight: .semibold))
                                .lineLimit(1)
                                .foregroundStyle(MenuBarHighlightStyle.primary(isHighlighted))

                            Spacer(minLength: 8)

                            Text(presenter.profileStatusText(profile: profile, usage: usage, isActive: isActive))
                                .font(.system(size: 10, weight: .medium))
                                .foregroundStyle(statusColor)
                                .lineLimit(1)
                        }

                        if let sessionText = presenter.usageText(title: "Session", window: usage?.session) {
                            usageLine(
                                left: sessionText,
                                rightDate: usage?.session.resetAt)
                        }

                        if let weeklyText = presenter.usageText(title: "Weekly", window: usage?.weekly) {
                            usageLine(
                                left: weeklyText,
                                rightDate: usage?.weekly.resetAt)
                        }

                        if let footerText = presenter.profileFooterText(profile: profile, usage: usage) {
                            Text(footerText)
                                .font(.system(size: 10))
                                .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                                .lineLimit(1)
                        }
                    }
                }
            } else {
                EmptyView()
            }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
        .opacity(isDimmed && !isHighlighted ? 0.72 : 1)
    }

    private func usageLine(
        left: String,
        rightDate: Date?) -> some View
    {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text(left)
                .font(.system(size: 10, weight: .medium))
                .monospacedDigit()
                .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                .lineLimit(1)

            Spacer(minLength: 8)

            if let rightDate {
                ResetRelativeDateText(date: rightDate)
                    .font(.system(size: 10))
                    .monospacedDigit()
                    .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                    .lineLimit(1)
            }
        }
    }

    private var presenter: MenuBarPresenter {
        MenuBarPresenter(session: session)
    }

    private var profile: Profile? {
        session.profiles.first { $0.id == profileID }
    }

    private var usage: UsageSnapshot? {
        session.usageSnapshot(for: profileID)
    }

    private var isActive: Bool {
        session.activeProfileId == profileID
    }

    private var isDimmed: Bool {
        !(profile?.enabled ?? true)
    }

    private var symbolColor: Color {
        guard let profile else {
            return MenuBarHighlightStyle.primary(isHighlighted)
        }

        if let severity = presenter.profileStatusSeverity(
            profile: profile,
            usage: usage,
            isActive: isActive)
        {
            return MenuBarHighlightStyle.severityIcon(isHighlighted, severity: severity)
        }

        return MenuBarHighlightStyle.primary(isHighlighted)
    }

    private var statusColor: Color {
        guard let profile else {
            return MenuBarHighlightStyle.secondary(isHighlighted)
        }

        if let severity = presenter.profileStatusSeverity(
            profile: profile,
            usage: usage,
            isActive: isActive)
        {
            return MenuBarHighlightStyle.severityText(isHighlighted, severity: severity)
        }

        return MenuBarHighlightStyle.secondary(isHighlighted)
    }
}
