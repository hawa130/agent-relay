@testable import AgentRelayUI
import XCTest

final class UsageProgressLayoutTests: XCTestCase {
    func testMenuBarProgressFillWidthClampsPercentAndRespectsMinimumVisibleWidth() {
        XCTAssertEqual(MenuBarUsageProgressLayout.fillWidth(percent: -10, totalWidth: 120), 0, accuracy: 0.001)
        XCTAssertEqual(MenuBarUsageProgressLayout.fillWidth(percent: 0, totalWidth: 120), 0, accuracy: 0.001)
        XCTAssertEqual(MenuBarUsageProgressLayout.fillWidth(percent: 2, totalWidth: 120), 6, accuracy: 0.001)
        XCTAssertEqual(MenuBarUsageProgressLayout.fillWidth(percent: 50, totalWidth: 120), 60, accuracy: 0.001)
        XCTAssertEqual(MenuBarUsageProgressLayout.fillWidth(percent: 140, totalWidth: 120), 120, accuracy: 0.001)
    }

    func testUsageMetricFillWidthUsesRatioAndMinimumVisibleWidth() {
        XCTAssertEqual(UsageMetricProgressLayout.fillWidth(ratio: -0.4, totalWidth: 160), 0, accuracy: 0.001)
        XCTAssertEqual(UsageMetricProgressLayout.fillWidth(ratio: 0, totalWidth: 160), 0, accuracy: 0.001)
        XCTAssertEqual(UsageMetricProgressLayout.fillWidth(ratio: 0.02, totalWidth: 160), 8, accuracy: 0.001)
        XCTAssertEqual(UsageMetricProgressLayout.fillWidth(ratio: 0.5, totalWidth: 160), 80, accuracy: 0.001)
        XCTAssertEqual(UsageMetricProgressLayout.fillWidth(ratio: 1.8, totalWidth: 160), 160, accuracy: 0.001)
    }
}
