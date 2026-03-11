import Foundation
import XCTest
@testable import RelayMacOSUI

final class UsageCardNoteResolverTests: XCTestCase {
    func testUsesUsageNoteBeforeRefreshError() {
        let usage = UsageSnapshot(
            profileId: "p_1",
            profileName: "work",
            source: .local,
            confidence: .high,
            stale: true,
            lastRefreshedAt: Date(),
            nextResetAt: nil,
            session: UsageWindow(
                usedPercent: 20,
                windowMinutes: 300,
                resetAt: nil,
                status: .healthy,
                exact: true
            ),
            weekly: UsageWindow(
                usedPercent: 30,
                windowMinutes: 10080,
                resetAt: nil,
                status: .healthy,
                exact: true
            ),
            autoSwitchReason: nil,
            canAutoSwitch: false,
            message: "Usage may be outdated. Codex connection failed: timeout",
            remoteError: UsageRemoteError(kind: .network, httpStatus: nil)
        )

        XCTAssertEqual(
            UsageCardNoteResolver.note(
                usage: usage,
                usageRefreshError: "remote usage timed out"
            ),
            UsageCardNote(
                text: "Usage may be outdated. Codex connection failed: timeout",
                severity: .warning
            )
        )
    }

    func testFallsBackToRefreshErrorWhenSnapshotHasNoNote() {
        XCTAssertEqual(
            UsageCardNoteResolver.note(
                usage: nil,
                usageRefreshError: "Codex connection failed: dns error"
            ),
            UsageCardNote(
                text: "Codex connection failed: dns error",
                severity: .warning
            )
        )
    }

    func testIgnoresEmptyRefreshError() {
        XCTAssertNil(
            UsageCardNoteResolver.note(
                usage: nil,
                usageRefreshError: "   "
            )
        )
    }

    func testNonStaleWebEnhancedNoteIsNotWarning() {
        let usage = UsageSnapshot(
            profileId: "p_1",
            profileName: "remote",
            source: .webEnhanced,
            confidence: .high,
            stale: false,
            lastRefreshedAt: Date(),
            nextResetAt: nil,
            session: UsageWindow(
                usedPercent: 20,
                windowMinutes: 300,
                resetAt: nil,
                status: .healthy,
                exact: true
            ),
            weekly: UsageWindow(
                usedPercent: 30,
                windowMinutes: 10080,
                resetAt: nil,
                status: .healthy,
                exact: true
            ),
            autoSwitchReason: nil,
            canAutoSwitch: false,
            message: "Remote usage synced",
            remoteError: nil
        )

        XCTAssertEqual(
            UsageCardNoteResolver.note(
                usage: usage,
                usageRefreshError: nil
            ),
            UsageCardNote(text: "Remote usage synced", severity: nil)
        )
    }

    func testAccountRemoteErrorUsesDangerSeverity() {
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
                exact: false
            ),
            weekly: UsageWindow(
                usedPercent: nil,
                windowMinutes: 10080,
                resetAt: nil,
                status: .unknown,
                exact: false
            ),
            autoSwitchReason: nil,
            canAutoSwitch: false,
            message: "Usage may be outdated. Codex connection failed: failed to fetch codex rate limits: GET https://chatgpt.com/backend-api/wham/usage failed: 402 Payment Required",
            remoteError: UsageRemoteError(kind: .account, httpStatus: 402)
        )

        XCTAssertEqual(
            UsageCardNoteResolver.note(
                usage: usage,
                usageRefreshError: nil
            ),
            UsageCardNote(
                text: "Usage may be outdated. Codex connection failed: failed to fetch codex rate limits: GET https://chatgpt.com/backend-api/wham/usage failed: 402 Payment Required",
                severity: .danger
            )
        )
    }
}
