import SwiftUI

enum MenuBarUsageProgressLayout {
    static func fillWidth(percent: Double, totalWidth: CGFloat) -> CGFloat {
        let clampedPercent = min(100, max(0, percent))
        guard clampedPercent > 0 else {
            return 0
        }

        return max(totalWidth * clampedPercent / 100, 6)
    }
}

struct MenuBarUsageProgressBar: View {
    let percent: Double
    let tint: Color
    let accessibilityLabel: String
    @Environment(\.menuItemHighlighted) private var isHighlighted

    private var clampedPercent: Double {
        min(100, max(0, percent))
    }

    var body: some View {
        GeometryReader { proxy in
            Capsule()
                .fill(MenuBarHighlightStyle.progressTrack(isHighlighted))
                .overlay(alignment: .leading) {
                    Capsule()
                        .fill(MenuBarHighlightStyle.progressTint(isHighlighted, fallback: tint))
                        .frame(width: fillWidth(for: proxy.size.width))
                }
        }
        .frame(height: 4)
        .transaction { transaction in
            transaction.animation = nil
        }
        .accessibilityLabel(accessibilityLabel)
        .accessibilityValue("\(Int(clampedPercent)) percent")
    }

    private func fillWidth(for totalWidth: CGFloat) -> CGFloat {
        MenuBarUsageProgressLayout.fillWidth(percent: clampedPercent, totalWidth: totalWidth)
    }
}
