import AppKit
import RelayMacOSUI

@MainActor
final class RelayAppDelegate: NSObject, NSApplicationDelegate {
    let model = RelayAppModel()
    private var statusItemController: RelayStatusItemController?

    func applicationDidFinishLaunching(_ notification: Notification) {
        _ = notification
        statusItemController = RelayStatusItemController(
            model: model,
            openSettings: { [weak self] in
                self?.openSettingsWindow()
            }
        )
    }

    private func openSettingsWindow() {
        NSApp.activate(ignoringOtherApps: true)

        if NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil) {
            return
        }

        _ = NSApp.sendAction(Selector(("showPreferencesWindow:")), to: nil, from: nil)
    }
}
