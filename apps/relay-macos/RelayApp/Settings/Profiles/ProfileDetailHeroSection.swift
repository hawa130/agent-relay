import SwiftUI

struct ProfileDetailHeroSection: View {
    let profile: Profile
    let planHint: String?
    let isActive: Bool
    let isMutatingProfiles: Bool
    let currentFailureEvents: [FailureEvent]
    let isEnabled: Binding<Bool>

    var body: some View {
        SectionSurfaceCard(nil) {
            VStack(alignment: .leading, spacing: 12) {
                HStack(alignment: .top, spacing: 12) {
                    ProfileHeroAgentIcon(agent: profile.agent)

                    VStack(alignment: .leading, spacing: 0) {
                        ProfileAgentLabel(
                            title: profile.agent.rawValue,
                            showsActiveBadge: isActive,
                            accountState: profile.accountState)

                        Text(profile.nickname)
                            .font(.system(size: 19, weight: .semibold, design: .rounded))

                        HStack(spacing: 6) {
                            ProfileStatusBadge(
                                title: profile.enabled ? "Enabled" : "Disabled",
                                dotColor: profile.enabled
                                    ? NativePreferencesTheme.Colors.statusIcon(.success)
                                    : NativePreferencesTheme.Colors.disabledIndicator)
                            if let planHint, !planHint.isEmpty {
                                ProfileInfoBadge(title: "Plan", value: planHint)
                            }
                            ProfileInfoBadge(title: "Priority", value: "\(profile.priority)")
                        }
                        .padding(.top, 4)
                    }

                    Spacer(minLength: 0)

                    VStack(alignment: .trailing, spacing: 8) {
                        Toggle("Enabled", isOn: isEnabled)
                            .toggleStyle(.switch)
                            .labelsHidden()
                            .disabled(isMutatingProfiles)
                    }
                }

                if !currentFailureEvents.isEmpty {
                    ProfileCurrentStatusSection(events: currentFailureEvents)
                }
            }
        }
    }
}
