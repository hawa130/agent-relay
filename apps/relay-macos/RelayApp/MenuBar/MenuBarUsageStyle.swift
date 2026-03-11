import Foundation
import SwiftUI

extension UsageWindow {
    var menuBarDisplayValue: String {
        if let usedPercent {
            return String(format: "%.0f%%", usedPercent)
        }

        return status.shortLabel
    }

    var menuBarProgressPercent: Double {
        if let usedPercent {
            return min(max(usedPercent, 0), 100)
        }

        return status == .exhausted ? 100 : 0
    }
}

extension UsageStatus {
    var menuBarTint: Color {
        NativePreferencesTheme.Colors.usageTint(self)
    }
}
