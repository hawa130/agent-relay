import SwiftUI

struct MenuBarUsageCardHeaderView: View {
    let model: MenuBarCurrentCardModel
    @Environment(\.menuItemHighlighted) private var isHighlighted

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            HStack(alignment: .firstTextBaseline) {
                Text(model.providerName)
                    .font(.headline)
                    .fontWeight(.semibold)
                    .foregroundStyle(MenuBarHighlightStyle.primary(isHighlighted))
                    .lineLimit(1)

                Spacer()

                Text(model.email)
                    .font(.subheadline)
                    .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .layoutPriority(1)
            }

            HStack(alignment: .firstTextBaseline) {
                Text(model.subtitleText)
                    .font(.footnote)
                    .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                    .lineLimit(1)
                    .layoutPriority(1)

                Spacer()

                if let planText = model.planText, !planText.isEmpty {
                    Text(planText)
                        .font(.footnote)
                        .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                        .lineLimit(1)
                }
            }
        }
    }
}
