import SwiftUI

struct MenuBarUsageCardSectionView: View {
    let model: MenuBarCurrentCardModel
    @Environment(\.menuItemHighlighted) private var isHighlighted

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            ForEach(model.metrics) { metric in
                MenuBarMetricRow(model: metric)
            }

            if !model.usageNotes.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(model.usageNotes, id: \.self) { note in
                        Text(note)
                            .font(.footnote)
                            .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                            .lineLimit(2)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }
            }
        }
        .padding(.bottom, 6)
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
