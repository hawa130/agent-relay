# Development

## Workspace Shape

The repository keeps the Rust workspace at two package levels:

- `apps/relay-cli`
- `crates/relay-core`

The native macOS control plane lives in `apps/relay-macos`.

Inside `relay-core`, prefer module boundaries over new crates:

- `models`
- `services`
- `store`
- `platform`
- `adapters`

## Common Commands

Format, lint, and test with the standard workspace commands:

```bash
just fmt
just fmt-check
just lint
just check
just test
just test-rust
just test-macos
```

Additional build helpers:

```bash
just build-release
just build-macos
just build-linux
just build-all
just build-app
```

`just build-linux` and `just build-all` assume the Linux Rust target and a compatible cross-linker are already installed. `just test-macos` runs Swift tests with an isolated home and module cache under `apps/relay-macos`.

Swift tooling is managed with third-party tools installed through Homebrew:

```bash
brew install swiftformat swiftlint xcodegen
```

`just fmt` and `just fmt-check` include `swiftformat` for the macOS app source tree. `just lint` runs both `cargo clippy --workspace --all-targets -- -D warnings` and `swiftlint lint --config .swiftlint.yml`. `just check` runs formatting verification, linting, and tests together.

For the macOS app's Xcode workflow:

```bash
cd apps/relay-macos
./scripts/generate-xcodeproj.sh
open AgentRelay.xcodeproj
```

Use the generated project with the local SwiftPM checkout cache when verifying from the CLI:

```bash
xcodebuild -project apps/relay-macos/AgentRelay.xcodeproj \
  -scheme AgentRelay \
  -destination 'platform=macOS' \
  -clonedSourcePackagesDirPath apps/relay-macos/.build \
  -disableAutomaticPackageResolution \
  -onlyUsePackageVersionsFromResolvedFile \
  build
```

The generated project preserves the same target split as `Package.swift`: `RelayMacOSUI` contains the SwiftUI sources and resources, `AgentRelay` wraps the executable entrypoint, and `RelayMacOSTests` continues to test `RelayMacOSUI` directly.

## Local Iteration

Run the CLI directly from source:

```bash
cargo run -p agrelay-cli --bin agrelay -- --help
```

Run with isolated state when you need deterministic local testing:

```bash
AGRELAY_HOME=/tmp/agrelay-dev CODEX_HOME=/tmp/codex-dev \
  cargo run -p agrelay-cli --bin agrelay -- doctor --json
```

Use temp directories for any test or manual workflow that touches filesystem state.

## Schema Workflow

Schema changes follow the SeaORM 2.x entity-first workflow:

1. update the hand-written entities under `relay-core::store::entities`
2. adjust store and service logic as needed
3. delete `relay.db` during development if the change is breaking
4. run tests so write bootstrap recreates or syncs the schema

The entities are the schema source of truth. Keep schema guidance aligned with `docs/sqlite-schema.md`.

## Test Strategy

The project uses several verification layers:

- `relay-core` model tests for serde and protocol contracts
- `relay-core` store tests for SeaORM entities and state-file behavior
- `relay-core` service tests with temp stores and fake adapters
- CLI integration tests in `apps/relay-cli/tests/cli.rs`
- Swift decoding and daemon-client tests in `apps/relay-macos`

Tests that touch the filesystem should use temp directories and isolated homes.

## Contributor Constraints

- keep the CLI as the only execution layer
- keep JSON and RPC contracts stable
- keep live config writes transactional and recoverable
- do not touch project-local `.codex/`
- prefer extending `relay-core` modules over adding new packages
- keep business logic in `services`, persistence details in `store`, and provider-specific behavior in `adapters`

## Release Verification

Before cutting a release:

1. run `just fmt-check`
2. run `just test`
3. run `cargo clippy --workspace --all-targets -- -D warnings`
4. verify `cargo install --path apps/relay-cli`
5. smoke test `agrelay doctor --json`
6. smoke test `agrelay switch` against temp Codex homes

For the full repo gate, prefer `just check`.

## Supporting Docs

- SQLite schema workflow: `docs/sqlite-schema.md`
- Linux support reference: `docs/linux-support.md`
- Security release checklist: `docs/security-checklist.md`
