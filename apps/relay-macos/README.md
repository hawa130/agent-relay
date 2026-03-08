# Relay for macOS

`apps/relay-macos` is the native macOS control plane for Relay.

V1 rules:

- The app is a control plane only.
- All real profile, switch, validation, and diagnostics operations go through `relay --json`.
- No UI code mutates Codex configuration files directly.

## Build

Build from the package directory:

```bash
cd apps/relay-macos
swift build
```

Build a real `.app` bundle:

```bash
cd apps/relay-macos
./scripts/build-app.sh
```

Output:

```bash
apps/relay-macos/dist/RelayMacOS.app
```

The bundle includes an embedded `relay` CLI at:

```bash
RelayMacOS.app/Contents/Resources/bin/relay
```

Run the menu bar app:

```bash
cd apps/relay-macos
swift run RelayMacOS
```

Override the embedded CLI if needed:

```bash
RELAY_CLI_PATH=/absolute/path/to/relay swift run RelayMacOS
```

## Current Structure

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

## Current Scope

- `MenuBarExtra` entry point
- CLI client wrapper via `Process`
- status polling and JSON decoding
- profile list and manual switch actions
- settings window with profile enable/disable and auto-switch control
- activity window with events, logs, and diagnostics export
- launch-at-login toggle wrapper
- user notifications for switch success/failure

## Notes

- `swift run` is useful for source-level iteration, but it is not the right final distribution shape for a menu bar app.
- For reliable menu bar behavior, login item integration, and Finder launch, use the `.app` bundle built by `./scripts/build-app.sh`.
- The app resolves `relay` in this order: `RELAY_CLI_PATH`, bundled `Contents/Resources/bin/relay`, then `PATH`.
- `SMAppService` launch-at-login support requires running from a proper app bundle; the toggle is wired now but may report unsupported when running directly from `swift run`.
- The app expects the Relay CLI JSON contract to stay stable.
