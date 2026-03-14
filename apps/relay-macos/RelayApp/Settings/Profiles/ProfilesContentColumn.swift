import AppKit
import SwiftUI

struct ProfilesContentColumn: View {
    let filteredProfiles: [Profile]
    let selectedProfile: Binding<String?>
    let selectedFilterTitle: String
    let profileCountSummary: String
    let emptyStateDescription: String
    let activeProfileId: String?
    let isFetchingEnabledUsage: Bool
    let usageSnapshot: (String) -> UsageSnapshot?
    let isFetchingUsage: (String) -> Bool
    let usageRefreshError: (String) -> String?
    let onRefreshUsage: () -> Void
    let onAddProfile: () -> Void

    var body: some View {
        List(selection: selectedProfile) {
            ForEach(filteredProfiles) { profile in
                ProfileListRow(
                    profile: profile,
                    usage: usageSnapshot(profile.id),
                    isActive: activeProfileId == profile.id,
                    isFetchingUsage: isFetchingUsage(profile.id),
                    usageRefreshError: usageRefreshError(profile.id))
                    .tag(Optional(profile.id))
            }

            if filteredProfiles.isEmpty {
                ContentUnavailableView(
                    "No Profiles",
                    systemImage: "person.crop.square",
                    description: Text(emptyStateDescription))
                    .disabled(true)
            }
        }
        .listStyle(.inset)
        .navigationSplitViewColumnWidth(min: 260, ideal: 340, max: 400)
        .toolbar {
            ToolbarItemGroup(placement: .navigation) {
                ProfilesContentToolbarTitle(
                    title: selectedFilterTitle,
                    profileCountSummary: profileCountSummary)
            }

            ToolbarItemGroup(placement: .secondaryAction) {
                Spacer(minLength: 0)
            }

            ToolbarItemGroup(placement: .confirmationAction) {
                if isFetchingEnabledUsage {
                    ProgressView()
                        .controlSize(.small)
                        .frame(width: 28, height: 28)
                        .help("Refreshing usage")
                } else {
                    NativeToolbarSymbolButton(
                        "Refresh Usage",
                        systemImage: "arrow.clockwise",
                        helpText: "Refresh Usage For Enabled Profiles. Option-click to refresh all profiles.",
                        action: onRefreshUsage)
                        .labelStyle(.iconOnly)
                        .buttonStyle(.bordered)
                }

                NativeToolbarSymbolButton("Add Profile", systemImage: "plus", action: onAddProfile)
            }
        }
    }
}
