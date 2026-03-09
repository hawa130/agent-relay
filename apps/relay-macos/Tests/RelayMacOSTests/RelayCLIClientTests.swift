import Foundation
import XCTest
@testable import RelayMacOSUI

final class RelayCLIClientTests: XCTestCase {
    func testFetchStatusUsesJSONAndDecodesCurrentFields() async throws {
        let fixture = try RelayCLIFixture.make()
        defer { fixture.cleanup() }

        let client = RelayCLIClient(relayCLIPathOverride: fixture.scriptPath, environment: [:])
        let status = try await client.fetchStatus()

        XCTAssertEqual(status.liveAgentHome, "/Users/test/.codex")
        XCTAssertEqual(status.activeState.activeProfileId, "p_active")
        XCTAssertEqual(status.profileCount, 1)
    }

    func testRefreshUsageAndUsageSettingsUseJSONCommands() async throws {
        let fixture = try RelayCLIFixture.make()
        defer { fixture.cleanup() }

        let client = RelayCLIClient(relayCLIPathOverride: fixture.scriptPath, environment: [:])
        let usage = try await client.refreshUsage(profileId: "p_alt")
        let settings = try await client.setUsageSettings(
            UsageSettingsDraft(
                sourceMode: .webEnhanced,
                menuOpenRefreshStaleAfterSeconds: 5,
                backgroundRefreshEnabled: false,
                backgroundRefreshIntervalSeconds: 300
            )
        )

        XCTAssertEqual(usage.profileId, "p_alt")
        XCTAssertEqual(usage.source, .local)
        XCTAssertEqual(settings.usageSourceMode, .webEnhanced)
        XCTAssertEqual(settings.menuOpenRefreshStaleAfterSeconds, 5)
        XCTAssertFalse(settings.usageBackgroundRefreshEnabled)
    }

    func testImportCodexCommandUsesAgentAwareJSONWithoutInlineArgs() async throws {
        let fixture = try RelayCLIFixture.make()
        defer { fixture.cleanup() }

        let client = RelayCLIClient(relayCLIPathOverride: fixture.scriptPath, environment: [:])
        let profile = try await client.importCodexProfile(nickname: "live", priority: 100)
        let payloadData = try Data(contentsOf: URL(fileURLWithPath: fixture.payloadPath))
        let payload = try XCTUnwrap(
            JSONSerialization.jsonObject(with: payloadData) as? [String: Any]
        )

        XCTAssertEqual(profile.id, "p_live")
        XCTAssertEqual(payload["agent"] as? String, "codex")
        XCTAssertEqual(payload["nickname"] as? String, "live")
        XCTAssertEqual(payload["priority"] as? Int, 100)
    }

    func testLoginCodexCommandUsesJSON() async throws {
        let fixture = try RelayCLIFixture.make()
        defer { fixture.cleanup() }

        let client = RelayCLIClient(relayCLIPathOverride: fixture.scriptPath, environment: [:])
        let link = try await client.loginCodexProfile(nickname: "browser", priority: 90)
        let payloadData = try Data(contentsOf: URL(fileURLWithPath: fixture.payloadPath))
        let payload = try XCTUnwrap(
            JSONSerialization.jsonObject(with: payloadData) as? [String: Any]
        )

        XCTAssertEqual(link.profile.id, "p_browser")
        XCTAssertEqual(link.probeIdentity.accountId, "acct-123")
        XCTAssertEqual(payload["agent"] as? String, "codex")
        XCTAssertEqual(payload["nickname"] as? String, "browser")
        XCTAssertEqual(payload["priority"] as? Int, 90)
    }
}

private struct RelayCLIFixture {
    let root: URL
    let scriptPath: String
    let payloadPath: String

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
            payloadPath: root.appendingPathComponent("last-input.json").path
        )
    }

    func cleanup() {
        try? FileManager.default.removeItem(at: root)
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
  "--json usage refresh --input-json -")
    payload="$(cat)"
    printf '%s' "$payload" > "$script_dir/last-input.json"
    cat <<'EOF'
{"success":true,"error_code":null,"message":"usage refreshed","data":{"profile_id":"p_alt","profile_name":"alt","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":29.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":31.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}} 
EOF
    ;;
  "--json usage config set --input-json -")
    payload="$(cat)"
    printf '%s' "$payload" > "$script_dir/last-input.json"
    cat <<'EOF'
{"success":true,"error_code":null,"message":"usage settings updated","data":{"auto_switch_enabled":false,"cooldown_seconds":600,"usage_source_mode":"WebEnhanced","menu_open_refresh_stale_after_seconds":5,"usage_background_refresh_enabled":false,"usage_background_refresh_interval_seconds":300}}
EOF
    ;;
  "--json profiles import --input-json -")
    payload="$(cat)"
    printf '%s' "$payload" > "$script_dir/last-input.json"
    cat <<'EOF'
{"success":true,"error_code":null,"message":"profile imported","data":{"id":"p_live","nickname":"live","agent":"Codex","priority":100,"enabled":true,"agent_home":"/tmp/live-home","config_path":"/tmp/live-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"}}
EOF
    ;;
  "--json profiles login --input-json -")
    payload="$(cat)"
    printf '%s' "$payload" > "$script_dir/last-input.json"
    cat <<'EOF'
{"success":true,"error_code":null,"message":"codex login profile created","data":{"profile":{"id":"p_browser","nickname":"browser","agent":"Codex","priority":90,"enabled":true,"agent_home":"/tmp/browser-home","config_path":"/tmp/browser-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"},"probe_identity":{"profile_id":"p_browser","provider":"CodexOfficial","principal_id":"acct-123","display_name":"browser@example.com","credentials":{"account_id":"acct-123","access_token":"access-token"},"metadata":{"email":"browser@example.com","plan_hint":"team"}},"activated":false}}
EOF
    ;;
  *)
    cat <<EOF
{"success":false,"error_code":"BAD_COMMAND","message":"unexpected command: $cmd","data":null}
EOF
    ;;
esac
"""
