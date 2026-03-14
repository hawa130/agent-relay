import SwiftUI

struct ProfileStateBadge: View {
    let title: String
    let kind: NativePreferencesTheme.Badge.Kind

    var body: some View {
        NativeBadgeSurface(kind: kind) {
            Text(title)
                .font(NativePreferencesTheme.Typography.badge)
                .foregroundStyle(NativePreferencesTheme.Badge.text(kind))
        }
    }
}
