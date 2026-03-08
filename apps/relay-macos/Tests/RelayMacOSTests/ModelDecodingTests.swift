import Foundation
import XCTest
@testable import RelayMacOSUI

final class ModelDecodingTests: XCTestCase {
    func testStatusReportDecodesLegacyHomeKeyAndActiveProfileID() throws {
        let json = """
        {
          "relay_home": "/tmp/relay",
          "live_codex_home": "/Users/test/.codex",
          "profile_count": 2,
          "active_state": {
            "active_profile_id": "p_active",
            "last_switch_at": "2026-03-08T12:27:12Z",
            "last_switch_result": "Success",
            "auto_switch_enabled": true,
            "last_error": null
          },
          "settings": {
            "auto_switch_enabled": true,
            "cooldown_seconds": 600,
            "usage_source_mode": "Auto",
            "menu_open_refresh_stale_after_seconds": 10,
            "usage_background_refresh_enabled": true,
            "usage_background_refresh_interval_seconds": 120
          }
        }
        """.data(using: .utf8)!

        let report = try JSONDecoder.relayDecoder.decode(StatusReport.self, from: json)

        XCTAssertEqual(report.liveAgentHome, "/Users/test/.codex")
        XCTAssertEqual(report.activeState.activeProfileID, "p_active")
        XCTAssertEqual(report.activeState.lastSwitchResult, .success)
        XCTAssertTrue(report.activeState.autoSwitchEnabled)
        XCTAssertEqual(report.settings.usageSourceMode, .auto)
    }

    func testProfileDecodesLegacyCodexHomeKey() throws {
        let json = """
        {
          "id": "p_1",
          "nickname": "work",
          "agent": "Codex",
          "priority": 100,
          "enabled": true,
          "codex_home": "/Users/test/.relay/profiles/work",
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

        XCTAssertEqual(report.profileID, "p_target")
        XCTAssertEqual(report.previousProfileID, "p_prev")
        XCTAssertEqual(report.checkpointID, "cp_1")
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
          "message": "codex app-server rate limit RPC"
        }
        """.data(using: .utf8)!

        let snapshot = try JSONDecoder.relayDecoder.decode(UsageSnapshot.self, from: json)

        XCTAssertEqual(snapshot.profileID, "p_usage")
        XCTAssertEqual(snapshot.source, .local)
        XCTAssertEqual(snapshot.confidence, .high)
    }

    func testAppSettingsDecodeNewUsageFieldsWithDefaults() throws {
        let json = """
        {
          "auto_switch_enabled": false,
          "cooldown_seconds": 600
        }
        """.data(using: .utf8)!

        let settings = try JSONDecoder.relayDecoder.decode(AppSettings.self, from: json)

        XCTAssertEqual(settings.usageSourceMode, .auto)
        XCTAssertEqual(settings.menuOpenRefreshStaleAfterSeconds, 10)
        XCTAssertTrue(settings.usageBackgroundRefreshEnabled)
        XCTAssertEqual(settings.usageBackgroundRefreshIntervalSeconds, 120)
    }

    func testCodexLinkResultDecodesProbeIdentity() throws {
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
            "account_id": "acct-123",
            "email": "browser@example.com",
            "plan_hint": "team"
          },
          "activated": false
        }
        """.data(using: .utf8)!

        let result = try JSONDecoder.relayDecoder.decode(CodexLinkResult.self, from: json)

        XCTAssertEqual(result.profile.id, "p_browser")
        XCTAssertEqual(result.probeIdentity.accountID, "acct-123")
        XCTAssertFalse(result.activated)
    }
}
