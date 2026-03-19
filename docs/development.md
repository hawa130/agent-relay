# Development

For the workspace layout and module boundaries, see [Repository Shape](../AGENTS.md#repository-shape) in AGENTS.md. For contributor constraints and architecture rules, see [AGENTS.md](../AGENTS.md).

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

For macOS app build, Xcode project generation, and Swift tooling setup, see [`apps/relay-macos/README.md`](../apps/relay-macos/README.md).

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

## Release Verification

Run `just check` for the full verification gate (formatting, linting, and tests). Additionally, before cutting a release:

1. verify `cargo install --path apps/relay-cli`
2. smoke test `agrelay doctor --json`
3. smoke test `agrelay switch` against temp Codex homes

## Supporting Docs

- SQLite schema workflow: [`docs/sqlite-schema.md`](./sqlite-schema.md)
- Linux support reference: [`docs/linux-support.md`](./linux-support.md)
- Security release checklist: [`docs/security-checklist.md`](./security-checklist.md)
