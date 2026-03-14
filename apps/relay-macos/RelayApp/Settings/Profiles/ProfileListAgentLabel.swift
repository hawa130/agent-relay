import SwiftUI

struct ProfileListAgentLabel: View {
    let agent: AgentKind
    let accountState: ProfileAccountState

    var body: some View {
        HStack(spacing: 5) {
            if AgentSettingsCatalog.descriptor(for: agent) != nil {
                AgentIcon(agent: agent, size: 12, tint: .secondary)
                    .frame(width: 12, height: 12)
            } else {
                Image(systemName: "terminal")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(.secondary)
                    .frame(width: 12, height: 12)
            }

            Text(agent.rawValue.uppercased())
                .font(NativePreferencesTheme.Typography.detail.weight(.semibold))
                .foregroundStyle(NativePreferencesTheme.Colors.mutedText)

            ProfileAccountStatusDot(accountState: accountState, diameter: 7)
        }
    }
}
