import SwiftUI

struct MenuBarMetricRowModel: Identifiable {
    let id: String
    let title: String
    let percent: Double
    let percentLabel: String
    let resetText: String?
    let detailLeftText: String?
    let detailRightText: String?
    let tint: Color
}
