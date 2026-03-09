import SwiftUI

struct MenuBarCurrentCardModel {
    let providerName: String
    let email: String
    let subtitleText: String
    let planText: String?
    let metrics: [MenuBarMetricRowModel]
    let placeholder: String?
    let usageNotes: [String]
}

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
