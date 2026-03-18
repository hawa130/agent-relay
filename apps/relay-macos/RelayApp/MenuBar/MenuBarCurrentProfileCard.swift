import SwiftUI

struct MenuBarCurrentProfileCard: View {
    @ObservedObject var session: RelayAppModel

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            if let activeProfile {
                MenuBarUsageCardHeaderView(
                    providerName: activeProfile.agent.rawValue,
                    nickname: activeProfile.nickname,
                    subtitle: subtitle,
                    planText: usage?.source.displayName)

                if metrics.isEmpty {
                    Text("No usage yet")
                        .font(.subheadline)
                        .foregroundStyle(Color(nsColor: .secondaryLabelColor))
                } else {
                    MenuBarUsageCardSectionView(
                        metrics: metrics,
                        usageNotes: presenter.currentCardNotes(usage: usage))
                }
            } else {
                Text("No active profile")
                    .font(.subheadline)
                    .foregroundStyle(Color(nsColor: .secondaryLabelColor))
            }
        }
        .padding(.horizontal, 13)
        .padding(.top, 1)
        .padding(.bottom, 1)
        .frame(width: 300, alignment: .leading)
    }

    private var presenter: MenuBarPresenter {
        MenuBarPresenter(session: session)
    }

    private var activeProfile: Profile? {
        session.activeProfile
    }

    private var usage: UsageSnapshot? {
        guard let activeProfile else {
            return nil
        }
        return session.usageSnapshot(for: activeProfile.id)
    }

    private var subtitle: MenuBarHeaderSubtitle {
        if session.isRefreshingUsageList {
            return .refreshing
        }
        if let lastRefresh = session.lastRefresh {
            return .updated(lastRefresh)
        }
        return .waiting
    }

    private var metrics: [MenuBarMetricRowModel] {
        guard let usage else {
            return []
        }

        return [
            MenuBarMetricRowModel(
                id: "session",
                title: "Session",
                percent: usage.session.menuBarProgressPercent,
                percentLabel: "\(usage.session.menuBarDisplayValue) used",
                resetDate: usage.session.resetAt,
                detailLeftText: nil,
                detailRightText: nil,
                tint: usage.session.status.menuBarTint),
            MenuBarMetricRowModel(
                id: "weekly",
                title: "Weekly",
                percent: usage.weekly.menuBarProgressPercent,
                percentLabel: "\(usage.weekly.menuBarDisplayValue) used",
                resetDate: usage.weekly.resetAt,
                detailLeftText: nil,
                detailRightText: nil,
                tint: usage.weekly.status.menuBarTint)
        ]
    }
}
