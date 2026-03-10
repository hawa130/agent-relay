# Development

## Workspace

Rust packages:

- `apps/relay-cli`
- `crates/relay-core`

`relay-core` is split by modules:

- `models`
- `services`
- `store`
- `platform`
- `adapters`

## Local Commands

Format and test:

```bash
cargo fmt --all
cargo test
```

Build helpers with `just`:

```bash
just fmt
just test
just test-rust
just test-macos
just release
just macos
just linux
just all
just app
```

`just linux` and `just all` assume the Linux Rust target and a compatible
cross-linker are already installed locally.

`just test-macos` runs Swift tests with an isolated home and module cache under
`apps/relay-macos`.

Run the CLI directly from source:

```bash
cargo run -p relay-cli --bin relay -- --help
```

Run with isolated state:

```bash
RELAY_HOME=/tmp/relay-dev CODEX_HOME=/tmp/codex-dev \
  cargo run -p relay-cli --bin relay -- doctor --json
```

`relay-cli` now runs on a Tokio entrypoint because `relay-core` uses SeaORM's async database API and reuses a single store connection per app bootstrap.

Schema changes use a SeaORM 2.x entity-first workflow:

1. update the hand-written entities under `relay-core::store::entities`
2. adjust store and service logic as needed
3. delete `relay.db` for breaking schema changes during development
4. run tests so write bootstrap rebuilds the schema through schema sync

## Test Strategy

Current coverage includes:

- unit tests for SeaORM store/state/adapters
- integration tests for CLI profile CRUD
- integration tests for import, switch, rollback, events, logs, and diagnostics

The CLI integration suite lives in:

- `apps/relay-cli/tests/cli.rs`

## Design Constraints

- keep CLI as the only execution layer
- keep JSON output stable
- keep live config writes transactional
- do not touch project-local `.codex/`
- prefer extending `relay-core` modules over adding new packages

## Release Notes for Developers

Before cutting a release:

1. run `cargo fmt --all`
2. run `cargo clippy --workspace --all-targets -- -D warnings`
3. run `cargo test`
4. verify `cargo install --path apps/relay-cli`
5. smoke test `relay doctor --json`
6. smoke test `relay switch` against temp Codex homes

## Supporting Docs

- SQLite schema workflow: `docs/sqlite-schema.md`
- Linux support matrix and test plan: `docs/linux-support.md`
- Security release checklist: `docs/security-checklist.md`
