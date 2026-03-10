# Relay

Relay is a CLI-first local profile orchestrator for coding agents.

V1 currently targets `Codex` and provides:

- profile CRUD
- import from the current live Codex home
- transactional profile switching with rollback
- local-first usage reporting with session, weekly, reset, freshness, and source labels
- auto-switch policy toggle
- event history
- log tailing
- diagnostics export
- a native macOS menu bar control plane that supervises a persistent Relay daemon over JSON-RPC

The CLI remains the only execution layer. The macOS menu bar app is a native control plane that supervises `relay daemon --stdio` and communicates over stdio JSON-RPC.

## Status

The V1 CLI is implemented and tested.

Implemented command groups:

```bash
relay doctor
relay status
relay settings show
relay daemon --stdio

relay list
relay show
relay show <id>
relay edit <id>
relay remove <id>
relay enable <id>
relay disable <id>
relay switch
relay switch <id>
relay refresh
relay refresh <id>
relay refresh --all
relay autoswitch show
relay autoswitch enable
relay autoswitch disable
relay autoswitch set

relay activity events list
relay activity logs tail
relay activity diagnostics export

relay codex add
relay codex import
relay codex login
relay codex recover
relay codex relink <id>
```

## Install

Prerequisites:

- Rust toolchain with `cargo`
- a local `codex` CLI installation for real profile switching

Install from this repository:

```bash
cargo install --path apps/relay-cli
```

If `relay` is not found afterwards, add Cargo's bin directory to your shell path:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Quick Start

Inspect the environment:

```bash
relay doctor --json
```

Use an isolated Relay home while testing:

```bash
export RELAY_HOME=/tmp/relay-demo
relay status --json
```

Add a profile from an existing Codex home:

```bash
relay codex add \
  --nickname work \
  --agent-home /path/to/codex-home \
  --json
```

Import the currently live Codex profile into Relay-managed storage:

```bash
relay codex import --nickname imported-live --json
```

Rebuild database profile records from saved Relay snapshots after `relay.db` loss:

```bash
relay codex recover --json
```

Activate profiles:

```bash
relay list --json
relay switch <profile-id> --json
relay switch --json
```

Inspect runtime state:

```bash
relay show --json
relay refresh --json
relay activity events list --limit 20 --json
relay activity logs tail --lines 50 --json
relay activity diagnostics export --json
```

Programmatic clients can also start a persistent daemon session:

```bash
relay daemon --stdio
```

The daemon speaks newline-delimited JSON-RPC 2.0 on `stdin` and `stdout`. It is intended for the macOS app and other host programs, not for interactive human terminal use.

## How It Works

Relay does not replace the whole `~/.codex` directory.

For V1 it manages a narrow file set:

- `config.toml`
- `auth.json`
- `version.json`

Switching flow:

1. validate the target profile
2. snapshot the live managed files
3. atomically copy the target managed files into the live Codex home
4. validate the new live state
5. rollback on failure

This keeps the runtime safer than copying logs, sessions, and unrelated state.

## Development

Common commands:

```bash
cargo fmt --all
cargo test
cargo run -p relay-cli --bin relay -- --help
cargo run -p relay-cli --bin relay -- daemon --stdio
```

SeaORM workflow:

```bash
# 1. edit the hand-written SeaORM entities in relay-core
# 2. delete relay.db if you made a breaking schema change
cargo test
```

Relay now uses SeaORM 2.x entity-first schema sync. The entities in
`relay-core` are the schema source of truth. If a local dev database predates
the current entity-first layout, remove `relay.db` and let Relay recreate it on
next bootstrap.

Additional docs:

- [Architecture](./docs/architecture.md)
- [Install and Usage](./docs/install.md)
- [Development](./docs/development.md)
- [SQLite Schema](./docs/sqlite-schema.md)
