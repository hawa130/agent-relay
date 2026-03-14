import SwiftUI

struct ProfileStatusBadge: View {
    let title: String
    let dotColor: Color

    var body: some View {
        HStack(spacing: 5) {
            Circle()
                .fill(dotColor)
                .frame(width: 5, height: 5)

            Text(title)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(NativePreferencesTheme.Badge.text(.neutral))
        }
        .padding(.horizontal, 6)
        .padding(.vertical, 2)
        .background(NativePreferencesTheme.Badge.fill(.neutral), in: Capsule())
        .fixedSize()
    }
}
