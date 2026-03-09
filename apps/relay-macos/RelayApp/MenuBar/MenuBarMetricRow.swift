import SwiftUI

struct MenuBarMetricRow: View {
    let model: MenuBarMetricRowModel
    @Environment(\.menuItemHighlighted) private var isHighlighted

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(model.title)
                .font(.system(size: 11.5, weight: .semibold))
                .foregroundStyle(MenuBarHighlightStyle.primary(isHighlighted))

            MenuBarUsageProgressBar(
                percent: model.percent,
                tint: model.tint,
                accessibilityLabel: model.title
            )

            VStack(alignment: .leading, spacing: 2) {
                HStack(alignment: .firstTextBaseline) {
                    Text(model.percentLabel)
                        .font(.system(size: 10.5))
                        .foregroundStyle(MenuBarHighlightStyle.primary(isHighlighted))
                        .lineLimit(1)

                    Spacer()

                    if let resetText = model.resetText {
                        Text(resetText)
                            .font(.system(size: 10.5))
                            .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                            .lineLimit(1)
                    }
                }

                if model.detailLeftText != nil || model.detailRightText != nil {
                    HStack(alignment: .firstTextBaseline) {
                        if let detailLeftText = model.detailLeftText {
                            Text(detailLeftText)
                                .font(.system(size: 10.5))
                                .foregroundStyle(MenuBarHighlightStyle.primary(isHighlighted))
                                .lineLimit(1)
                        }

                        Spacer()

                        if let detailRightText = model.detailRightText {
                            Text(detailRightText)
                                .font(.system(size: 10.5))
                                .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                                .lineLimit(1)
                        }
                    }
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
