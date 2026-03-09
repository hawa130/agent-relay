import SwiftUI

struct UsageBadgeRow: View {
    let usage: UsageSnapshot

    var body: some View {
        HStack(spacing: 5) {
            UsageBadge(title: "S", window: usage.session, stale: usage.stale)
            UsageBadge(title: "W", window: usage.weekly, stale: usage.stale)
        }
    }
}

struct UsageBadge: View {
    let title: String
    let window: UsageWindow
    let stale: Bool

    var body: some View {
        Text("\(title) \(label)")
            .font(.system(size: 10, weight: .semibold, design: .monospaced))
            .padding(.horizontal, 5)
            .padding(.vertical, 2.5)
            .background(NativePreferencesTheme.Badge.fill(kind).opacity(stale ? 0.78 : 1), in: Capsule())
            .foregroundStyle(stale ? .secondary : NativePreferencesTheme.Badge.text(kind))
    }

    private var label: String {
        if let usedPercent = window.usedPercent {
            return String(format: "%.0f%%", usedPercent)
        }
        return window.status.shortLabel
    }

    private var kind: NativePreferencesTheme.Badge.Kind {
        switch window.status {
        case .healthy:
            return .success
        case .warning:
            return .warning
        case .exhausted:
            return .danger
        case .unknown:
            return .neutral
        }
    }
}

extension UsageStatus {
    var shortLabel: String {
        switch self {
        case .healthy:
            return "OK"
        case .warning:
            return "Warn"
        case .exhausted:
            return "Full"
        case .unknown:
            return "?"
        }
    }
}
