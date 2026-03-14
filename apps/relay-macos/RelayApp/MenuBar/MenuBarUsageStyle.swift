import Foundation
import SwiftUI

extension UsageWindow {
    var menuBarDisplayValue: String {
        if let usedPercent {
            return usedPercent.formatted(.number.precision(.fractionLength(0))) + "%"
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
