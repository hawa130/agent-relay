import LaunchAtLogin
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
                            AgentIcon(agent: descriptor.agent, size: 16, tint: .secondary)
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

private struct GeneralSettingsDetailView: View {
    @ObservedObject var model: SettingsPaneModel

    private var refreshIntervalOptions: [Int] {
        Array(Set([0, 15, 30, 60, 120, 180, 300, 600, 900, model.refreshIntervalSeconds])).sorted()
    }

    private var networkQueryConcurrencyOptions: [Int] {
        Array(Set([1, 2, 4, 6, 8, 10, 12, 16, 24, 32, model.networkQueryConcurrency])).sorted()
    }

    var body: some View {
        Form {
            Section("Behavior") {
                Toggle(
                    "Enable automatic failover",
                    isOn: Binding(
                        get: { model.autoSwitchEnabled },
                        set: { enabled in
                            Task {
                                await model.setAutoSwitch(enabled: enabled)
                            }
                        }))

                LaunchAtLogin.Toggle("Launch at login")

                Picker(
                    "Background refresh",
                    selection: Binding(
                        get: { model.refreshIntervalSeconds },
                        set: { seconds in
                            Task {
                                await model.setRefreshInterval(seconds: seconds)
                            }
                        })) {
                    ForEach(refreshIntervalOptions, id: \.self) { seconds in
                        Text(refreshIntervalLabel(for: seconds)).tag(seconds)
                    }
                }

                Picker(
                    "Network query concurrency",
                    selection: Binding(
                        get: { model.networkQueryConcurrency },
                        set: { value in
                            Task {
                                await model.setNetworkQueryConcurrency(value)
                            }
                        })) {
                    ForEach(networkQueryConcurrencyOptions, id: \.self) { value in
                        Text("\(value)").tag(value)
                    }
                }
            }

            Section("Engine") {
                NativeDetailRow(title: "Connection", value: engineStateLabel)

                Button("Restart AgentRelay Engine") {
                    Task {
                        await model.restartEngine()
                    }
                }
                .disabled(model.engineConnectionState == .starting)
            }

            Section("Application") {
                NativeDetailRow(title: "Profiles", value: "\(model.profilesCount)")
            }

            Section("About") {
                NativeDetailRow(title: "Version", value: appVersion)
            }
        }
        .formStyle(.grouped)
    }

    private func refreshIntervalLabel(for seconds: Int) -> String {
        if seconds == 0 {
            return "Off"
        }

        if seconds < 60 {
            return "\(seconds) sec"
        }

        let minutes = seconds / 60
        return minutes == 1 ? "1 min" : "\(minutes) min"
    }

    private var appVersion: String {
        let version = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String
        let build = Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String

        switch (version, build) {
        case let (version?, build?) where version != build:
            return "\(version) (\(build))"
        case let (version?, _):
            return version
        case let (_, build?):
            return build
        default:
            return "Development"
        }
    }

    private var engineStateLabel: String {
        switch model.engineConnectionState {
        case .starting:
            "Starting"
        case .ready:
            "Connected"
        case .degraded:
            "Degraded"
        }
    }
}

private struct AgentSettingsDetailView: View {
    let descriptor: AgentSettingsDescriptor
    @ObservedObject var model: SettingsPaneModel

    var body: some View {
        Form {
            switch descriptor.agent {
            case .codex:
                Section("Usage") {
                    Picker(
                        "Usage source",
                        selection: Binding(
                            get: { model.codexSettings?.usageSourceMode ?? .auto },
                            set: { mode in
                                Task {
                                    await model.setUsageSourceMode(mode)
                                }
                            })) {
                        ForEach(UsageSourceMode.allCases, id: \.self) { mode in
                            Text(mode.displayName).tag(mode)
                        }
                    }
                    .pickerStyle(.segmented)

                    Text((model.codexSettings?.usageSourceMode ?? .auto).helpText)
                        .font(NativePreferencesTheme.Typography.detail)
                        .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
        .formStyle(.grouped)
    }
}

struct SectionSurfaceCard<Content: View>: View {
    let title: String?
    let headerAccessory: AnyView?
    let content: Content

    init(_ title: String? = nil, headerAccessory: AnyView? = nil, @ViewBuilder content: () -> Content) {
        self.title = title
        self.headerAccessory = headerAccessory
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: NativePreferencesTheme.Metrics.sectionContentSpacing) {
                if title != nil || headerAccessory != nil {
                    HStack(alignment: .center, spacing: 8) {
                        if let title {
                            Text(title)
                                .font(NativePreferencesTheme.Typography.sectionLabel)
                                .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
                        }

                        Spacer(minLength: 0)

                        if let headerAccessory {
                            headerAccessory
                        }
                    }
                }
                content
            }
            .font(NativePreferencesTheme.Typography.body)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.vertical, 4)
        }
        .padding(.horizontal, 2)
    }
}

struct NativePaneScrollView<Content: View>: View {
    @ViewBuilder let content: Content

    var body: some View {
        ScrollView {
            content
                .frame(maxWidth: .infinity, alignment: .topLeading)
                .padding(.horizontal, NativePreferencesTheme.Metrics.paneHorizontalPadding)
                .padding(.vertical, NativePreferencesTheme.Metrics.paneVerticalPadding)
        }
        .background(NativePreferencesTheme.Colors.paneBackground)
    }
}

struct NativeDetailRow: View {
    let title: String
    let value: String

    var body: some View {
        LabeledContent(title, value: value)
    }
}

struct NativeStepperRow: View {
    let title: String
    let valueText: String
    @Binding var value: Int
    let range: ClosedRange<Int>
    var step: Int = 1

    var body: some View {
        LabeledContent {
            HStack(spacing: 10) {
                Text(valueText)
                    .monospacedDigit()
                Stepper("", value: $value, in: range, step: step)
                    .labelsHidden()
            }
        } label: {
            Text(title)
        }
    }
}
