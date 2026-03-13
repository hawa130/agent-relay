import SwiftUI

struct MenuBarUsageProgressBar: View {
    let percent: Double
    let tint: Color
    let accessibilityLabel: String
    @Environment(\.menuItemHighlighted) private var isHighlighted

    private var clampedPercent: Double {
        min(100, max(0, percent))
    }

    var body: some View {
        Capsule()
            .fill(MenuBarHighlightStyle.progressTrack(isHighlighted))
            .frame(height: 4)
            .overlay(alignment: .leading) {
                GeometryReader { proxy in
                    Capsule()
                        .fill(MenuBarHighlightStyle.progressTint(isHighlighted, fallback: tint))
                        .frame(width: fillWidth(for: proxy.size.width))
                }
            }
            .transaction { transaction in
                transaction.animation = nil
            }
            .accessibilityLabel(accessibilityLabel)
            .accessibilityValue("\(Int(clampedPercent)) percent")
    }

    private func fillWidth(for totalWidth: CGFloat) -> CGFloat {
        guard clampedPercent > 0 else {
            return 0
        }

        return max(totalWidth * clampedPercent / 100, 6)
    }
}
