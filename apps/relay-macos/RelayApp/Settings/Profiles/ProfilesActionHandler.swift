import AppKit
import SwiftUI

@MainActor
struct ProfilesActionHandler {
    let model: ProfilesPaneModel
    let selectedProfile: () -> Profile?
    let setDeletingProfile: (Profile?) -> Void
    let selectProfile: (String?) -> Void

    func showAddProfile() {
        model.presentAddSheet()
    }

    func showEditProfile() {
        model.presentEditForSelectedProfile()
    }

    func showEditProfile(_ profile: Profile) {
        selectProfile(profile.id)
        model.presentEditForSelectedProfile()
    }

    func stageDeleteSelectedProfile() {
        setDeletingProfile(selectedProfile())
    }

    func stageDeleteProfile(_ profile: Profile) {
        selectProfile(profile.id)
        setDeletingProfile(profile)
    }

    func switchToProfile(_ profile: Profile) {
        selectProfile(profile.id)
        Task {
            await model.switchToProfile(profile.id)
        }
    }

    func setProfileEnabled(_ profile: Profile, enabled: Bool) {
        selectProfile(profile.id)
        Task {
            await model.setProfileEnabled(profile.id, enabled: enabled)
        }
    }

    func refreshUsageFromToolbar() {
        let scope = UsageToolbarRefreshScopeResolver.resolve(
            modifierFlags: NSApp.currentEvent?.modifierFlags ?? [])
        Task {
            switch scope {
            case .enabled:
                await model.refreshEnabledUsage()
            case .all:
                await model.refreshAllUsage()
            }
        }
    }

    func refreshSelectedProfileUsage() {
        guard let profileId = selectedProfile()?.id else {
            return
        }

        Task {
            await model.refreshUsage(profileId: profileId)
        }
    }
}
