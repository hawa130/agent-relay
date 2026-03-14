import SwiftUI

struct ProfileAccountStatusDot: View {
    let accountState: ProfileAccountState
    let diameter: CGFloat

    var body: some View {
        Image(systemName: accountStatusSymbol)
            .font(.system(size: diameter, weight: .bold))
            .foregroundStyle(accountStatusColor)
            .frame(width: diameter + 4, height: diameter + 4)
            .help(accountStatusHelp)
            .accessibilityLabel(accountStatusHelp)
    }

    private var accountStatusColor: Color {
        switch accountState {
        case .healthy:
            NativePreferencesTheme.Colors.statusIcon(.success)
        case .accountUnavailable:
            NativePreferencesTheme.Colors.statusIcon(.danger)
        }
    }

    private var accountStatusHelp: String {
        switch accountState {
        case .healthy:
            "Account status healthy"
        case .accountUnavailable:
            "Account status unavailable for auto-switch"
        }
    }

    private var accountStatusSymbol: String {
        switch accountState {
        case .healthy:
            "checkmark.circle.fill"
        case .accountUnavailable:
            "exclamationmark.circle.fill"
        }
    }
}
