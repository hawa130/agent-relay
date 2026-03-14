import SwiftUI

enum AddProfileSheetFlowState: Equatable {
    case requesting
    case notSignedIn(detail: String)
    case failed(detail: String)

    var isRequesting: Bool {
        if case .requesting = self {
            return true
        }
        return false
    }

    var primaryActionTitle: String? {
        switch self {
        case .requesting:
            nil
        case .notSignedIn, .failed:
            "Try Again"
        }
    }

    var secondaryActionTitle: String {
        switch self {
        case .requesting:
            "Cancel"
        case .notSignedIn, .failed:
            "Back"
        }
    }

    var bodyText: String {
        switch self {
        case .requesting:
            "Complete the browser login, or cancel."
        case .notSignedIn:
            "Login did not complete."
        case let .failed(detail):
            detail
        }
    }

    var statusTitle: String {
        switch self {
        case .requesting:
            "Add Account..."
        case .notSignedIn, .failed:
            "Add Account"
        }
    }

    var statusSubtitle: String {
        switch self {
        case .requesting:
            "Requesting login..."
        case .notSignedIn:
            "Not signed in"
        case .failed:
            "Login failed"
        }
    }

    var statusDetail: String? {
        switch self {
        case .requesting:
            nil
        case let .notSignedIn(detail), let .failed(detail):
            detail
        }
    }

    var symbolName: String {
        switch self {
        case .requesting:
            "key.fill"
        case .notSignedIn:
            "person.crop.circle.badge.xmark"
        case .failed:
            "exclamationmark.triangle.fill"
        }
    }

    var accentColor: Color {
        switch self {
        case .requesting:
            .secondary
        case .notSignedIn:
            NativePreferencesTheme.Colors.statusIcon(.warning)
        case .failed:
            NativePreferencesTheme.Colors.statusIcon(.danger)
        }
    }

    var backgroundColor: Color {
        switch self {
        case .requesting:
            NativePreferencesTheme.Colors.pendingFill
        case .notSignedIn:
            NativePreferencesTheme.Colors.statusFill(.warning)
        case .failed:
            NativePreferencesTheme.Colors.statusFill(.danger)
        }
    }

    static func from(result: AddAccountResult) -> Self? {
        switch result {
        case .success, .cancelled:
            nil
        case let .notSignedIn(detail):
            .notSignedIn(detail: detail)
        case let .failed(detail):
            .failed(detail: detail)
        }
    }
}
