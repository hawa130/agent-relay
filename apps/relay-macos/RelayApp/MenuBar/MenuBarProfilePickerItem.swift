import SwiftUI

struct MenuBarProfilePickerItem: View {
    @ObservedObject var session: RelayAppModel
    let profileID: String
    @Environment(\.menuItemHighlighted) private var isHighlighted

    var body: some View {
        Group {
            if let profile {
                HStack(alignment: .center, spacing: 10) {
                    ringOrIcon

                    VStack(alignment: .leading, spacing: 3) {
                        HStack(alignment: .firstTextBaseline, spacing: 6) {
                            Text(profile.nickname)
                                .font(.system(size: 12.5, weight: .semibold))
                                .lineLimit(1)
                                .foregroundStyle(MenuBarHighlightStyle.primary(isHighlighted))

                            if isActive {
                                currentBadge
                            }

                            Spacer(minLength: 4)

                            if !isActive {
                                Text(presenter.profileStatusText(profile: profile, usage: usage, isActive: false))
                                    .font(.system(size: 10, weight: .medium))
                                    .foregroundStyle(statusColor)
                                    .lineLimit(1)
                            }
                        }

                        if let session = usage?.session, session.status != .unknown,
                           let sessionText = presenter.usageText(title: "Session", window: session)
                        {
                            usageLine(
                                left: sessionText,
                                rightDate: session.resetAt)
                        }

                        if let weekly = usage?.weekly, weekly.status != .unknown,
                           let weeklyText = presenter.usageText(title: "Weekly", window: weekly)
                        {
                            usageLine(
                                left: weeklyText,
                                rightDate: weekly.resetAt)
                        }
                    }
                }
            } else {
                EmptyView()
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 6)
        .opacity(isDimmed && !isHighlighted ? 0.72 : 1)
    }

    @ViewBuilder
    private var ringOrIcon: some View {
        if let usage, !usage.ringProgressItems.isEmpty {
            MultiRingProgressView(
                items: usage.ringProgressItems,
                size: .mini,
                style: RingProgressStyle(
                    trackColor: isHighlighted ? .white : .secondary,
                    trackOpacity: isHighlighted ? 0.22 : 0.14))
            { _ in
                EmptyView()
            }
            .frame(width: 26, height: 26)
        } else {
            Image(systemName: presenter.profileSymbolName(
                profile: profile!, usage: usage, isActive: isActive))
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(symbolColor)
                .frame(width: 26, height: 26)
        }
    }

    private var currentBadge: some View {
        Text("Current")
            .font(.system(size: 9, weight: .semibold))
            .foregroundStyle(isHighlighted
                ? MenuBarHighlightStyle.selectionText
                : Color.accentColor)
            .padding(.horizontal, 5)
            .padding(.vertical, 1.5)
            .background(
                isHighlighted
                    ? Color.white.opacity(0.2)
                    : Color.accentColor.opacity(0.12),
                in: Capsule())
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
