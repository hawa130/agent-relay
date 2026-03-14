@testable import AgentRelayUI
import XCTest

final class ProfileRowContextMenuTests: XCTestCase {
    private func profile(enabled: Bool = true) -> Profile {
        Profile(
            id: "p_1",
            nickname: "work",
            agent: .codex,
            priority: 100,
            enabled: enabled,
            accountState: .healthy,
            accountErrorHTTPStatus: nil,
            agentHome: nil,
            configPath: nil,
            authMode: .configFilesystem,
            createdAt: Date(),
            updatedAt: Date())
    }

    func testMenuUsesDisableActionForEnabledProfile() {
        let model = ProfileRowContextMenuModel(
            profile: profile(enabled: true),
            isActive: false,
            isMutatingProfiles: false,
            isSwitching: false)

        XCTAssertEqual(model.toggleEnabledTitle, "Disable")
        XCTAssertTrue(model.canToggleEnabled)
    }

    func testMenuUsesEnableActionForDisabledProfile() {
        let model = ProfileRowContextMenuModel(
            profile: profile(enabled: false),
            isActive: false,
            isMutatingProfiles: false,
            isSwitching: false)

        XCTAssertEqual(model.toggleEnabledTitle, "Enable")
        XCTAssertTrue(model.canToggleEnabled)
        XCTAssertFalse(model.canMakeCurrent)
    }

    func testCurrentActionDisabledForActiveProfile() {
        let model = ProfileRowContextMenuModel(
            profile: profile(enabled: true),
            isActive: true,
            isMutatingProfiles: false,
            isSwitching: false)

        XCTAssertFalse(model.canMakeCurrent)
    }

    func testAllMutatingActionsDisabledWhileMutatingProfiles() {
        let model = ProfileRowContextMenuModel(
            profile: profile(enabled: true),
            isActive: false,
            isMutatingProfiles: true,
            isSwitching: false)

        XCTAssertFalse(model.canEdit)
        XCTAssertFalse(model.canToggleEnabled)
        XCTAssertFalse(model.canDelete)
        XCTAssertFalse(model.canMakeCurrent)
    }

    func testCurrentActionDisabledWhileSwitching() {
        let model = ProfileRowContextMenuModel(
            profile: profile(enabled: true),
            isActive: false,
            isMutatingProfiles: false,
            isSwitching: true)

        XCTAssertFalse(model.canMakeCurrent)
    }
}
