import XCTest
@testable import RelayMacOSUI

final class ProfileListRowStatusIndicatorTests: XCTestCase {
    func testLoadingIndicatorTakesPrecedenceOverWarning() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            isFetchingUsage: true,
            usageRefreshError: "remote usage timed out",
            isStale: true
        )

        XCTAssertEqual(indicator, .loading)
    }

    func testWarningIndicatorTakesPrecedenceOverStale() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            isFetchingUsage: false,
            usageRefreshError: "remote usage timed out",
            isStale: true
        )

        XCTAssertEqual(indicator, .warning(message: "remote usage timed out"))
    }

    func testWarningIndicatorUsesErrorMessage() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            isFetchingUsage: false,
            usageRefreshError: "remote usage timed out",
            isStale: false
        )

        XCTAssertEqual(indicator, .warning(message: "remote usage timed out"))
    }

    func testStaleIndicatorShowsWhenUsageIsOld() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            isFetchingUsage: false,
            usageRefreshError: nil,
            isStale: true
        )

        XCTAssertEqual(indicator, .stale)
    }

    func testIndicatorIsNilWithoutLoadingOrError() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            isFetchingUsage: false,
            usageRefreshError: nil,
            isStale: false
        )

        XCTAssertNil(indicator)
    }

    func testIndicatorIgnoresEmptyErrorMessage() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            isFetchingUsage: false,
            usageRefreshError: "",
            isStale: false
        )

        XCTAssertNil(indicator)
    }
}
