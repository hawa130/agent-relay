import SwiftUI

struct ProfilesDetailPane: View {
    let selectedProfile: Profile?
    let selectedProfileIsActive: Bool
    let isMutatingProfiles: Bool
    let selectedCurrentFailureEvents: [FailureEvent]
    let selectedProfileEnabled: Binding<Bool>
    let usageSnapshot: UsageSnapshot?
    let isFetchingUsage: Bool
    let usageNote: UsageCardNote?
    let selectedProfileActive: Binding<Bool>
    let isActiveToggleDisabled: Bool
    let onEditProfile: () -> Void
    let onDeleteProfile: () -> Void
    let onRefreshUsage: () -> Void

    var body: some View {
        Group {
            if let profile = selectedProfile {
                NativePaneScrollView {
                    VStack(alignment: .leading, spacing: NativePreferencesTheme.Metrics.sectionSpacing) {
                        ProfileDetailHeroSection(
                            profile: profile,
                            isActive: selectedProfileIsActive,
                            isMutatingProfiles: isMutatingProfiles,
                            currentFailureEvents: selectedCurrentFailureEvents,
                            isEnabled: selectedProfileEnabled)
                        ProfileDetailUsageCard(
                            usage: usageSnapshot,
                            isFetchingUsage: isFetchingUsage,
                            note: usageNote,
                            onRefresh: onRefreshUsage)
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            } else {
                ContentUnavailableView(
                    "No Profile Selected",
                    systemImage: "person.crop.square",
                    description: Text("Choose a profile on the left to inspect its details and actions."))
                    .frame(maxWidth: .infinity, minHeight: 420)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
                    .background(NativePreferencesTheme.Colors.paneBackground)
            }
        }
        .navigationSplitViewColumnWidth(min: 380, ideal: 460)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .toolbar {
            ToolbarItemGroup(placement: .primaryAction) {
                Toggle(isOn: selectedProfileActive) {
                    Label(
                        Self.activateProfileLabel(isActive: selectedProfileIsActive),
                        systemImage: Self.activateProfileSymbol(isActive: selectedProfileIsActive))
                }
                .toggleStyle(.button)
                .buttonStyle(.bordered)
                .disabled(isActiveToggleDisabled)
                .help(Self.activateProfileLabel(isActive: selectedProfileIsActive))
                .accessibilityLabel(Self.activateProfileLabel(isActive: selectedProfileIsActive))
            }

            ToolbarItemGroup(placement: .confirmationAction) {
                NativeToolbarSymbolButton(
                    "Edit Profile",
                    systemImage: "square.and.pencil",
                    isEnabled: selectedProfile != nil && !isMutatingProfiles,
                    action: onEditProfile)

                NativeToolbarSymbolButton(
                    "Delete Profile",
                    systemImage: "trash",
                    role: .destructive,
                    isEnabled: selectedProfile != nil && !isMutatingProfiles,
                    action: onDeleteProfile)
            }
        }
    }

    nonisolated static func activateProfileLabel(isActive: Bool) -> String {
        isActive ? "Profile is active" : "Activate Profile"
    }

    nonisolated static func activateProfileSymbol(isActive: Bool) -> String {
        isActive ? "checkmark.circle.fill" : "checkmark.circle"
    }
}
