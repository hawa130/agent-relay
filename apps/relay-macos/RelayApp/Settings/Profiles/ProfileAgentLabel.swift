import SwiftUI

struct ProfileAgentLabel: View {
    let title: String
    let showsActiveBadge: Bool
    let accountState: ProfileAccountState

    var body: some View {
        HStack(spacing: 6) {
            Text(title)
                .font(NativePreferencesTheme.Typography.detail.weight(.semibold))
                .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
                .textCase(.uppercase)

            ProfileAccountStatusDot(accountState: accountState, diameter: 8)
        }
        .overlay(alignment: .topTrailing) {
            if showsActiveBadge {
                ProfileStateBadge(title: "Active", kind: .info)
                    .offset(x: 52, y: -1)
                    .allowsHitTesting(false)
            }
        }
    }
}
