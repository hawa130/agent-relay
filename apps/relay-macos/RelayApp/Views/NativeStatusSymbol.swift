import SwiftUI

struct NativeStatusSymbol: View {
    let systemName: String
    let color: Color
    let accessibilityLabel: String
    var font: Font = NativePreferencesTheme.Typography.meta.weight(.semibold)

    var body: some View {
        Image(systemName: systemName)
            .font(font)
            .foregroundStyle(color)
            .accessibilityLabel(accessibilityLabel)
            .help(accessibilityLabel)
    }
}
