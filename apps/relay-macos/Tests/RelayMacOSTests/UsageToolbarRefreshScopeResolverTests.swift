import AppKit
@testable import RelayMacOSUI
import XCTest

final class UsageToolbarRefreshScopeResolverTests: XCTestCase {
    func testDefaultsToEnabledRefreshWithoutOptionModifier() {
        XCTAssertEqual(
            UsageToolbarRefreshScopeResolver.resolve(modifierFlags: []),
            .enabled)
    }

    func testOptionModifierSelectsAllRefresh() {
        XCTAssertEqual(
            UsageToolbarRefreshScopeResolver.resolve(modifierFlags: [.option]),
            .all)
    }
}
