import SwiftUI

struct NativePaneScrollView<Content: View>: View {
    @ViewBuilder let content: Content

    var body: some View {
        ScrollView {
            content
                .frame(maxWidth: .infinity, alignment: .topLeading)
                .padding(.horizontal, NativePreferencesTheme.Metrics.paneHorizontalPadding)
                .padding(.vertical, NativePreferencesTheme.Metrics.paneVerticalPadding)
        }
        .background(NativePreferencesTheme.Colors.paneBackground)
    }
}
