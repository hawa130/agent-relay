import Foundation
import XCTest
@testable import RelayMacOSUI

final class RelayDaemonClientTests: XCTestCase {
    func testStartInitializesSessionAndReturnsInitialState() async throws {
        let fixture = try RelayDaemonFixture.make()
        defer { fixture.cleanup() }

        let client = RelayDaemonClient(
            relayCLIPathOverride: fixture.scriptPath,
            environment: [:]
        )
        let initial = try await client.start()

        XCTAssertEqual(initial.status.activeState.activeProfileId, "p_active_1")
        XCTAssertEqual(initial.status.profileCount, 1)
        XCTAssertEqual(initial.engine.connectionState, .ready)

        await client.stop()

        let commands = try fixture.commands()
        XCTAssertEqual(commands.first, "daemon --stdio")
        XCTAssertTrue(commands.contains("rpc initialize"))
        XCTAssertTrue(commands.contains("rpc session/subscribe"))
        XCTAssertTrue(commands.contains("rpc shutdown"))
    }

    func testConcurrentRequestsMatchResponsesByRequestID() async throws {
        let fixture = try RelayDaemonFixture.make(mode: "out_of_order_responses")
        defer { fixture.cleanup() }

        let client = RelayDaemonClient(
            relayCLIPathOverride: fixture.scriptPath,
            environment: ["RELAY_DAEMON_FIXTURE_MODE": "out_of_order_responses"]
        )

        async let status = client.fetchStatus()
        async let usage = client.fetchCurrentUsage()

        let resolvedStatus = try await status
        let resolvedUsage = try await usage

        XCTAssertEqual(resolvedStatus.activeState.activeProfileId, "p_active_1")
        XCTAssertEqual(resolvedUsage.profileId, "p_active_1")

        await client.stop()
    }

    func testUsageNotificationIsDecodedAndDelivered() async throws {
        let fixture = try RelayDaemonFixture.make(mode: "notification_then_refresh_response")
        defer { fixture.cleanup() }

        let client = RelayDaemonClient(
            relayCLIPathOverride: fixture.scriptPath,
            environment: ["RELAY_DAEMON_FIXTURE_MODE": "notification_then_refresh_response"]
        )

        async let refreshed = client.refreshEnabledUsage()
        let notification = try await nextNotification(from: client.notifications)
        let snapshots = try await refreshed

        guard case let .usageUpdated(payload) = notification else {
            return XCTFail("expected usage.updated notification")
        }

        XCTAssertEqual(payload.trigger, .manual)
        XCTAssertEqual(payload.snapshots.first?.profileId, "p_alt")
        XCTAssertEqual(snapshots.first?.profileId, "p_alt")

        await client.stop()
    }

    func testNotificationCanArriveBeforeResponseWithoutBreakingRequest() async throws {
        let fixture = try RelayDaemonFixture.make(mode: "notification_then_refresh_response")
        defer { fixture.cleanup() }

        let client = RelayDaemonClient(
            relayCLIPathOverride: fixture.scriptPath,
            environment: ["RELAY_DAEMON_FIXTURE_MODE": "notification_then_refresh_response"]
        )

        _ = try await client.start()
        let updateTask = Task {
            try await nextNotification(from: client.notifications)
        }
        let snapshots = try await client.refreshEnabledUsage()
        let update = try await updateTask.value

        XCTAssertEqual(snapshots.first?.profileId, "p_alt")
        guard case let .usageUpdated(payload) = update else {
            return XCTFail("expected usage.updated notification")
        }
        XCTAssertEqual(payload.snapshots.first?.profileId, "p_alt")

        await client.stop()
    }

    func testQueryStateNotificationIsDecodedAndDelivered() async throws {
        let fixture = try RelayDaemonFixture.make(mode: "query_state_then_refresh_response")
        defer { fixture.cleanup() }

        let client = RelayDaemonClient(
            relayCLIPathOverride: fixture.scriptPath,
            environment: ["RELAY_DAEMON_FIXTURE_MODE": "query_state_then_refresh_response"]
        )

        async let refreshed = client.refreshEnabledUsage()
        let notification = try await nextNotification(from: client.notifications)
        let snapshots = try await refreshed

        guard case let .queryStateUpdated(payload) = notification else {
            return XCTFail("expected query_state.updated notification")
        }

        XCTAssertEqual(payload.states.first?.key.kind, .usageProfile)
        XCTAssertEqual(payload.states.first?.key.profileId, "p_alt")
        XCTAssertEqual(payload.states.first?.status, .pending)
        XCTAssertEqual(payload.states.first?.trigger, .manual)
        XCTAssertEqual(snapshots.first?.profileId, "p_alt")

        await client.stop()
    }

    func testPendingRequestFailsWhenDaemonExitsUnexpectedly() async throws {
        let fixture = try RelayDaemonFixture.make(mode: "crash_on_status")
        defer { fixture.cleanup() }

        let client = RelayDaemonClient(
            relayCLIPathOverride: fixture.scriptPath,
            environment: ["RELAY_DAEMON_FIXTURE_MODE": "crash_on_status"]
        )

        do {
            _ = try await client.fetchStatus()
            XCTFail("expected fetchStatus to fail after daemon exit")
        } catch {
            let nsError = error as NSError
            XCTAssertTrue(nsError.localizedDescription.contains("relay daemon exited"))
        }
    }

    func testRequestTimeoutDoesNotPoisonSubsequentRequests() async throws {
        let fixture = try RelayDaemonFixture.make(mode: "drop_status_response")
        defer { fixture.cleanup() }

        let client = RelayDaemonClient(
            relayCLIPathOverride: fixture.scriptPath,
            requestTimeoutSeconds: 1.0,
            environment: ["RELAY_DAEMON_FIXTURE_MODE": "drop_status_response"]
        )

        do {
            _ = try await client.fetchStatus()
            XCTFail("expected fetchStatus to time out")
        } catch let RelayCLIClientError.commandFailed(code, message) {
            XCTAssertEqual(code, "RELAY_DAEMON_TIMEOUT")
            XCTAssertTrue(message.contains("timed out"))
        } catch {
            XCTFail("unexpected error: \(error)")
        }

        let usage = try await client.fetchCurrentUsage()
        XCTAssertEqual(usage.profileId?.hasPrefix("p_active_"), true)

        await client.stop()
    }

    func testRestartCreatesFreshDaemonSession() async throws {
        let fixture = try RelayDaemonFixture.make()
        defer { fixture.cleanup() }

        let client = RelayDaemonClient(
            relayCLIPathOverride: fixture.scriptPath,
            environment: [:]
        )

        let first = try await client.start()
        let second = try await client.restart()

        XCTAssertEqual(first.status.activeState.activeProfileId, "p_active_1")
        XCTAssertEqual(second.status.activeState.activeProfileId, "p_active_2")
        XCTAssertEqual(second.status.profileCount, 2)

        await client.stop()

        let launches = try fixture.launchCount()
        XCTAssertEqual(launches, 2)
    }
}

private struct RelayDaemonFixture {
    let root: URL
    let scriptPath: String
    let commandsPath: URL
    let launchesPath: URL

    static func make(mode _: String? = nil) throws -> Self {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(
            "relay-daemon-client-tests-\(UUID().uuidString)",
            isDirectory: true
        )
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)

        let scriptURL = root.appendingPathComponent("relay-daemon-fixture.sh")
        try fixtureScript.data(using: .utf8)!.write(to: scriptURL)
        try FileManager.default.setAttributes(
            [.posixPermissions: 0o755],
            ofItemAtPath: scriptURL.path
        )

        return Self(
            root: root,
            scriptPath: scriptURL.path,
            commandsPath: root.appendingPathComponent("commands.log"),
            launchesPath: root.appendingPathComponent("launches.count")
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
        return contents.split(separator: "\n").map(String.init)
    }

    func launchCount() throws -> Int {
        guard FileManager.default.fileExists(atPath: launchesPath.path) else {
            return 0
        }

        let contents = try String(contentsOf: launchesPath, encoding: .utf8)
        return Int(contents.trimmingCharacters(in: .whitespacesAndNewlines)) ?? 0
    }
}

private enum NotificationTimeout: Error {
    case timedOut
    case streamClosed
}

private func nextNotification(
    from stream: AsyncStream<RelaySessionUpdate>,
    timeoutNanoseconds: UInt64 = 2_000_000_000
) async throws -> RelaySessionUpdate {
    try await withThrowingTaskGroup(of: RelaySessionUpdate.self) { group in
        group.addTask {
            var iterator = stream.makeAsyncIterator()
            guard let update = await iterator.next() else {
                throw NotificationTimeout.streamClosed
            }
            return update
        }
        group.addTask {
            try await Task.sleep(nanoseconds: timeoutNanoseconds)
            throw NotificationTimeout.timedOut
        }

        let result = try await group.next()
        group.cancelAll()
        return try XCTUnwrap(result)
    }
}

private let fixtureScript = """
#!/bin/sh
set -eu

script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
printf '%s\n' "$*" >> "$script_dir/commands.log"

case "$*" in
  "daemon --stdio")
    launches_file="$script_dir/launches.count"
    launches=0
    if [ -f "$launches_file" ]; then
      launches="$(cat "$launches_file")"
    fi
    launches=$((launches + 1))
    printf '%s' "$launches" > "$launches_file"

    while IFS= read -r line; do
      printf '%s\n' "raw $line" >> "$script_dir/commands.log"
      method="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get("method", ""))' "$line")"
      id="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get("id", ""))' "$line")"
      mode="${RELAY_DAEMON_FIXTURE_MODE:-default}"
      case "$method" in
        initialize)
          printf '%s\n' 'rpc initialize' >> "$script_dir/commands.log"
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"protocol_version":"1","server_info":{"name":"relay","version":"0.1.0"},"capabilities":{"supports_subscriptions":true,"supports_health_updates":true},"initial_state":{"status":{"relay_home":"/tmp/relay","live_agent_home":"/Users/test/.codex","profile_count":$launches,"active_state":{"active_profile_id":"p_active_$launches","last_switch_at":"2026-03-08T12:27:12Z","last_switch_result":"Success","auto_switch_enabled":false,"last_error":null},"settings":{"auto_switch_enabled":false,"cooldown_seconds":600,"refresh_interval_seconds":60}},"profiles":[{"profile":{"id":"p_active_$launches","nickname":"active-$launches","agent":"Codex","priority":100,"enabled":true,"agent_home":"/tmp/active-home","config_path":"/tmp/active-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"},"is_active":true,"usage_summary":{"profile_id":"p_active_$launches","profile_name":"active-$launches","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":18.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":22.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}}],"codex_settings":{"usage_source_mode":"Auto"},"engine":{"started_at":"2026-03-08T12:27:12Z","connection_state":"Ready"}}}}
EOF
          ;;
        session/subscribe)
          printf '%s\n' 'rpc session/subscribe' >> "$script_dir/commands.log"
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"subscribed_topics":["usage.updated","query_state.updated","active_state.updated","switch.completed","switch.failed","health.updated"]}}
EOF
          ;;
        relay/status/get)
          printf '%s\n' 'rpc relay/status/get' >> "$script_dir/commands.log"
          if [ "$mode" = "crash_on_status" ]; then
            exit 9
          fi
          if [ "$mode" = "drop_status_response" ]; then
            continue
          fi
          if [ "$mode" = "out_of_order_responses" ]; then
            sleep 0.2
          fi
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"relay_home":"/tmp/relay","live_agent_home":"/Users/test/.codex","profile_count":$launches,"active_state":{"active_profile_id":"p_active_$launches","last_switch_at":"2026-03-08T12:27:12Z","last_switch_result":"Success","auto_switch_enabled":false,"last_error":null},"settings":{"auto_switch_enabled":false,"cooldown_seconds":600,"refresh_interval_seconds":60}}}
EOF
          ;;
        relay/usage/get)
          printf '%s\n' 'rpc relay/usage/get' >> "$script_dir/commands.log"
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"snapshot":{"profile_id":"p_active_$launches","profile_name":"active-$launches","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":18.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":22.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}}}
EOF
          ;;
        relay/usage/refresh)
          printf '%s\n' 'rpc relay/usage/refresh' >> "$script_dir/commands.log"
          if [ "$mode" = "notification_then_refresh_response" ]; then
            cat <<EOF
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"usage.updated","seq":1,"timestamp":"2026-03-08T12:27:12Z","payload":{"snapshots":[{"profile_id":"p_alt","profile_name":"alt","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":29.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":31.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}],"trigger":"Manual"}}}
EOF
            sleep 0.1
          elif [ "$mode" = "query_state_then_refresh_response" ]; then
            cat <<EOF
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"query_state.updated","seq":1,"timestamp":"2026-03-08T12:27:12Z","payload":{"states":[{"key":{"kind":"UsageProfile","profile_id":"p_alt"},"status":"Pending","trigger":"Manual","updated_at":"2026-03-08T12:27:12Z"}]}}}
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"usage.updated","seq":2,"timestamp":"2026-03-08T12:27:12Z","payload":{"snapshots":[{"profile_id":"p_alt","profile_name":"alt","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":29.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":31.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}],"trigger":"Manual"}}}
{"jsonrpc":"2.0","method":"session/update","params":{"topic":"query_state.updated","seq":3,"timestamp":"2026-03-08T12:27:13Z","payload":{"states":[]}}}
EOF
            sleep 0.1
          fi
          cat <<EOF
{"jsonrpc":"2.0","id":"$id","result":{"snapshots":[{"profile_id":"p_alt","profile_name":"alt","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":29.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":31.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}]}}
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
          printf '%s\n' "rpc $method" >> "$script_dir/commands.log"
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
