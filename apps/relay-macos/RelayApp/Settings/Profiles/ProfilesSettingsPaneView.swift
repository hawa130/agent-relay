import AppKit
import SwiftUI

public struct ProfilesSettingsPaneView: View {
    @ObservedObject var model: ProfilesPaneModel
    @State private var deletingProfile: Profile?

    public init(model: ProfilesPaneModel) {
        self.model = model
    }

    public var body: some View {
        NavigationSplitView {
            ProfilesSidebarColumn(
                selectedFilter: presentationState.selectedFilter,
                profileCount: profileCount(for:))
        } content: {
            ProfilesContentColumn(
                filteredProfiles: model.filteredProfiles,
                selectedProfile: presentationState.selectedProfile,
                selectedFilterTitle: model.selectedFilter.title,
                profileCountSummary: profileCountSummary,
                emptyStateDescription: emptyStateDescription,
                activeProfileId: model.activeProfileId,
                isFetchingEnabledUsage: model.isFetchingEnabledUsage,
                usageSnapshot: model.usageSnapshot(for:),
                isFetchingUsage: model.isFetchingUsage(profileId:),
                usageRefreshError: model.usageRefreshError(profileId:),
                onRefreshUsage: actionHandler.refreshUsageFromToolbar,
                onAddProfile: actionHandler.showAddProfile)
        } detail: {
            ProfilesDetailPane(
                selectedProfile: selectionState.selectedProfile,
                selectedProfileIsActive: selectionState.selectedProfileIsActive,
                isMutatingProfiles: model.isMutatingProfiles,
                selectedCurrentFailureEvents: selectionState.selectedCurrentFailureEvents,
                selectedProfileEnabled: presentationState.selectedProfileEnabled,
                usageSnapshot: selectionState.selectedProfileUsageSnapshot,
                isFetchingUsage: selectionState.selectedProfileIsFetchingUsage,
                usageNote: selectionState.selectedProfileUsageNote,
                selectedProfileActive: presentationState.selectedProfileActive,
                isActiveToggleDisabled: selectionState.isActiveToggleDisabled,
                onEditProfile: actionHandler.showEditProfile,
                onDeleteProfile: actionHandler.stageDeleteSelectedProfile,
                onRefreshUsage: actionHandler.refreshSelectedProfileUsage)
        }
        .navigationSplitViewStyle(.balanced)
        .sheet(
            isPresented: Binding(
                get: { model.isPresentingAddSheet },
                set: { isPresented in
                    if !isPresented {
                        model.dismissAddSheet()
                    }
                })) {
            AddProfileSheet(
                agents: model.agents,
                profileCountForAgent: { agent in
                    model.profileCount(for: agent)
                },
                isBusy: model.isMutatingProfiles,
                onImportProfile: { agent in
                    await model.importProfile(agent: agent, nickname: nil, priority: 100)
                },
                onStartLogin: { agent in
                    await model.addAccount(agent: agent)
                },
                onCancelLogin: {
                    await model.cancelAddAccount()
                })
        }
        .sheet(
            item: Binding(
                get: { model.editingProfile },
                set: { profile in
                    if profile == nil {
                        model.dismissEditSheet()
                    }
                })) { profile in
            ProfileEditorSheet(
                title: "Edit Profile",
                initialDraft: ProfileDraft(profile: profile),
                mode: .edit(profile))
            { draft in
                await model.editProfile(profileId: profile.id, draft: draft)
            }
        }
        .alert("Remove Profile?", isPresented: presentationState.deletingProfileAlertIsPresented, presenting: deletingProfile) { profile in
            Button("Remove", role: .destructive) {
                Task {
                    await model.removeProfile(profile.id)
                    deletingProfile = nil
                }
            }
            Button("Cancel", role: .cancel) {
                deletingProfile = nil
            }
        } message: { profile in
            Text("Remove profile \"\(profile.nickname)\" from AgentRelay?")
        }
    }

    private var selectionState: ProfilesSelectionState {
        let selectedProfile = model.selectedProfile
        let selectedProfileUsageSnapshot = selectedProfile.flatMap { profile in
            model.usageSnapshot(for: profile.id)
        }
        let selectedProfileIsFetchingUsage = selectedProfile.map { profile in
            model.isFetchingUsage(profileId: profile.id)
        } ?? false
        let selectedProfileUsageNote = selectedProfile.flatMap { profile in
            UsageCardNoteResolver.note(
                usage: model.usageSnapshot(for: profile.id),
                usageRefreshError: model.usageRefreshError(profileId: profile.id))
        }
        let selectedProfileIsActive = selectedProfile.map { profile in
            model.activeProfileId == profile.id
        } ?? false
        let isActiveToggleDisabled = selectedProfile.map { profile in
            model.isMutatingProfiles || model.isSwitching || !profile.enabled || model.activeProfileId == profile.id
        } ?? true
        let selectedCurrentFailureEvents = selectedProfile.map { profile in
            model.currentFailureEvents(for: profile.id)
        } ?? []

        return ProfilesSelectionState(
            selectedProfile: selectedProfile,
            selectedProfileUsageSnapshot: selectedProfileUsageSnapshot,
            selectedProfileIsFetchingUsage: selectedProfileIsFetchingUsage,
            selectedProfileUsageNote: selectedProfileUsageNote,
            selectedProfileIsActive: selectedProfileIsActive,
            isActiveToggleDisabled: isActiveToggleDisabled,
            selectedCurrentFailureEvents: selectedCurrentFailureEvents)
    }

    private var presentationState: ProfilesPresentationState {
        ProfilesPresentationState(
            selectedProfile: Binding(
                get: {
                    guard let selectedProfileId = model.selectedProfileId,
                          model.filteredProfiles.contains(where: { $0.id == selectedProfileId })
                    else {
                        return nil
                    }
                    return selectedProfileId
                },
                set: { profileId in
                    model.selectProfile(profileId)
                }),
            selectedFilter: Binding(
                get: { model.selectedFilter },
                set: { filter in
                    if let filter {
                        model.selectFilter(filter)
                    }
                }),
            deletingProfileAlertIsPresented: Binding(
                get: { deletingProfile != nil },
                set: { isPresented in
                    if !isPresented {
                        deletingProfile = nil
                    }
                }),
            selectedProfileActive: Binding(
                get: { selectionState.selectedProfileIsActive },
                set: { isActive in
                    guard isActive, let profile = selectionState.selectedProfile else {
                        return
                    }

                    Task {
                        await model.switchToProfile(profile.id)
                    }
                }),
            selectedProfileEnabled: Binding(
                get: { selectionState.selectedProfile?.enabled ?? false },
                set: { enabled in
                    guard let profile = selectionState.selectedProfile else {
                        return
                    }

                    Task {
                        await model.setProfileEnabled(profile.id, enabled: enabled)
                    }
                }))
    }

    private var profileCountSummary: String {
        let count = model.selectedFilterProfileCount
        return count == 1 ? "1 profile" : "\(count) profiles"
    }

    private var emptyStateDescription: String {
        model.selectedFilterEmptyStateDescription
    }

    private func profileCount(for item: ProfilesSidebarFilter) -> Int {
        model.profileCount(for: item)
    }

    private var actionHandler: ProfilesActionHandler {
        ProfilesActionHandler(
            model: model,
            selectedProfile: { selectionState.selectedProfile },
            setDeletingProfile: { deletingProfile = $0 })
    }
}
