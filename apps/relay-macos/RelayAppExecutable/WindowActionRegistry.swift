import AgentRelayUI
import SwiftUI

@MainActor
final class WindowActionRegistry {
    static let shared = WindowActionRegistry()

    private var openWindowAction: OpenWindowAction?

    private init() {}

    func install(_ action: OpenWindowAction) {
        openWindowAction = action
    }

    func open(_ windowID: RelayWindowID) {
        openWindowAction?(id: windowID.rawValue)
    }
}
