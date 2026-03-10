import AppKit
import RelayMacOSUI
import SwiftUI

private extension NSToolbarItem.Identifier {
    static let addProfile = Self("relay.addProfile")
}

@MainActor
enum RelayWindowStyle {
    case manager
    case settings

    var contentSize: NSSize {
        switch self {
        case .manager:
            return NSSize(width: 980, height: 680)
        case .settings:
            return NSSize(width: 760, height: 620)
        }
    }

    var minSize: NSSize {
        switch self {
        case .manager:
            return NSSize(width: 900, height: 620)
        case .settings:
            return NSSize(width: 680, height: 560)
        }
    }

    var toolbarStyle: NSWindow.ToolbarStyle {
        switch self {
        case .manager:
            return .unified
        case .settings:
            return .unifiedCompact
        }
    }
}

@MainActor
class RelayWindowController: NSWindowController, NSWindowDelegate {
    init(
        windowID: RelayWindowID,
        title: String,
        style: RelayWindowStyle,
        rootView: AnyView
    ) {
        let hostingController = NSHostingController(rootView: rootView)
        let window = NSWindow(
            contentRect: NSRect(origin: .zero, size: style.contentSize),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )

        window.contentViewController = hostingController
        window.title = title
        window.minSize = style.minSize
        window.toolbarStyle = style.toolbarStyle
        window.identifier = NSUserInterfaceItemIdentifier("relay.\(windowID.rawValue)")
        window.isReleasedWhenClosed = false
        window.center()

        super.init(window: window)
        self.window?.delegate = self
        shouldCascadeWindows = false
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func presentAndActivate() {
        NSApp.activate(ignoringOtherApps: true)
        showWindow(nil)
        window?.makeKeyAndOrderFront(nil)
    }

    func windowWillClose(_ notification: Notification) {
        _ = notification
    }
}

@MainActor
final class ProfilesWindowController: RelayWindowController, NSToolbarDelegate {
    private let onAddProfile: () -> Void

    init(
        title: String,
        rootView: AnyView,
        onAddProfile: @escaping () -> Void
    ) {
        self.onAddProfile = onAddProfile
        super.init(
            windowID: .profiles,
            title: title,
            style: .manager,
            rootView: rootView
        )

        let toolbar = NSToolbar(identifier: "relay.profiles.toolbar")
        toolbar.delegate = self
        toolbar.displayMode = .iconOnly
        toolbar.allowsUserCustomization = false
        toolbar.autosavesConfiguration = false
        window?.toolbar = toolbar
    }

    func toolbarDefaultItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
        _ = toolbar
        return [NSToolbarItem.Identifier.flexibleSpace, NSToolbarItem.Identifier.addProfile]
    }

    func toolbarAllowedItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
        _ = toolbar
        return [NSToolbarItem.Identifier.flexibleSpace, NSToolbarItem.Identifier.addProfile]
    }

    func toolbar(
        _ toolbar: NSToolbar,
        itemForItemIdentifier itemIdentifier: NSToolbarItem.Identifier,
        willBeInsertedIntoToolbar flag: Bool
    ) -> NSToolbarItem? {
        _ = toolbar
        _ = flag

        guard itemIdentifier == .addProfile else {
            return nil
        }

        let item = NSToolbarItem(itemIdentifier: itemIdentifier)
        item.label = "Add"
        item.paletteLabel = "Add"
        item.toolTip = "Add Profile"
        item.image = NSImage(
            systemSymbolName: "plus",
            accessibilityDescription: "Add Profile"
        )
        item.target = self
        item.action = #selector(handleAddProfile)
        return item
    }

    @objc private func handleAddProfile() {
        onAddProfile()
    }
}
