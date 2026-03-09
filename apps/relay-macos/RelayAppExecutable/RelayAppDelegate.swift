import AppKit
@preconcurrency import Settings
import RelayMacOSUI

@MainActor
final class RelayAppDelegate: NSObject, NSApplicationDelegate {
    private let model = RelayAppModel()
    private lazy var settingsSessionModel = SettingsSessionModel(session: model)
    private lazy var profilesPaneModel = ProfilesPaneModel(session: model)
    private lazy var activityPaneModel = ActivityPaneModel(session: model)
    private var statusItemController: RelayStatusItemController?
    private lazy var settingsWindowController = SettingsWindowController(
        panes: [
            Settings.Pane(
                identifier: .relayGeneral,
                title: SettingsPaneID.general.title,
                toolbarIcon: Self.toolbarIcon(SettingsPaneID.general.symbol, description: SettingsPaneID.general.title)
            ) {
                GeneralSettingsPaneView(model: self.settingsSessionModel)
                    .frame(
                        width: NativePreferencesTheme.Metrics.windowWidth,
                        height: NativePreferencesTheme.Metrics.windowHeight,
                        alignment: .topLeading
                    )
            },
            Settings.Pane(
                identifier: .relayProfiles,
                title: SettingsPaneID.profiles.title,
                toolbarIcon: Self.toolbarIcon(SettingsPaneID.profiles.symbol, description: SettingsPaneID.profiles.title)
            ) {
                ProfilesSettingsPaneView(model: self.profilesPaneModel)
                    .frame(
                        width: NativePreferencesTheme.Metrics.windowWidth,
                        height: NativePreferencesTheme.Metrics.windowHeight,
                        alignment: .topLeading
                    )
            },
            Settings.Pane(
                identifier: .relayActivity,
                title: SettingsPaneID.activity.title,
                toolbarIcon: Self.toolbarIcon(SettingsPaneID.activity.symbol, description: SettingsPaneID.activity.title)
            ) {
                ActivitySettingsPaneView(model: self.activityPaneModel)
                    .frame(
                        width: NativePreferencesTheme.Metrics.windowWidth,
                        height: NativePreferencesTheme.Metrics.windowHeight,
                        alignment: .topLeading
                    )
            },
            Settings.Pane(
                identifier: .relayAbout,
                title: SettingsPaneID.about.title,
                toolbarIcon: Self.toolbarIcon(SettingsPaneID.about.symbol, description: SettingsPaneID.about.title)
            ) {
                AboutSettingsPaneView(model: self.settingsSessionModel)
                    .frame(
                        width: NativePreferencesTheme.Metrics.windowWidth,
                        height: NativePreferencesTheme.Metrics.windowHeight,
                        alignment: .topLeading
                    )
            },
        ],
        style: .toolbarItems,
        animated: false
    )

    func applicationDidFinishLaunching(_ notification: Notification) {
        _ = notification
        NSApp.setActivationPolicy(.accessory)
        model.start()
        statusItemController = RelayStatusItemController(
            model: model,
            openSettings: { [weak self] in
                self?.openSettingsWindow()
            }
        )
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        _ = sender
        return false
    }

    private func openSettingsWindow() {
        NSApp.activate(ignoringOtherApps: true)
        settingsWindowController.show(pane: SettingsPaneID.persistedSelection.settingsIdentifier)
        Task { [weak self] in
            await self?.settingsSessionModel.refreshIfStale()
        }
    }

    private static func toolbarIcon(_ symbolName: String, description: String) -> NSImage {
        NSImage(systemSymbolName: symbolName, accessibilityDescription: description)
            ?? NSImage(named: NSImage.preferencesGeneralName)
            ?? NSImage()
    }
}

@MainActor
private extension Settings.PaneIdentifier {
    static let relayGeneral = Self("general")
    static let relayProfiles = Self("profiles")
    static let relayActivity = Self("activity")
    static let relayAbout = Self("about")
}

@MainActor
private extension SettingsPaneID {
    var settingsIdentifier: Settings.PaneIdentifier {
        switch self {
        case .general:
            return .relayGeneral
        case .profiles:
            return .relayProfiles
        case .activity:
            return .relayActivity
        case .about:
            return .relayAbout
        }
    }
}
