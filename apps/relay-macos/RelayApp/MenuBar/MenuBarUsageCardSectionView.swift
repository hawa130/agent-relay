import SwiftUI

struct MenuBarUsageCardSectionView: View {
    let metrics: [MenuBarMetricRowModel]
    let usageNotes: [UsageCardNote]
    @Environment(\.menuItemHighlighted) private var isHighlighted

    var body: some View {
        VStack(alignment: .leading, spacing: 9) {
            ForEach(metrics) { metric in
                MenuBarMetricRow(model: metric)
            }

            if !usageNotes.isEmpty {
                VStack(alignment: .leading, spacing: 3) {
                    ForEach(usageNotes.indices, id: \.self) { index in
                        let note = usageNotes[index]
                        Text(note.text)
                            .font(.system(size: 10.5))
                            .foregroundStyle(MenuBarHighlightStyle.note(isHighlighted, note: note))
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
