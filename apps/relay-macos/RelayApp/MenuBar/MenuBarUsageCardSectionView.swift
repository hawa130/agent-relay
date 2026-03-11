import SwiftUI

struct MenuBarUsageCardSectionView: View {
    let metrics: [MenuBarMetricRowModel]
    let usageNotes: [String]
    @Environment(\.menuItemHighlighted) private var isHighlighted

    var body: some View {
        VStack(alignment: .leading, spacing: 9) {
            ForEach(metrics) { metric in
                MenuBarMetricRow(model: metric)
            }

            if !usageNotes.isEmpty {
                VStack(alignment: .leading, spacing: 3) {
                    ForEach(usageNotes, id: \.self) { note in
                        Text(note)
                            .font(.system(size: 10.5))
                            .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                            .lineLimit(2)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }
            }
        }
        .padding(.bottom, 4)
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
