import AgentRelayUI
import AppKit

@MainActor
final class RelayAppDelegate: NSObject, NSApplicationDelegate {
    private let model = RelayAppModel()
    lazy var settingsPaneModel = SettingsPaneModel(session: model)
    lazy var profilesPaneModel = ProfilesPaneModel(session: model)
    private var statusItemController: RelayStatusItemController?

    func applicationDidFinishLaunching(_ notification: Notification) {
        _ = notification
        NSApp.setActivationPolicy(.accessory)
        model.start()
        statusItemController = RelayStatusItemController(
            model: model,
            openWindow: { [weak self] windowID in
                self?.openWindow(windowID)
            })
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        _ = sender
        return false
    }

    func applicationWillTerminate(_ notification: Notification) {
        _ = notification
        Task {
            await model.stop()
        }
    }

    private func openWindow(_ windowID: RelayWindowID) {
        NSApp.activate(ignoringOtherApps: true)
        WindowActionRegistry.shared.open(windowID)
    }
}
