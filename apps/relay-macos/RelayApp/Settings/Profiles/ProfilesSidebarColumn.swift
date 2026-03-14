import SwiftUI

struct ProfilesSidebarColumn: View {
    let selectedFilter: Binding<ProfilesSidebarFilter?>
    let profileCount: (ProfilesSidebarFilter) -> Int

    var body: some View {
        List(ProfilesSidebarFilter.allCases, selection: selectedFilter) { item in
            ProfilesSidebarItemLabel(item: item)
                .badge(profileCount(item))
                .tag(item)
        }
        .listStyle(.sidebar)
        .navigationSplitViewColumnWidth(140)
    }
}
