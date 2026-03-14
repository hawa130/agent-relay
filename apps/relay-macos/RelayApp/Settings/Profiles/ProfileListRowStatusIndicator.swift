import SwiftUI

struct ProfileListRowStatusIndicator: View {
    enum Kind: Equatable {
        case loading
        case warning(message: String)
        case stale

        init?(
            profile: Profile,
            isFetchingUsage: Bool,
            usage: UsageSnapshot?,
            usageRefreshError: String?,
            isStale: Bool)
        {
            if isFetchingUsage {
                self = .loading
            } else if profile.accountState == .accountUnavailable {
                let statusDetail = profile.accountErrorHTTPStatus.map { " (HTTP \($0))" } ?? ""
                self = .warning(message: "Account unavailable for auto-switch\(statusDetail)")
            } else if let note = usage?.userFacingNote,
                      let severity = UsageCardNoteResolver.severity(for: usage)
            {
                switch severity {
                case .warning:
                    self = .warning(message: note)
                }
            } else if let usageRefreshError, !usageRefreshError.isEmpty {
                self = .warning(message: usageRefreshError)
            } else if isStale {
                self = .stale
            } else {
                return nil
            }
        }
    }

    let indicator: Kind

    var body: some View {
        Group {
            switch indicator {
            case .loading:
                ProgressView()
                    .controlSize(.mini)
            case let .warning(message):
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(NativePreferencesTheme.Colors.statusIcon(.warning))
                    .help(message)
            case .stale:
                Image(systemName: "exclamationmark.triangle")
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(.secondary)
                    .help("Usage data may be stale")
            }
        }
        .frame(width: 12, height: 12)
    }
}
