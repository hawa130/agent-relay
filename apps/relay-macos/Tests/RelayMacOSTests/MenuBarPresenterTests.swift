import Foundation
@testable import AgentRelayUI
import XCTest

@MainActor
final class MenuBarPresenterTests: XCTestCase {
    func testCurrentCardNotesCarryStructuredSeverity() {
        let presenter = MenuBarPresenter(session: RelayAppModel())
        let usage = UsageSnapshot(
            profileId: "p_1",
            profileName: "work",
            source: .webEnhanced,
            confidence: .medium,
            stale: true,
            lastRefreshedAt: Date(),
            nextResetAt: nil,
            session: UsageWindow(
                usedPercent: nil,
                windowMinutes: 300,
                resetAt: nil,
                status: .unknown,
                exact: false),
            weekly: UsageWindow(
                usedPercent: nil,
                windowMinutes: 10080,
                resetAt: nil,
                status: .unknown,
                exact: false),
            autoSwitchReason: nil,
            canAutoSwitch: false,
            message: "Usage may be outdated. Codex connection failed: failed to fetch codex rate limits: GET https://chatgpt.com/backend-api/wham/usage failed: 402 Payment Required",
            remoteError: UsageRemoteError(kind: .other, httpStatus: 402))

        XCTAssertEqual(
            presenter.currentCardNotes(usage: usage),
            [
                UsageCardNote(
                    text: "Usage may be outdated. Codex connection failed: failed to fetch codex rate limits: GET https://chatgpt.com/backend-api/wham/usage failed: 402 Payment Required",
                    severity: .warning)
            ])
    }

    func testProfileSymbolUsesTriangleForStructuredRemoteError() {
        let presenter = MenuBarPresenter(session: RelayAppModel())
        let profile = Profile(
            id: "p_1",
            nickname: "work",
            agent: .codex,
            priority: 100,
            enabled: true,
            agentHome: nil,
            configPath: nil,
            authMode: .configFilesystem,
            createdAt: Date(),
            updatedAt: Date())
        let usage = UsageSnapshot(
            profileId: "p_1",
            profileName: "work",
            source: .webEnhanced,
            confidence: .medium,
            stale: true,
            lastRefreshedAt: Date(),
            nextResetAt: nil,
            session: UsageWindow(
                usedPercent: nil,
                windowMinutes: 300,
                resetAt: nil,
                status: .unknown,
                exact: false),
            weekly: UsageWindow(
                usedPercent: nil,
                windowMinutes: 10080,
                resetAt: nil,
                status: .unknown,
                exact: false),
            autoSwitchReason: nil,
            canAutoSwitch: false,
            message: "Usage may be outdated. Codex connection failed: timeout",
            remoteError: UsageRemoteError(kind: .network, httpStatus: nil))

        XCTAssertEqual(
            presenter.profileSymbolName(profile: profile, usage: usage, isActive: false),
            "exclamationmark.triangle.fill")
        XCTAssertEqual(
            presenter.profileStatusSeverity(profile: profile, usage: usage, isActive: false),
            .warning)
    }
}
