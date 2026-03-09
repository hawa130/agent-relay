import AppKit
@preconcurrency import Settings
import RelayMacOSUI

@MainActor
final class RelayAppDelegate: NSObject, NSApplicationDelegate {
    private let model = RelayAppModel()
    private lazy var settingsPaneModel = SettingsPaneModel(session: model)
    private lazy var profilesPaneModel = ProfilesPaneModel(session: model)
    private var statusItemController: RelayStatusItemController?
    private lazy var settingsWindowController = SettingsWindowController(
        panes: [
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
                identifier: .relaySettings,
                title: SettingsPaneID.settings.title,
                toolbarIcon: Self.toolbarIcon(SettingsPaneID.settings.symbol, description: SettingsPaneID.settings.title)
            ) {
                SettingsPaneView(model: self.settingsPaneModel)
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
            openPreferencesPane: { [weak self] pane in
                self?.openSettingsWindow(pane: pane)
            }
        )
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        _ = sender
        return false
    }

    private func openSettingsWindow(pane: SettingsPaneID) {
        NSApp.activate(ignoringOtherApps: true)
        switch pane {
        case .profiles:
            settingsWindowController.show(pane: .relayProfiles)
        case .settings:
            settingsWindowController.show(pane: .relaySettings)
        }

        Task { [weak self] in
            switch pane {
            case .profiles:
                await self?.profilesPaneModel.refreshIfStale()
            case .settings:
                await self?.settingsPaneModel.refreshIfStale()
            }
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
    static let relaySettings = Self("settings")
    static let relayProfiles = Self("profiles")
}
