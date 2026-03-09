import Foundation
import XCTest
@testable import RelayMacOSUI

@MainActor
final class RelayAppModelTests: XCTestCase {
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

        XCTAssertNil(model.lastErrorMessage)
        XCTAssertEqual(model.usageSnapshot(for: "p_alt")?.profileId, "p_alt")
        XCTAssertEqual(try fixture.commands(), ["--json refresh --input-json -"])
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

        XCTAssertNil(model.lastErrorMessage)
        XCTAssertEqual(model.usageSnapshot(for: "p_alt")?.profileId, "p_alt")
        XCTAssertEqual(try fixture.commands(), ["--json refresh"])
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

        XCTAssertFalse(model.isRefreshingUsage(profileId: "p_alt"))
        XCTAssertEqual(try fixture.commands(), ["--json refresh --input-json -"])
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

        XCTAssertFalse(model.isRefreshingEnabledUsage)
        XCTAssertEqual(try fixture.commands(), ["--json refresh"])
    }
}

private struct RelayAppModelFixture {
    let root: URL
    let scriptPath: String
    let commandsPath: URL

    static func make() throws -> Self {
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

private let fixtureScript = """
#!/bin/sh
set -eu

script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
printf '%s\n' "$*" >> "$script_dir/commands.log"

case "$*" in
  "--json refresh --input-json -")
    cat >/dev/null
    sleep 0.2
    cat <<'EOF'
{"success":true,"error_code":null,"message":"usage refreshed","data":{"profile_id":"p_alt","profile_name":"alt","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":29.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":31.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}}
EOF
    ;;
  "--json refresh")
    sleep 0.2
    cat <<'EOF'
{"success":true,"error_code":null,"message":"enabled profiles refreshed","data":[{"profile_id":"p_alt","profile_name":"alt","source":"Local","confidence":"High","stale":false,"last_refreshed_at":"2026-03-08T12:27:12Z","next_reset_at":"2026-03-08T17:06:00Z","session":{"used_percent":29.0,"window_minutes":300,"reset_at":"2026-03-08T17:06:00Z","status":"Healthy","exact":true},"weekly":{"used_percent":31.0,"window_minutes":10080,"reset_at":"2026-03-12T06:36:18Z","status":"Healthy","exact":true},"auto_switch_reason":null,"can_auto_switch":false,"message":"local usage"}]}
EOF
    ;;
  *)
    cat <<EOF
{"success":false,"error_code":"BAD_COMMAND","message":"unexpected command: $*","data":null}
EOF
    ;;
esac
"""
