import SwiftUI

struct ProfileInfoBadge: View {
    let title: String
    let value: String

    var body: some View {
        HStack(spacing: 0) {
            Text(title)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(NativePreferencesTheme.Badge.text(.neutral))
                .padding(.leading, 6)
                .padding(.trailing, 5)
                .padding(.vertical, 2)

            Rectangle()
                .fill(NativePreferencesTheme.Badge.text(.neutral).opacity(0.22))
                .frame(width: 1)
                .padding(.vertical, 2)

            Text(value)
                .font(.system(size: 10, weight: .regular))
                .foregroundStyle(NativePreferencesTheme.Badge.text(.neutral))
                .padding(.leading, 5)
                .padding(.trailing, 6)
                .padding(.vertical, 2)
        }
        .background(NativePreferencesTheme.Badge.fill(.neutral), in: Capsule())
        .fixedSize()
    }
}
