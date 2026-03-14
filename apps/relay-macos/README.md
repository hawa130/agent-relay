# AgentRelay for macOS

`apps/relay-macos` is the native macOS app for AgentRelay. It gives the product a menu bar control plane for profile switching, status visibility, activity inspection, and settings, while keeping the operational logic in the shared CLI/core runtime.

The app supervises a long-lived `agrelay daemon --stdio` session and speaks stdio JSON-RPC. Profile mutation, switching, validation, usage refresh, and diagnostics remain in the CLI/core runtime; the app does not mutate live Codex files directly.

For the product overview, start with [`README.md`](../../README.md). For CLI install and operator workflows, use [`docs/install.md`](../../docs/install.md).

## What The App Is For

- provide a native macOS surface for people who want AgentRelay without living in the terminal
- keep a persistent daemon-backed session alive for live status, refresh, and notifications
- expose the same profile and diagnostics capabilities as the CLI through a desktop control plane
- package the Rust CLI inside the app bundle for a more self-contained local experience

## Tooling

Install the Swift contributor tools with Homebrew:

```bash
brew install swiftformat swiftlint xcodegen
```

The repository uses `swiftformat` for formatting and `swiftlint` for lint checks through the shared `just` workflow.

## Run During Development

Run the menu bar app from source:

```bash
cd apps/relay-macos
swift run AgentRelay
```

Override the CLI binary if needed during development:

```bash
AGRELAY_CLI_PATH=/absolute/path/to/agrelay swift run AgentRelay
```

`swift run` is useful for source-level iteration, but the `.app` bundle is the supported distribution shape for reliable menu bar behavior and login-item integration.

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

Output bundle:

```bash
apps/relay-macos/dist/AgentRelay.app
```

Embedded CLI path:

```bash
AgentRelay.app/Contents/Helpers/agrelay
```

## Xcode Workflow

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

Xcode project files are generated from `apps/relay-macos/project.yml` via XcodeGen. Regenerate after changing target structure or package references.

## Structure

```text
RelayApp/
  App/
  MenuBar/
  Models/
  Resources/
  Services/
  Settings/
  Views/
```

The generated Xcode project mirrors the SwiftPM layout with a separate `AgentRelayUI` module, so source-level imports and tests stay aligned across `swift test` and Xcode.

## Integration Notes

- the app resolves `agrelay` in this order: `AGRELAY_CLI_PATH`, bundled `Contents/Helpers/agrelay`, legacy bundled `Contents/Resources/bin/agrelay`, then `PATH`
- Xcode builds run `apps/relay-macos/scripts/package-agrelay.sh` as a post-build phase so clicking Build in Xcode also compiles and embeds the Rust CLI into the app bundle
- `SMAppService` launch-at-login support requires running from a proper app bundle; the toggle may report unsupported when launched directly from `swift run`
- the app expects the AgentRelay daemon RPC contract to remain stable across compatible releases
