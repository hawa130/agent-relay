import SwiftUI

public struct SettingsPaneView: View {
    @ObservedObject var model: SettingsPaneModel

    public init(model: SettingsPaneModel) {
        self.model = model
    }

    public var body: some View {
        NavigationSplitView {
            sidebar
        } detail: {
            detail
        }
        .navigationSplitViewStyle(.balanced)
        .navigationTitle(titleText)
    }

    private var sidebar: some View {
        List(selection: selectedItemBinding) {
            Label("General", systemImage: "gearshape")
                .tag(SettingsSidebarSelection.general)

            if !model.agents.isEmpty {
                Section("Agents") {
                    ForEach(model.agents) { descriptor in
                        Label {
                            Text(descriptor.title)
                        } icon: {
                            AgentIcon(agent: descriptor.agent, size: 16)
                                .frame(width: 18, height: 18)
                        }
                        .tag(SettingsSidebarSelection.agent(descriptor.agent))
                    }
                }
            }
        }
        .listStyle(.sidebar)
        .frame(width: 200)
        .toolbar(removing: .sidebarToggle)
    }

    private var detail: some View {
        Group {
            detailView(for: model.selectedItem)
        }
        .navigationSplitViewColumnWidth(500)
    }

    @ViewBuilder
    private func detailView(for selection: SettingsSidebarSelection) -> some View {
        switch selection {
        case .general:
            GeneralSettingsDetailView(model: model)
        case let .agent(agent):
            if let descriptor = AgentSettingsCatalog.descriptor(for: agent) {
                AgentSettingsDetailView(descriptor: descriptor, model: model)
            } else {
                ContentUnavailableView(
                    "Settings Unavailable",
                    systemImage: "slider.horizontal.3",
                    description: Text("This agent does not expose configurable settings yet."))
            }
        }
    }

    private var selectedItemBinding: Binding<SettingsSidebarSelection?> {
        Binding(
            get: { model.selectedItem },
            set: { selection in
                if let selection {
                    model.selectItem(selection)
                }
            })
    }

    private var titleText: String {
        switch model.selectedItem {
        case .general:
            "General"
        case let .agent(agent):
            AgentSettingsCatalog.descriptor(for: agent)?.title ?? "Settings"
        }
    }
}
