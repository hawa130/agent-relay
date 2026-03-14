struct ProfileRowContextMenuModel {
    let profile: Profile
    let isActive: Bool
    let isMutatingProfiles: Bool
    let isSwitching: Bool

    var toggleEnabledTitle: String {
        profile.enabled ? "Disable" : "Enable"
    }

    var canMakeCurrent: Bool {
        !isMutatingProfiles && !isSwitching && profile.enabled && !isActive
    }

    var canEdit: Bool {
        !isMutatingProfiles
    }

    var canToggleEnabled: Bool {
        !isMutatingProfiles
    }

    var canDelete: Bool {
        !isMutatingProfiles
    }
}
