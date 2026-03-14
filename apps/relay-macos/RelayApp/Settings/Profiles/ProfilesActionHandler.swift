import AppKit
import SwiftUI

@MainActor
struct ProfilesActionHandler {
    let model: ProfilesPaneModel
    let selectedProfile: () -> Profile?
    let setDeletingProfile: (Profile?) -> Void

    func showAddProfile() {
        model.presentAddSheet()
    }

    func showEditProfile() {
        model.presentEditForSelectedProfile()
    }

    func stageDeleteSelectedProfile() {
        setDeletingProfile(selectedProfile())
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
