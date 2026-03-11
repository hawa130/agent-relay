import Foundation
import XCTest
@testable import RelayMacOSUI

final class ModelDecodingTests: XCTestCase {
    func testStatusReportDecodesCurrentFields() throws {
        let json = """
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
        """.data(using: .utf8)!

        let report = try JSONDecoder.relayDecoder.decode(StatusReport.self, from: json)

        XCTAssertEqual(report.liveAgentHome, "/Users/test/.codex")
        XCTAssertEqual(report.activeState.activeProfileId, "p_active")
        XCTAssertEqual(report.activeState.lastSwitchResult, .success)
        XCTAssertTrue(report.activeState.autoSwitchEnabled)
        XCTAssertEqual(report.settings.cooldownSeconds, 600)
    }

    func testProfileDecodesCurrentAgentHomeKey() throws {
        let json = """
        {
          "id": "p_1",
          "nickname": "work",
          "agent": "Codex",
          "priority": 100,
          "enabled": true,
          "agent_home": "/Users/test/.relay/profiles/work",
          "config_path": "/Users/test/.relay/profiles/work/config.toml",
          "auth_mode": "ConfigFilesystem",
          "created_at": "2026-03-08T12:27:12Z",
          "updated_at": "2026-03-08T12:27:12Z"
        }
        """.data(using: .utf8)!

        let profile = try JSONDecoder.relayDecoder.decode(Profile.self, from: json)

        XCTAssertEqual(profile.agentHome, "/Users/test/.relay/profiles/work")
        XCTAssertEqual(profile.authMode, .configFilesystem)
    }

    func testSwitchReportDecodesSnakeCaseIDFields() throws {
        let json = """
        {
          "profile_id": "p_target",
          "previous_profile_id": "p_prev",
          "checkpoint_id": "cp_1",
          "rollback_performed": false,
          "switched_at": "2026-03-08T12:27:12Z",
          "message": "switched"
        }
        """.data(using: .utf8)!

        let report = try JSONDecoder.relayDecoder.decode(SwitchReport.self, from: json)

        XCTAssertEqual(report.profileId, "p_target")
        XCTAssertEqual(report.previousProfileId, "p_prev")
        XCTAssertEqual(report.checkpointId, "cp_1")
    }

    func testUsageSnapshotDecodesProfileID() throws {
        let json = """
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
            "kind": "Account",
            "http_status": 402
          }
        }
        """.data(using: .utf8)!

        let snapshot = try JSONDecoder.relayDecoder.decode(UsageSnapshot.self, from: json)

        XCTAssertEqual(snapshot.profileId, "p_usage")
        XCTAssertEqual(snapshot.source, .local)
        XCTAssertEqual(snapshot.confidence, .high)
        XCTAssertEqual(snapshot.remoteError, UsageRemoteError(kind: .account, httpStatus: 402))
    }

    func testAppSettingsDecodeCurrentFields() throws {
        let json = """
        {
          "auto_switch_enabled": false,
          "cooldown_seconds": 600
        }
        """.data(using: .utf8)!

        let settings = try JSONDecoder.relayDecoder.decode(AppSettings.self, from: json)

        XCTAssertEqual(settings.cooldownSeconds, 600)
        XCTAssertEqual(settings.networkQueryConcurrency, 10)
    }

    func testCodexSettingsDecodeCurrentFields() throws {
        let json = """
        {
          "usage_source_mode": "WebEnhanced"
        }
        """.data(using: .utf8)!

        let settings = try JSONDecoder.relayDecoder.decode(CodexSettings.self, from: json)

        XCTAssertEqual(settings.usageSourceMode, .webEnhanced)
    }

    func testAgentLinkResultDecodesProbeIdentity() throws {
        let json = """
        {
          "profile": {
            "id": "p_browser",
            "nickname": "browser",
            "agent": "Codex",
            "priority": 90,
            "enabled": true,
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
        """.data(using: .utf8)!

        let result = try JSONDecoder.relayDecoder.decode(AgentLinkResult.self, from: json)

        XCTAssertEqual(result.profile.id, "p_browser")
        XCTAssertEqual(result.probeIdentity.accountId, "acct-123")
        XCTAssertFalse(result.activated)
    }

    func testProfileDetailDecodesAggregateFields() throws {
        let json = """
        {
          "profile": {
            "id": "p_1",
            "nickname": "work",
            "agent": "Codex",
            "priority": 100,
            "enabled": true,
            "agent_home": "/Users/test/.relay/profiles/work",
            "config_path": "/Users/test/.relay/profiles/work/config.toml",
            "auth_mode": "ConfigFilesystem",
            "created_at": "2026-03-08T12:27:12Z",
            "updated_at": "2026-03-08T12:27:12Z"
          },
          "is_active": false,
          "usage": {
            "profile_id": "p_1",
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
            "message": "local usage"
          },
          "last_failure_event": null,
          "switch_eligible": true,
          "switch_ineligibility_reason": null
        }
        """.data(using: .utf8)!

        let detail = try JSONDecoder.relayDecoder.decode(ProfileDetail.self, from: json)

        XCTAssertEqual(detail.profile.id, "p_1")
        XCTAssertFalse(detail.isActive)
        XCTAssertEqual(detail.usage?.profileId, "p_1")
        XCTAssertTrue(detail.switchEligible)
    }

    func testProfileListItemDecodesAggregateListFields() throws {
        let json = """
        {
          "profile": {
            "id": "p_1",
            "nickname": "work",
            "agent": "Codex",
            "priority": 100,
            "enabled": true,
            "agent_home": "/Users/test/.relay/profiles/work",
            "config_path": "/Users/test/.relay/profiles/work/config.toml",
            "auth_mode": "ConfigFilesystem",
            "created_at": "2026-03-08T12:27:12Z",
            "updated_at": "2026-03-08T12:27:12Z"
          },
          "is_active": true,
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
        """.data(using: .utf8)!

        let item = try JSONDecoder.relayDecoder.decode(ProfileListItem.self, from: json)

        XCTAssertEqual(item.profile.id, "p_1")
        XCTAssertTrue(item.isActive)
        XCTAssertEqual(item.usageSummary?.profileId, "p_1")
    }

    func testUsageSnapshotUserFacingNoteRewritesInternalMessages() throws {
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
            message: "Usage is currently unavailable.",
            remoteError: nil
        )
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
            message: "Using local usage because enhanced usage is unavailable.",
            remoteError: nil
        )
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
                exact: true
            ),
            weekly: UsageWindow(
                usedPercent: 15,
                windowMinutes: 10080,
                resetAt: nil,
                status: .healthy,
                exact: true
            ),
            autoSwitchReason: nil,
            canAutoSwitch: false,
            message: nil,
            remoteError: nil
        )

        XCTAssertEqual(fallback.userFacingNote, "Usage is currently unavailable.")
        XCTAssertEqual(
            localFallback.userFacingNote,
            "Using local usage because enhanced usage is unavailable."
        )
        XCTAssertNil(official.userFacingNote)
    }
}
