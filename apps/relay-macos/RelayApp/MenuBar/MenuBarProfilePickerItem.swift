import SwiftUI

struct MenuBarProfilePickerItem: View {
    let profileName: String
    let statusText: String
    let sessionText: String?
    let sessionResetText: String?
    let weeklyText: String?
    let weeklyResetText: String?
    let footerText: String?
    let symbolName: String
    let isDimmed: Bool
    @Environment(\.menuItemHighlighted) private var isHighlighted

    var body: some View {
        HStack(alignment: .top, spacing: 8) {
            Image(systemName: symbolName)
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(MenuBarHighlightStyle.primary(isHighlighted))
                .frame(width: 13, height: 13)
                .padding(.top, 2)

            VStack(alignment: .leading, spacing: 3) {
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    Text(profileName)
                        .font(.system(size: 12.5, weight: .semibold))
                        .lineLimit(1)
                        .foregroundStyle(MenuBarHighlightStyle.primary(isHighlighted))

                    Spacer(minLength: 8)

                    Text(statusText)
                        .font(.system(size: 10, weight: .medium))
                        .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                        .lineLimit(1)
                }

                if let sessionText {
                    usageLine(left: sessionText, right: sessionResetText)
                }

                if let weeklyText {
                    usageLine(left: weeklyText, right: weeklyResetText)
                }

                if let footerText {
                    Text(footerText)
                        .font(.system(size: 10))
                        .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                        .lineLimit(1)
                }
            }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
        .opacity(isDimmed && !isHighlighted ? 0.72 : 1)
    }

    @ViewBuilder
    private func usageLine(left: String, right: String?) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text(left)
                .font(.system(size: 10, weight: .medium))
                .monospacedDigit()
                .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                .lineLimit(1)

            Spacer(minLength: 8)

            if let right {
                Text(right)
                    .font(.system(size: 10))
                    .monospacedDigit()
                    .foregroundStyle(MenuBarHighlightStyle.secondary(isHighlighted))
                    .lineLimit(1)
            }
        }
    }
}
