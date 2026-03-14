import SwiftUI

struct ProfileListUsageLine: View {
    let title: String
    let value: String
    let resetDate: Date?

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text("\(title) \(value)")
                .font(.system(size: 10))
                .foregroundStyle(NativePreferencesTheme.Colors.mutedText)

            Spacer(minLength: 2)

            if let resetDate {
                ResetRelativeDateText(date: resetDate)
                    .font(.system(size: 10))
                    .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
