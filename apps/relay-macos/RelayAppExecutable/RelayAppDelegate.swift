import AppKit
import RelayMacOSUI
import SwiftUI

@MainActor
final class RelayAppDelegate: NSObject, NSApplicationDelegate {
    private let model = RelayAppModel()
    private lazy var settingsPaneModel = SettingsPaneModel(session: model)
    private lazy var profilesPaneModel = ProfilesPaneModel(session: model)
    private var statusItemController: RelayStatusItemController?
    private lazy var profilesWindowController = ProfilesWindowController(
        title: RelayWindowID.profiles.title,
        rootView: AnyView(
            ProfilesSettingsPaneView(model: self.profilesPaneModel)
        ),
        onAddProfile: { [weak self] in
            self?.profilesPaneModel.presentAddSheet()
        }
    )
    private lazy var settingsWindowController = SettingsWindowController(
        title: RelayWindowID.settings.title,
        rootView: AnyView(
            SettingsPaneView(model: self.settingsPaneModel)
        )
    )

    func applicationDidFinishLaunching(_ notification: Notification) {
        _ = notification
        NSApp.setActivationPolicy(.accessory)
        model.start()
        statusItemController = RelayStatusItemController(
            model: model,
            openWindow: { [weak self] windowID in
                self?.openWindow(windowID)
            }
        )
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        _ = sender
        return false
    }

    private func openWindow(_ windowID: RelayWindowID) {
        switch windowID {
        case .profiles:
            profilesWindowController.presentAndActivate()
        case .settings:
            settingsWindowController.presentAndActivate()
        }

        Task { [weak self] in
            switch windowID {
            case .profiles:
                await self?.profilesPaneModel.refreshIfStale()
            case .settings:
                await self?.settingsPaneModel.refreshIfStale()
            }
        }
    }
}
