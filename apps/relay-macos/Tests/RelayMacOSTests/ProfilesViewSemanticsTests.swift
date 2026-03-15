@testable import AgentRelayUI
import XCTest

final class ProfilesViewSemanticsTests: XCTestCase {
    func testUsageRefreshButtonToolbarVariantUsesIconOnlyControlWidth() {
        XCTAssertEqual(
            UsageRefreshButton.Variant.toolbar.iconFrameWidth,
            28,
            accuracy: 0.001)
        XCTAssertFalse(UsageRefreshButton.Variant.toolbar.usesIconOnlyLabel)
    }

    func testUsageRefreshButtonCardVariantUsesCompactIconWidth() {
        XCTAssertEqual(
            UsageRefreshButton.Variant.card.iconFrameWidth,
            14,
            accuracy: 0.001)
        XCTAssertTrue(UsageRefreshButton.Variant.card.usesIconOnlyLabel)
    }

    @MainActor
    func testActivateProfileActionLabelReflectsCurrentState() {
        XCTAssertEqual(
            ProfilesDetailPane.activateProfileLabel(isActive: false),
            "Set as current")
        XCTAssertEqual(
            ProfilesDetailPane.activateProfileLabel(isActive: true),
            "Set as current")
    }

    @MainActor
    func testActivateProfileSymbolReflectsCurrentState() {
        XCTAssertEqual(
            ProfilesDetailPane.activateProfileSymbol(isActive: false),
            "checkmark.circle")
        XCTAssertEqual(
            ProfilesDetailPane.activateProfileSymbol(isActive: true),
            "checkmark.circle.fill")
    }

    func testUsageRefreshButtonLabelUsesIdleAndLoadingStates() {
        XCTAssertEqual(
            UsageRefreshButton.accessibilityLabel(isRefreshing: false),
            "Refresh Usage")
        XCTAssertEqual(
            UsageRefreshButton.accessibilityLabel(isRefreshing: true),
            "Refreshing usage")
    }

    func testUsageRefreshButtonSwitchesFromIconToLoaderWhenRefreshing() {
        XCTAssertEqual(
            UsageRefreshButton.labelOpacity(isRefreshing: false),
            1,
            accuracy: 0.001)
        XCTAssertEqual(
            UsageRefreshButton.progressOpacity(isRefreshing: false),
            0,
            accuracy: 0.001)
        XCTAssertEqual(
            UsageRefreshButton.labelOpacity(isRefreshing: true),
            0,
            accuracy: 0.001)
        XCTAssertEqual(
            UsageRefreshButton.progressOpacity(isRefreshing: true),
            1,
            accuracy: 0.001)
    }

    func testStepperAccessibilityValueUsesCurrentValueText() {
        XCTAssertEqual(
            NativeStepperRow.accessibilityValueText("15 minutes"),
            "15 minutes")
    }

    func testProfileAccountStatusDotUsesPlainCircleWithoutSymbolGlyphs() {
        XCTAssertEqual(ProfileAccountStatusDot.symbolName, "circle.fill")
    }
}
