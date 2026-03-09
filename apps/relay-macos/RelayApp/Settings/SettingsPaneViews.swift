import LaunchAtLogin
import SwiftUI

public struct GeneralSettingsPaneView: View {
    @ObservedObject var model: SettingsSessionModel

    public init(model: SettingsSessionModel) {
        self.model = model
    }

    public var body: some View {
        NativePaneScrollView {
            VStack(alignment: .leading, spacing: NativePreferencesTheme.Metrics.sectionSpacing) {
                SettingsSurfaceCard("Behavior") {
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

                SettingsSurfaceCard("Usage") {
                    Picker(
                        "Usage source",
                        selection: Binding(
                            get: { model.status?.settings.usageSourceMode ?? .auto },
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

                    Stepper(
                        value: Binding(
                            get: { model.status?.settings.menuOpenRefreshStaleAfterSeconds ?? 10 },
                            set: { value in
                                Task {
                                    await model.setMenuOpenRefreshStaleAfterSeconds(value)
                                }
                            }
                        ),
                        in: 1...60
                    ) {
                        Text("Menu-open debounce: \(model.status?.settings.menuOpenRefreshStaleAfterSeconds ?? 10)s")
                    }

                    Toggle(
                        "Background usage refresh",
                        isOn: Binding(
                            get: { model.status?.settings.usageBackgroundRefreshEnabled ?? true },
                            set: { enabled in
                                Task {
                                    await model.setBackgroundRefreshEnabled(enabled)
                                }
                            }
                        )
                    )

                    Stepper(
                        value: Binding(
                            get: { model.status?.settings.usageBackgroundRefreshIntervalSeconds ?? 120 },
                            set: { value in
                                Task {
                                    await model.setBackgroundRefreshIntervalSeconds(value)
                                }
                            }
                        ),
                        in: 30...3600,
                        step: 30
                    ) {
                        Text("Background interval: \(model.status?.settings.usageBackgroundRefreshIntervalSeconds ?? 120)s")
                    }
                }

                if let error = model.lastErrorMessage {
                    SettingsSurfaceCard("Last Error") {
                        Text(error)
                            .foregroundStyle(.red)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                }
            }
        }
        .onAppear {
            SettingsPaneID.persistedSelection = .general
        }
    }

}

public struct ActivitySettingsPaneView: View {
    @ObservedObject var model: ActivityPaneModel

    public init(model: ActivityPaneModel) {
        self.model = model
    }

    public var body: some View {
        ActivityView(model: model)
            .onAppear {
                SettingsPaneID.persistedSelection = .activity
            }
    }
}

public struct AboutSettingsPaneView: View {
    @ObservedObject var model: SettingsSessionModel

    public init(model: SettingsSessionModel) {
        self.model = model
    }

    public var body: some View {
        NativePaneScrollView {
            VStack(alignment: .leading, spacing: NativePreferencesTheme.Metrics.sectionSpacing) {
                SettingsSurfaceCard("Application") {
                    NativeDetailRow(title: "Version", value: appVersion)
                    NativeDetailRow(title: "Profiles", value: "\(model.profilesCount)")
                    NativeDetailRow(title: "Platform", value: model.doctor?.platform ?? "-")
                }
            }
        }
        .onAppear {
            SettingsPaneID.persistedSelection = .about
        }
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

struct SettingsSurfaceCard<Content: View>: View {
    let title: String
    let content: Content

    init(_ title: String, @ViewBuilder content: () -> Content) {
        self.title = title
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: NativePreferencesTheme.Metrics.sectionContentSpacing) {
                Text(title)
                    .font(NativePreferencesTheme.Typography.sectionLabel)
                    .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
                    .textCase(.uppercase)
                content
            }
            .font(NativePreferencesTheme.Typography.body)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.vertical, 6)

            Divider()
                .padding(.top, 14)
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
