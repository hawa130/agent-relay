import SwiftUI

struct ProfilesSidebarItemLabel: View {
    let item: ProfilesSidebarFilter

    var body: some View {
        switch item {
        case .all:
            Label(item.title, systemImage: "square.grid.2x2")
        case .codex:
            Label {
                Text(item.title)
            } icon: {
                AgentIcon(agent: .codex, size: 14)
                    .frame(width: 16, height: 16)
            }
        }
    }
}
