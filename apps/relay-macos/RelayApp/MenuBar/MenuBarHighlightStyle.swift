import SwiftUI

extension EnvironmentValues {
    @Entry var menuItemHighlighted: Bool = false
}

enum MenuBarHighlightStyle {
    static let selectionText = Color(nsColor: .selectedMenuItemTextColor)
    static let normalPrimaryText = Color(nsColor: .controlTextColor)
    static let normalSecondaryText = Color(nsColor: .secondaryLabelColor)

    static func primary(_ highlighted: Bool) -> Color {
        highlighted ? selectionText : normalPrimaryText
    }

    static func secondary(_ highlighted: Bool) -> Color {
        highlighted ? selectionText : normalSecondaryText
    }

    static func progressTrack(_ highlighted: Bool) -> Color {
        highlighted ? selectionText.opacity(0.22) : Color(nsColor: .tertiaryLabelColor).opacity(0.22)
    }

    static func progressTint(_ highlighted: Bool, fallback: Color) -> Color {
        highlighted ? selectionText : fallback
    }

    static func selectionBackground(_ highlighted: Bool) -> Color {
        highlighted ? Color(nsColor: .selectedContentBackgroundColor) : .clear
    }
}
