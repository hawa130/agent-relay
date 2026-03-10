import AppKit
import RelayMacOSUI
import SwiftUI

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
            return NSSize(width: 720, height: 540)
        case .settings:
            return NSSize(width: 660, height: 520)
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

    var usesSidebarTitlebarChrome: Bool {
        switch self {
        case .manager:
            return true
        case .settings:
            return true
        }
    }

    var titlebarTransparent: Bool {
        switch self {
        case .manager:
            return false
        case .settings:
            return true
        }
    }

    var usesFullSizeContentView: Bool {
        switch self {
        case .manager:
            return false
        case .settings:
            return true
        }
    }
}

@MainActor
class RelayWindowController: NSWindowController, NSWindowDelegate {
    private let style: RelayWindowStyle
    private var hasPresentedWindow = false

    init(
        windowID: RelayWindowID,
        title: String,
        style: RelayWindowStyle,
        rootView: AnyView
    ) {
        self.style = style
        let hostingController = NSHostingController(rootView: rootView)
        let window = NSWindow(
            contentRect: NSRect(origin: .zero, size: style.contentSize),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )

        window.contentViewController = hostingController
        window.title = title
        window.contentMinSize = style.minSize
        window.toolbarStyle = style.toolbarStyle
        if style.usesSidebarTitlebarChrome {
            if style.usesFullSizeContentView {
                window.styleMask.insert(.fullSizeContentView)
            }
            window.titlebarAppearsTransparent = style.titlebarTransparent
            window.titleVisibility = .hidden
            window.isMovableByWindowBackground = style.usesFullSizeContentView
        }
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
        if let window, !hasPresentedWindow {
            window.setContentSize(style.contentSize)
            window.center()
            hasPresentedWindow = true
        }
        NSApp.activate(ignoringOtherApps: true)
        showWindow(nil)
        window?.makeKeyAndOrderFront(nil)
    }

    func windowWillClose(_ notification: Notification) {
        _ = notification
    }
}
