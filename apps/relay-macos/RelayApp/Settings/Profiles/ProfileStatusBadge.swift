import SwiftUI

struct ProfileStatusBadge: View {
    let title: String
    let dotColor: Color

    var body: some View {
        NativeBadgeSurface(kind: .neutral) {
            HStack(spacing: 5) {
                Circle()
                    .fill(dotColor)
                    .frame(width: 5, height: 5)

                Text(title)
                    .font(NativePreferencesTheme.Typography.badge)
                    .foregroundStyle(NativePreferencesTheme.Badge.text(.neutral))
            }
        }
    }
}
