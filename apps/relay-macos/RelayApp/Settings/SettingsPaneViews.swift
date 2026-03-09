import LaunchAtLogin
import SwiftUI

public struct SettingsPaneView: View {
    @ObservedObject var model: SettingsPaneModel

    public init(model: SettingsPaneModel) {
        self.model = model
    }

    public var body: some View {
        HStack(spacing: 0) {
            sidebar
            Divider()
            detail
        }
        .background(NativePreferencesTheme.Colors.paneBackground)
    }

    private var sidebar: some View {
        VStack(alignment: .leading, spacing: 18) {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    VStack(alignment: .leading, spacing: 6) {
                        Button {
                            model.selectItem(.general)
                        } label: {
                            SettingsSidebarRow(
                                title: "General",
                                isSelected: model.selectedItem == .general
                            ) {
                                SettingsSidebarIconTile(isSelected: model.selectedItem == .general) {
                                    Image(systemName: "gearshape")
                                        .font(.system(size: 14, weight: .medium))
                                        .foregroundStyle(
                                            model.selectedItem == .general ? .white : .secondary
                                        )
                                }
                            }
                        }
                        .buttonStyle(.plain)
                    }

                    if !model.agents.isEmpty {
                        VStack(alignment: .leading, spacing: 6) {
                            Text("Agents")
                                .font(NativePreferencesTheme.Typography.sectionLabel)
                                .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
                                .textCase(.uppercase)
                                .padding(.horizontal, 4)

                            ForEach(model.agents) { descriptor in
                                Button {
                                    model.selectItem(.agent(descriptor.agent))
                                } label: {
                                    SettingsSidebarRow(
                                        title: descriptor.title,
                                        isSelected: model.selectedItem == .agent(descriptor.agent)
                                    ) {
                                        SettingsSidebarIconTile(
                                            isSelected: model.selectedItem == .agent(descriptor.agent)
                                        ) {
                                            AgentBrandIcon(
                                                descriptor: descriptor,
                                                size: 16,
                                                tint: model.selectedItem == .agent(descriptor.agent)
                                                    ? .white
                                                    : .secondary
                                            )
                                        }
                                    }
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .topLeading)
            }
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 16)
        .frame(
            minWidth: NativePreferencesTheme.Metrics.sidebarWidth,
            idealWidth: NativePreferencesTheme.Metrics.sidebarWidth,
            maxWidth: NativePreferencesTheme.Metrics.sidebarWidth + 20,
            maxHeight: .infinity,
            alignment: .topLeading
        )
        .background(NativePreferencesTheme.Colors.paneBackground)
    }

    private var detail: some View {
        Group {
            switch model.selectedItem {
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
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(NativePreferencesTheme.Colors.paneBackground)
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
        .scrollContentBackground(.hidden)
        .background(NativePreferencesTheme.Colors.paneBackground)
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
        .scrollContentBackground(.hidden)
        .background(NativePreferencesTheme.Colors.paneBackground)
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
        HStack(alignment: .top, spacing: 14) {
            accessory

            VStack(alignment: .leading, spacing: 4) {
                Text(title)
                    .font(.system(size: 16, weight: .medium, design: .rounded))

                Text(subtitle)
                    .font(NativePreferencesTheme.Typography.body)
                    .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
            }

            Spacer(minLength: 16)
        }
    }
}

private struct SettingsSidebarRow<Leading: View>: View {
    let title: String
    let isSelected: Bool
    let leading: Leading

    init(
        title: String,
        isSelected: Bool,
        @ViewBuilder leading: () -> Leading
    ) {
        self.title = title
        self.isSelected = isSelected
        self.leading = leading()
    }

    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            leading
                .frame(width: 28, height: 28)

            Text(title)
                .font(.system(size: 13, weight: .regular, design: .rounded))

            Spacer(minLength: 10)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 9)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(rowBackground, in: RoundedRectangle(cornerRadius: 9, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 9, style: .continuous)
                .strokeBorder(rowBorder, lineWidth: isSelected ? 1 : 0.5)
        )
        .contentShape(RoundedRectangle(cornerRadius: 9, style: .continuous))
    }

    private var rowBackground: Color {
        if isSelected {
            return Color.accentColor.opacity(0.14)
        }
        return NativePreferencesTheme.Colors.groupedBackground.opacity(0.55)
    }

    private var rowBorder: Color {
        if isSelected {
            return Color.accentColor.opacity(0.28)
        }
        return NativePreferencesTheme.Colors.sectionBorder.opacity(0.55)
    }
}

private struct SettingsSidebarIconTile<Content: View>: View {
    let isSelected: Bool
    let content: Content

    init(isSelected: Bool = false, @ViewBuilder content: () -> Content) {
        self.isSelected = isSelected
        self.content = content()
    }

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .fill(isSelected ? Color.accentColor : Color.secondary.opacity(0.12))
                .frame(width: 28, height: 28)

            content
        }
        .frame(width: 28, height: 28)
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
            .padding(.vertical, 6)
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
        HStack(alignment: .firstTextBaseline, spacing: 12) {
            Text(title)
                .font(NativePreferencesTheme.Typography.detail.weight(.medium))
                .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
                .frame(width: NativePreferencesTheme.Metrics.detailLabelWidth, alignment: .leading)

            Text(value)
                .font(NativePreferencesTheme.Typography.body)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

struct NativeStepperRow: View {
    let title: String
    let valueText: String
    @Binding var value: Int
    let range: ClosedRange<Int>
    var step: Int = 1

    var body: some View {
        HStack(spacing: 12) {
            Text(title)
            Spacer()
            Text(valueText)
                .monospacedDigit()
                .foregroundStyle(.primary)
            Stepper("", value: $value, in: range, step: step)
                .labelsHidden()
        }
    }
}
