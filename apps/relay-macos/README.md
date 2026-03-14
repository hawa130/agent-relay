# AgentRelay for macOS

`apps/relay-macos` is the native macOS control plane for AgentRelay.

The app supervises a long-lived `agrelay daemon --stdio` session and speaks stdio JSON-RPC. Profile mutation, switching, validation, usage refresh, and diagnostics remain in the CLI/core runtime; the app does not mutate live Codex files directly.

Contributor tooling for the Swift code:

```bash
brew install swiftformat swiftlint xcodegen
```

The repository uses `swiftformat` for formatting and `swiftlint` for Swift lint checks through the shared `just` workflow.

## Build

Build from the package directory:

```bash
cd apps/relay-macos
swift build
```

Build a distributable `.app` bundle:

```bash
cd apps/relay-macos
./scripts/build-app.sh
```

Generate an Xcode project for local app development:

```bash
cd apps/relay-macos
./scripts/generate-xcodeproj.sh
open AgentRelay.xcodeproj
```

Verify the generated Xcode project from the CLI without re-resolving packages:

```bash
cd apps/relay-macos
xcodebuild -project AgentRelay.xcodeproj \
  -scheme AgentRelay \
  -destination 'platform=macOS' \
  -clonedSourcePackagesDirPath .build \
  -disableAutomaticPackageResolution \
  -onlyUsePackageVersionsFromResolvedFile \
  build
```

Output bundle:

```bash
apps/relay-macos/dist/AgentRelay.app
```

Embedded CLI path:

```bash
AgentRelay.app/Contents/Resources/bin/agrelay
```

## Run

Run the menu bar app from source:

```bash
cd apps/relay-macos
swift run AgentRelay
```

Override the embedded CLI if needed:

```bash
AGRELAY_CLI_PATH=/absolute/path/to/agrelay swift run AgentRelay
```

## Structure

```text
RelayApp/
  App/
  MenuBar/
  Settings/
  Activity/
  Services/
  Models/
  Resources/
```

## Notes

- `swift run` is useful for source-level iteration, but the `.app` bundle is the supported distribution shape for reliable menu bar behavior and login-item integration
- Xcode project files are generated from `apps/relay-macos/project.yml` via XcodeGen; regenerate after changing target structure or package references
- the generated Xcode project mirrors the SwiftPM layout with a separate `RelayMacOSUI` module, so source-level imports and tests stay aligned across `swift test` and Xcode
- the app resolves `agrelay` in this order: `AGRELAY_CLI_PATH`, bundled `Contents/Resources/bin/agrelay`, then `PATH`
- `SMAppService` launch-at-login support requires running from a proper app bundle; the toggle may report unsupported when launched directly from `swift run`
- the app expects the AgentRelay daemon RPC contract to remain stable across compatible releases
