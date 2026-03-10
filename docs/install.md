# Install and Usage

## Install

Install the local CLI from this repository:

```bash
cargo install --path apps/relay-cli
```

Expected executable:

```bash
relay --help
```

If the shell cannot find `relay`, add Cargo's bin directory:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Isolated Testing

Use a temporary Relay home while testing:

```bash
export RELAY_HOME=/tmp/relay-demo
relay status --json
```

Relay stores its own data under `RELAY_HOME` or `~/.relay`:

- `relay.db`
- `state.json`
- `logs/relay.log`
- `profiles/`
- `snapshots/`
- `exports/`

## Common Commands

Environment and discovery:

```bash
relay doctor --json
relay status --json
```

Settings:

```bash
relay settings show --json
relay codex settings show --json
relay codex settings set --json --input-json settings.json
```

Profile management:

```bash
relay list --json
relay show --json
relay show <id> --json
relay edit <id> --json --input-json profile.json
relay enable <id> --json
relay disable <id> --json
relay remove <id> --json

relay codex add --json --input-json codex-profile.json
relay codex import --nickname imported-live --json
relay codex login --nickname work --json
relay codex recover --json
relay codex relink <id> --json
```

Switching and usage:

```bash
relay switch --json
relay switch <id> --json
relay refresh --json
relay refresh <id> --json
relay refresh --all --json

relay autoswitch show --json
relay autoswitch enable --json
relay autoswitch disable --json
relay autoswitch set --json --input-json autoswitch.json
```

Observability:

```bash
relay activity events list --limit 20 --json
relay activity logs tail --lines 50 --json
relay activity diagnostics export --json
```

## Troubleshooting

If `relay doctor --json` reports no live Codex home:

- ensure `~/.codex` exists, or
- set `CODEX_HOME` explicitly before running Relay

If switching fails:

- inspect `relay activity logs tail --json`
- inspect `relay activity events list --json`
- export a bundle with `relay activity diagnostics export --json`

If you want deterministic test runs:

- set both `RELAY_HOME` and `CODEX_HOME` to temp directories

If `relay.db` is lost but `profiles/` still contains saved Codex homes:

- run `relay codex recover --json` to rebuild database profile records from those snapshots

## Additional References

- Development workflow: `docs/development.md`
- SQLite schema workflow: `docs/sqlite-schema.md`
- Linux support matrix: `docs/linux-support.md`
- Security release checklist: `docs/security-checklist.md`
