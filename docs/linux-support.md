# Linux Support

AgentRelay keeps the CLI portable across macOS and Linux. The native macOS menu bar app remains macOS-only.

## Support Matrix

| Area | macOS | Linux |
| --- | --- | --- |
| `agrelay` CLI build | Supported | Supported |
| SQLite-backed profile store | Supported | Supported |
| File-backed active state and usage cache | Supported | Supported |
| Codex home discovery via `CODEX_HOME` | Supported | Supported |
| Default AgentRelay home resolution | Supported | Supported |
| Transactional switch and rollback | Supported | Supported |
| Diagnostics export | Supported | Supported |
| Native menu bar app | Supported | Not supported |
| Launch-at-login UI control | Supported | Not applicable |

## Assumptions

- Linux users should prefer explicit `AGRELAY_HOME` and `CODEX_HOME` during initial setup, testing, and smoke runs
- Linux workflows are CLI-first; there is no Linux desktop control plane in the current product scope
- secret handling on Linux remains file-reference and environment-reference oriented

## Platform Test Plan

Minimum release gate:

1. `just fmt-check`
2. `just test`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `AGRELAY_HOME=/tmp/agrelay-smoke cargo run -p agrelay-cli --bin agrelay -- doctor --json`
5. `AGRELAY_HOME=/tmp/agrelay-smoke cargo run -p agrelay-cli --bin agrelay -- status --json`

Linux smoke scenarios:

1. add a profile with `agrelay codex add --agent-home ...` pointing at a temp Codex fixture
2. import a live Codex home from `CODEX_HOME`
3. switch to a second profile and confirm the target config is written
4. trigger a failed switch and confirm rollback plus failure event logging
5. export diagnostics and verify the archive exists

## Current Limits

- no native Linux desktop notifications
- no Linux GUI shell; the CLI is the supported Linux control plane
- no platform-specific keychain integration in the Linux path
