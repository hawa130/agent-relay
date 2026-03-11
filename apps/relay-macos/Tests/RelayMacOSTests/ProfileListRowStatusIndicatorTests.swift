import XCTest
@testable import RelayMacOSUI

final class ProfileListRowStatusIndicatorTests: XCTestCase {
    func testLoadingIndicatorTakesPrecedenceOverWarning() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            isFetchingUsage: true,
            usageRefreshError: "remote usage timed out"
        )

        XCTAssertEqual(indicator, .loading)
    }

    func testWarningIndicatorUsesErrorMessage() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            isFetchingUsage: false,
            usageRefreshError: "remote usage timed out"
        )

        XCTAssertEqual(indicator, .warning(message: "remote usage timed out"))
    }

    func testIndicatorIsNilWithoutLoadingOrError() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            isFetchingUsage: false,
            usageRefreshError: nil
        )

        XCTAssertNil(indicator)
    }

    func testIndicatorIgnoresEmptyErrorMessage() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            isFetchingUsage: false,
            usageRefreshError: ""
        )

        XCTAssertNil(indicator)
    }
}
