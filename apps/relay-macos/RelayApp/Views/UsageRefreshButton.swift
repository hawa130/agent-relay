import SwiftUI

struct UsageRefreshButton: View {
    enum Variant {
        case toolbar
        case card

        var iconFrameWidth: CGFloat {
            switch self {
            case .toolbar:
                28
            case .card:
                14
            }
        }

        var usesIconOnlyLabel: Bool {
            switch self {
            case .toolbar:
                false
            case .card:
                true
            }
        }
    }

    let isRefreshing: Bool
    let variant: Variant
    let helpText: String
    let action: () -> Void

    init(
        isRefreshing: Bool,
        variant: Variant = .card,
        helpText: String = "Refresh Usage",
        action: @escaping () -> Void)
    {
        self.isRefreshing = isRefreshing
        self.variant = variant
        self.helpText = helpText
        self.action = action
    }

    var body: some View {
        configuredButton
    }

    private var helpTextValue: String {
        isRefreshing ? Self.accessibilityLabel(isRefreshing: true) : helpText
    }

    nonisolated static func accessibilityLabel(isRefreshing: Bool) -> String {
        isRefreshing ? "Refreshing usage" : "Refresh Usage"
    }

    nonisolated static func labelOpacity(isRefreshing: Bool) -> Double {
        isRefreshing ? 0 : 1
    }

    nonisolated static func progressOpacity(isRefreshing: Bool) -> Double {
        isRefreshing ? 1 : 0
    }

    @ViewBuilder
    private var configuredButton: some View {
        if variant.usesIconOnlyLabel {
            button.labelStyle(.iconOnly)
        } else {
            button.labelStyle(DefaultLabelStyle())
        }
    }

    private var button: some View {
        Button(action: action) {
            Label(Self.accessibilityLabel(isRefreshing: isRefreshing), systemImage: "arrow.clockwise")
                .opacity(Self.labelOpacity(isRefreshing: isRefreshing))
                .overlay {
                    ProgressView()
                        .controlSize(.small)
                        .opacity(Self.progressOpacity(isRefreshing: isRefreshing))
                }
                .frame(width: variant.iconFrameWidth, height: 14)
        }
        .buttonStyle(.bordered)
        .disabled(isRefreshing)
        .help(helpTextValue)
        .accessibilityLabel(Self.accessibilityLabel(isRefreshing: isRefreshing))
    }
}
