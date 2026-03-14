@testable import AgentRelayUI
import XCTest

final class AddProfileCatalogProfileCountTextTests: XCTestCase {
    func testFormatsSingularProfileCount() {
        let descriptor = AgentSettingsDescriptor(
            agent: .codex,
            title: "Codex",
            vendorTitle: "OpenAI",
            subtitle: "",
            iconResourceName: "ProviderIcon-codex",
            accentColor: .secondary,
            visualScale: 1)

        XCTAssertEqual(
            AddProfileCatalogProfileText.text(for: descriptor, profileCount: 1),
            "OpenAI • 1 profile")
    }

    func testFormatsPluralProfileCount() {
        let descriptor = AgentSettingsDescriptor(
            agent: .codex,
            title: "Codex",
            vendorTitle: "OpenAI",
            subtitle: "",
            iconResourceName: "ProviderIcon-codex",
            accentColor: .secondary,
            visualScale: 1)

        XCTAssertEqual(
            AddProfileCatalogProfileText.text(for: descriptor, profileCount: 3),
            "OpenAI • 3 profiles")
    }
}
