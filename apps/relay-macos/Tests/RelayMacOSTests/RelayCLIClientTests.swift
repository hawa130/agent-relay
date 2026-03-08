import Foundation
import XCTest
@testable import RelayMacOSUI

final class RelayCLIClientTests: XCTestCase {
    func testFetchStatusUsesJSONAndDecodesLegacyFields() async throws {
        let fixture = try RelayCLIFixture.make()
        defer { fixture.cleanup() }

        let client = RelayCLIClient(relayCLIPathOverride: fixture.scriptPath, environment: [:])
        let status = try await client.fetchStatus()

        XCTAssertEqual(status.liveAgentHome, "/Users/test/.codex")
        XCTAssertEqual(status.activeState.activeProfileID, "p_active")
        XCTAssertEqual(status.profileCount, 1)
    }

    func testAddProfileSendsJSONInputOverStdin() async throws {
        let fixture = try RelayCLIFixture.make()
        defer { fixture.cleanup() }

        let client = RelayCLIClient(relayCLIPathOverride: fixture.scriptPath, environment: [:])
        let draft = ProfileDraft(
            nickname: "work",
            priority: 120,
            agentHome: "/tmp/work-home",
            configPath: "/tmp/work-home/config.toml",
            authMode: .configFilesystem,
            clearAgentHome: false,
            clearConfigPath: false
        )

        let profile = try await client.addProfile(draft)
        let payloadData = try Data(contentsOf: URL(fileURLWithPath: fixture.payloadPath))
        let payload = try JSONSerialization.jsonObject(with: payloadData) as? [String: Any]

        XCTAssertEqual(profile.id, "p_new")
        XCTAssertEqual(profile.nickname, "work")
        XCTAssertEqual(profile.agentHome, "/tmp/work-home")
        XCTAssertEqual(profile.priority, 120)
        XCTAssertEqual(payload?["nickname"] as? String, "work")
        XCTAssertEqual(payload?["priority"] as? Int, 120)
        XCTAssertEqual(payload?["agent_home"] as? String, "/tmp/work-home")
        XCTAssertEqual(payload?["config_path"] as? String, "/tmp/work-home/config.toml")
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
{"success":true,"error_code":null,"message":"status loaded","data":{"relay_home":"/tmp/relay","live_codex_home":"/Users/test/.codex","profile_count":1,"active_state":{"active_profile_id":"p_active","last_switch_at":"2026-03-08T12:27:12Z","last_switch_result":"Success","auto_switch_enabled":false,"last_error":null},"settings":{"auto_switch_enabled":false,"cooldown_seconds":600}}}
EOF
    ;;
  "--json profiles add --input-json -")
    payload="$(cat)"
    printf '%s' "$payload" > "$script_dir/last-input.json"
    cat <<'EOF'
{"success":true,"error_code":null,"message":"profile added","data":{"id":"p_new","nickname":"work","agent":"Codex","priority":120,"enabled":true,"agent_home":"/tmp/work-home","config_path":"/tmp/work-home/config.toml","auth_mode":"ConfigFilesystem","created_at":"2026-03-08T12:27:12Z","updated_at":"2026-03-08T12:27:12Z"}}
EOF
    ;;
  *)
    cat <<EOF
{"success":false,"error_code":"BAD_COMMAND","message":"unexpected command: $cmd","data":null}
EOF
    ;;
esac
"""
