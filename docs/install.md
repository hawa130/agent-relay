# Install and Usage

This document is the operator guide for AgentRelay. Use it for CLI installation, quick start workflows, command discovery, daemon integration, and troubleshooting on supported CLI platforms. For the product overview, start with [`README.md`](../README.md).

## Choose Your Surface

AgentRelay ships in two complementary forms:

- a native macOS app that supervises `agrelay daemon --stdio`
- a CLI for terminal-first usage, automation, and structured `--json` workflows on macOS and Linux

If you want to build or run the macOS app from source, also see [`apps/relay-macos/README.md`](../apps/relay-macos/README.md).

## Prerequisites

Install AgentRelay from this repository when you have:

- a Rust toolchain with `cargo`
- a local `codex` CLI installation for live-home discovery and real profile switching

If you are contributing to the macOS app, install the Swift tooling separately:

```bash
brew install swiftformat swiftlint xcodegen
```

## Install the CLI

Install `agrelay` from the workspace:

```bash
cargo install --path apps/relay-cli
```

Confirm the binary is available:

```bash
agrelay --help
```

If your shell cannot find `agrelay`, add Cargo's bin directory:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Quick Start

Check the local environment first:

```bash
agrelay doctor --json
```

Use an isolated AgentRelay home while testing:

```bash
export AGRELAY_HOME=/tmp/agrelay-demo
agrelay status --json
```

Bring an existing Codex setup under management or create a new one:

```bash
agrelay codex import --nickname imported-live --json
agrelay codex add --nickname work --agent-home /path/to/codex-home --json
```

Inspect profiles, switch, and refresh usage:

```bash
agrelay list --json
agrelay switch <profile-id> --json
agrelay refresh --json
agrelay activity events list --limit 20 --json
```

Recover profile records after `relay.db` loss when `profiles/` still exists:

```bash
agrelay codex recover --json
```

## Common Tasks

### Inspect environment and current state

```bash
agrelay doctor --json
agrelay status --json
```

### Manage settings

```bash
agrelay settings show --json
agrelay settings set --json --input-json app-settings.json
agrelay codex settings show --json
agrelay codex settings set --json --input-json codex-settings.json
```

### Manage profiles

```bash
agrelay list --json
agrelay show <id> --json
agrelay edit <id> --json --input-json profile.json
agrelay enable <id> --json
agrelay disable <id> --json
agrelay remove <id> --json
```

```bash
agrelay codex add --json --input-json codex-profile.json
agrelay codex import --nickname imported-live --json
agrelay codex login --nickname work --json
agrelay codex recover --json
agrelay codex relink <id> --json
agrelay codex settings show --json
agrelay codex settings set --json --input-json codex-settings.json
```

### Switch and refresh

```bash
agrelay switch --json
agrelay switch <id> --json
agrelay refresh --json
agrelay refresh <id> --json
agrelay refresh --all --json
agrelay autoswitch show --json
agrelay autoswitch enable --json
agrelay autoswitch disable --json
agrelay autoswitch set --json --input-json autoswitch.json
```

### Inspect logs and diagnostics

```bash
agrelay activity events list --limit 20 --json
agrelay activity logs tail --lines 50 --json
agrelay activity diagnostics export --json
```

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
agrelay codex settings show
agrelay codex settings set
```

All user-visible commands support `--json`.

## Daemon Integration

Host programs can start a persistent daemon session with:

```bash
agrelay daemon --stdio
```

This transport is intended for programmatic clients such as the macOS app. The client keeps the daemon process alive, consumes push notifications, and supervises restarts when needed.

## Runtime Paths

AgentRelay stores its own state under `AGRELAY_HOME` or `~/.agrelay`:

- `relay.db`
- `state.json`
- `logs/relay.log`
- `profiles/`
- `snapshots/`
- `exports/`

For deterministic smoke runs or manual testing, point both homes at temp directories:

```bash
export AGRELAY_HOME=/tmp/agrelay-demo
export CODEX_HOME=/tmp/codex-demo
agrelay status --json
```

## Troubleshooting

If `agrelay doctor --json` reports no live Codex home:

- ensure `~/.codex` exists, or
- set `CODEX_HOME` explicitly before running AgentRelay

If switching fails:

- inspect `agrelay activity logs tail --json`
- inspect `agrelay activity events list --json`
- export a bundle with `agrelay activity diagnostics export --json`

If `relay.db` is lost but `profiles/` still contains saved AgentRelay-managed profile directories:

- run `agrelay codex recover --json` to rebuild database profile records from those saved profiles

## Related Docs

- Product overview: [`README.md`](../README.md)
- Architecture reference: [`docs/architecture.md`](./architecture.md)
- Contributor workflow: [`docs/development.md`](./development.md)
- SQLite schema workflow: [`docs/sqlite-schema.md`](./sqlite-schema.md)
- Linux support reference: [`docs/linux-support.md`](./linux-support.md)
- Security release checklist: [`docs/security-checklist.md`](./security-checklist.md)
