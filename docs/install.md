# Install and Usage

This document is the operator guide for AgentRelay. For the product overview, start with [`README.md`](../README.md).

## Prerequisites

- a Rust toolchain with `cargo`
- a local `codex` CLI installation for live-home discovery and real profile switching

For macOS app development, see [`apps/relay-macos/README.md`](../apps/relay-macos/README.md) for additional Swift tooling.

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

## Discover Commands

Run `agrelay --help` for the full command surface. Every user-visible command supports `--json`. For structured JSON input schemas, see [`docs/input-json-schemas.md`](./input-json-schemas.md).

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
