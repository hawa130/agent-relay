import SwiftUI

struct ProfileCurrentStatusSection: View {
    let events: [FailureEvent]

    var body: some View {
        SectionSurfaceCard("Status") {
            ForEach(events) { event in
                HStack(alignment: .top, spacing: 8) {
                    NativeStatusSymbol(
                        systemName: "exclamationmark.triangle.fill",
                        color: NativePreferencesTheme.Colors.statusIcon(.warning),
                        accessibilityLabel: event.reason.displayName,
                        font: .system(size: 11, weight: .semibold))
                        .padding(.top, 2)

                    VStack(alignment: .leading, spacing: 3) {
                        Text(event.reason.displayName)
                            .font(NativePreferencesTheme.Typography.detail.weight(.semibold))

                        Text(event.message)
                            .font(NativePreferencesTheme.Typography.detail)
                            .foregroundStyle(.secondary)

                        NativeMetaText(text: event.createdAt.formatted(date: .abbreviated, time: .standard))
                    }
                }
            }
        }
    }
}
