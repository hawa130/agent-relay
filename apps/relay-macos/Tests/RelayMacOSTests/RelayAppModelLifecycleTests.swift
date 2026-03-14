@testable import AgentRelayUI
import Foundation
import XCTest

@MainActor
final class RelayAppModelLifecycleTests: XCTestCase {
    func testStopShutsDownDaemonSession() async throws {
        let fixture = try RelayLifecycleFixture.make()
        defer { fixture.cleanup() }

        let scope = RelayLifecycleFixtureEnvironment(relayCLIPath: fixture.scriptPath)
        scope.install()
        defer { scope.uninstall() }

        let model = RelayAppModel()
        model.start()

        try await waitUntilLifecycle {
            (try? fixture.commands().contains("rpc session/subscribe")) == true
        }

        await model.stop()

        try await waitUntilLifecycle {
            (try? fixture.commands().contains("rpc shutdown")) == true
        }
    }
}

private struct RelayLifecycleFixtureEnvironment {
    let relayCLIPath: String
    let originalRelayCLIPath = getenv("AGRELAY_CLI_PATH").map { String(cString: $0) }

    func install() {
        setenv("AGRELAY_CLI_PATH", relayCLIPath, 1)
    }

    func uninstall() {
        if let originalRelayCLIPath {
            setenv("AGRELAY_CLI_PATH", originalRelayCLIPath, 1)
        } else {
            unsetenv("AGRELAY_CLI_PATH")
        }
    }
}

private struct RelayLifecycleFixture {
    let root: URL
    let scriptPath: String
    let commandsPath: URL

    static func make() throws -> Self {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(
            "relay-app-model-lifecycle-tests-\(UUID().uuidString)",
            isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)

        let scriptURL = root.appendingPathComponent("relay-fixture.sh")
        try fixtureScript.write(to: scriptURL, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: scriptURL.path)

        return Self(
            root: root,
            scriptPath: scriptURL.path,
            commandsPath: root.appendingPathComponent("commands.log"))
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
}

private enum RelayLifecycleWaitTimeout: Error {
    case timedOut
}

@MainActor
private func waitUntilLifecycle(
    timeout: Duration = .seconds(2),
    condition: @escaping @MainActor () -> Bool) async throws
{
    let clock = ContinuousClock()
    let deadline = clock.now + timeout
    while clock.now < deadline {
        if condition() {
            return
        }
        try await Task.sleep(for: .milliseconds(50))
    }
    throw RelayLifecycleWaitTimeout.timedOut
}

private let fixtureScript = #"""
#!/bin/sh
set -eu

script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
printf '%s\n' "$*" >> "$script_dir/commands.log"

emit_json() {
  python3 -c 'import json,sys
text=sys.stdin.read()
decoder=json.JSONDecoder()
index=0
length=len(text)
while True:
    while index < length and text[index].isspace():
        index += 1
    if index >= length:
        break
    value, index = decoder.raw_decode(text, index)
    print(json.dumps(value, separators=(",", ":")))'
}

case "$*" in
  "daemon --stdio")
    while IFS= read -r line; do
      printf '%s\n' "raw $line" >> "$script_dir/commands.log"
      method="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get("method", ""))' "$line")"
      id="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get("id", ""))' "$line")"
      case "$method" in
        initialize)
          printf '%s\n' 'rpc initialize' >> "$script_dir/commands.log"
          emit_json <<EOF
{
  "jsonrpc": "2.0",
  "id": "$id",
  "result": {
    "protocol_version": "1",
    "initial_state": {
      "status": {
        "relay_home": "/tmp/relay",
        "live_agent_home": "/Users/test/.codex",
        "profile_count": 0,
        "active_state": {
          "active_profile_id": null,
          "last_switch_at": null,
          "last_switch_result": null,
          "auto_switch_enabled": false
        },
        "settings": {
          "auto_switch_enabled": false,
          "cooldown_seconds": 600,
          "refresh_interval_seconds": 60,
          "network_query_concurrency": 10
        }
      },
      "profiles": [],
      "codex_settings": {
        "usage_source_mode": "Auto"
      },
      "engine": {
        "started_at": "2026-03-08T12:27:12Z",
        "connection_state": "Ready"
      }
    }
  }
}
EOF
          ;;
        session/subscribe)
          printf '%s\n' 'rpc session/subscribe' >> "$script_dir/commands.log"
          emit_json <<EOF
{
  "jsonrpc": "2.0",
  "id": "$id",
  "result": {
    "subscribed_topics": [
      "usage.updated",
      "query_state.updated",
      "active_state.updated",
      "settings.updated",
      "profiles.updated",
      "activity.events.updated",
      "activity.logs.updated",
      "doctor.updated",
      "switch.completed",
      "switch.failed",
      "task.updated",
      "health.updated"
    ]
  }
}
EOF
          ;;
        shutdown)
          printf '%s\n' 'rpc shutdown' >> "$script_dir/commands.log"
          emit_json <<EOF
{
  "jsonrpc": "2.0",
  "id": "$id",
  "result": {
    "accepted": true
  }
}
EOF
          exit 0
          ;;
        *)
          emit_json <<EOF
{
  "jsonrpc": "2.0",
  "id": "$id",
  "result": {}
}
EOF
          ;;
      esac
    done
    ;;
  *)
    emit_json <<EOF
{
  "success": false,
  "error_code": "BAD_COMMAND",
  "message": "unexpected command: $*",
  "data": null
}
EOF
    ;;
esac
"""#
