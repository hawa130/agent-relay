@testable import RelayMacOSUI
import XCTest

final class ProfilesViewSemanticsTests: XCTestCase {
    func testUsageRefreshButtonToolbarVariantUsesIconOnlyControlWidth() {
        XCTAssertEqual(
            UsageRefreshButton.Variant.toolbar.iconFrameWidth,
            28,
            accuracy: 0.001)
    }

    func testUsageRefreshButtonCardVariantUsesCompactIconWidth() {
        XCTAssertEqual(
            UsageRefreshButton.Variant.card.iconFrameWidth,
            14,
            accuracy: 0.001)
    }

    @MainActor
    func testActivateProfileActionLabelReflectsCurrentState() {
        XCTAssertEqual(
            ProfilesDetailPane.activateProfileLabel(isActive: false),
            "Activate Profile")
        XCTAssertEqual(
            ProfilesDetailPane.activateProfileLabel(isActive: true),
            "Profile is active")
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

    func testStepperAccessibilityValueUsesCurrentValueText() {
        XCTAssertEqual(
            NativeStepperRow.accessibilityValueText("15 minutes"),
            "15 minutes")
    }

    func testProfileAccountStatusDotUsesPlainCircleWithoutSymbolGlyphs() {
        XCTAssertEqual(ProfileAccountStatusDot.symbolName, "circle.fill")
    }
}
