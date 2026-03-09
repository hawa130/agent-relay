import SwiftUI

struct MenuBarUsageCardHeaderView: View {
    let model: MenuBarCurrentCardModel
    @Environment(\.menuItemHighlighted) private var isHighlighted

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack(alignment: .firstTextBaseline) {
                Text(model.providerName)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(MenuBarHighlightStyle.primary(isHighlighted))
                    .lineLimit(1)

                Spacer()

                Text(model.email)
                    .font(.system(size: 11))
                    .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .layoutPriority(1)
            }

            HStack(alignment: .firstTextBaseline) {
                Text(model.subtitleText)
                    .font(.system(size: 10.5))
                    .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                    .lineLimit(1)
                    .layoutPriority(1)

                Spacer()

                if let planText = model.planText, !planText.isEmpty {
                    Text(planText)
                        .font(.system(size: 10.5))
                        .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                        .lineLimit(1)
                }
            }
        }
    }
}
