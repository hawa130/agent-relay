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
                            AgentBrandIcon(descriptor: descriptor, size: 16, tint: .secondary)
                                .frame(width: 18, height: 18)
                        }
                        .tag(SettingsSidebarSelection.agent(descriptor.agent))
                    }
                }
            }
        }
        .listStyle(.sidebar)
        .frame(minWidth: 220, idealWidth: 260, maxWidth: 300, maxHeight: .infinity)
        .toolbar(removing: .sidebarToggle)
    }

    private var detail: some View {
        Group {
            detailView(for: model.selectedItem)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
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
                    description: Text("This agent does not expose configurable settings yet.")
                )
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
            }
        )
    }

    private var titleText: String {
        switch model.selectedItem {
        case .general:
            return "General"
        case let .agent(agent):
            return AgentSettingsCatalog.descriptor(for: agent)?.title ?? "Settings"
        }
    }
}

private struct GeneralSettingsDetailView: View {
    @ObservedObject var model: SettingsPaneModel

    var body: some View {
        Form {
            Section {
                SettingsDetailHeader(
                    title: "General",
                    subtitle: "System-wide preferences that affect Relay regardless of agent."
                ) {
                    SettingsDetailIconTile {
                        Image(systemName: "gearshape")
                            .font(.system(size: 18, weight: .medium))
                            .foregroundStyle(.secondary)
                    }
                }
            }

            Section("Behavior") {
                Toggle(
                    "Enable automatic failover",
                    isOn: Binding(
                        get: { model.autoSwitchEnabled },
                        set: { enabled in
                            Task {
                                await model.setAutoSwitch(enabled: enabled)
                            }
                        }
                    )
                )

                LaunchAtLogin.Toggle("Launch at login")

                Stepper(
                    value: Binding(
                        get: { model.refreshIntervalSeconds },
                        set: { seconds in
                            Task {
                                await model.setRefreshInterval(seconds: seconds)
                            }
                        }
                    ),
                    in: 15...900,
                    step: 15
                ) {
                    NativeDetailRow(
                        title: "Background refresh",
                        value: "\(model.refreshIntervalSeconds) sec"
                    )
                }
            }

            Section("Engine") {
                NativeDetailRow(title: "Connection", value: engineStateLabel)

                Button("Restart Relay Engine") {
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

            if let error = model.lastErrorMessage {
                Section("Last Error") {
                    Text(error)
                        .foregroundStyle(.red)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
        .formStyle(.grouped)
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
            return "Starting"
        case .ready:
            return "Connected"
        case .degraded:
            return "Degraded"
        }
    }
}

private struct AgentSettingsDetailView: View {
    let descriptor: AgentSettingsDescriptor
    @ObservedObject var model: SettingsPaneModel

    var body: some View {
        Form {
            Section {
                SettingsDetailHeader(
                    title: descriptor.title,
                    subtitle: "Settings here apply to all \(descriptor.title) profiles."
                ) {
                    SettingsDetailIconTile {
                        AgentBrandIcon(descriptor: descriptor, size: 20, tint: .secondary)
                    }
                }
            }

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
                            }
                        )
                    ) {
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

            if let error = model.lastErrorMessage {
                Section("Last Error") {
                    Text(error)
                        .foregroundStyle(.red)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
        .formStyle(.grouped)
    }
}

private struct SettingsDetailHeader<Accessory: View>: View {
    let title: String
    let subtitle: String
    let accessory: Accessory

    init(
        title: String,
        subtitle: String,
        @ViewBuilder accessory: () -> Accessory
    ) {
        self.title = title
        self.subtitle = subtitle
        self.accessory = accessory()
    }

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            accessory

            VStack(alignment: .leading, spacing: 4) {
                Text(title)
                    .font(.system(size: 16, weight: .medium, design: .rounded))

                Text(subtitle)
                    .font(NativePreferencesTheme.Typography.body)
                    .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
            }

            Spacer(minLength: 12)
        }
    }
}

private struct SettingsDetailIconTile<Content: View>: View {
    let fill: Color
    let content: Content

    init(fill: Color = Color.secondary.opacity(0.12), @ViewBuilder content: () -> Content) {
        self.fill = fill
        self.content = content()
    }

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .fill(fill)
                .frame(width: 40, height: 40)

            content
        }
        .frame(width: 40, height: 40)
    }
}

struct SettingsSurfaceCard<Content: View>: View {
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
                                .textCase(.uppercase)
                        }

                        Spacer()

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
    let content: Content

    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

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
