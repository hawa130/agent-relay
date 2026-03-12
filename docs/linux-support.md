# Linux Support Matrix

AgentRelay V1 keeps the CLI portable across macOS and Linux. The macOS menu bar app remains macOS-only.

## Support Matrix

| Area | macOS | Linux |
| --- | --- | --- |
| `agrelay` CLI build | Supported | Supported |
| SQLite-backed profile store | Supported | Supported |
| File-backed active state/cache | Supported | Supported |
| Codex home discovery via `CODEX_HOME` | Supported | Supported |
| Default home path resolution | Supported | Supported |
| Transactional switch + rollback | Supported | Supported |
| Diagnostics export | Supported | Supported |
| Native menu bar app | Supported target | Not planned for V1 |
| Launch at login UI control | Supported target | Not applicable |

## Assumptions

- Linux users should prefer explicit `AGRELAY_HOME` and `CODEX_HOME` during initial setup and tests.
- Secret handling on Linux remains file-reference and environment-reference focused in V1.
- No Linux desktop shell integration is in scope for V1.

## Platform Test Plan

Minimum release gate:

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test`
4. `AGRELAY_HOME=/tmp/agrelay-smoke cargo run -p agrelay-cli --bin agrelay -- doctor --json`
5. `AGRELAY_HOME=/tmp/agrelay-smoke cargo run -p agrelay-cli --bin agrelay -- status --json`

Linux smoke scenarios:

1. Add a profile with `codex --agent-home` pointing at a temp Codex fixture.
2. Import a live Codex home from `CODEX_HOME`.
3. Switch to a second profile and confirm the target config is written.
4. Trigger a failed switch and confirm rollback plus failure event logging.
5. Export diagnostics and verify the archive exists.

Known V1 limits:

- No desktop notifications on Linux.
- No keychain integration beyond future work.
- No GUI shell; CLI is the only supported Linux control plane.
