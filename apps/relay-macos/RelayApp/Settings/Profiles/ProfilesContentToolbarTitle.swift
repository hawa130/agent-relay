import SwiftUI

struct ProfilesContentToolbarTitle: View {
    let title: String
    let profileCountSummary: String

    var body: some View {
        VStack(alignment: .leading, spacing: 1) {
            Text(title)
                .font(.headline)

            Text(profileCountSummary)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
