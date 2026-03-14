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
        Button(action: action) {
            Group {
                if isRefreshing {
                    ProgressView()
                        .controlSize(.small)
                } else {
                    Image(systemName: "arrow.clockwise")
                }
            }
            .frame(width: variant.iconFrameWidth, height: 14)
        }
        .buttonStyle(.bordered)
        .disabled(isRefreshing)
        .help(helpTextValue)
        .accessibilityLabel(Self.accessibilityLabel(isRefreshing: isRefreshing))
        .labelStyle(.iconOnly)
    }

    private var helpTextValue: String {
        isRefreshing ? Self.accessibilityLabel(isRefreshing: true) : helpText
    }

    nonisolated static func accessibilityLabel(isRefreshing: Bool) -> String {
        isRefreshing ? "Refreshing usage" : "Refresh Usage"
    }
}
