import Foundation
import XCTest
@testable import RelayMacOSUI

@MainActor
final class RelayAppModelDaemonNotificationTests: XCTestCase {
    func testStartAppliesHealthUpdateNotification() async throws {
        let fixture = try RelayAppModelNotificationFixture.make(mode: "health_update")
        defer { fixture.cleanup() }

        let scope = RelayFixtureEnvironment(
            relayCLIPath: fixture.scriptPath,
            fixtureMode: "health_update"
        )
        scope.install()
        defer { scope.uninstall() }

        let model = RelayAppModel()
        model.start()

        try await waitUntil {
            model.engineConnectionState == .degraded
                && model.lastErrorMessage == "Relay engine degraded for test."
        }
    }

    func testStartAppliesSwitchFailedNotification() async throws {
        let fixture = try RelayAppModelNotificationFixture.make(mode: "switch_failed")
        defer { fixture.cleanup() }

        let scope = RelayFixtureEnvironment(
            relayCLIPath: fixture.scriptPath,
            fixtureMode: "switch_failed"
        )
        scope.install()
        defer { scope.uninstall() }

        let model = RelayAppModel()
        model.start()

        try await waitUntil {
            model.lastErrorMessage == "RELAY_CONFLICT: no eligible profile available"
        }
    }

    func testStartAppliesUsageAndActiveStateNotifications() async throws {
        let fixture = try RelayAppModelNotificationFixture.make(mode: "usage_active_update")
        defer { fixture.cleanup() }

        let scope = RelayFixtureEnvironment(
            relayCLIPath: fixture.scriptPath,
            fixtureMode: "usage_active_update"
        )
        scope.install()
        defer { scope.uninstall() }

        let model = RelayAppModel()
        model.start()

        try await waitUntil {
            model.activeProfileId == "p_alt"
                && model.usage?.profileId == "p_alt"
                && model.usageSnapshot(for: "p_alt")?.source == .webEnhanced
        }
    }
}

private struct RelayFixtureEnvironment {
    let relayCLIPath: String
    let fixtureMode: String?
    let originalRelayCLIPath = getenv("RELAY_CLI_PATH").map { String(cString: $0) }
    let originalFixtureMode = getenv("RELAY_FIXTURE_MODE").map { String(cString: $0) }

    func install() {
        setenv("RELAY_CLI_PATH", relayCLIPath, 1)
        if let fixtureMode {
            setenv("RELAY_FIXTURE_MODE", fixtureMode, 1)
        } else {
            unsetenv("RELAY_FIXTURE_MODE")
        }
    }

    func uninstall() {
        if let originalRelayCLIPath {
            setenv("RELAY_CLI_PATH", originalRelayCLIPath, 1)
        } else {
            unsetenv("RELAY_CLI_PATH")
        }
        if let originalFixtureMode {
            setenv("RELAY_FIXTURE_MODE", originalFixtureMode, 1)
        } else {
            unsetenv("RELAY_FIXTURE_MODE")
        }
    }
}

private struct RelayAppModelNotificationFixture {
    let root: URL
    let scriptPath: String

    static func make(mode _: String) throws -> Self {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(
            "relay-app-model-notification-tests-\(UUID().uuidString)",
            isDirectory: true
        )
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)

        let scriptURL = root.appendingPathComponent("relay-fixture.sh")
        try fixtureScript.data(using: .utf8)!.write(to: scriptURL)
        try FileManager.default.setAttributes(
            [.posixPermissions: 0o755],
            ofItemAtPath: scriptURL.path
        )

        return Self(root: root, scriptPath: scriptURL.path)
    }

    func cleanup() {
        try? FileManager.default.removeItem(at: root)
    }
}

private enum WaitTimeout: Error {
    case timedOut
}

@MainActor
private func waitUntil(
    timeoutNanoseconds: UInt64 = 3_000_000_000,
    condition: @escaping @MainActor () -> Bool
) async throws {
    let deadline = DispatchTime.now().uptimeNanoseconds + timeoutNanoseconds
    while DispatchTime.now().uptimeNanoseconds < deadline {
        if condition() {
            return
        }
        try await Task.sleep(nanoseconds: 50_000_000)
    }
    throw WaitTimeout.timedOut
}

private let fixtureScript = """
#!/bin/sh
set -eu

mode="${RELAY_FIXTURE_MODE:-default}"

active_profile_item='{"profile":{"id":"p_active","nickname":"active","agent":"Codex","priority":100,"enabled":true,"agent_home":"/tmp/active-home","config_path":"/tmp/active-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"},"is_active":true,"usage_summary":{"profile_id":"p_active","profile_name":"active","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":18.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":22.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}}'
alt_profile_item='{"profile":{"id":"p_alt","nickname":"alt","agent":"Codex","priority":90,"enabled":true,"agent_home":"/tmp/alt-home","config_path":"/tmp/alt-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"},"is_active":false,"usage_summary":{"profile_id":"p_alt","profile_name":"alt","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":29.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":31.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}}'

case "$*" in
  "daemon --stdio")
    while IFS= read -r line; do
      method="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get("method", ""))' "$line")"
      id="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get("id", ""))' "$line")"
      case "$method" in
        initialize)
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"protocol_version":"1","server_info":{"name":"relay","version":"0.1.0"},"capabilities":{"supports_subscriptions":true,"supports_health_updates":true},"initial_state":{"status":{"relay_home":"/tmp/relay","live_agent_home":"/Users/test/.codex","profile_count":2,"active_state":{"active_profile_id":"p_active","last_switch_at":"2026-03-08T12:27:12Z","last_switch_result":"Success","auto_switch_enabled":false,"last_error":null},"settings":{"auto_switch_enabled":false,"cooldown_seconds":600,"refresh_interval_seconds":60}},"profiles":[$active_profile_item,$alt_profile_item],"codex_settings":{"usage_source_mode":"Auto"},"engine":{"started_at":"2026-03-08T12:27:12Z","connection_state":"Ready"}}}}
EOF
          ;;
        session/subscribe)
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"subscribed_topics":["usage.updated","active_state.updated","settings.updated","profiles.updated","activity.events.updated","activity.logs.updated","doctor.updated","switch.completed","switch.failed","health.updated"]}}
EOF
          case "$mode" in
            health_update)
              sleep 0.1
              cat <<EOF
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"health.updated","seq":1,"timestamp":"2026-03-08T12:27:12Z","payload":{"state":"Degraded","detail":"Relay engine degraded for test."}}}
EOF
              ;;
            switch_failed)
              sleep 0.1
              cat <<EOF
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"switch.failed","seq":1,"timestamp":"2026-03-08T12:27:12Z","payload":{"error_code":"RELAY_CONFLICT","message":"no eligible profile available","profile_id":"p_active","trigger":"Auto"}}}
EOF
              ;;
            usage_active_update)
              sleep 0.05
              cat <<EOF
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"usage.updated","seq":1,"timestamp":"2026-03-08T12:27:12Z","payload":{"snapshots":[{"profile_id":"p_alt","profile_name":"alt","source":"WebEnhanced","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":61.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Warning","exact":true},"weekly":{"used_percent":31.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"push usage"}],"trigger":"Manual"}}}
EOF
              sleep 0.05
              cat <<EOF
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"active_state.updated","seq":2,"timestamp":"2026-03-08T12:27:13Z","payload":{"active_state":{"active_profile_id":"p_alt","last_switch_at":"2026-03-08T12:27:13Z","last_switch_result":"Success","auto_switch_enabled":false,"last_error":null},"active_profile":$alt_profile_item}}}
EOF
              ;;
          esac
          ;;
        relay/status/get)
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"relay_home":"/tmp/relay","live_agent_home":"/Users/test/.codex","profile_count":2,"active_state":{"active_profile_id":"p_active","last_switch_at":"2026-03-08T12:27:12Z","last_switch_result":"Success","auto_switch_enabled":false,"last_error":null},"settings":{"auto_switch_enabled":false,"cooldown_seconds":600,"refresh_interval_seconds":60}}}
EOF
          ;;
        relay/settings/get)
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"app":{"auto_switch_enabled":false,"cooldown_seconds":600,"refresh_interval_seconds":60},"codex":{"usage_source_mode":"Auto"}}}
EOF
          ;;
        relay/profiles/list)
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":[$active_profile_item,$alt_profile_item]}
EOF
          ;;
        relay/doctor/get)
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"platform":"macOS","relay_home":"/tmp/relay","relay_db_path":"/tmp/relay/relay.db","relay_log_path":"/tmp/relay/logs/relay.log","primary_agent":"Codex","live_agent_home":"/Users/test/.codex","agent_binary":"/usr/bin/codex","default_agent_home":"/Users/test/.codex","default_agent_home_exists":true,"managed_files":["config.toml","auth.json","version.json"]}}
EOF
          ;;
        relay/activity/events/list)
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"events":[]}}
EOF
          ;;
        relay/activity/logs/tail)
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"logs":{"path":"/tmp/relay/logs/relay.log","lines":[]}}}
EOF
          ;;
        shutdown)
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"accepted":true}}
EOF
          exit 0
          ;;
        *)
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{}}
EOF
          ;;
      esac
    done
    ;;
  *)
    cat <<EOF
{"success":false,"error_code":"BAD_COMMAND","message":"unexpected command: $*","data":null}
EOF
    ;;
esac
"""
