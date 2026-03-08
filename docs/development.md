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

Run the CLI directly from source:

```bash
cargo run -p relay-cli --bin relay -- --help
```

Run with isolated state:

```bash
RELAY_HOME=/tmp/relay-dev CODEX_HOME=/tmp/codex-dev \
  cargo run -p relay-cli --bin relay -- doctor --json
```

## Test Strategy

Current coverage includes:

- unit tests for store/state/adapters
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
2. run `cargo test`
3. verify `cargo install --path apps/relay-cli`
4. smoke test `relay doctor --json`
5. smoke test `relay switch` against temp Codex homes

