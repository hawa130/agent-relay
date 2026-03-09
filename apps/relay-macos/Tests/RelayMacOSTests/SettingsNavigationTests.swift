import AppKit
import XCTest
@testable import RelayMacOSUI

final class SettingsNavigationTests: XCTestCase {
    func testCodexBrandingResourceLoadsFromModuleBundle() throws {
        let descriptor = try XCTUnwrap(AgentSettingsCatalog.descriptor(for: .codex))
        let image = descriptor.iconImage()

        XCTAssertNotNil(image)
        XCTAssertEqual(image?.size.width, 18)
        XCTAssertEqual(image?.size.height, 18)
        XCTAssertTrue(image?.isTemplate ?? false)
    }
}
