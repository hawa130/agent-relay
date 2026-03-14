import SwiftUI

struct SectionSurfaceCard<HeaderAccessory: View, Content: View>: View {
    let title: String?
    let headerAccessory: HeaderAccessory
    let content: Content

    init(
        _ title: String? = nil,
        @ViewBuilder headerAccessory: () -> HeaderAccessory,
        @ViewBuilder content: () -> Content)
    {
        self.title = title
        self.headerAccessory = headerAccessory()
        self.content = content()
    }

    init(_ title: String? = nil, @ViewBuilder content: () -> Content) where HeaderAccessory == EmptyView {
        self.init(title, headerAccessory: { EmptyView() }, content: content)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: NativePreferencesTheme.Metrics.sectionContentSpacing) {
                if title != nil || HeaderAccessory.self != EmptyView.self {
                    HStack(alignment: .center, spacing: 8) {
                        if let title {
                            Text(title)
                                .font(NativePreferencesTheme.Typography.sectionLabel)
                                .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
                        }

                        Spacer(minLength: 0)
                        headerAccessory
                    }
                }
                content
            }
            .font(NativePreferencesTheme.Typography.body)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.vertical, 4)
        }
        .padding(.horizontal, 2)
    }
}
