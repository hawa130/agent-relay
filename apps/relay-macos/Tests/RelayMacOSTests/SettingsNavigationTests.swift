import AppKit
import XCTest
@testable import RelayMacOSUI

final class SettingsNavigationTests: XCTestCase {
    func testLegacyTopLevelSettingsSelectionsMapToUnifiedSettingsPane() {
        XCTAssertEqual(SettingsPaneID.storedValue("general"), .settings)
        XCTAssertEqual(SettingsPaneID.storedValue("codex"), .settings)
        XCTAssertEqual(SettingsPaneID.storedValue("profiles"), .profiles)
    }

    func testLegacyCodexPaneMapsToCodexSidebarSelection() {
        XCTAssertEqual(
            SettingsSidebarSelection.storedValue(nil, legacyPaneValue: "codex"),
            .agent(.codex)
        )
        XCTAssertEqual(
            SettingsSidebarSelection.storedValue("general", legacyPaneValue: nil),
            .general
        )
        XCTAssertEqual(
            SettingsSidebarSelection.storedValue("agent:codex", legacyPaneValue: nil),
            .agent(.codex)
        )
    }

    func testCodexBrandingResourceLoadsFromModuleBundle() throws {
        let descriptor = try XCTUnwrap(AgentSettingsCatalog.descriptor(for: .codex))
        let image = descriptor.iconImage()

        XCTAssertNotNil(image)
        XCTAssertEqual(image?.size.width, 18)
        XCTAssertEqual(image?.size.height, 18)
        XCTAssertTrue(image?.isTemplate ?? false)
    }
}
