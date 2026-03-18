import SwiftUI

struct MenuBarPlanBadge: View {
    let title: String
    @Environment(\.menuItemHighlighted) private var isHighlighted

    var body: some View {
        Text(title)
            .font(.system(size: 9, weight: .medium))
            .foregroundStyle(
                isHighlighted
                    ? MenuBarHighlightStyle.selectionText
                    : Color(nsColor: .secondaryLabelColor))
            .padding(.horizontal, 4)
            .padding(.vertical, 1)
            .background(
                Capsule().fill(
                    isHighlighted
                        ? Color.white.opacity(0.2)
                        : Color(nsColor: .secondaryLabelColor).opacity(0.12)))
    }
}
