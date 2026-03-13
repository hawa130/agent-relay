# AgentRelay

AgentRelay is a CLI-first local profile orchestrator for coding agents. The external product name is `AgentRelay`, the CLI binary is `agrelay`, and the current provider scope is `Codex`.

The CLI is the execution engine for profile management, switching, validation, usage refresh, and diagnostics. The macOS app is a native control plane that supervises `agrelay daemon --stdio` over stdio JSON-RPC.

## What It Does

- manage local Codex profiles with add, import, edit, enable, disable, show, list, and remove workflows
- switch live Codex state transactionally with validation, rollback, and checkpointing
- refresh and inspect usage snapshots with source, freshness, and exhaustion context
- record activity events, expose logs, and export diagnostics bundles
- host a long-lived daemon session for the macOS control plane and other programmatic clients

## Command Surface

Core commands:

```bash
agrelay doctor
agrelay status
agrelay list
agrelay show
agrelay edit <id>
agrelay remove <id>
agrelay enable <id>
agrelay disable <id>
agrelay switch
agrelay refresh
agrelay daemon --stdio
```

Grouped commands:

```bash
agrelay settings show
agrelay settings set
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

All user-visible commands support `--json`.

## Install

Prerequisites:

- Rust toolchain with `cargo`
- a local `codex` CLI installation for real profile switching and live-home discovery

Install from this repository:

```bash
cargo install --path apps/relay-cli
```

If `agrelay` is not on your shell path afterwards:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Quick Start

Inspect the local environment:

```bash
agrelay doctor --json
```

Use an isolated AgentRelay home while testing:

```bash
export AGRELAY_HOME=/tmp/agrelay-demo
agrelay status --json
```

Add or import a Codex profile:

```bash
agrelay codex add --nickname work --agent-home /path/to/codex-home --json
agrelay codex import --nickname imported-live --json
```

Switch and inspect state:

```bash
agrelay list --json
agrelay switch <profile-id> --json
agrelay refresh --json
agrelay activity events list --limit 20 --json
```

Recover database profile records from saved AgentRelay profile directories after `relay.db` loss:

```bash
agrelay codex recover --json
```

## Daemon Integration

Programmatic clients can start a persistent daemon session with:

```bash
agrelay daemon --stdio
```

The daemon speaks newline-delimited JSON-RPC 2.0 on `stdin` and `stdout`. It is intended for host programs such as the macOS app rather than interactive terminal use. The daemon owns background refresh, policy evaluation, and state-change notifications for that host session.

## Switching Model

AgentRelay does not replace the entire `~/.codex` directory. It manages a narrow live file set:

- `config.toml`
- `auth.json`
- `version.json`

Switching follows a transactional flow:

1. validate the target profile and prerequisites
2. snapshot the current managed live files
3. write candidate files through temporary paths
4. atomically replace the managed live files
5. validate the new live state and roll back on failure

This keeps live-state mutation recoverable and avoids copying unrelated Codex history, logs, or session artifacts.

## Development

Common contributor commands:

```bash
just fmt
just fmt-check
just lint
just check
just test
cargo run -p agrelay-cli --bin agrelay -- --help
cargo run -p agrelay-cli --bin agrelay -- daemon --stdio
```

For contributor workflow, schema maintenance, platform checks, and release verification, see the docs below.

## Documentation Map

- [Architecture](./docs/architecture.md)
- [Install and Usage](./docs/install.md)
- [Development](./docs/development.md)
- [SQLite Schema](./docs/sqlite-schema.md)
- [Linux Support](./docs/linux-support.md)
- [Security Checklist](./docs/security-checklist.md)
- [macOS App Guide](./apps/relay-macos/README.md)
