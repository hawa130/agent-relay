# AgentRelay

AgentRelay is a CLI-first local profile orchestrator for coding agents.

V1 currently targets `Codex` and provides:

- profile CRUD
- import from the current live Codex home
- transactional profile switching with rollback
- local-first usage reporting with session, weekly, reset, freshness, and source labels
- auto-switch policy toggle
- event history
- log tailing
- diagnostics export
- a native macOS menu bar control plane that supervises a persistent AgentRelay daemon over JSON-RPC

The CLI remains the only execution layer. The macOS menu bar app is a native control plane that supervises `agrelay daemon --stdio` and communicates over stdio JSON-RPC.

## Status

The V1 CLI is implemented and tested.

Implemented command groups:

```bash
agrelay doctor
agrelay status
agrelay settings show
agrelay settings set
agrelay daemon --stdio

agrelay list
agrelay show
agrelay show <id>
agrelay edit <id>
agrelay remove <id>
agrelay enable <id>
agrelay disable <id>
agrelay switch
agrelay switch <id>
agrelay refresh
agrelay refresh <id>
agrelay refresh --all
agrelay autoswitch show
agrelay autoswitch enable
agrelay autoswitch disable
agrelay autoswitch set

agrelay activity events list
agrelay activity logs tail
agrelay activity diagnostics export

agrelay codex add
agrelay codex import
agrelay codex login
agrelay codex recover
agrelay codex relink <id>
```

## Install

Prerequisites:

- Rust toolchain with `cargo`
- a local `codex` CLI installation for real profile switching

Install from this repository:

```bash
cargo install --path apps/relay-cli
```

If `agrelay` is not found afterwards, add Cargo's bin directory to your shell path:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Quick Start

Inspect the environment:

```bash
agrelay doctor --json
```

Use an isolated AgentRelay home while testing:

```bash
export AGRELAY_HOME=/tmp/agrelay-demo
agrelay status --json
```

Add a profile from an existing Codex home:

```bash
agrelay codex add \
  --nickname work \
  --agent-home /path/to/codex-home \
  --json
```

Import the currently live Codex profile into AgentRelay-managed storage:

```bash
agrelay codex import --nickname imported-live --json
```

Rebuild database profile records from saved AgentRelay snapshots after `relay.db` loss:

```bash
agrelay codex recover --json
```

Activate profiles:

```bash
agrelay list --json
agrelay switch <profile-id> --json
agrelay switch --json
```

Inspect runtime state:

```bash
agrelay show --json
agrelay refresh --json
agrelay activity events list --limit 20 --json
agrelay activity logs tail --lines 50 --json
agrelay activity diagnostics export --json
```

Programmatic clients can also start a persistent daemon session:

```bash
agrelay daemon --stdio
```

The daemon speaks newline-delimited JSON-RPC 2.0 on `stdin` and `stdout`. It is intended for the macOS app and other host programs, not for interactive human terminal use.

## How It Works

AgentRelay does not replace the whole `~/.codex` directory.

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
just fmt
cargo test
cargo run -p agrelay-cli --bin agrelay -- --help
cargo run -p agrelay-cli --bin agrelay -- daemon --stdio
```

SeaORM workflow:

```bash
# 1. edit the hand-written SeaORM entities in relay-core
# 2. delete relay.db if you made a breaking schema change
cargo test
```

AgentRelay now uses SeaORM 2.x entity-first schema sync. The entities in
`relay-core` are the schema source of truth. If a local dev database predates
the current entity-first layout, remove `relay.db` and let AgentRelay recreate it on
next bootstrap.

Additional docs:

- [Architecture](./docs/architecture.md)
- [Install and Usage](./docs/install.md)
- [Development](./docs/development.md)
- [SQLite Schema](./docs/sqlite-schema.md)
