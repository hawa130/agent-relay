import SwiftUI

struct ProfileListUsageLine: View {
    let title: String
    let value: String
    let resetDate: Date?

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            NativeMetaText(text: "\(title) \(value)")

            Spacer(minLength: 2)

            if let resetDate {
                ResetRelativeDateText(date: resetDate)
                    .font(NativePreferencesTheme.Typography.meta)
                    .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
