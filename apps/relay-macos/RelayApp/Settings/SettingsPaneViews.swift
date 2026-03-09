import LaunchAtLogin
import SwiftUI

public struct GeneralSettingsPaneView: View {
    @ObservedObject var model: RelayAppModel

    public init(model: RelayAppModel) {
        self.model = model
    }

    public var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                paneHeader(
                    title: "General",
                    subtitle: "Relay environment, startup behavior, and usage refresh controls."
                )

                SettingsSurfaceCard("Relay") {
                    settingsRow("CLI", value: ProcessInfo.processInfo.environment["RELAY_CLI_PATH"] ?? "Bundled relay")
                    settingsRow("Relay Home", value: model.status?.relayHome ?? "-")
                    settingsRow("Live Agent Home", value: model.status?.liveAgentHome ?? "-")
                    settingsRow("Platform", value: model.doctor?.platform ?? "-")
                }

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
            .frame(maxWidth: .infinity, alignment: .topLeading)
            .padding(28)
        }
        .task {
            await model.refresh()
        }
        .onAppear {
            SettingsPaneID.persistedSelection = .general
        }
    }

    private func settingsRow(_ title: String, value: String) -> some View {
        LabeledContent(title, value: value)
    }
}

public struct ActivitySettingsPaneView: View {
    @ObservedObject var model: RelayAppModel

    public init(model: RelayAppModel) {
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
    @ObservedObject var model: RelayAppModel

    public init(model: RelayAppModel) {
        self.model = model
    }

    public var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                paneHeader(
                    title: "About Relay",
                    subtitle: "CLI-first local profile orchestration for coding agents."
                )

                SettingsSurfaceCard("Application") {
                    settingsRow("Version", value: appVersion)
                    settingsRow("CLI Path", value: ProcessInfo.processInfo.environment["RELAY_CLI_PATH"] ?? "Bundled relay")
                    settingsRow("Profiles", value: "\(model.status?.profileCount ?? model.profiles.count)")
                }

                SettingsSurfaceCard("Runtime") {
                    settingsRow("Relay Home", value: model.status?.relayHome ?? "-")
                    settingsRow("Live Agent Home", value: model.status?.liveAgentHome ?? "-")
                    settingsRow("Platform", value: model.doctor?.platform ?? "-")
                    settingsRow("Agent Binary", value: model.doctor?.agentBinary ?? "-")
                }

                SettingsSurfaceCard("Project") {
                    Text("Relay keeps the CLI as the single execution layer. The macOS app is a control plane over stable JSON commands.")
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .foregroundStyle(.secondary)
                }
            }
            .frame(maxWidth: .infinity, alignment: .topLeading)
            .padding(28)
        }
        .task {
            await model.refresh()
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

    private func settingsRow(_ title: String, value: String) -> some View {
        LabeledContent(title, value: value)
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
        VStack(alignment: .leading, spacing: 14) {
            Text(title)
                .font(.system(size: 18, weight: .semibold, design: .rounded))
            content
        }
        .padding(20)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.background.secondary, in: RoundedRectangle(cornerRadius: 18, style: .continuous))
    }
}

func paneHeader(title: String, subtitle: String) -> some View {
    VStack(alignment: .leading, spacing: 6) {
        Text(title)
            .font(.system(size: 30, weight: .semibold, design: .rounded))
        Text(subtitle)
            .foregroundStyle(.secondary)
    }
}
