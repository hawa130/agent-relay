# Install and Usage

## Prerequisites

Install AgentRelay from this repository when you have:

- a Rust toolchain with `cargo`
- a local `codex` CLI installation for live-home discovery and real profile switching
- Homebrew available for installing contributor tooling such as `swiftformat` and `swiftlint` on macOS

Contributor tooling for the macOS app:

```bash
brew install swiftformat swiftlint
```

## Install

Install the CLI from the workspace:

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

## Runtime Paths

AgentRelay stores its own state under `AGRELAY_HOME` or `~/.agrelay`:

- `relay.db`
- `state.json`
- `logs/relay.log`
- `profiles/`
- `snapshots/`
- `exports/`

Use a temporary home for isolated testing:

```bash
export AGRELAY_HOME=/tmp/agrelay-demo
agrelay status --json
```

## Common Workflows

### Environment And Discovery

```bash
agrelay doctor --json
agrelay status --json
```

### Settings

```bash
agrelay settings show --json
agrelay settings set --json --input-json app-settings.json
agrelay codex settings show --json
agrelay codex settings set --json --input-json codex-settings.json
```

### Profile Management

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
```

### Switching And Refresh

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

### Diagnostics

```bash
agrelay activity events list --limit 20 --json
agrelay activity logs tail --lines 50 --json
agrelay activity diagnostics export --json
```

## Daemon Usage

Host programs can start a persistent daemon session with:

```bash
agrelay daemon --stdio
```

This transport is intended for programmatic clients such as the macOS menu bar app. The client keeps the daemon process alive, consumes push notifications, and supervises restarts when needed.

## Troubleshooting

If `agrelay doctor --json` reports no live Codex home:

- ensure `~/.codex` exists, or
- set `CODEX_HOME` explicitly before running AgentRelay

If switching fails:

- inspect `agrelay activity logs tail --json`
- inspect `agrelay activity events list --json`
- export a bundle with `agrelay activity diagnostics export --json`

If you want deterministic test or smoke runs:

- set both `AGRELAY_HOME` and `CODEX_HOME` to temp directories

If `relay.db` is lost but `profiles/` still contains saved AgentRelay-managed profile directories:

- run `agrelay codex recover --json` to rebuild database profile records from those saved profiles

## Related Docs

- Architecture reference: `docs/architecture.md`
- Contributor workflow: `docs/development.md`
- SQLite schema workflow: `docs/sqlite-schema.md`
- Linux support reference: `docs/linux-support.md`
- Security release checklist: `docs/security-checklist.md`
