import RelayMacOSUI
import SwiftUI

@main
struct RelayMacOSApp: App {
    @NSApplicationDelegateAdaptor(RelayAppDelegate.self) private var appDelegate

    var body: some Scene {
        Window("Settings", id: "settings") {
            SettingsView(model: appDelegate.model)
                .frame(minWidth: 920, minHeight: 640)
        }
        .defaultSize(width: 920, height: 640)
        .windowResizability(.contentSize)
    }
}
