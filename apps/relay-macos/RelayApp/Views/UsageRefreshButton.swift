import SwiftUI

struct UsageRefreshButton: View {
    let isRefreshing: Bool
    let helpText: String
    let action: () -> Void

    init(
        isRefreshing: Bool,
        helpText: String = "Refresh Usage",
        action: @escaping () -> Void)
    {
        self.isRefreshing = isRefreshing
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
            .frame(width: 14, height: 14)
        }
        .buttonStyle(.bordered)
        .disabled(isRefreshing)
        .help(helpTextValue)
        .accessibilityLabel(Self.accessibilityLabel(isRefreshing: isRefreshing))
    }

    private var helpTextValue: String {
        isRefreshing ? Self.accessibilityLabel(isRefreshing: true) : helpText
    }

    nonisolated static func accessibilityLabel(isRefreshing: Bool) -> String {
        isRefreshing ? "Refreshing usage" : "Refresh Usage"
    }
}
