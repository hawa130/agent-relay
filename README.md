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

Implemented commands:

```bash
relay doctor
relay status
relay profiles list
relay profiles add
relay profiles edit
relay profiles remove
relay profiles enable
relay profiles disable
relay profiles import-codex
relay switch <id>
relay switch next
relay usage
relay auto-switch enable
relay auto-switch disable
relay auto-switch set
relay events list
relay logs tail
relay diagnostics export
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
relay profiles add \
  --nickname work \
  --codex-home /path/to/codex-home \
  --json
```

Import the currently live Codex profile into Relay-managed storage:

```bash
relay profiles import-codex --nickname imported-live --json
```

Switch profiles:

```bash
relay profiles list --json
relay switch <profile-id> --json
relay switch next --json
```

Inspect runtime state:

```bash
relay usage --json
relay events list --limit 20 --json
relay logs tail --lines 50 --json
relay diagnostics export --json
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
- [Todo](./docs/todo.md)
- [Install and Usage](./docs/install.md)
- [Development](./docs/development.md)
