import AppKit
import XCTest
@testable import RelayMacOSUI

final class SettingsNavigationTests: XCTestCase {
    @MainActor
    func testSelectionDefaultsToGeneral() {
        let model = SettingsPaneModel(session: RelayAppModel())

        XCTAssertEqual(model.selectedItem, .general)
    }

    @MainActor
    func testSelectionSwitchesToCodexAgent() {
        let model = SettingsPaneModel(session: RelayAppModel())

        model.selectItem(.agent(.codex))
        XCTAssertEqual(model.selectedItem, .agent(.codex))
    }

    @MainActor
    func testSelectingCurrentItemKeepsSelectionStable() {
        let model = SettingsPaneModel(session: RelayAppModel())

        model.selectItem(.agent(.codex))
        model.selectItem(.agent(.codex))

        XCTAssertEqual(model.selectedItem, .agent(.codex))
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
