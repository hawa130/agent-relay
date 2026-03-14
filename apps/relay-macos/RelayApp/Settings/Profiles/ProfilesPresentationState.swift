import SwiftUI

struct ProfilesPresentationState {
    let selectedProfile: Binding<String?>
    let selectedFilter: Binding<ProfilesSidebarFilter?>
    let deletingProfileAlertIsPresented: Binding<Bool>
    let selectedProfileActive: Binding<Bool>
    let selectedProfileEnabled: Binding<Bool>
}
