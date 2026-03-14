import SwiftUI

struct ProfileHeroAgentIcon: View {
    let agent: AgentKind

    var body: some View {
        Group {
            if AgentSettingsCatalog.descriptor(for: agent) != nil {
                ZStack {
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .fill(NativePreferencesTheme.Colors.subtleFill)
                        .frame(width: 40, height: 40)

                    AgentIcon(agent: agent, size: 20)
                }
            } else {
                ZStack {
                    RoundedRectangle(cornerRadius: 10, style: .continuous)
                        .fill(NativePreferencesTheme.Colors.subtleFill)
                        .frame(width: 40, height: 40)

                    Image(systemName: "terminal")
                        .font(.system(size: 18, weight: .medium))
                        .foregroundStyle(.secondary)
                }
            }
        }
        .frame(width: 40, height: 40)
    }
}
