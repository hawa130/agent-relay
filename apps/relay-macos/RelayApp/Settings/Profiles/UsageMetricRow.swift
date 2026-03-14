import SwiftUI

enum UsageMetricProgressLayout {
    static func fillWidth(ratio: Double, totalWidth: CGFloat) -> CGFloat {
        let clampedRatio = min(max(ratio, 0), 1)
        guard clampedRatio > 0 else {
            return 0
        }

        return max(8, totalWidth * clampedRatio)
    }
}

struct UsageMetricRow: View {
    let title: String
    let window: UsageWindow
    let stale: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(title)
                    .font(.system(size: 13, weight: .semibold))
                Spacer()
                Text(usedPercentText ?? window.status.rawValue.capitalized)
                    .font(NativePreferencesTheme.Typography.detail)
                    .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
            }

            GeometryReader { proxy in
                RoundedRectangle(cornerRadius: 999, style: .continuous)
                    .fill(NativePreferencesTheme.Colors.progressTrack)
                    .overlay(alignment: .leading) {
                        RoundedRectangle(cornerRadius: 999, style: .continuous)
                            .fill(tint.opacity(stale ? 0.45 : 0.8))
                            .frame(width: UsageMetricProgressLayout.fillWidth(ratio: usageFillRatio, totalWidth: proxy.size.width))
                    }
            }
            .frame(height: NativePreferencesTheme.Metrics.usageBarHeight)

            HStack {
                Text(window.resetAt.map { "Resets \($0.formatted(date: .abbreviated, time: .shortened))" } ?? "No reset window")
                    .font(NativePreferencesTheme.Typography.detail)
                    .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
                Spacer()
                Text(window.exact ? "Exact" : "Estimate")
                    .font(NativePreferencesTheme.Typography.detail)
                    .foregroundStyle(NativePreferencesTheme.Colors.mutedText)
            }
        }
    }

    private var tint: Color {
        NativePreferencesTheme.Colors.usageTint(window.status)
    }

    private var usageFillRatio: Double {
        min(max(window.usedPercent ?? 0, 0), 100) / 100
    }

    private var usedPercentText: String? {
        window.usedPercent.map {
            $0.formatted(.number.precision(.fractionLength(0))) + "% used"
        }
    }
}
