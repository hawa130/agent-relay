import Foundation
import XCTest
@testable import RelayMacOSUI

@MainActor
final class RelayAppModelTests: XCTestCase {
    func testAddAccountReturnsNotSignedInWhenBrowserLoginIsCancelled() async throws {
        let fixture = try RelayAppModelFixture.make(mode: .loginCancelled)
        defer { fixture.cleanup() }
        let originalRelayCLIPath = getenv("RELAY_CLI_PATH").map { String(cString: $0) }
        let originalFixtureMode = getenv("RELAY_FIXTURE_MODE").map { String(cString: $0) }
        setenv("RELAY_CLI_PATH", fixture.scriptPath, 1)
        setenv("RELAY_FIXTURE_MODE", "login_cancelled", 1)
        defer {
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

        let model = RelayAppModel()
        let result = await model.addAccount(agent: .codex, priority: 100)

        XCTAssertEqual(
            result,
            .notSignedIn(detail: "Browser sign-in was cancelled or did not complete.")
        )
        XCTAssertEqual(
            model.lastErrorMessage,
            "Codex: Not signed in. Browser sign-in was cancelled or did not complete."
        )
        XCTAssertFalse(model.isMutatingProfiles)
        let commands = try fixture.commands()
        XCTAssertEqual(commands.first, "daemon --stdio")
        XCTAssertTrue(commands.contains("rpc relay/profiles/login"))
    }

    func testAddAccountReturnsFailedForUnexpectedLoginError() async throws {
        let fixture = try RelayAppModelFixture.make(mode: .loginFailed)
        defer { fixture.cleanup() }
        let originalRelayCLIPath = getenv("RELAY_CLI_PATH").map { String(cString: $0) }
        let originalFixtureMode = getenv("RELAY_FIXTURE_MODE").map { String(cString: $0) }
        setenv("RELAY_CLI_PATH", fixture.scriptPath, 1)
        setenv("RELAY_FIXTURE_MODE", "login_failed", 1)
        defer {
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

        let model = RelayAppModel()
        let result = await model.addAccount(agent: .codex, priority: 100)

        XCTAssertEqual(result, .failed(detail: "RELAY_EXTERNAL_COMMAND: codex binary not found"))
        XCTAssertEqual(model.lastErrorMessage, "RELAY_EXTERNAL_COMMAND: codex binary not found")
        XCTAssertFalse(model.isMutatingProfiles)
        let commands = try fixture.commands()
        XCTAssertEqual(commands.first, "daemon --stdio")
        XCTAssertTrue(commands.contains("rpc relay/profiles/login"))
    }

    func testRefreshUsageOnlyRunsUsageRefreshCommand() async throws {
        let fixture = try RelayAppModelFixture.make()
        defer { fixture.cleanup() }
        let originalRelayCLIPath = getenv("RELAY_CLI_PATH").map { String(cString: $0) }
        setenv("RELAY_CLI_PATH", fixture.scriptPath, 1)
        defer {
            if let originalRelayCLIPath {
                setenv("RELAY_CLI_PATH", originalRelayCLIPath, 1)
            } else {
                unsetenv("RELAY_CLI_PATH")
            }
        }

        let model = RelayAppModel()

        await model.refreshUsage(profileId: "p_alt")
        try await waitUntil {
            model.usageSnapshot(for: "p_alt")?.profileId == "p_alt"
        }

        XCTAssertNil(model.lastErrorMessage)
        XCTAssertEqual(model.usageSnapshot(for: "p_alt")?.profileId, "p_alt")
        let commands = try fixture.commands()
        XCTAssertEqual(commands.first, "daemon --stdio")
        XCTAssertTrue(commands.contains("rpc initialize"))
        XCTAssertTrue(commands.contains("rpc relay/usage/refresh"))
    }

    func testRefreshEnabledUsageOnlyRunsBulkRefreshCommand() async throws {
        let fixture = try RelayAppModelFixture.make()
        defer { fixture.cleanup() }
        let originalRelayCLIPath = getenv("RELAY_CLI_PATH").map { String(cString: $0) }
        setenv("RELAY_CLI_PATH", fixture.scriptPath, 1)
        defer {
            if let originalRelayCLIPath {
                setenv("RELAY_CLI_PATH", originalRelayCLIPath, 1)
            } else {
                unsetenv("RELAY_CLI_PATH")
            }
        }

        let model = RelayAppModel()

        await model.refreshEnabledUsage()
        try await waitUntil {
            model.usageSnapshot(for: "p_alt")?.profileId == "p_alt"
        }

        XCTAssertNil(model.lastErrorMessage)
        XCTAssertEqual(model.usageSnapshot(for: "p_alt")?.profileId, "p_alt")
        let commands = try fixture.commands()
        XCTAssertEqual(commands.first, "daemon --stdio")
        XCTAssertTrue(commands.contains("rpc initialize"))
        XCTAssertTrue(commands.contains("rpc relay/usage/refresh"))
    }

    func testRefreshUsageIgnoresConcurrentDuplicateRequests() async throws {
        let fixture = try RelayAppModelFixture.make()
        defer { fixture.cleanup() }
        let originalRelayCLIPath = getenv("RELAY_CLI_PATH").map { String(cString: $0) }
        setenv("RELAY_CLI_PATH", fixture.scriptPath, 1)
        defer {
            if let originalRelayCLIPath {
                setenv("RELAY_CLI_PATH", originalRelayCLIPath, 1)
            } else {
                unsetenv("RELAY_CLI_PATH")
            }
        }

        let model = RelayAppModel()

        async let first: Void = model.refreshUsage(profileId: "p_alt")
        await Task.yield()
        async let second: Void = model.refreshUsage(profileId: "p_alt")
        _ = await (first, second)

        try await waitUntil {
            !model.isRefreshingUsage(profileId: "p_alt")
        }
        let commands = try fixture.commands()
        XCTAssertEqual(commands.first, "daemon --stdio")
        XCTAssertTrue(commands.contains("rpc initialize"))
        XCTAssertTrue(commands.contains("rpc relay/usage/refresh"))
    }

    func testRefreshEnabledUsageIgnoresConcurrentDuplicateRequests() async throws {
        let fixture = try RelayAppModelFixture.make()
        defer { fixture.cleanup() }
        let originalRelayCLIPath = getenv("RELAY_CLI_PATH").map { String(cString: $0) }
        setenv("RELAY_CLI_PATH", fixture.scriptPath, 1)
        defer {
            if let originalRelayCLIPath {
                setenv("RELAY_CLI_PATH", originalRelayCLIPath, 1)
            } else {
                unsetenv("RELAY_CLI_PATH")
            }
        }

        let model = RelayAppModel()

        async let first: Void = model.refreshEnabledUsage()
        await Task.yield()
        async let second: Void = model.refreshEnabledUsage()
        _ = await (first, second)

        try await waitUntil {
            !model.isRefreshingEnabledUsage
        }
        let commands = try fixture.commands()
        XCTAssertEqual(commands.first, "daemon --stdio")
        XCTAssertTrue(commands.contains("rpc initialize"))
        XCTAssertTrue(commands.contains("rpc relay/usage/refresh"))
    }

    func testRemoveProfileDoesNotBlockOnFollowupRefresh() async throws {
        let fixture = try RelayAppModelFixture.make(mode: .removeDelayedRefresh)
        defer { fixture.cleanup() }
        let originalRelayCLIPath = getenv("RELAY_CLI_PATH").map { String(cString: $0) }
        let originalFixtureMode = getenv("RELAY_FIXTURE_MODE").map { String(cString: $0) }
        setenv("RELAY_CLI_PATH", fixture.scriptPath, 1)
        setenv("RELAY_FIXTURE_MODE", "remove_delayed_refresh", 1)
        defer {
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

        let model = RelayAppModel()
        let clock = ContinuousClock()
        let elapsed = await clock.measure {
            await model.removeProfile("p_alt")
        }

        XCTAssertLessThan(elapsed, .milliseconds(400))
        try await waitUntil {
            !model.profiles.contains(where: { $0.id == "p_alt" })
        }

        let commands = try fixture.commands()
        XCTAssertEqual(commands.first, "daemon --stdio")
        XCTAssertTrue(commands.contains("rpc relay/profiles/remove"))
        XCTAssertTrue(commands.contains("rpc session/subscribe"))
        XCTAssertFalse(commands.contains("rpc relay/status/get"))
        XCTAssertFalse(commands.contains("rpc relay/profiles/list"))
    }
}

private struct RelayAppModelFixture {
    enum Mode {
        case refresh
        case loginCancelled
        case loginFailed
        case removeDelayedRefresh
    }

    let root: URL
    let scriptPath: String
    let commandsPath: URL

    static func make(mode _: Mode = .refresh) throws -> Self {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(
            "relay-app-model-tests-\(UUID().uuidString)",
            isDirectory: true
        )
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)

        let scriptURL = root.appendingPathComponent("relay-fixture.sh")
        try fixtureScript.data(using: .utf8)!.write(to: scriptURL)
        try FileManager.default.setAttributes(
            [.posixPermissions: 0o755],
            ofItemAtPath: scriptURL.path
        )

        return Self(
            root: root,
            scriptPath: scriptURL.path,
            commandsPath: root.appendingPathComponent("commands.log")
        )
    }

    func cleanup() {
        try? FileManager.default.removeItem(at: root)
    }

    func commands() throws -> [String] {
        guard FileManager.default.fileExists(atPath: commandsPath.path) else {
            return []
        }

        let contents = try String(contentsOf: commandsPath, encoding: .utf8)
        return contents
            .split(separator: "\n")
            .map(String.init)
    }
}

private enum WaitTimeout: Error {
    case timedOut
}

@MainActor
private func waitUntil(
    timeoutNanoseconds: UInt64 = 2_000_000_000,
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

script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
printf '%s\n' "$*" >> "$script_dir/commands.log"

active_profile_item='{"profile":{"id":"p_active","nickname":"active","agent":"Codex","priority":100,"enabled":true,"agent_home":"/tmp/active-home","config_path":"/tmp/active-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"},"is_active":true,"usage_summary":{"profile_id":"p_active","profile_name":"active","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":18.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":22.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}}'
alt_profile_item='{"profile":{"id":"p_alt","nickname":"alt","agent":"Codex","priority":110,"enabled":true,"agent_home":"/tmp/alt-home","config_path":"/tmp/alt-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"},"is_active":false,"usage_summary":{"profile_id":"p_alt","profile_name":"alt","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":29.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":31.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}}'

case "$*" in
  "daemon --stdio")
    while IFS= read -r line; do
      printf '%s\n' "raw $line" >> "$script_dir/commands.log"
      method="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get("method", ""))' "$line")"
      id="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get("id", ""))' "$line")"
      case "$method" in
        initialize)
          printf '%s\n' 'rpc initialize' >> "$script_dir/commands.log"
          mode="${RELAY_FIXTURE_MODE:-refresh}"
          if [ "$mode" = "remove_delayed_refresh" ]; then
            cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"protocol_version":"1","initial_state":{"status":{"relay_home":"/tmp/relay","live_agent_home":"/Users/test/.codex","profile_count":2,"active_state":{"active_profile_id":"p_active","last_switch_at":"2026-03-08T12:27:12Z","last_switch_result":"Success","auto_switch_enabled":false,"last_error":null},"settings":{"auto_switch_enabled":false,"cooldown_seconds":600,"refresh_interval_seconds":60}},"profiles":[{"profile":{"id":"p_active","nickname":"active","agent":"Codex","priority":100,"enabled":true,"agent_home":"/tmp/active-home","config_path":"/tmp/active-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"},"is_active":true,"usage_summary":{"profile_id":"p_active","profile_name":"active","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":18.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":22.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}},{"profile":{"id":"p_alt","nickname":"alt","agent":"Codex","priority":110,"enabled":true,"agent_home":"/tmp/alt-home","config_path":"/tmp/alt-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"},"is_active":false,"usage_summary":{"profile_id":"p_alt","profile_name":"alt","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":29.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":31.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}}],"codex_settings":{"usage_source_mode":"Auto"},"engine":{"started_at":"2026-03-08T12:27:12Z","connection_state":"Ready"}}}}
EOF
            continue
          fi
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"protocol_version":"1","initial_state":{"status":{"relay_home":"/tmp/relay","live_agent_home":"/Users/test/.codex","profile_count":1,"active_state":{"active_profile_id":"p_active","last_switch_at":"2026-03-08T12:27:12Z","last_switch_result":"Success","auto_switch_enabled":false,"last_error":null},"settings":{"auto_switch_enabled":false,"cooldown_seconds":600,"refresh_interval_seconds":60}},"profiles":[],"codex_settings":{"usage_source_mode":"Auto"},"engine":{"started_at":"2026-03-08T12:27:12Z","connection_state":"Ready"}}}}
EOF
          ;;
        session/subscribe)
          printf '%s\n' 'rpc session/subscribe' >> "$script_dir/commands.log"
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"subscribed_topics":["usage.updated","query_state.updated","active_state.updated","settings.updated","profiles.updated","activity.events.updated","activity.logs.updated","doctor.updated","switch.completed","switch.failed","health.updated"]}}
EOF
          mode="${RELAY_FIXTURE_MODE:-refresh}"
          cat <<EOF
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"settings.updated","seq":1,"timestamp":"2026-03-08T12:27:12Z","payload":{"settings":{"app":{"auto_switch_enabled":false,"cooldown_seconds":600,"refresh_interval_seconds":60},"codex":{"usage_source_mode":"Auto"}}}}}
EOF
          if [ "$mode" = "remove_delayed_refresh" ]; then
            cat <<EOF
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"profiles.updated","seq":2,"timestamp":"2026-03-08T12:27:12Z","payload":{"profiles":[$active_profile_item,$alt_profile_item]}}}
EOF
          else
            cat <<EOF
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"profiles.updated","seq":2,"timestamp":"2026-03-08T12:27:12Z","payload":{"profiles":[]}}}
EOF
          fi
          ;;
        relay/usage/refresh)
          printf '%s\n' 'rpc relay/usage/refresh' >> "$script_dir/commands.log"
          cat <<EOF
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"query_state.updated","seq":3,"timestamp":"2026-03-08T12:27:12Z","payload":{"states":[{"key":{"kind":"UsageProfile","profile_id":"p_alt"},"status":"Pending","trigger":"Manual","updated_at":"2026-03-08T12:27:12Z"}]}}}
EOF
          sleep 0.2
          cat <<EOF
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"usage.updated","seq":4,"timestamp":"2026-03-08T12:27:12Z","payload":{"snapshots":[{"profile_id":"p_alt","profile_name":"alt","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":29.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":31.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}],"trigger":"Manual"}}}
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"query_state.updated","seq":5,"timestamp":"2026-03-08T12:27:13Z","payload":{"states":[]}}}
{"jsonrpc":"2.0","id":"$id","result":{"snapshots":[{"profile_id":"p_alt","profile_name":"alt","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":29.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":31.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}]}}
EOF
          ;;
        relay/profiles/login)
          printf '%s\n' 'rpc relay/profiles/login' >> "$script_dir/commands.log"
          case "${RELAY_FIXTURE_MODE:-refresh}" in
            "login_cancelled")
              cat <<EOF
{"jsonrpc":"2.0","id":"$id","error":{"code":-32000,"message":"codex login timed out waiting for browser sign-in","data":{"relay_error_code":"RELAY_EXTERNAL_COMMAND"}}}
EOF
              ;;
            "login_failed")
              cat <<EOF
{"jsonrpc":"2.0","id":"$id","error":{"code":-32000,"message":"codex binary not found","data":{"relay_error_code":"RELAY_EXTERNAL_COMMAND"}}}
EOF
              ;;
            *)
              cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"profile":{"id":"p_browser","nickname":"browser","agent":"Codex","priority":90,"enabled":true,"agent_home":"/tmp/browser-home","config_path":"/tmp/browser-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"},"probe_identity":{"profile_id":"p_browser","account_id":"acct-123","email":"browser@example.com","plan_hint":"team","provider":"CodexOfficial","principal_id":"acct-123","display_name":"browser@example.com"},"activated":false}}
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"profiles.updated","seq":4,"timestamp":"2026-03-08T12:27:12Z","payload":{"profiles":[$active_profile_item,{"profile":{"id":"p_browser","nickname":"browser","agent":"Codex","priority":90,"enabled":true,"agent_home":"/tmp/browser-home","config_path":"/tmp/browser-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"},"is_active":false,"usage_summary":null}]}}}
EOF
              ;;
          esac
          ;;
        relay/profiles/remove)
          printf '%s\n' 'rpc relay/profiles/remove' >> "$script_dir/commands.log"
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"id":"p_alt","nickname":"alt","agent":"Codex","priority":110,"enabled":true,"agent_home":"/tmp/alt-home","config_path":"/tmp/alt-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"}}
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"profiles.updated","seq":5,"timestamp":"2026-03-08T12:27:12Z","payload":{"profiles":[$active_profile_item]}}}
EOF
          ;;
        relay/settings/update)
          printf '%s\n' 'rpc relay/settings/update' >> "$script_dir/commands.log"
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"app":{"auto_switch_enabled":true,"cooldown_seconds":600,"refresh_interval_seconds":60},"codex":{"usage_source_mode":"Auto"}}}
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"settings.updated","seq":6,"timestamp":"2026-03-08T12:27:12Z","payload":{"settings":{"app":{"auto_switch_enabled":true,"cooldown_seconds":600,"refresh_interval_seconds":60},"codex":{"usage_source_mode":"Auto"}}}}}
EOF
          ;;
        relay/activity/refresh)
          printf '%s\n' 'rpc relay/activity/refresh' >> "$script_dir/commands.log"
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"events":[],"logs":{"path":"/tmp/relay/logs/relay.log","lines":[]}}}
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"activity.events.updated","seq":7,"timestamp":"2026-03-08T12:27:12Z","payload":{"events":[]}}}
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"activity.logs.updated","seq":8,"timestamp":"2026-03-08T12:27:12Z","payload":{"logs":{"path":"/tmp/relay/logs/relay.log","lines":[]}}}}
EOF
          ;;
        relay/doctor/refresh)
          printf '%s\n' 'rpc relay/doctor/refresh' >> "$script_dir/commands.log"
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"platform":"macOS","relay_home":"/tmp/relay","relay_db_path":"/tmp/relay/relay.db","relay_log_path":"/tmp/relay/logs/relay.log","primary_agent":"Codex","live_agent_home":"/Users/test/.codex","agent_binary":"/usr/bin/codex","default_agent_home":"/Users/test/.codex","default_agent_home_exists":true,"managed_files":["config.toml","auth.json","version.json"]}}
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"doctor.updated","seq":9,"timestamp":"2026-03-08T12:27:12Z","payload":{"report":{"platform":"macOS","relay_home":"/tmp/relay","relay_db_path":"/tmp/relay/relay.db","relay_log_path":"/tmp/relay/logs/relay.log","primary_agent":"Codex","live_agent_home":"/Users/test/.codex","agent_binary":"/usr/bin/codex","default_agent_home":"/Users/test/.codex","default_agent_home_exists":true,"managed_files":["config.toml","auth.json","version.json"]}}}}
EOF
          ;;
        shutdown)
          printf '%s\n' 'rpc shutdown' >> "$script_dir/commands.log"
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
