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
                        selectedProfileIsActive ? "Profile is active" : "Activate Profile",
                        systemImage: selectedProfileIsActive ? "checkmark.circle.fill" : "checkmark.circle")
                }
                .labelStyle(.iconOnly)
                .toggleStyle(.button)
                .disabled(isActiveToggleDisabled)
                .help(selectedProfileIsActive ? "Profile is active" : "Activate Profile")
                .accessibilityLabel(selectedProfileIsActive ? "Profile is active" : "Activate Profile")
            }

            ToolbarItemGroup(placement: .confirmationAction) {
                Button {
                    onEditProfile()
                } label: {
                    Label("Edit Profile", systemImage: "square.and.pencil")
                }
                .accessibilityLabel("Edit Profile")
                .help("Edit Profile")
                .disabled(selectedProfile == nil || isMutatingProfiles)

                Button(role: .destructive) {
                    onDeleteProfile()
                } label: {
                    Label("Delete Profile", systemImage: "trash")
                }
                .accessibilityLabel("Delete Profile")
                .help("Delete Profile")
                .disabled(selectedProfile == nil || isMutatingProfiles)
            }
        }
    }
}
