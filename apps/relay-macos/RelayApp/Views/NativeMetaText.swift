import SwiftUI

struct NativeMetaText: View {
    let text: String
    var color: Color = NativePreferencesTheme.Colors.mutedText

    var body: some View {
        Text(text)
            .font(NativePreferencesTheme.Typography.meta)
            .foregroundStyle(color)
    }
}
