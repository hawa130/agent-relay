import XCTest
@testable import RelayMacOSUI

final class ProfileListRowStatusIndicatorTests: XCTestCase {
    private func profile(
        accountState: ProfileAccountState = .healthy,
        accountErrorHTTPStatus: Int? = nil
    ) -> Profile {
        Profile(
            id: "p_1",
            nickname: "work",
            agent: .codex,
            priority: 100,
            enabled: true,
            accountState: accountState,
            accountErrorHTTPStatus: accountErrorHTTPStatus,
            agentHome: nil,
            configPath: nil,
            authMode: .configFilesystem,
            createdAt: Date(),
            updatedAt: Date()
        )
    }

    func testLoadingIndicatorTakesPrecedenceOverWarning() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            profile: profile(),
            isFetchingUsage: true,
            usage: nil,
            usageRefreshError: "remote usage timed out",
            isStale: true
        )

        XCTAssertEqual(indicator, .loading)
    }

    func testWarningIndicatorTakesPrecedenceOverStale() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            profile: profile(),
            isFetchingUsage: false,
            usage: nil,
            usageRefreshError: "remote usage timed out",
            isStale: true
        )

        XCTAssertEqual(indicator, .warning(message: "remote usage timed out"))
    }

    func testWarningIndicatorUsesErrorMessage() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            profile: profile(),
            isFetchingUsage: false,
            usage: nil,
            usageRefreshError: "remote usage timed out",
            isStale: false
        )

        XCTAssertEqual(indicator, .warning(message: "remote usage timed out"))
    }

    func testStaleIndicatorShowsWhenUsageIsOld() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            profile: profile(),
            isFetchingUsage: false,
            usage: nil,
            usageRefreshError: nil,
            isStale: true
        )

        XCTAssertEqual(indicator, .stale)
    }

    func testIndicatorIsNilWithoutLoadingOrError() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            profile: profile(),
            isFetchingUsage: false,
            usage: nil,
            usageRefreshError: nil,
            isStale: false
        )

        XCTAssertNil(indicator)
    }

    func testIndicatorIgnoresEmptyErrorMessage() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            profile: profile(),
            isFetchingUsage: false,
            usage: nil,
            usageRefreshError: "",
            isStale: false
        )

        XCTAssertNil(indicator)
    }

    func testWarningIndicatorUsesStructuredOtherError() {
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
            remoteError: UsageRemoteError(kind: .other, httpStatus: 402)
        )

        let indicator = ProfileListRowStatusIndicator.Kind(
            profile: profile(),
            isFetchingUsage: false,
            usage: usage,
            usageRefreshError: nil,
            isStale: true
        )

        XCTAssertEqual(
            indicator,
            .warning(message: "Usage may be outdated. Codex connection failed: failed to fetch codex rate limits: GET https://chatgpt.com/backend-api/wham/usage failed: 402 Payment Required")
        )
    }

    func testAccountUnavailableIndicatorUsesProfileState() {
        let indicator = ProfileListRowStatusIndicator.Kind(
            profile: profile(accountState: .accountUnavailable, accountErrorHTTPStatus: 402),
            isFetchingUsage: false,
            usage: nil,
            usageRefreshError: nil,
            isStale: false
        )

        XCTAssertEqual(indicator, .warning(message: "Account unavailable for auto-switch (HTTP 402)"))
    }
}
