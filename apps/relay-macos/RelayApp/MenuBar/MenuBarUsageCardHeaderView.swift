import SwiftUI

enum MenuBarHeaderSubtitle {
    case refreshing
    case updated(Date)
    case waiting
}

struct MenuBarUsageCardHeaderView: View {
    let providerName: String
    let nickname: String
    let subtitle: MenuBarHeaderSubtitle
    let planText: String?
    @Environment(\.menuItemHighlighted) private var isHighlighted

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack(alignment: .firstTextBaseline) {
                HStack(alignment: .center, spacing: 5) {
                    Text(nickname)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(MenuBarHighlightStyle.primary(isHighlighted))
                        .lineLimit(1)

                    if let planText, !planText.isEmpty {
                        MenuBarPlanBadge(title: planText)
                    }
                }

                Spacer()

                Text(providerName)
                    .font(.system(size: 11))
                    .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .layoutPriority(1)
            }

            HStack(alignment: .firstTextBaseline) {
                subtitleView
                    .layoutPriority(1)

                Spacer()
            }
        }
    }

    @ViewBuilder
    private var subtitleView: some View {
        switch subtitle {
        case .refreshing:
            Text("Refreshing…")
                .font(.system(size: 10.5))
                .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                .lineLimit(1)
        case let .updated(date):
            AdaptiveRelativeDateText(prefix: "Updated ", date: date, style: .named)
                .font(.system(size: 10.5))
                .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                .lineLimit(1)
        case .waiting:
            Text("Waiting for refresh")
                .font(.system(size: 10.5))
                .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                .lineLimit(1)
        }
    }
}

extension String {
    var capitalizingFirst: String {
        guard let first else { return self }
        return first.uppercased() + dropFirst()
    }
}
