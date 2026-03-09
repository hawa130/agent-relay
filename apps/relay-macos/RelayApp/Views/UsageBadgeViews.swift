import SwiftUI

struct UsageBadgeRow: View {
    let usage: UsageSnapshot

    var body: some View {
        HStack(spacing: 6) {
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
            .font(.caption2.monospacedDigit().weight(.semibold))
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(tint.opacity(stale ? 0.18 : 0.28), in: Capsule())
            .foregroundStyle(stale ? .secondary : tint)
    }

    private var label: String {
        if let usedPercent = window.usedPercent {
            return String(format: "%.0f%%", usedPercent)
        }
        return window.status.shortLabel
    }

    private var tint: Color {
        switch window.status {
        case .healthy:
            return .green
        case .warning:
            return .orange
        case .exhausted:
            return .red
        case .unknown:
            return .gray
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
