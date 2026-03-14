import SwiftUI

struct NativeBadgeSurface<Content: View>: View {
    let kind: NativePreferencesTheme.Badge.Kind
    @ViewBuilder let content: Content

    var body: some View {
        content
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(NativePreferencesTheme.Badge.fill(kind), in: Capsule())
            .fixedSize()
    }
}
