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
            .success
        case .warning:
            .warning
        case .exhausted:
            .danger
        case .unknown:
            .neutral
        }
    }
}

extension UsageSnapshot {
    var ringProgressItems: [RingProgressItem] {
        [
            RingProgressItem(
                id: "session",
                label: "Session",
                shortLabel: "S",
                progress: session.ringProgress,
                tone: session.status.ringTone,
                isDimmed: stale,
                valueText: session.ringValueText,
                detailText: session.resetBadgeText),
            RingProgressItem(
                id: "weekly",
                label: "Weekly",
                shortLabel: "W",
                progress: weekly.ringProgress,
                tone: weekly.status.ringTone,
                isDimmed: stale,
                valueText: weekly.ringValueText,
                detailText: weekly.resetBadgeText)
        ]
    }
}

extension UsageWindow {
    var ringProgress: Double {
        if let usedPercent {
            return min(max(usedPercent, 0), 100) / 100
        }

        return status == .exhausted ? 1 : 0
    }

    var ringValueText: String {
        if let usedPercent {
            return String(format: "%.0f%%", usedPercent)
        }

        return status.shortLabel
    }

    var resetBadgeText: String? {
        guard let resetAt else {
            return nil
        }

        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .short
        return "Resets \(formatter.localizedString(for: resetAt, relativeTo: Date()))"
    }
}

extension UsageStatus {
    var shortLabel: String {
        switch self {
        case .healthy:
            "OK"
        case .warning:
            "Warn"
        case .exhausted:
            "Full"
        case .unknown:
            "?"
        }
    }

    var ringTone: RingProgressTone {
        switch self {
        case .healthy:
            .positive
        case .warning:
            .warning
        case .exhausted:
            .critical
        case .unknown:
            .neutral
        }
    }
}
