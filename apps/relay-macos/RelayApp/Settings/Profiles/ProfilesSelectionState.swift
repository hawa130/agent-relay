import SwiftUI

struct ProfilesSelectionState {
    let selectedProfile: Profile?
    let selectedProfileUsageSnapshot: UsageSnapshot?
    let selectedProfileIsFetchingUsage: Bool
    let selectedProfileUsageNote: UsageCardNote?
    let selectedProfileIsActive: Bool
    let isActiveToggleDisabled: Bool
    let selectedCurrentFailureEvents: [FailureEvent]
}
