import SwiftUI

struct ProfileInfoBadge: View {
    let title: String
    let value: String

    var body: some View {
        NativeBadgeSurface(kind: .neutral) {
            HStack(spacing: 0) {
                Text(title)
                    .font(NativePreferencesTheme.Typography.badge)
                    .foregroundStyle(NativePreferencesTheme.Badge.text(.neutral))

                Rectangle()
                    .fill(NativePreferencesTheme.Badge.text(.neutral).opacity(0.22))
                    .frame(width: 1)
                    .padding(.vertical, 2)
                    .padding(.horizontal, 5)

                Text(value)
                    .font(NativePreferencesTheme.Typography.badgeValue)
                    .foregroundStyle(NativePreferencesTheme.Badge.text(.neutral))
            }
        }
    }
}
