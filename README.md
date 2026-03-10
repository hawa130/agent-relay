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
- a native macOS menu bar control plane that calls Relay CLI JSON

The CLI remains the only execution layer. The macOS menu bar app is a native control plane on top of Relay CLI JSON.

## Status

The V1 CLI is implemented and tested.

Implemented command groups:

```bash
relay doctor
relay status
relay settings show
relay settings set

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
```

Additional docs:

- [Architecture](./docs/architecture.md)
- [Install and Usage](./docs/install.md)
- [Development](./docs/development.md)
