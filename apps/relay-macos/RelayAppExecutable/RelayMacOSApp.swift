import SwiftUI
import RelayMacOSUI

@main
struct RelayMacOSApp: App {
    @NSApplicationDelegateAdaptor(RelayAppDelegate.self) private var appDelegate

    var body: some Scene {
        Window(RelayWindowID.profiles.title, id: RelayWindowID.profiles.rawValue) {
            ProfilesWindowRootView(model: appDelegate.profilesPaneModel)
        }
        .defaultLaunchBehavior(.suppressed)
        .restorationBehavior(.disabled)
        .defaultSize(width: 980, height: 680)
        .windowResizability(.contentMinSize)
        .windowToolbarStyle(.unified(showsTitle: false))
        .commands {
            WindowActionRegistryCommands()
        }

        Window(RelayWindowID.settings.title, id: RelayWindowID.settings.rawValue) {
            SettingsWindowRootView(model: appDelegate.settingsPaneModel)
        }
        .defaultLaunchBehavior(.suppressed)
        .restorationBehavior(.disabled)
        .defaultSize(width: 760, height: 620)
        .windowResizability(.contentMinSize)
        .windowToolbarStyle(.unifiedCompact(showsTitle: true))
    }
}

private struct ProfilesWindowRootView: View {
    @ObservedObject var model: ProfilesPaneModel

    var body: some View {
        ProfilesSettingsPaneView(model: model)
            .frame(minWidth: 720, minHeight: 540)
            .background(WindowActionInstaller())
            .task {
                await model.refreshIfStale()
            }
    }
}

private struct SettingsWindowRootView: View {
    @ObservedObject var model: SettingsPaneModel

    var body: some View {
        SettingsPaneView(model: model)
            .frame(minWidth: 660, minHeight: 520)
            .background(WindowActionInstaller())
            .task {
                await model.refreshIfStale()
            }
    }
}

private struct WindowActionInstaller: View {
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        Color.clear
            .allowsHitTesting(false)
            .task {
                WindowActionRegistry.shared.install(openWindow)
            }
    }
}

private struct WindowActionRegistryCommands: Commands {
    @Environment(\.openWindow) private var openWindow

    var body: some Commands {
        let _ = WindowActionRegistry.shared.install(openWindow)
        CommandGroup(after: .appInfo) {}
    }
}
