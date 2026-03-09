import SwiftUI

struct MenuBarActionRow: View {
    let title: String
    let systemImage: String
    let trailing: String?
    let showsChevron: Bool

    init(
        title: String,
        systemImage: String,
        trailing: String? = nil,
        showsChevron: Bool = false
    ) {
        self.title = title
        self.systemImage = systemImage
        self.trailing = trailing
        self.showsChevron = showsChevron
    }

    @Environment(\.menuItemHighlighted) private var isHighlighted

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: systemImage)
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                .frame(width: 14)

            Text(title)
                .font(.system(size: 14))
                .foregroundStyle(MenuBarHighlightStyle.primary(isHighlighted))

            Spacer()

            if let trailing {
                Text(trailing)
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
            }

            if showsChevron {
                Image(systemName: "chevron.right")
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
            }
        }
        .padding(.vertical, 8)
        .contentShape(Rectangle())
    }
}
