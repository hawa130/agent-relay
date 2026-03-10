import Foundation
import XCTest
@testable import RelayMacOSUI

final class RelayCLIClientTests: XCTestCase {
    func testFetchCurrentUsageAndProfileListUseNewCommands() async throws {
        let fixture = try RelayCLIFixture.make()
        defer { fixture.cleanup() }

        let client = RelayCLIClient(relayCLIPathOverride: fixture.scriptPath, environment: [:])
        let current = try await client.fetchCurrentUsage()
        let profileList = try await client.fetchProfileList()

        XCTAssertEqual(current.profileId, "p_active")
        XCTAssertEqual(current.source, .local)
        XCTAssertEqual(profileList.count, 2)
        XCTAssertEqual(profileList[1].profile.id, "p_alt")
        XCTAssertEqual(profileList[1].usageSummary?.profileId, "p_alt")
    }

    func testFetchStatusUsesJSONAndDecodesCurrentFields() async throws {
        let fixture = try RelayCLIFixture.make()
        defer { fixture.cleanup() }

        let client = RelayCLIClient(relayCLIPathOverride: fixture.scriptPath, environment: [:])
        let status = try await client.fetchStatus()

        XCTAssertEqual(status.liveAgentHome, "/Users/test/.codex")
        XCTAssertEqual(status.activeState.activeProfileId, "p_active")
        XCTAssertEqual(status.profileCount, 1)
    }

    func testRefreshUsageAndCodexSettingsUseJSONCommands() async throws {
        let fixture = try RelayCLIFixture.make()
        defer { fixture.cleanup() }

        let client = RelayCLIClient(relayCLIPathOverride: fixture.scriptPath, environment: [:])
        let usage = try await client.refreshUsage(profileId: "p_alt")
        let settings = try await client.setCodexSettings(
            CodexSettingsDraft(sourceMode: .webEnhanced)
        )

        XCTAssertEqual(usage.profileId, "p_alt")
        XCTAssertEqual(usage.source, .local)
        XCTAssertEqual(settings.usageSourceMode, .webEnhanced)
    }

    func testImportProfileCommandUsesAgentSpecificJSON() async throws {
        let fixture = try RelayCLIFixture.make()
        defer { fixture.cleanup() }

        let client = RelayCLIClient(relayCLIPathOverride: fixture.scriptPath, environment: [:])
        let profile = try await client.importProfile(agent: .codex, nickname: "live", priority: 100)
        let payloadData = try Data(contentsOf: URL(fileURLWithPath: fixture.payloadPath))
        let payload = try XCTUnwrap(
            JSONSerialization.jsonObject(with: payloadData) as? [String: Any]
        )

        XCTAssertEqual(profile.id, "p_live")
        XCTAssertEqual(payload["nickname"] as? String, "live")
        XCTAssertEqual(payload["priority"] as? Int, 100)
    }

    func testLoginProfileCommandUsesAgentSpecificJSON() async throws {
        let fixture = try RelayCLIFixture.make()
        defer { fixture.cleanup() }

        let client = RelayCLIClient(relayCLIPathOverride: fixture.scriptPath, environment: [:])
        let link = try await client.loginProfile(agent: .codex, nickname: "browser", priority: 90)
        let payloadData = try Data(contentsOf: URL(fileURLWithPath: fixture.payloadPath))
        let payload = try XCTUnwrap(
            JSONSerialization.jsonObject(with: payloadData) as? [String: Any]
        )

        XCTAssertEqual(link.profile.id, "p_browser")
        XCTAssertEqual(link.probeIdentity.accountId, "acct-123")
        XCTAssertFalse(link.activated)
        XCTAssertEqual(payload["nickname"] as? String, "browser")
        XCTAssertEqual(payload["priority"] as? Int, 90)
    }

    func testLoginProfileCancellationStopsRunningRelayProcess() async throws {
        let fixture = try RelayCLIFixture.make()
        defer { fixture.cleanup() }

        let client = RelayCLIClient(
            relayCLIPathOverride: fixture.scriptPath,
            environment: ["RELAY_FIXTURE_MODE": "slow_login"]
        )

        let task = Task {
            try await client.loginProfile(agent: .codex, nickname: "browser", priority: 90)
        }

        try await Task.sleep(nanoseconds: 200_000_000)
        task.cancel()

        do {
            _ = try await task.value
            XCTFail("expected login task to be cancelled")
        } catch is CancellationError {
        } catch {
            XCTFail("expected CancellationError, got \(error)")
        }

        XCTAssertTrue(try fixture.waitForCancelledLoginSignal())
    }

    func testSwitchToNextProfileUsesDedicatedCommand() async throws {
        let fixture = try RelayCLIFixture.make()
        defer { fixture.cleanup() }

        let client = RelayCLIClient(relayCLIPathOverride: fixture.scriptPath, environment: [:])
        let report = try await client.switchToNextProfile()

        XCTAssertEqual(report.profileId, "p_next")
        XCTAssertEqual(report.previousProfileId, "p_active")
    }
}

private struct RelayCLIFixture {
    let root: URL
    let scriptPath: String
    let payloadPath: String
    let cancelledPath: String

    static func make() throws -> Self {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(
            "relay-cli-client-tests-\(UUID().uuidString)",
            isDirectory: true
        )
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        let scriptURL = root.appendingPathComponent("relay-fixture.sh")
        try fixtureScript.data(using: .utf8)!.write(to: scriptURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: scriptURL.path)
        return Self(
            root: root,
            scriptPath: scriptURL.path,
            payloadPath: root.appendingPathComponent("last-input.json").path,
            cancelledPath: root.appendingPathComponent("cancelled.log").path
        )
    }

    func cleanup() {
        try? FileManager.default.removeItem(at: root)
    }

    func waitForCancelledLoginSignal(timeout: TimeInterval = 2.0) throws -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        let cancelledURL = URL(fileURLWithPath: cancelledPath)

        while Date() < deadline {
            if FileManager.default.fileExists(atPath: cancelledURL.path) {
                return true
            }
            Thread.sleep(forTimeInterval: 0.05)
        }

        return false
    }
}

private let fixtureScript = """
#!/bin/sh
set -eu

cmd="$*"
script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"

case "$cmd" in
  "--json status")
    cat <<'EOF'
{"success":true,"error_code":null,"message":"status loaded","data":{"relay_home":"/tmp/relay","live_agent_home":"/Users/test/.codex","profile_count":1,"active_state":{"active_profile_id":"p_active","last_switch_at":"2026-03-08T12:27:12Z","last_switch_result":"Success","auto_switch_enabled":false,"last_error":null},"settings":{"auto_switch_enabled":false,"cooldown_seconds":600}}}
EOF
    ;;
  "--json show")
    cat <<'EOF'
{"success":true,"error_code":null,"message":"profile detail loaded","data":{"profile":{"id":"p_active","nickname":"active","agent":"Codex","priority":100,"enabled":true,"agent_home":"/tmp/active-home","config_path":"/tmp/active-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"},"is_active":true,"usage":{"profile_id":"p_active","profile_name":"active","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":18.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":22.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"},"last_failure_event":null,"switch_eligible":true,"switch_ineligibility_reason":null}}
EOF
    ;;
  "--json list")
    cat <<'EOF'
{"success":true,"error_code":null,"message":"profiles loaded","data":[{"profile":{"id":"p_active","nickname":"active","agent":"Codex","priority":100,"enabled":true,"agent_home":"/tmp/active-home","config_path":"/tmp/active-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"},"is_active":true,"usage_summary":{"profile_id":"p_active","profile_name":"active","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":18.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":22.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}},{"profile":{"id":"p_alt","nickname":"alt","agent":"Codex","priority":90,"enabled":true,"agent_home":"/tmp/alt-home","config_path":"/tmp/alt-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"},"is_active":false,"usage_summary":{"profile_id":"p_alt","profile_name":"alt","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":29.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":31.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}}]}
EOF
    ;;
  "--json refresh --input-json -")
    payload="$(cat)"
    printf '%s' "$payload" > "$script_dir/last-input.json"
    cat <<'EOF'
{"success":true,"error_code":null,"message":"usage refreshed","data":{"profile_id":"p_alt","profile_name":"alt","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":29.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":31.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}} 
EOF
    ;;
  "--json codex settings")
    cat <<'EOF'
{"success":true,"error_code":null,"message":"codex settings loaded","data":{"usage_source_mode":"Auto"}}
EOF
    ;;
  "--json codex settings set --input-json -")
    payload="$(cat)"
    printf '%s' "$payload" > "$script_dir/last-input.json"
    cat <<'EOF'
{"success":true,"error_code":null,"message":"codex settings updated","data":{"usage_source_mode":"WebEnhanced"}}
EOF
    ;;
  "--json codex import --input-json -")
    payload="$(cat)"
    printf '%s' "$payload" > "$script_dir/last-input.json"
    cat <<'EOF'
{"success":true,"error_code":null,"message":"profile imported","data":{"id":"p_live","nickname":"live","agent":"Codex","priority":100,"enabled":true,"agent_home":"/tmp/live-home","config_path":"/tmp/live-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"}}
EOF
    ;;
  "--json codex login --input-json -")
    payload="$(cat)"
    printf '%s' "$payload" > "$script_dir/last-input.json"
    case "${RELAY_FIXTURE_MODE:-default}" in
      "slow_login")
        trap 'printf cancelled > "$script_dir/cancelled.log"; exit 0' TERM INT
        while :; do
          sleep 1
        done
        ;;
      *)
        cat <<'EOF'
{"success":true,"error_code":null,"message":"codex login profile created","data":{"profile":{"id":"p_browser","nickname":"browser","agent":"Codex","priority":90,"enabled":true,"agent_home":"/tmp/browser-home","config_path":"/tmp/browser-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"},"probe_identity":{"profile_id":"p_browser","provider":"CodexOfficial","principal_id":"acct-123","display_name":"browser@example.com","credentials":{"account_id":"acct-123","access_token":"access-token"},"metadata":{"email":"browser@example.com","plan_hint":"team"}},"activated":false}}
EOF
        ;;
    esac
    ;;
  "--json switch")
    cat <<'EOF'
{"success":true,"error_code":null,"message":"switched to next profile","data":{"profile_id":"p_next","previous_profile_id":"p_active","checkpoint_id":"cp-123","rollback_performed":false,"switched_at":"2026-03-08T12:27:12Z","message":"switched to next profile"}}
EOF
    ;;
  *)
    cat <<EOF
{"success":false,"error_code":"BAD_COMMAND","message":"unexpected command: $cmd","data":null}
EOF
    ;;
esac
"""
