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
        XCTAssertEqual(try fixture.commands(), ["--json codex login --input-json -"])
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
        XCTAssertEqual(try fixture.commands(), ["--json codex login --input-json -"])
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
    enum Mode {
        case refresh
        case loginCancelled
        case loginFailed
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

private let fixtureScript = """
#!/bin/sh
set -eu

script_dir="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
printf '%s\n' "$*" >> "$script_dir/commands.log"

case "$*" in
  "--json codex login --input-json -")
    cat >/dev/null
    case "${RELAY_FIXTURE_MODE:-refresh}" in
      "login_cancelled")
        cat <<'EOF'
{"success":false,"error_code":"RELAY_EXTERNAL_COMMAND","message":"codex login timed out waiting for browser sign-in","data":null}
EOF
        ;;
      "login_failed")
        cat <<'EOF'
{"success":false,"error_code":"RELAY_EXTERNAL_COMMAND","message":"codex binary not found","data":null}
EOF
        ;;
      *)
        cat <<'EOF'
{"success":false,"error_code":"BAD_COMMAND","message":"unexpected login mode","data":null}
EOF
        ;;
    esac
    ;;
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
