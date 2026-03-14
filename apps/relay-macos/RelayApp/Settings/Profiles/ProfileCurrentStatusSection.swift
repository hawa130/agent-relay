import SwiftUI

struct ProfileCurrentStatusSection: View {
    let events: [FailureEvent]

    var body: some View {
        SectionSurfaceCard("Status") {
            ForEach(events) { event in
                HStack(alignment: .top, spacing: 8) {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(NativePreferencesTheme.Colors.statusIcon(.warning))
                        .padding(.top, 2)

                    VStack(alignment: .leading, spacing: 3) {
                        Text(event.reason.displayName)
                            .font(NativePreferencesTheme.Typography.detail.weight(.semibold))

                        Text(event.message)
                            .font(NativePreferencesTheme.Typography.detail)
                            .foregroundStyle(.secondary)

                        Text(event.createdAt.formatted(date: .abbreviated, time: .standard))
                            .font(.system(size: 10))
                            .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
                    }
                }
            }
        }
    }
}
