# Install and Usage

## Install

Install the local CLI from this repository:

```bash
cargo install --path apps/relay-cli
```

Expected executable:

```bash
agrelay --help
```

If the shell cannot find `agrelay`, add Cargo's bin directory:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Isolated Testing

Use a temporary AgentRelay home while testing:

```bash
export AGRELAY_HOME=/tmp/agrelay-demo
agrelay status --json
```

AgentRelay stores its own data under `AGRELAY_HOME` or `~/.agrelay`:

- `relay.db`
- `state.json`
- `logs/relay.log`
- `profiles/`
- `snapshots/`
- `exports/`

## Common Commands

Environment and discovery:

```bash
agrelay doctor --json
agrelay status --json
```

Settings:

```bash
agrelay settings show --json
agrelay settings set --json --input-json app-settings.json
agrelay codex settings show --json
agrelay codex settings set --json --input-json settings.json
```

Profile management:

```bash
agrelay list --json
agrelay show --json
agrelay show <id> --json
agrelay edit <id> --json --input-json profile.json
agrelay enable <id> --json
agrelay disable <id> --json
agrelay remove <id> --json

agrelay codex add --json --input-json codex-profile.json
agrelay codex import --nickname imported-live --json
agrelay codex login --nickname work --json
agrelay codex recover --json
agrelay codex relink <id> --json
```

Switching and usage:

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

Programmatic daemon transport:

```bash
agrelay daemon --stdio
```

`agrelay daemon --stdio` exposes a single-client stdio JSON-RPC session intended for host programs such as the macOS menu bar app. The macOS app keeps this process alive, consumes push updates, and supervises restarts when needed.

Observability:

```bash
agrelay activity events list --limit 20 --json
agrelay activity logs tail --lines 50 --json
agrelay activity diagnostics export --json
```

## Troubleshooting

If `agrelay doctor --json` reports no live Codex home:

- ensure `~/.codex` exists, or
- set `CODEX_HOME` explicitly before running AgentRelay

If switching fails:

- inspect `agrelay activity logs tail --json`
- inspect `agrelay activity events list --json`
- export a bundle with `agrelay activity diagnostics export --json`

If you want deterministic test runs:

- set both `AGRELAY_HOME` and `CODEX_HOME` to temp directories

If `relay.db` is lost but `profiles/` still contains saved Codex homes:

- run `agrelay codex recover --json` to rebuild database profile records from those snapshots

## Additional References

- Development workflow: `docs/development.md`
- SQLite schema workflow: `docs/sqlite-schema.md`
- Linux support matrix: `docs/linux-support.md`
- Security release checklist: `docs/security-checklist.md`
