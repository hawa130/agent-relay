import Foundation
@testable import AgentRelayUI
import XCTest

private func utf8Data(_ string: String) -> Data {
    Data(string.utf8)
}

final class ModelDecodingTests: XCTestCase {
    func testStatusReportDecodesCurrentFields() throws {
        let json = utf8Data(
            """
            {
              "relay_home": "/tmp/relay",
              "live_agent_home": "/Users/test/.codex",
              "profile_count": 2,
              "active_state": {
                "active_profile_id": "p_active",
                "last_switch_at": "2026-03-08T12:27:12Z",
                "last_switch_result": "Success",
                "auto_switch_enabled": true
              },
              "settings": {
                "auto_switch_enabled": true,
                "cooldown_seconds": 600
              }
            }
            """)

        let report = try JSONDecoder.relayDecoder.decode(StatusReport.self, from: json)

        XCTAssertEqual(report.liveAgentHome, "/Users/test/.codex")
        XCTAssertEqual(report.activeState.activeProfileId, "p_active")
        XCTAssertEqual(report.activeState.lastSwitchResult, .success)
        XCTAssertTrue(report.activeState.autoSwitchEnabled)
        XCTAssertEqual(report.settings.cooldownSeconds, 600)
    }

    func testProfileDecodesCurrentAgentHomeKey() throws {
        let json = utf8Data(
            """
            {
              "id": "p_1",
              "nickname": "work",
              "agent": "Codex",
              "priority": 100,
              "enabled": true,
              "account_state": "AccountUnavailable",
              "account_error_http_status": 402,
              "agent_home": "/Users/test/.relay/profiles/work",
              "config_path": "/Users/test/.relay/profiles/work/config.toml",
              "auth_mode": "ConfigFilesystem",
              "created_at": "2026-03-08T12:27:12Z",
              "updated_at": "2026-03-08T12:27:12Z"
            }
            """)

        let profile = try JSONDecoder.relayDecoder.decode(Profile.self, from: json)

        XCTAssertEqual(profile.agentHome, "/Users/test/.relay/profiles/work")
        XCTAssertEqual(profile.authMode, .configFilesystem)
        XCTAssertEqual(profile.accountState, .accountUnavailable)
        XCTAssertEqual(profile.accountErrorHTTPStatus, 402)
    }

    func testSwitchReportDecodesSnakeCaseIDFields() throws {
        let json = utf8Data(
            """
            {
              "profile_id": "p_target",
              "previous_profile_id": "p_prev",
              "checkpoint_id": "cp_1",
              "rollback_performed": false,
              "switched_at": "2026-03-08T12:27:12Z",
              "message": "switched"
            }
            """)

        let report = try JSONDecoder.relayDecoder.decode(SwitchReport.self, from: json)

        XCTAssertEqual(report.profileId, "p_target")
        XCTAssertEqual(report.previousProfileId, "p_prev")
        XCTAssertEqual(report.checkpointId, "cp_1")
    }

    func testUsageSnapshotDecodesProfileID() throws {
        let json = utf8Data(
            """
            {
              "profile_id": "p_usage",
              "profile_name": "work",
              "source": "Local",
              "confidence": "High",
              "stale": false,
              "last_refreshed_at": "2026-03-08T12:27:12Z",
              "next_reset_at": "2026-03-08T17:06:00Z",
              "session": {
                "used_percent": 29.0,
                "window_minutes": 300,
                "reset_at": "2026-03-08T17:06:00Z",
                "status": "Healthy",
                "exact": true
              },
              "weekly": {
                "used_percent": 31.0,
                "window_minutes": 10080,
                "reset_at": "2026-03-12T06:36:18Z",
                "status": "Healthy",
                "exact": true
              },
              "auto_switch_reason": null,
              "can_auto_switch": false,
              "message": "codex app-server rate limit RPC",
              "remote_error": {
                "kind": "Other",
                "http_status": 402
              }
            }
            """)

        let snapshot = try JSONDecoder.relayDecoder.decode(UsageSnapshot.self, from: json)

        XCTAssertEqual(snapshot.profileId, "p_usage")
        XCTAssertEqual(snapshot.source, .local)
        XCTAssertEqual(snapshot.confidence, .high)
        XCTAssertEqual(snapshot.remoteError, UsageRemoteError(kind: .other, httpStatus: 402))
    }

    func testAppSettingsDecodeCurrentFields() throws {
        let json = utf8Data(
            """
            {
              "auto_switch_enabled": false,
              "cooldown_seconds": 600
            }
            """)

        let settings = try JSONDecoder.relayDecoder.decode(AppSettings.self, from: json)

        XCTAssertEqual(settings.cooldownSeconds, 600)
        XCTAssertEqual(settings.networkQueryConcurrency, 10)
    }

    func testCodexSettingsDecodeCurrentFields() throws {
        let json = utf8Data(
            """
            {
              "usage_source_mode": "WebEnhanced"
            }
            """)

        let settings = try JSONDecoder.relayDecoder.decode(CodexSettings.self, from: json)

        XCTAssertEqual(settings.usageSourceMode, .webEnhanced)
    }

    func testAgentLinkResultDecodesProbeIdentity() throws {
        let json = utf8Data(
            """
            {
              "profile": {
                "id": "p_browser",
                "nickname": "browser",
                "agent": "Codex",
                "priority": 90,
                "enabled": true,
                "account_state": "Healthy",
                "account_error_http_status": null,
                "account_state_updated_at": null,
                "agent_home": "/tmp/browser-home",
                "config_path": "/tmp/browser-home/config.toml",
                "auth_mode": "ConfigFilesystem",
                "created_at": "2026-03-08T12:27:12Z",
                "updated_at": "2026-03-08T12:27:12Z"
              },
              "probe_identity": {
                "profile_id": "p_browser",
                "provider": "CodexOfficial",
                "principal_id": "acct-123",
                "display_name": "browser@example.com",
                "credentials": {
                  "account_id": "acct-123",
                  "access_token": "access-token"
                },
                "metadata": {
                  "email": "browser@example.com",
                  "plan_hint": "team"
                }
              },
              "activated": false
            }
            """)

        let result = try JSONDecoder.relayDecoder.decode(AgentLinkResult.self, from: json)

        XCTAssertEqual(result.profile.id, "p_browser")
        XCTAssertEqual(result.probeIdentity.accountId, "acct-123")
        XCTAssertFalse(result.activated)
    }

    func testProfileListItemDecodesAggregateListFields() throws {
        let json = utf8Data(
            """
            {
              "profile": {
                "id": "p_1",
                "nickname": "work",
                "agent": "Codex",
                "priority": 100,
                "enabled": true,
                "account_state": "Healthy",
                "account_error_http_status": null,
                "account_state_updated_at": null,
                "agent_home": "/Users/test/.relay/profiles/work",
                "config_path": "/Users/test/.relay/profiles/work/config.toml",
                "auth_mode": "ConfigFilesystem",
                "created_at": "2026-03-08T12:27:12Z",
                "updated_at": "2026-03-08T12:27:12Z"
              },
              "is_active": true,
              "current_failure_events": [
                {
                  "id": "ev_1",
                  "profile_id": "p_1",
                  "reason": "ValidationFailed",
                  "message": "still broken",
                  "cooldown_until": null,
                  "created_at": "2026-03-08T12:27:12Z",
                  "resolved_at": null
                }
              ],
              "usage_summary": {
                "profile_id": "p_1",
                "profile_name": "work",
                "source": "Local",
                "confidence": "High",
                "stale": false,
                "last_refreshed_at": "2026-03-08T12:27:12Z",
                "next_reset_at": "2026-03-08T17:06:00Z",
                "session": {
                  "used_percent": 18.0,
                  "window_minutes": 300,
                  "reset_at": "2026-03-08T17:06:00Z",
                  "status": "Healthy",
                  "exact": true
                },
                "weekly": {
                  "used_percent": 22.0,
                  "window_minutes": 10080,
                  "reset_at": "2026-03-12T06:36:18Z",
                  "status": "Healthy",
                  "exact": true
                },
                "auto_switch_reason": null,
                "can_auto_switch": false,
                "message": "local usage"
              }
            }
            """)

        let item = try JSONDecoder.relayDecoder.decode(ProfileListItem.self, from: json)

        XCTAssertEqual(item.profile.id, "p_1")
        XCTAssertTrue(item.isActive)
        XCTAssertEqual(item.usageSummary?.profileId, "p_1")
        XCTAssertEqual(item.currentFailureEvents.count, 1)
        XCTAssertNil(item.currentFailureEvents[0].resolvedAt)
    }

    func testProfileListItemDecodesAccountUsageAndAccountStateFields() throws {
        let json = utf8Data(
            """
            {
              "profile": {
                "id": "p_2",
                "nickname": "suspended",
                "agent": "Codex",
                "priority": 90,
                "enabled": false,
                "account_state": "AccountUnavailable",
                "account_error_http_status": 401,
                "account_state_updated_at": "2026-03-12T10:00:00Z",
                "agent_home": "/Users/test/.relay/profiles/suspended",
                "config_path": "/Users/test/.relay/profiles/suspended/config.toml",
                "auth_mode": "ConfigFilesystem",
                "created_at": "2026-03-08T12:27:12Z",
                "updated_at": "2026-03-12T10:00:00Z"
              },
              "is_active": false,
              "current_failure_events": [
                {
                  "id": "ev_2",
                  "profile_id": "p_2",
                  "reason": "AccountUnavailable",
                  "message": "account unavailable",
                  "cooldown_until": null,
                  "created_at": "2026-03-12T10:00:00Z",
                  "resolved_at": null
                }
              ],
              "usage_summary": {
                "profile_id": "p_2",
                "profile_name": "suspended",
                "source": "Fallback",
                "confidence": "Low",
                "stale": true,
                "last_refreshed_at": "2026-03-12T10:00:00Z",
                "next_reset_at": null,
                "session": {
                  "used_percent": null,
                  "window_minutes": 300,
                  "reset_at": null,
                  "status": "Unknown",
                  "exact": false
                },
                "weekly": {
                  "used_percent": null,
                  "window_minutes": 10080,
                  "reset_at": null,
                  "status": "Unknown",
                  "exact": false
                },
                "auto_switch_reason": "AccountUnavailable",
                "can_auto_switch": false,
                "message": "account unavailable",
                "remote_error": {
                  "kind": "Account",
                  "http_status": 401
                }
              }
            }
            """)

        let item = try JSONDecoder.relayDecoder.decode(ProfileListItem.self, from: json)

        XCTAssertEqual(item.profile.accountState, .accountUnavailable)
        XCTAssertEqual(item.profile.accountErrorHTTPStatus, 401)
        XCTAssertEqual(item.usageSummary?.remoteError?.kind, .account)
        XCTAssertEqual(item.usageSummary?.autoSwitchReason, .accountUnavailable)
        XCTAssertEqual(item.currentFailureEvents.first?.reason, .accountUnavailable)
    }

    func testUsageSnapshotUserFacingNoteRewritesInternalMessages() {
        let fallback = UsageSnapshot(
            profileId: "p_1",
            profileName: "work",
            source: .fallback,
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
            message: "Usage is currently unavailable.",
            remoteError: nil)
        let localFallback = UsageSnapshot(
            profileId: "p_2",
            profileName: "local",
            source: .local,
            confidence: .high,
            stale: false,
            lastRefreshedAt: Date(),
            nextResetAt: nil,
            session: UsageWindow(
                usedPercent: 20,
                windowMinutes: 300,
                resetAt: nil,
                status: .healthy,
                exact: true),
            weekly: UsageWindow(
                usedPercent: 30,
                windowMinutes: 10080,
                resetAt: nil,
                status: .healthy,
                exact: true),
            autoSwitchReason: nil,
            canAutoSwitch: false,
            message: "Using local usage because enhanced usage is unavailable.",
            remoteError: nil)
        let official = UsageSnapshot(
            profileId: "p_3",
            profileName: "remote",
            source: .webEnhanced,
            confidence: .high,
            stale: false,
            lastRefreshedAt: Date(),
            nextResetAt: nil,
            session: UsageWindow(
                usedPercent: 10,
                windowMinutes: 300,
                resetAt: nil,
                status: .healthy,
                exact: true),
            weekly: UsageWindow(
                usedPercent: 15,
                windowMinutes: 10080,
                resetAt: nil,
                status: .healthy,
                exact: true),
            autoSwitchReason: nil,
            canAutoSwitch: false,
            message: nil,
            remoteError: nil)

        XCTAssertEqual(fallback.userFacingNote, "Usage is currently unavailable.")
        XCTAssertEqual(
            localFallback.userFacingNote,
            "Using local usage because enhanced usage is unavailable.")
        XCTAssertNil(official.userFacingNote)
    }

    func testFailureReasonDisplayNameFormatsUserFacingText() {
        XCTAssertEqual(FailureReason.accountUnavailable.displayName, "Account Unavailable")
        XCTAssertEqual(FailureReason.authInvalid.displayName, "Authentication Invalid")
        XCTAssertEqual(FailureReason.rateLimited.displayName, "Rate Limited")
    }
}
