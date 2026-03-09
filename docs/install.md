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

Profile management:

```bash
relay profiles list --json
relay profiles add codex --nickname work --agent-home /path/to/codex-home --json
relay profiles edit <id> --nickname updated --json
relay profiles enable <id> --json
relay profiles disable <id> --json
relay profiles remove <id> --json
relay profiles import codex --nickname imported-live --json
```

Switching:

```bash
relay switch <id> --json
relay switch next --json
relay auto-switch enable --json
relay auto-switch disable --json
```

Observability:

```bash
relay events list --limit 20 --json
relay logs tail --lines 50 --json
relay diagnostics export --json
```

## Troubleshooting

If `relay doctor --json` reports no live Codex home:

- ensure `~/.codex` exists, or
- set `CODEX_HOME` explicitly before running Relay

If switching fails:

- inspect `relay logs tail --json`
- inspect `relay events list --json`
- export a bundle with `relay diagnostics export --json`

If you want deterministic test runs:

- set both `RELAY_HOME` and `CODEX_HOME` to temp directories

## Additional References

- Development workflow: `docs/development.md`
- SQLite migration policy: `docs/sqlite-migrations.md`
- Linux support matrix: `docs/linux-support.md`
- Security release checklist: `docs/security-checklist.md`
