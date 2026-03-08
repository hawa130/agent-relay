import RelayMacOSUI
import SwiftUI

@main
struct RelayMacOSApp: App {
    @StateObject private var model = RelayAppModel()

    var body: some Scene {
        MenuBarExtra {
            MenuBarView(model: model)
        } label: {
            Label(model.menuBarTitle, systemImage: model.menuBarSymbol)
        }

        Window("Settings", id: "settings") {
            SettingsView(model: model)
                .frame(minWidth: 920, minHeight: 640)
        }
        .defaultSize(width: 920, height: 640)
        .windowResizability(.contentSize)
    }
}
