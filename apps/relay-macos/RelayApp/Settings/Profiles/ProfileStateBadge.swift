import SwiftUI

struct ProfileStateBadge: View {
    let title: String
    let kind: NativePreferencesTheme.Badge.Kind

    var body: some View {
        Text(title)
            .font(.system(size: 10, weight: .semibold))
            .foregroundStyle(NativePreferencesTheme.Badge.text(kind))
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(NativePreferencesTheme.Badge.fill(kind), in: Capsule())
            .fixedSize()
    }
}
